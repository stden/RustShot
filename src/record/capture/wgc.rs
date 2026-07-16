//! Resilient Windows Graphics Capture monitor backend.

mod platform;

use self::platform::{
    capture_format_for_existing_factory, capture_format_for_monitor, create_d3d11_device,
};
use super::super::{
    copy_bgra_region_bottom_up, copy_scrgb_region_bottom_up, frame_pool_update, FramePoolUpdate,
    Region,
};
use super::{CaptureRuntimeError, GraphicsCaptureInitError};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
};
use std::time::{Duration, Instant};
use windows::core::{factory, IInspectable, Interface};
use windows::Foundation::TypedEventHandler;
use windows::Graphics::Capture::{
    Direct3D11CaptureFrame, Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession,
};
use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Graphics::SizeInt32;
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D, D3D11_BOX, D3D11_CPU_ACCESS_READ,
    D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_READ, D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_R16G16B16A16_FLOAT,
};
use windows::Win32::Graphics::Dxgi::{IDXGIDevice, IDXGIFactory1};
use windows::Win32::Graphics::Gdi::HMONITOR;
use windows::Win32::System::WinRT::Direct3D11::{
    CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess,
};
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
use windows::Win32::System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED};

const FRAME_BUFFER_COUNT: i32 = 2;
const FORMAT_CHECK_INTERVAL: Duration = Duration::from_secs(1);

/// A monitor handle detached from windows-rs's raw-pointer wrapper so it can
/// cross threads (same rationale as `SendableHwnd` in the capture backend:
/// `HMONITOR` is a process-global identifier, and storing it as `isize`
/// makes the carrier `Send` by construction).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct SendableHmonitor(isize);

impl From<HMONITOR> for SendableHmonitor {
    fn from(monitor: HMONITOR) -> Self {
        Self(monitor.0 as isize)
    }
}

impl SendableHmonitor {
    /// Wrap a raw handle value received from an external crate boundary
    /// (`screenshots` exposes monitor handles as `isize`).
    pub(super) const fn from_raw(raw: isize) -> Self {
        Self(raw)
    }

    /// The raw handle value, for comparing against `screenshots`-crate ids.
    pub(super) const fn raw(self) -> isize {
        self.0
    }

