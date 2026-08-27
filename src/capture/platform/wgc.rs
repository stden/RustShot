//! One-shot window capture through `Windows.Graphics.Capture`.

use super::rgba_from_bgra_rows;
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
use windows::Win32::Foundation::{HMODULE, HWND};
use windows::Win32::Graphics::Direct3D::{
    D3D_DRIVER_TYPE, D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP,
};
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D, D3D11_CPU_ACCESS_READ,
    D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_READ, D3D11_SDK_VERSION,
    D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Dxgi::IDXGIDevice;
use windows::Win32::System::WinRT::Direct3D11::{
    CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess,
};
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
use windows::Win32::System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED};

const FRAME_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Debug)]
pub(super) enum WgcCaptureError {
    Unsupported(String),
    Failed(String),
}

impl From<String> for WgcCaptureError {
    fn from(error: String) -> Self {
        Self::Failed(error)
    }
}

struct RoApartment;

impl Drop for RoApartment {
    fn drop(&mut self) {
        // SAFETY: pairs the successful `RoInitialize` in `capture_hwnd`, which is
        // the only place this guard is constructed and does so on the same thread
        // immediately after that call returns `Ok`; the error path returns before
        // the guard exists, so the counts stay balanced. The guard is neither
        // `Clone` nor `Copy`, so this runs exactly once. Any WinRT object still
        // referenced elsewhere survives because `pin_process_mta` holds the MTA
        // open independently of this thread.
        unsafe { RoUninitialize() };
    }
}

pub(super) fn capture_hwnd(hwnd: HWND) -> Result<image::RgbaImage, WgcCaptureError> {
    // Keep the MTA alive past this worker thread's RoUninitialize; see
    // `pin_process_mta` for the access-violation this prevents.
    super::pin_process_mta();
    // SAFETY: no pointers are passed. This runs on the freshly spawned
    // `rustshot-window-capture` worker (the sole caller of this function), which
    // has entered no other apartment, so requesting the MTA cannot conflict; if it
    // somehow does, the HRESULT is mapped to `Unsupported` below and the
    // `RoApartment` guard is never created, so no unbalanced `RoUninitialize`
    // happens.
    unsafe { RoInitialize(RO_INIT_MULTITHREADED) }.map_err(|error| {
        WgcCaptureError::Unsupported(format!("WinRT initialization failed: {error}"))
    })?;
    let _apartment = RoApartment;

    let supported = GraphicsCaptureSession::IsSupported()
        .map_err(|error| WgcCaptureError::Unsupported(format!("support check failed: {error}")))?;
    if !supported {
        return Err(WgcCaptureError::Unsupported(
            "the operating system does not support Windows Graphics Capture".to_string(),
        ));
    }

    // `IsSupported` was introduced before direct HWND targeting. On Windows
    // 10 1803/1809 the support check can succeed while this interop factory is
    // absent, which must select the legacy fallback rather than fail capture.
    let interop =
        factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>().map_err(|error| {
            WgcCaptureError::Unsupported(format!(
                "GraphicsCaptureItem HWND interop is unavailable: {error}"
            ))
        })?;

    capture_supported_hwnd(hwnd, &interop)
}

