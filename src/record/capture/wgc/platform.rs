//! Monitor discovery, Advanced Color inspection, and D3D device creation.

use super::CaptureFormat;
use windows::core::Interface;
use windows::Win32::Foundation::HMODULE;
use windows::Win32::Graphics::Direct3D::{
    D3D_DRIVER_TYPE, D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP,
};
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, D3D11_CREATE_DEVICE_BGRA_SUPPORT,
    D3D11_SDK_VERSION,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_COLOR_SPACE_RGB_FULL_G22_NONE_P709;
use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1, IDXGIOutput6};
use windows::Win32::Graphics::Gdi::HMONITOR;

const DEFAULT_HDR_PEAK_NITS: f32 = 1_000.0;

pub(super) fn capture_format_for_monitor(
    monitor: HMONITOR,
) -> Result<(IDXGIFactory1, CaptureFormat), String> {
    let factory: IDXGIFactory1 = unsafe { CreateDXGIFactory1() }
        .map_err(|error| format!("Failed to create DXGI factory: {error}"))?;
    let format = capture_format_for_existing_factory(&factory, monitor)?;
    Ok((factory, format))
}

pub(super) fn capture_format_for_existing_factory(
    factory: &IDXGIFactory1,
    monitor: HMONITOR,
) -> Result<CaptureFormat, String> {
    for adapter_index in 0.. {
        let Ok(adapter) = (unsafe { factory.EnumAdapters1(adapter_index) }) else {
            break;
        };
        for output_index in 0.. {
            let Ok(output) = (unsafe { adapter.EnumOutputs(output_index) }) else {
                break;
            };
            let Ok(description) = (unsafe { output.GetDesc() }) else {
                continue;
            };
            if description.Monitor != monitor {
                continue;
            }

            let Ok(output6) = output.cast::<IDXGIOutput6>() else {
                log::warn!(
                    "IDXGIOutput6 is unavailable for the selected monitor; using SDR BGRA capture"
                );
                return Ok(CaptureFormat::Bgra8);
            };
            let advanced = unsafe { output6.GetDesc1() }
                .map_err(|error| format!("Failed to read Advanced Color state: {error}"))?;
            if advanced.ColorSpace == DXGI_COLOR_SPACE_RGB_FULL_G22_NONE_P709 {
                return Ok(CaptureFormat::Bgra8);
            }
            let peak_luminance_nits =
                if advanced.MaxLuminance.is_finite() && advanced.MaxLuminance >= 80.0 {
                    advanced.MaxLuminance
                } else {
                    DEFAULT_HDR_PEAK_NITS
                };
            return Ok(CaptureFormat::ScRgb16 {
                peak_luminance_nits,
            });
        }
    }

    log::warn!("DXGI could not match the selected monitor; using SDR BGRA capture");
    Ok(CaptureFormat::Bgra8)
}

pub(super) fn create_d3d11_device() -> Result<(ID3D11Device, ID3D11DeviceContext), String> {
    match create_d3d11_device_with_driver(D3D_DRIVER_TYPE_HARDWARE) {
        Ok(device) => Ok(device),
        Err(hardware_error) => create_d3d11_device_with_driver(D3D_DRIVER_TYPE_WARP).map_err(
            |warp_error| {
                format!(
                    "Failed to create recording D3D11 device (hardware: {hardware_error}; WARP: {warp_error})"
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

    Ok((
        device.ok_or_else(|| "D3D11 returned no recording device".to_string())?,
        context.ok_or_else(|| "D3D11 returned no recording device context".to_string())?,
    ))
}