    pub(super) fn hmonitor(self) -> HMONITOR {
        HMONITOR(self.0 as *mut core::ffi::c_void)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum CaptureFormat {
    Bgra8,
    ScRgb16 { peak_luminance_nits: f32 },
}

impl CaptureFormat {
    fn pixel_format(self) -> DirectXPixelFormat {
        match self {
            Self::Bgra8 => DirectXPixelFormat::B8G8R8A8UIntNormalized,
            Self::ScRgb16 { .. } => DirectXPixelFormat::R16G16B16A16Float,
        }
    }

    fn dxgi_format(self) -> DXGI_FORMAT {
        match self {
            Self::Bgra8 => DXGI_FORMAT_B8G8R8A8_UNORM,
            Self::ScRgb16 { .. } => DXGI_FORMAT_R16G16B16A16_FLOAT,
        }
    }

    fn bytes_per_pixel(self) -> usize {
        match self {
            Self::Bgra8 => 4,
            Self::ScRgb16 { .. } => 8,
        }
    }
}

struct RoGuard;

impl RoGuard {
    fn init() -> Result<Self, GraphicsCaptureInitError> {
        // Keep the MTA alive past this thread's RoUninitialize; see
        // `capture::pin_process_mta` for the access-violation this prevents.
        crate::capture::pin_process_mta();
        unsafe { RoInitialize(RO_INIT_MULTITHREADED) }.map_err(|error| {
            GraphicsCaptureInitError::Unsupported(format!("WinRT initialization failed: {error}"))
        })?;
        Ok(Self)
    }
}

impl Drop for RoGuard {
    fn drop(&mut self) {
        unsafe { RoUninitialize() };
    }
}

pub(super) struct GraphicsCaptureMonitor {
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    winrt_device: IDirect3DDevice,
    staging: Option<ID3D11Texture2D>,
    frame_pool: Direct3D11CaptureFramePool,
    session: GraphicsCaptureSession,
    item: GraphicsCaptureItem,
    receiver: mpsc::Receiver<()>,
    _frame_handler: TypedEventHandler<Direct3D11CaptureFramePool, IInspectable>,
    _closed_handler: TypedEventHandler<GraphicsCaptureItem, IInspectable>,
    frame_token: i64,
    closed_token: i64,
    item_closed: Arc<AtomicBool>,
    dxgi_factory: IDXGIFactory1,
    monitor: HMONITOR,
    last_format_check: Instant,
    pool_size: SizeInt32,
    format: CaptureFormat,
    region: Region,
    full_monitor: bool,
    _ro: RoGuard,
}

impl GraphicsCaptureMonitor {
    pub(super) fn new(
        raw_monitor: SendableHmonitor,
        region: Region,
    ) -> Result<Self, GraphicsCaptureInitError> {
        Self::new_inner(raw_monitor, Some(region))
    }

    pub(super) fn new_full(
        raw_monitor: SendableHmonitor,
    ) -> Result<Self, GraphicsCaptureInitError> {
        Self::new_inner(raw_monitor, None)
    }

    fn new_inner(
        raw_monitor: SendableHmonitor,
        requested_region: Option<Region>,
    ) -> Result<Self, GraphicsCaptureInitError> {
        let ro = RoGuard::init()?;
        let supported = GraphicsCaptureSession::IsSupported().map_err(|error| {
            GraphicsCaptureInitError::Unsupported(format!("support check failed: {error}"))
        })?;
        if !supported {
            return Err(GraphicsCaptureInitError::Unsupported(
                "the operating system does not support Windows Graphics Capture".to_string(),
            ));
        }

        let interop =
            factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>().map_err(|error| {
                GraphicsCaptureInitError::Unsupported(format!(
                    "GraphicsCaptureItem monitor interop is unavailable: {error}"
                ))
            })?;
        let monitor = raw_monitor.hmonitor();
        if monitor == HMONITOR::default() {
            return Err(GraphicsCaptureInitError::Failed(
                "Monitor handle is null".to_string(),
            ));
        }
        let item: GraphicsCaptureItem =
            unsafe { interop.CreateForMonitor(monitor) }.map_err(|error| {
                GraphicsCaptureInitError::Failed(format!(
                    "Failed to create a capture item for the monitor: {error}"
                ))
            })?;
        let initial_size = item.Size().map_err(|error| {
            GraphicsCaptureInitError::Failed(format!(
                "Failed to read monitor capture size: {error}"
            ))
        })?;
        validate_size(initial_size).map_err(GraphicsCaptureInitError::Failed)?;
        let full_monitor = requested_region.is_none();
        let region = resolve_capture_region(requested_region, initial_size)
            .map_err(GraphicsCaptureInitError::Failed)?;

        let (dxgi_factory, format) =
            capture_format_for_monitor(monitor).map_err(GraphicsCaptureInitError::Failed)?;
        let (device, context) = create_d3d11_device().map_err(GraphicsCaptureInitError::Failed)?;
        let dxgi_device: IDXGIDevice = device.cast().map_err(|error| {
            GraphicsCaptureInitError::Failed(format!("Failed to query IDXGIDevice: {error}"))
        })?;
        let inspectable =
            unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi_device) }.map_err(|error| {
                GraphicsCaptureInitError::Failed(format!(
                    "Failed to create the WinRT D3D device: {error}"
                ))
            })?;
        let winrt_device: IDirect3DDevice = inspectable.cast().map_err(|error| {
            GraphicsCaptureInitError::Failed(format!("Failed to query IDirect3DDevice: {error}"))
        })?;

        let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
            &winrt_device,
            format.pixel_format(),
            FRAME_BUFFER_COUNT,
            initial_size,
        )
        .map_err(|error| {
            GraphicsCaptureInitError::Failed(format!(
                "Failed to create monitor capture frame pool: {error}"
            ))
        })?;
        let session = frame_pool.CreateCaptureSession(&item).map_err(|error| {
            GraphicsCaptureInitError::Failed(format!(
                "Failed to create monitor capture session: {error}"
            ))
        })?;

        session.SetIsCursorCaptureEnabled(false).map_err(|error| {
            GraphicsCaptureInitError::Unsupported(format!(
                "cursor-free monitor capture is unavailable: {error}"
            ))
        })?;

        let (sender, receiver) = mpsc::sync_channel(1);
        let frame_sender = sender.clone();
        let frame_handler =
            TypedEventHandler::<Direct3D11CaptureFramePool, IInspectable>::new(move |_, _| {
                let _ = frame_sender.try_send(());
                Ok(())
            });
        let frame_token = frame_pool.FrameArrived(&frame_handler).map_err(|error| {
            GraphicsCaptureInitError::Failed(format!(
                "Failed to register monitor frame callback: {error}"
            ))
        })?;