fn capture_supported_hwnd(
    hwnd: HWND,
    interop: &IGraphicsCaptureItemInterop,
) -> Result<image::RgbaImage, WgcCaptureError> {
    // SAFETY: `interop` is the live `GraphicsCaptureItem` activation factory that
    // `capture_hwnd` obtained on this thread after `RoInitialize` succeeded, so
    // WinRT is still initialized for the duration of the call. `hwnd` is passed by
    // value and only read by the compositor; a window that is gone or not
    // capturable is reported as a failing HRESULT. windows-rs supplies the IID and
    // owns the returned interface pointer, so the out-parameter is not ours.
    let item: GraphicsCaptureItem = unsafe { interop.CreateForWindow(hwnd) }
        .map_err(|error| format!("Failed to create a capture item for the window: {error}"))?;
    let initial_size = item
        .Size()
        .map_err(|error| format!("Failed to read capture item size: {error}"))?;
    if initial_size.Width <= 0 || initial_size.Height <= 0 {
        return Err(WgcCaptureError::Failed(
            "Capture item has zero or negative size".to_string(),
        ));
    }

    let (native_device, context) = create_d3d11_device()?;
    let dxgi_device: IDXGIDevice = native_device
        .cast()
        .map_err(|error| format!("Failed to query IDXGIDevice: {error}"))?;
    // SAFETY: `dxgi_device` is a live `IDXGIDevice` obtained by `cast()` from the
    // D3D11 device created immediately above, so the borrow windows-rs turns into
    // the raw interface pointer is valid for the whole call; the callee only
    // AddRefs it. WinRT is initialized on this thread by `capture_hwnd`, which is
    // this function's only caller. The returned `IInspectable` is owned by
    // windows-rs.
    let inspectable = unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi_device) }
        .map_err(|error| format!("Failed to create the WinRT D3D device: {error}"))?;
    let winrt_device: IDirect3DDevice = inspectable
        .cast()
        .map_err(|error| format!("Failed to query IDirect3DDevice: {error}"))?;

    let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
        &winrt_device,
        DirectXPixelFormat::B8G8R8A8UIntNormalized,
        2,
        initial_size,
    )
    .map_err(|error| format!("Failed to create capture frame pool: {error}"))?;
    let session = frame_pool
        .CreateCaptureSession(&item)
        .map_err(|error| format!("Failed to create capture session: {error}"))?;

    // Match the established screenshot behavior: never include the pointer.
    // Windows 10 1903 lacks this session interface, so use the cursor-free GDI
    // backend there instead of silently returning a different image.
    session.SetIsCursorCaptureEnabled(false).map_err(|error| {
        WgcCaptureError::Unsupported(format!(
            "cursor-free window capture is unavailable: {error}"
        ))
    })?;

    let (sender, receiver) = mpsc::sync_channel(1);
    let frame_sender = sender.clone();
    let handler =
        TypedEventHandler::<Direct3D11CaptureFramePool, IInspectable>::new(move |_, _| {
            let _ = frame_sender.try_send(());
            Ok(())
        });
    let token = frame_pool
        .FrameArrived(&handler)
        .map_err(|error| format!("Failed to register frame callback: {error}"))?;

    let item_closed = Arc::new(AtomicBool::new(false));
    let closed_flag = item_closed.clone();
    let closed_handler =
        TypedEventHandler::<GraphicsCaptureItem, IInspectable>::new(move |_, _| {
            closed_flag.store(true, Ordering::Release);
            let _ = sender.try_send(());
            Ok(())
        });
    let closed_token = item
        .Closed(&closed_handler)
        .map_err(|error| format!("Failed to register window closed callback: {error}"))?;

    let result = (|| -> Result<image::RgbaImage, WgcCaptureError> {
        session
            .StartCapture()
            .map_err(|error| format!("Failed to start window capture: {error}"))?;
        let deadline = Instant::now() + FRAME_TIMEOUT;
        let mut pool_size = initial_size;

        loop {
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .ok_or_else(|| {
                    WgcCaptureError::Failed(
                        "Timed out waiting for a stable window capture frame".to_string(),
                    )
                })?;
            receiver
                .recv_timeout(remaining)
                .map_err(|error| match error {
                    mpsc::RecvTimeoutError::Timeout => WgcCaptureError::Failed(
                        "Timed out waiting for a window capture frame".to_string(),
                    ),
                    mpsc::RecvTimeoutError::Disconnected => WgcCaptureError::Failed(
                        "Window capture frame callback disconnected".to_string(),
                    ),
                })?;
            while receiver.try_recv().is_ok() {}
            if item_closed.load(Ordering::Acquire) {
                return Err(WgcCaptureError::Failed(
                    "Window closed before capture completed".to_string(),
                ));
            }

            let mut frame = frame_pool
                .TryGetNextFrame()
                .map_err(|error| format!("Failed to acquire window capture frame: {error}"))?;
            while let Ok(next) = frame_pool.TryGetNextFrame() {
                let _ = frame.Close();
                frame = next;
            }
            let content_size = frame
                .ContentSize()
                .map_err(|error| format!("Failed to read window capture size: {error}"))?;
            if content_size.Width <= 0 || content_size.Height <= 0 {
                let _ = frame.Close();
                return Err(WgcCaptureError::Failed(
                    "Window capture frame has zero or negative size".to_string(),
                ));
            }

            if content_size.Width != pool_size.Width || content_size.Height != pool_size.Height {
                let _ = frame.Close();
                while let Ok(pending) = frame_pool.TryGetNextFrame() {
                    let _ = pending.Close();
                }
                frame_pool
                    .Recreate(
                        &winrt_device,
                        DirectXPixelFormat::B8G8R8A8UIntNormalized,
                        2,
                        content_size,
                    )
                    .map_err(|error| {
                        format!("Failed to recreate resized window frame pool: {error}")
                    })?;
                pool_size = content_size;
                continue;
            }

            let image = read_frame(&frame, &native_device, &context);
            let _ = frame.Close();
            return image.map_err(WgcCaptureError::Failed);
        }
    })();

    let _ = frame_pool.RemoveFrameArrived(token);
    let _ = item.RemoveClosed(closed_token);
    let _ = session.Close();
    let _ = frame_pool.Close();
    result
}

