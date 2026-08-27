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
        // SAFETY: `RoInitialize` takes no pointers. Its contract is that every
        // successful call is balanced by exactly one `RoUninitialize` on the SAME
        // thread, and that the requested apartment agrees with any this thread
        // already joined. Balancing it is `RoGuard`'s entire purpose, and the `?`
        // below means the failure path produces no guard - so no unpaired
        // `RoUninitialize`. A disagreeing apartment is reported as
        // `RPC_E_CHANGED_MODE`, not undefined behaviour, so it lands in the
        // `map_err` like any other failure.
        unsafe { RoInitialize(RO_INIT_MULTITHREADED) }.map_err(|error| {
            GraphicsCaptureInitError::Unsupported(format!("WinRT initialization failed: {error}"))
        })?;
        Ok(Self)
    }
}

impl Drop for RoGuard {
    fn drop(&mut self) {
        // SAFETY: paired one-to-one with the successful `RoInitialize` in `init`,
        // which is the only constructor of `RoGuard`, and `drop` runs at most once
        // per value. It also runs on the initializing thread, as `RoUninitialize`
        // requires — and exactly one field buys that: the guard is only ever stored
        // in `GraphicsCaptureMonitor::_ro`, and that struct holds
        // `monitor: HMONITOR`, a bare `*mut c_void`. windows-rs DOES give `Send` to
        // the COM and WinRT interfaces here (`ID3D11Device`, `IDXGIFactory1`,
        // `GraphicsCaptureItem` and the rest all carry `unsafe impl Send`), so they
        // contribute nothing; the raw handle is the only thing keeping the struct,
        // and therefore this guard, on the thread that built it.
        //
        // So do NOT store the monitor as this module's own `SendableHmonitor`
        // carrier, whose entire purpose is to be `Send`. That single change would
        // make `GraphicsCaptureMonitor` `Send`, let the guard drop on a thread that
        // never called `RoInitialize`, and decrement that thread's apartment count
        // instead of its own. If the field ever has to move, give `RoGuard` an
        // explicit `PhantomData<*const ()>` first so the affinity stops depending
        // on a neighbouring field's type.
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
        // SAFETY: `interop` is a live, ref-counted `IGraphicsCaptureItemInterop`
        // handed to us by the activation factory above, so its vtable pointer is
        // valid for this call, and WinRT is initialized on this thread by `ro`,
        // which the interop factory requires. `monitor` is passed by value and was
        // rejected as null immediately above. windows-rs queries the result by
        // `GraphicsCaptureItem`'s IID, so a type mismatch would fail with
        // `E_NOINTERFACE` rather than mis-type the returned pointer.
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
        // SAFETY: `dxgi_device` is a live `IDXGIDevice` obtained by `cast()` from
        // the D3D11 device created two statements above, and the borrow lasts for
        // the whole call. The out-parameter is a local that windows-rs fills with an
        // owned `IInspectable`. This produces a WinRT object, and the thread is in
        // the multithreaded apartment for the whole of `new_inner` thanks to `ro`.
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
        // SAFETY: `self.dxgi_factory` is the factory created in `new_inner` and kept
        // alive by this struct for as long as `&mut self` exists, so its vtable
        // pointer is valid. `IsCurrent` takes no arguments and only compares the
        // factory's cached adapter enumeration against the current one.
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
                // SAFETY: `self.device` is the D3D11 device owned by this struct, so
                // it is alive here. `GetDeviceRemovedReason` takes no arguments and
                // is explicitly still callable on a removed device - reporting that
                // is what it is for - so a lost GPU cannot make this call invalid.
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
        // SAFETY: `access` is a live `IDirect3DDxgiInterfaceAccess` cast from
        // `surface`, which the local binding keeps alive across this call.
        // `GetInterface` is a `QueryInterface` under an IID windows-rs derives from
        // the requested type, so asking for `ID3D11Texture2D` either yields a real
        // texture reference or fails with `E_NOINTERFACE`.
        let texture: ID3D11Texture2D = unsafe { access.GetInterface() }.map_err(|error| {
            self.recoverable_error(format!("Failed to query monitor D3D11 texture: {error}"))
        })?;

        let mut description = D3D11_TEXTURE2D_DESC::default();
        // SAFETY: `texture` is the live texture just acquired, and `description` is a
        // stack `D3D11_TEXTURE2D_DESC` that is already fully initialized by
        // `default()`, so the write-only out-pointer is valid and no field is left
        // uninitialized regardless of what the runtime fills in.
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
        // SAFETY: `self.context`, `staging` and `texture` are live COM references
        // held by `self` or by locals for the whole call, and `source_box` is the only
        // pointer - it points at the stack `D3D11_BOX` above. Subresource index 0
        // exists on any 2D texture, and for `staging` it is the *only* one, since
        // `ensure_staging_texture` sets `MipLevels = ArraySize = 1`. The box is in
        // range: `validate_capture_region` just proved the region's origin is
        // non-negative - which is what makes the `as u32` casts faithful - and that
        // its right/bottom edges lie within `description.Width` and
        // `description.Height`, i.e. within `texture`. The destination fits because
        // `staging` was sized to exactly the region.
        unsafe {
            self.context
                .CopySubresourceRegion(&staging, 0, 0, 0, 0, &texture, 0, Some(&source_box))
        };

        // A blocking map, on purpose: no D3D11_MAP_FLAG_DO_NOT_WAIT and one
        // staging texture, not two. `Map` waits for the copy above to retire,
        // which costs one region readback (8 MB at 1080p, 33 MB at 4K) on the
        // capture thread. The textbook remedy - copy into texture N while
        // mapping N-1 - must not be applied here: this same call also serves
        // single screenshots through `capture_monitor_image`, whose worker keeps
        // a live session cached for `SOURCE_IDLE_TIMEOUT` and returns the first
        // frame that reports `true`. One frame of latency would hand the
        // selection overlay, `--screenshot` and the MCP tools the *previous*
        // capture - pixels up to five seconds old - and `select_region` crops
        // that buffer instead of re-capturing, so the staleness reaches the
        // saved file. Priming the first frame does not help; the warm-session
        // repeat call is the broken one. Pipeline this only behind an explicit
        // per-session flag that leaves the screenshot path synchronous, and
        // only once a measurement shows the wait actually costs frames: the
        // recording loop sleeps to a wall-clock deadline anyway, so today the
        // wait is paid out of time the thread would have spent idle.
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        // SAFETY: `self.context` and `staging` are live. `D3D11_MAP_READ` is a legal
        // map type for this resource because `ensure_staging_texture` created it with
        // `D3D11_USAGE_STAGING` and `D3D11_CPU_ACCESS_READ`; subresource 0 is its only
        // subresource. `mapped` is a stack out-parameter, already initialized, valid
        // for the call. On success the mapping is released by exactly one `Unmap`
        // below: the closure's result is bound to `copy_result` first, so every exit
        // from it - including the early `return`s - reaches that `Unmap`.
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
            // SAFETY: `Map` returned `Ok`, so `pData` addresses a readable CPU mapping
            // of `staging` that stays valid until the `Unmap` below, and `bytes` dies
            // with this closure, before that. The pointer was null-checked above, and
            // `u8` needs no alignment. Nothing else may touch the mapping meanwhile:
            // `Map` gives the CPU exclusive access to the resource, and `staging` is
            // reachable only through `&mut self`. The length stays inside the mapping,
            // which spans `RowPitch * Height` bytes: `Height` is
            // `region.physical_height` (that is how `ensure_staging_texture` sized it,
            // and a zero height was rejected by `validate_capture_region`), and
            // `mapped_len` is `RowPitch * (height - 1) + row_bytes` with
            // `row_bytes <= RowPitch` checked above - so the final row is read no
            // further than `row_bytes` into itself. `row_bytes` uses
            // `bytes_per_pixel`, which matches the texture's real pixel size because
            // `description.Format` was compared against `self.format.dxgi_format()`
            // before the staging texture was created from that same description. Both
            // products are `checked_*`, so an implausibly large region yields a `Fatal`
            // error rather than a wrapped-around short length.
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
        // SAFETY: paired with the `Map` above, which returned `Ok` - the `?` on it
        // skips this line otherwise - on the same context, resource and subresource.
        // The closure has already returned, so `bytes` and every reference into the
        // mapping are gone before it is torn down.
        unsafe { self.context.Unmap(&staging, 0) };
        copy_result
    }