        let item_closed = Arc::new(AtomicBool::new(false));
        let closed_flag = item_closed.clone();
        let closed_handler =
            TypedEventHandler::<GraphicsCaptureItem, IInspectable>::new(move |_, _| {
                closed_flag.store(true, Ordering::Release);
                let _ = sender.try_send(());
                Ok(())
            });
        let closed_token = item.Closed(&closed_handler).map_err(|error| {
            GraphicsCaptureInitError::Failed(format!(
                "Failed to register monitor closed callback: {error}"
            ))
        })?;

        session.StartCapture().map_err(|error| {
            GraphicsCaptureInitError::Failed(format!("Failed to start monitor capture: {error}"))
        })?;

        if let CaptureFormat::ScRgb16 {
            peak_luminance_nits,
        } = format
        {
            log::info!(
                "Advanced Color monitor detected; recording FP16 scRGB with {peak_luminance_nits:.0}-nit HDR-to-SDR tone mapping"
            );
        }

        Ok(Self {
            device,
            context,
            winrt_device,
            staging: None,
            frame_pool,
            session,
            item,
            receiver,
            _frame_handler: frame_handler,
            _closed_handler: closed_handler,
            frame_token,
            closed_token,
            item_closed,
            dxgi_factory,
            monitor,
            last_format_check: Instant::now(),
            pool_size: initial_size,
            format,
            region,
            full_monitor,
            _ro: ro,
        })
    }

    pub(super) fn region(&self) -> Region {
        self.region
    }

    pub(super) fn capture_into(
        &mut self,
        output_width: u32,
        output_height: u32,
        output: &mut [u8],
        wait: Duration,
    ) -> Result<bool, CaptureRuntimeError> {
        if !unsafe { self.dxgi_factory.IsCurrent() }.as_bool() {
            return Err(CaptureRuntimeError::Recoverable(
                "display configuration changed".to_string(),
            ));
        }
        if self.last_format_check.elapsed() >= FORMAT_CHECK_INTERVAL {
            self.last_format_check = Instant::now();
            let current_format =
                capture_format_for_existing_factory(&self.dxgi_factory, self.monitor).map_err(
                    |error| {
                        CaptureRuntimeError::Recoverable(format!(
                            "Failed to refresh monitor color state: {error}"
                        ))
                    },
                )?;
            if capture_format_changed(self.format, current_format) {
                return Err(CaptureRuntimeError::Recoverable(
                    "Monitor Advanced Color state changed".to_string(),
                ));
            }
        }
        if self.item_closed.load(Ordering::Acquire) {
            return Err(CaptureRuntimeError::Recoverable(
                "the monitor capture target closed".to_string(),
            ));
        }

        match self.receiver.recv_timeout(wait) {
            Ok(()) => {}
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if let Err(error) = unsafe { self.device.GetDeviceRemovedReason() } {
                    return Err(CaptureRuntimeError::Recoverable(format!(
                        "D3D11 device was removed while waiting for a frame: {error}"
                    )));
                }
                return Ok(false);
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(CaptureRuntimeError::Recoverable(
                    "monitor capture callback disconnected".to_string(),
                ))
            }
        }
        while self.receiver.try_recv().is_ok() {}
        if self.item_closed.load(Ordering::Acquire) {
            return Err(CaptureRuntimeError::Recoverable(
                "the monitor capture target closed".to_string(),
            ));
        }

        let frame = self.take_latest_frame()?;
        let content_size = frame.ContentSize().map_err(|error| {
            self.recoverable_error(format!("Failed to read monitor frame size: {error}"))
        })?;
        validate_size(content_size).map_err(CaptureRuntimeError::Recoverable)?;
        if self.full_monitor
            && (content_size.Width != self.pool_size.Width
                || content_size.Height != self.pool_size.Height)
        {
            let _ = frame.Close();
            return Err(CaptureRuntimeError::Recoverable(
                "Monitor dimensions changed".to_string(),
            ));
        }
        let update = frame_pool_update(
            (self.pool_size.Width as u32, self.pool_size.Height as u32),
            (content_size.Width as u32, content_size.Height as u32),
            self.region,
        )
        .map_err(|error| {
            CaptureRuntimeError::Fatal(format!(
                "The selected recording region no longer fits after the monitor changed size: {error}"
            ))
        })?;

        let copy_result = self.copy_frame(&frame, output_width, output_height, output);
        let _ = frame.Close();
        copy_result?;

        if update == FramePoolUpdate::Recreate {
            self.recreate_pool(content_size)?;
        }
        Ok(true)
    }

    fn take_latest_frame(&self) -> Result<Direct3D11CaptureFrame, CaptureRuntimeError> {
        let mut latest = self.frame_pool.TryGetNextFrame().map_err(|error| {
            self.recoverable_error(format!("Failed to acquire monitor capture frame: {error}"))
        })?;
        while let Ok(next) = self.frame_pool.TryGetNextFrame() {
            let _ = latest.Close();
            latest = next;
        }
        Ok(latest)
    }

    fn recreate_pool(&mut self, new_size: SizeInt32) -> Result<(), CaptureRuntimeError> {
        while let Ok(frame) = self.frame_pool.TryGetNextFrame() {
            let _ = frame.Close();
        }
        self.frame_pool
            .Recreate(
                &self.winrt_device,
                self.format.pixel_format(),
                FRAME_BUFFER_COUNT,
                new_size,
            )
            .map_err(|error| {
                self.recoverable_error(format!(
                    "Failed to recreate the resized monitor frame pool: {error}"
                ))
            })?;
        self.pool_size = new_size;
        self.staging = None;
        while self.receiver.try_recv().is_ok() {}
        Ok(())
    }

    fn copy_frame(
        &mut self,
        frame: &Direct3D11CaptureFrame,
        output_width: u32,
        output_height: u32,
        output: &mut [u8],
    ) -> Result<(), CaptureRuntimeError> {
        let surface = frame.Surface().map_err(|error| {
            self.recoverable_error(format!("Failed to access monitor capture surface: {error}"))
        })?;
        let access: IDirect3DDxgiInterfaceAccess = surface.cast().map_err(|error| {
            self.recoverable_error(format!("Failed to access monitor DXGI surface: {error}"))
        })?;
        let texture: ID3D11Texture2D = unsafe { access.GetInterface() }.map_err(|error| {
            self.recoverable_error(format!("Failed to query monitor D3D11 texture: {error}"))
        })?;

        let mut description = D3D11_TEXTURE2D_DESC::default();
        unsafe { texture.GetDesc(&mut description) };
        if description.Format != self.format.dxgi_format() {
            return Err(CaptureRuntimeError::Recoverable(format!(
                "Monitor capture texture has unexpected DXGI format {:?}",
                description.Format
            )));
        }
        super::super::validate_capture_region(self.region, description.Width, description.Height)
            .map_err(CaptureRuntimeError::Fatal)?;
        self.ensure_staging_texture(description)?;
        let staging = self
            .staging
            .as_ref()
            .expect("staging texture must exist")
            .clone();
        let source_box = D3D11_BOX {
            left: self.region.physical_x as u32,
            top: self.region.physical_y as u32,
            front: 0,
            right: self.region.physical_x as u32 + self.region.physical_width,
            bottom: self.region.physical_y as u32 + self.region.physical_height,
            back: 1,
        };
        unsafe {
            self.context
                .CopySubresourceRegion(&staging, 0, 0, 0, 0, &texture, 0, Some(&source_box))
        };

        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        unsafe {
            self.context
                .Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
        }
        .map_err(|error| {
            self.recoverable_error(format!("Failed to map monitor staging texture: {error}"))
        })?;
        let copy_result = (|| {
            if mapped.pData.is_null() {
                return Err(CaptureRuntimeError::Recoverable(
                    "Mapped monitor texture returned a null data pointer".to_string(),
                ));
            }
            let row_pitch = mapped.RowPitch as usize;
            let row_bytes = (self.region.physical_width as usize)
                .checked_mul(self.format.bytes_per_pixel())
                .ok_or_else(|| {
                    CaptureRuntimeError::Fatal("Mapped monitor row is too large".to_string())
                })?;
            if row_pitch < row_bytes {
                return Err(CaptureRuntimeError::Recoverable(format!(
                    "Mapped monitor row pitch {row_pitch} is smaller than {row_bytes} bytes"
                )));
            }
            let mapped_len = row_pitch
                .checked_mul(self.region.physical_height.saturating_sub(1) as usize)
                .and_then(|offset| offset.checked_add(row_bytes))
                .ok_or_else(|| {
                    CaptureRuntimeError::Fatal("Mapped monitor frame is too large".to_string())
                })?;
            let bytes =
                unsafe { std::slice::from_raw_parts(mapped.pData.cast::<u8>(), mapped_len) };
            let local_region = Region {
                physical_x: 0,
                physical_y: 0,
                physical_width: self.region.physical_width,
                physical_height: self.region.physical_height,
            };
            match self.format {
                CaptureFormat::Bgra8 => copy_bgra_region_bottom_up(
                    bytes,
                    row_pitch,
                    self.region.physical_width,
                    self.region.physical_height,
                    local_region,
                    output_width,
                    output_height,
                    output,
                ),
                CaptureFormat::ScRgb16 {
                    peak_luminance_nits,
                } => copy_scrgb_region_bottom_up(
                    bytes,
                    row_pitch,
                    self.region.physical_width,
                    self.region.physical_height,
                    local_region,
                    output_width,
                    output_height,
                    peak_luminance_nits,
                    output,
                ),
            }
            .map_err(CaptureRuntimeError::Fatal)
        })();
        unsafe { self.context.Unmap(&staging, 0) };
        copy_result
    }

    fn ensure_staging_texture(
        &mut self,
        mut description: D3D11_TEXTURE2D_DESC,
    ) -> Result<(), CaptureRuntimeError> {
        if self.staging.is_some() {
            return Ok(());
        }

        description.Width = self.region.physical_width;
        description.Height = self.region.physical_height;
        description.MipLevels = 1;
        description.ArraySize = 1;
        description.Usage = D3D11_USAGE_STAGING;
        description.BindFlags = 0;
        description.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
        description.MiscFlags = 0;
        let mut staging = None;
        unsafe {
            self.device
                .CreateTexture2D(&description, None, Some(&mut staging))
        }
        .map_err(|error| {
            self.recoverable_error(format!("Failed to create monitor staging texture: {error}"))
        })?;
        self.staging = Some(staging.ok_or_else(|| {
            CaptureRuntimeError::Recoverable(
                "D3D11 returned no monitor staging texture".to_string(),
            )
        })?);
        Ok(())
    }

    fn recoverable_error(&self, context: String) -> CaptureRuntimeError {
        match unsafe { self.device.GetDeviceRemovedReason() } {
            Ok(()) => CaptureRuntimeError::Recoverable(context),
            Err(device_error) => CaptureRuntimeError::Recoverable(format!(
                "{context}; D3D11 device removal reason: {device_error}"
            )),
        }
    }
}