fn create_d3d11_device() -> Result<(ID3D11Device, ID3D11DeviceContext), String> {
    match create_d3d11_device_with_driver(D3D_DRIVER_TYPE_HARDWARE) {
        Ok(device) => Ok(device),
        Err(hardware_error) => {
            create_d3d11_device_with_driver(D3D_DRIVER_TYPE_WARP).map_err(|warp_error| {
                format!(
                    "Failed to create D3D11 device (hardware: {hardware_error}; WARP: {warp_error})"
                )
            })
        }
    }
}

fn create_d3d11_device_with_driver(
    driver_type: D3D_DRIVER_TYPE,
) -> Result<(ID3D11Device, ID3D11DeviceContext), String> {
    let mut device = None;
    let mut context = None;
    // SAFETY: the adapter, feature-level list and feature-level out-parameter are
    // all `None`, so the only pointers reaching D3D11 are the two out-parameters,
    // which borrow the live `device`/`context` locals for the call and are written
    // before it returns. `HMODULE::default()` (a null software rasterizer) is what
    // the API requires for every driver type except `D3D_DRIVER_TYPE_SOFTWARE`,
    // and `create_d3d11_device` only ever passes HARDWARE or WARP.
    unsafe {
        D3D11CreateDevice(
            None,
            driver_type,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            None,
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            Some(&mut context),
        )
    }
    .map_err(|error| error.to_string())?;

    let device = device.ok_or_else(|| "D3D11 returned no device".to_string())?;
    let context = context.ok_or_else(|| "D3D11 returned no immediate context".to_string())?;
    Ok((device, context))
}