    /// Create the staging texture on the first frame and keep it afterwards.
    ///
    /// Caching it across frames is only sound because `self.region` is fixed for
    /// the lifetime of this object - it is set in `new_inner` and never
    /// reassigned - so the cached texture can never be the wrong size for the
    /// copy. `recreate_pool` still clears it, so that after a monitor resize the
    /// next frame re-derives `Format` and `SampleDesc` from the new pool's
    /// textures rather than from the retired ones.
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
        // SAFETY: `self.device` is the device owned by this struct. `description` is
        // the fully initialized stack struct above, borrowed read-only for the call.
        // `None` for the initial data is what a staging texture wants - there is
        // nothing to upload - and `staging` is a stack `Option` that windows-rs fills
        // with an owned reference; it starts as `None`, so nothing is overwritten
        // without being dropped, and the `None` case is handled below rather than
        // assumed away.
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
        // SAFETY: `self.device` is the D3D11 device owned by this struct, so it is
        // alive for the `&self` borrow. `GetDeviceRemovedReason` takes no arguments
        // and is designed to be called on a device that has already been removed,
        // which is precisely the case this helper exists to describe.
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
    fn a_capture_item_with_no_pixels_is_rejected() {
        // A monitor that is being torn down reports a zero-sized capture item.
        // Accepting it would size the staging texture and the frame buffer to
        // nothing, and the failure would surface much later as a copy error.
        for size in [
            SizeInt32 {
                Width: 0,
                Height: 1080,
            },
            SizeInt32 {
                Width: 1920,
                Height: 0,
            },
            SizeInt32 {
                Width: -1920,
                Height: -1080,
            },
        ] {
            assert_eq!(
                resolve_capture_region(None, size),
                Err("Monitor capture item has zero or negative size".to_string()),
                "accepted {}x{}",
                size.Width,
                size.Height
            );
        }
    }