impl Drop for GraphicsCaptureMonitor {
    fn drop(&mut self) {
        let _ = self.frame_pool.RemoveFrameArrived(self.frame_token);
        let _ = self.item.RemoveClosed(self.closed_token);
        let _ = self.session.Close();
        let _ = self.frame_pool.Close();
    }
}

fn validate_size(size: SizeInt32) -> Result<(), String> {
    if size.Width <= 0 || size.Height <= 0 {
        Err("Monitor capture item has zero or negative size".to_string())
    } else {
        Ok(())
    }
}

fn resolve_capture_region(
    requested_region: Option<Region>,
    item_size: SizeInt32,
) -> Result<Region, String> {
    validate_size(item_size)?;
    let region = requested_region.unwrap_or(Region {
        physical_x: 0,
        physical_y: 0,
        physical_width: item_size.Width as u32,
        physical_height: item_size.Height as u32,
    });
    super::super::validate_capture_region(region, item_size.Width as u32, item_size.Height as u32)?;
    Ok(region)
}

fn capture_format_changed(current: CaptureFormat, observed: CaptureFormat) -> bool {
    match (current, observed) {
        (CaptureFormat::Bgra8, CaptureFormat::Bgra8) => false,
        (
            CaptureFormat::ScRgb16 {
                peak_luminance_nits: current_peak,
            },
            CaptureFormat::ScRgb16 {
                peak_luminance_nits: observed_peak,
            },
        ) => (current_peak - observed_peak).abs() >= 1.0,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_monitor_region_comes_from_graphics_capture_item_size() {
        let region = resolve_capture_region(
            None,
            SizeInt32 {
                Width: 2880,
                Height: 1800,
            },
        )
        .unwrap();

        assert_eq!(
            region,
            Region {
                physical_x: 0,
                physical_y: 0,
                physical_width: 2880,
                physical_height: 1800,
            }
        );
    }

    #[test]
    fn hdr_format_transition_requires_source_rebuild() {
        assert!(!capture_format_changed(
            CaptureFormat::Bgra8,
            CaptureFormat::Bgra8
        ));
        assert!(capture_format_changed(
            CaptureFormat::Bgra8,
            CaptureFormat::ScRgb16 {
                peak_luminance_nits: 1_000.0,
            }
        ));
        assert!(capture_format_changed(
            CaptureFormat::ScRgb16 {
                peak_luminance_nits: 1_000.0,
            },
            CaptureFormat::ScRgb16 {
                peak_luminance_nits: 600.0,
            }
        ));
    }
}