fn read_frame(
    frame: &Direct3D11CaptureFrame,
    device: &ID3D11Device,
    context: &ID3D11DeviceContext,
) -> Result<image::RgbaImage, String> {
    let content_size = frame
        .ContentSize()
        .map_err(|error| format!("Failed to read frame content size: {error}"))?;
    if content_size.Width <= 0 || content_size.Height <= 0 {
        return Err("Captured frame has zero or negative size".to_string());
    }
    let width = content_size.Width as u32;
    let height = content_size.Height as u32;

    let surface = frame
        .Surface()
        .map_err(|error| format!("Failed to access capture surface: {error}"))?;
    let access: IDirect3DDxgiInterfaceAccess = surface
        .cast()
        .map_err(|error| format!("Failed to access capture DXGI surface: {error}"))?;
    // SAFETY: `access` is a live `IDirect3DDxgiInterfaceAccess` cast from the
    // frame's surface, which `frame` keeps alive for this whole function. The
    // requested IID is derived by windows-rs from the `ID3D11Texture2D` annotation,
    // so the returned pointer really is that interface (or the call fails and we
    // return `Err`) — it is not a blind reinterpretation.
    let texture: ID3D11Texture2D = unsafe { access.GetInterface() }
        .map_err(|error| format!("Failed to query captured D3D11 texture: {error}"))?;

    let mut description = D3D11_TEXTURE2D_DESC::default();
    // SAFETY: `texture` is the interface the successful `GetInterface` above just
    // handed us, and `GetDesc` writes only the single `D3D11_TEXTURE2D_DESC` that
    // `&mut description` borrows from the stack for the duration of the call.
    unsafe { texture.GetDesc(&mut description) };
    if description.Format != DXGI_FORMAT_B8G8R8A8_UNORM {
        return Err(format!(
            "Window capture texture has unexpected DXGI format {:?}",
            description.Format
        ));
    }
    if width > description.Width || height > description.Height {
        return Err(format!(
            "Window resized during capture: content is {width}x{height}, surface is {}x{}",
            description.Width, description.Height
        ));
    }

    let mut staging_description = description;
    staging_description.Usage = D3D11_USAGE_STAGING;
    staging_description.BindFlags = 0;
    staging_description.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
    staging_description.MiscFlags = 0;

    let mut staging = None;
    // SAFETY: `&staging_description` borrows a fully initialized desc (a copy of
    // the source texture's own, with only usage/CPU-access/bind/misc flags
    // rewritten) and `Some(&mut staging)` borrows a live local out-parameter; both
    // borrows last for the call. `None` initial data is correct here because a
    // staging texture is filled by the `CopyResource` below rather than at
    // creation. `device` is the D3D11 device the WGC surface was created on.
    unsafe { device.CreateTexture2D(&staging_description, None, Some(&mut staging)) }
        .map_err(|error| format!("Failed to create staging texture: {error}"))?;
    let staging = staging.ok_or_else(|| "D3D11 returned no staging texture".to_string())?;
    // SAFETY: `context` is `device`'s immediate context and both resources belong to
    // that same `device`, so they are copy-compatible. `staging_description` was
    // cloned from `description`, so the two textures agree on type, dimensions, mip
    // count, sample count and format — exactly what `CopyResource` requires — and
    // neither is currently mapped. Both interfaces are live locals.
    unsafe { context.CopyResource(&staging, &texture) };

    let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
    // SAFETY: `staging` was created with `D3D11_USAGE_STAGING` and
    // `D3D11_CPU_ACCESS_READ`, so `D3D11_MAP_READ` on subresource 0 — which always
    // exists — is a legal mapping; the preceding `CopyResource` on this same
    // immediate context is the only write to it and is ordered before this call.
    // `Some(&mut mapped)` borrows a live local out-parameter.
    unsafe { context.Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped)) }
        .map_err(|error| format!("Failed to map staging texture: {error}"))?;

    let read_result = (|| {
        if mapped.pData.is_null() {
            return Err("Mapped staging texture returned a null data pointer".to_string());
        }
        let row_pitch = mapped.RowPitch as usize;
        let row_bytes = (width as usize)
            .checked_mul(4)
            .ok_or_else(|| "Mapped frame row is too large".to_string())?;
        if row_pitch < row_bytes {
            return Err(format!(
                "Mapped row pitch {row_pitch} is smaller than the {row_bytes}-byte content row"
            ));
        }
        let mapped_len = row_pitch
            .checked_mul(height.saturating_sub(1) as usize)
            .and_then(|offset| offset.checked_add(row_bytes))
            .ok_or_else(|| "Mapped frame buffer is too large".to_string())?;
        // SAFETY: the `Map` above succeeded, so `pData` addresses the mapped
        // subresource for `RowPitch * Height` bytes and stays valid until the
        // `Unmap` that runs after this closure returns. It was null-checked just
        // above, `mapped_len` is `RowPitch * (height - 1) + width * 4` computed with
        // checked arithmetic, and `width`/`height` were verified not to exceed the
        // texture's own `Width`/`Height`, so the slice cannot leave the mapping.
        // Alignment is trivially satisfied for `u8`, and nothing else can touch the
        // staging texture while the borrow lives.
        let mapped_bytes =
            unsafe { std::slice::from_raw_parts(mapped.pData.cast::<u8>(), mapped_len) };
        let rgba = rgba_from_bgra_rows(mapped_bytes, row_pitch, width, height)?;
        image::RgbaImage::from_raw(width, height, rgba)
            .ok_or_else(|| "Failed to construct image from captured frame".to_string())
    })();

    // SAFETY: pairs the successful `Map` on the same context, texture and
    // subresource; the closure above has returned, so `mapped_bytes` is gone and no
    // borrow of the mapping outlives this call. Reached unconditionally, so the
    // mapping is released even when the closure returned an error.
    unsafe { context.Unmap(&staging, 0) };
    read_result
}