    #[test]
    fn a_requested_region_outside_the_capture_item_is_rejected() {
        // The region comes from a selection made against an earlier monitor
        // layout, so it can be stale by the time the capture item exists.
        let error = resolve_capture_region(
            Some(Region {
                physical_x: 0,
                physical_y: 0,
                physical_width: 4000,
                physical_height: 1800,
            }),
            SizeInt32 {
                Width: 2880,
                Height: 1800,
            },
        )
        .expect_err("a region wider than the monitor cannot be captured");
        assert!(!error.is_empty(), "the rejection must be explained");
    }

    #[test]
    fn each_capture_format_describes_itself_consistently() {
        // The staging texture is allocated from `bytes_per_pixel` and filled by
        // the GPU according to `dxgi_format`. If the two ever disagreed, an HDR
        // monitor would read half a frame past the end of its buffer.
        assert_eq!(CaptureFormat::Bgra8.bytes_per_pixel(), 4);
        assert_eq!(
            CaptureFormat::ScRgb16 {
                peak_luminance_nits: 1_000.0,
            }
            .bytes_per_pixel(),
            8
        );

        let sdr = CaptureFormat::Bgra8;
        let hdr = CaptureFormat::ScRgb16 {
            peak_luminance_nits: 1_000.0,
        };
        assert_ne!(sdr.pixel_format(), hdr.pixel_format());
        assert_ne!(sdr.dxgi_format(), hdr.dxgi_format());
        // The peak luminance describes the content, not the layout: two HDR
        // formats that differ only in brightness share one texture format.
        assert_eq!(
            hdr.dxgi_format(),
            CaptureFormat::ScRgb16 {
                peak_luminance_nits: 600.0,
            }
            .dxgi_format()
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
        // Below the one-nit threshold nothing is rebuilt: the reported peak
        // luminance jitters slightly between frames, and treating that as a
        // format change would tear down and recreate the capture session over
        // and over for the whole recording.
        assert!(!capture_format_changed(
            CaptureFormat::ScRgb16 {
                peak_luminance_nits: 1_000.0,
            },
            CaptureFormat::ScRgb16 {
                peak_luminance_nits: 1_000.4,
            }
        ));
    }
}
