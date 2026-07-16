//! One-shot window capture through `Windows.Graphics.Capture`.

use super::rgba_from_bgra_rows;
use std::sync::mpsc;
use std::time::Duration;
use windows::core::{factory, ComInterface, IInspectable};
use windows::Foundation::TypedEventHandler;
use windows::Graphics::Capture::{
    Direct3D11CaptureFrame, Direct3D11CaptureFramePool, GraphicsCaptureItem,
    GraphicsCaptureSession,
};
use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Win32::Foundation::{HMODULE, HWND};
use windows::Win32::Graphics::Direct3D::{
    D3D_DRIVER_TYPE, D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP,
};
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D,
    D3D11_CPU_ACCESS_READ, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_MAPPED_SUBRESOURCE,
    D3D11_MAP_READ, D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING,
};
use windows::Win32::Graphics::Dxgi::IDXGIDevice;
use windows::Win32::System::WinRT::Direct3D11::{
    CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess,
};
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
use windows::Win32::System::WinRT::{
    RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED,
};

const FRAME_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Debug)]
pub(super) enum WgcCaptureError {
    Unsupported(String),
    Failed(String),
}

struct RoApartment;

impl Drop for RoApartment {
    fn drop(&mut self) {
        unsafe { RoUninitialize() };
    }
}

pub(super) fn capture_hwnd(hwnd: HWND) -> Result<image::RgbaImage, WgcCaptureError> {
    unsafe { RoInitialize(RO_INIT_MULTITHREADED) }.map_err(|error| {
        WgcCaptureError::Unsupported(format!("WinRT initialization failed: {error}"))
    })?;
    let _apartment = RoApartment;

    let supported = GraphicsCaptureSession::IsSupported().map_err(|error| {
        WgcCaptureError::Unsupported(format!("support check failed: {error}"))
    })?;
    if !supported {
        return Err(WgcCaptureError::Unsupported(
            "the operating system does not support Windows Graphics Capture".to_string(),
        ));
    }

    capture_supported_hwnd(hwnd).map_err(WgcCaptureError::Failed)
}

fn capture_supported_hwnd(hwnd: HWND) -> Result<image::RgbaImage, String> {
    let interop = factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()
        .map_err(|error| format!("Failed to activate GraphicsCaptureItem interop: {error}"))?;
    let item: GraphicsCaptureItem = unsafe { interop.CreateForWindow(hwnd) }
        .map_err(|error| format!("Failed to create a capture item for the window: {error}"))?;
    let initial_size = item
        .Size()
        .map_err(|error| format!("Failed to read capture item size: {error}"))?;
    if initial_size.Width <= 0 || initial_size.Height <= 0 {
        return Err("Capture item has zero or negative size".to_string());
    }

    let (native_device, context) = create_d3d11_device()?;
    let dxgi_device: IDXGIDevice = native_device
        .cast()
        .map_err(|error| format!("Failed to query IDXGIDevice: {error}"))?;
    let inspectable = unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi_device) }
        .map_err(|error| format!("Failed to create the WinRT D3D device: {error}"))?;
    let winrt_device: IDirect3DDevice = inspectable
        .cast()
        .map_err(|error| format!("Failed to query IDirect3DDevice: {error}"))?;

    let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
        &winrt_device,
        DirectXPixelFormat::B8G8R8A8UIntNormalized,
        1,
        initial_size,
    )
    .map_err(|error| format!("Failed to create capture frame pool: {error}"))?;
    let session = frame_pool
        .CreateCaptureSession(&item)
        .map_err(|error| format!("Failed to create capture session: {error}"))?;

    // Match the previous screenshot behavior: the mouse pointer is not part of
    // a window screenshot. These properties are unavailable on older builds,
    // so failure to set them must not abort capture.
    let _ = session.SetIsCursorCaptureEnabled(false);
    let _ = session.SetIsBorderRequired(false);

    let (sender, receiver) = mpsc::sync_channel(1);
    let handler = TypedEventHandler::<Direct3D11CaptureFramePool, IInspectable>::new(
        move |_, _| {
            let _ = sender.try_send(());
            Ok(())
        },
    );
    let token = frame_pool
        .FrameArrived(&handler)
        .map_err(|error| format!("Failed to register frame callback: {error}"))?;

    let result = (|| {
        session
            .StartCapture()
            .map_err(|error| format!("Failed to start window capture: {error}"))?;
        receiver.recv_timeout(FRAME_TIMEOUT).map_err(|error| match error {
            mpsc::RecvTimeoutError::Timeout => {
                "Timed out waiting for a window capture frame".to_string()
            }
            mpsc::RecvTimeoutError::Disconnected => {
                "Window capture frame callback disconnected".to_string()
            }
        })?;

        let frame = frame_pool
            .TryGetNextFrame()
            .map_err(|error| format!("Failed to acquire window capture frame: {error}"))?;
        let image = read_frame(&frame, &native_device, &context);
        let _ = frame.Close();
        image
    })();

    let _ = frame_pool.RemoveFrameArrived(token);
    let _ = session.Close();
    let _ = frame_pool.Close();
    result
}

fn create_d3d11_device() -> Result<(ID3D11Device, ID3D11DeviceContext), String> {
    match create_d3d11_device_with_driver(D3D_DRIVER_TYPE_HARDWARE) {
        Ok(device) => Ok(device),
        Err(hardware_error) => create_d3d11_device_with_driver(D3D_DRIVER_TYPE_WARP).map_err(
            |warp_error| {
                format!(
                    "Failed to create D3D11 device (hardware: {hardware_error}; WARP: {warp_error})"
                )
            },
        ),
    }
}

fn create_d3d11_device_with_driver(
    driver_type: D3D_DRIVER_TYPE,
) -> Result<(ID3D11Device, ID3D11DeviceContext), String> {
    let mut device = None;
    let mut context = None;
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
    let texture: ID3D11Texture2D = unsafe { access.GetInterface() }
        .map_err(|error| format!("Failed to query captured D3D11 texture: {error}"))?;

    let mut description = D3D11_TEXTURE2D_DESC::default();
    unsafe { texture.GetDesc(&mut description) };
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
    unsafe { device.CreateTexture2D(&staging_description, None, Some(&mut staging)) }
        .map_err(|error| format!("Failed to create staging texture: {error}"))?;
    let staging = staging.ok_or_else(|| "D3D11 returned no staging texture".to_string())?;
    unsafe { context.CopyResource(&staging, &texture) };

    let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
    unsafe { context.Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped)) }
        .map_err(|error| format!("Failed to map staging texture: {error}"))?;

    let read_result = (|| {
        if mapped.pData.is_null() {
            return Err("Mapped staging texture returned a null data pointer".to_string());
        }
        let row_pitch = mapped.RowPitch as usize;
        let mapped_len = row_pitch
            .checked_mul(height as usize)
            .ok_or_else(|| "Mapped frame buffer is too large".to_string())?;
        let mapped_bytes =
            unsafe { std::slice::from_raw_parts(mapped.pData.cast::<u8>(), mapped_len) };
        let rgba = rgba_from_bgra_rows(mapped_bytes, row_pitch, width, height)?;
        image::RgbaImage::from_raw(width, height, rgba)
            .ok_or_else(|| "Failed to construct image from captured frame".to_string())
    })();

    unsafe { context.Unmap(&staging, 0) };
    read_result
}
