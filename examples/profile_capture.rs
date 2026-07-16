//! Stage-by-stage profiler for the screenshot capture paths: the `screenshots`
//! crate blit replicated as timed GDI calls, and the Windows.Graphics.Capture
//! pipeline from `capture/platform/wgc.rs` with a timer around every stage,
//! plus warm-reuse variants quantifying device/session caching.
//! Run: `cargo run --release --example profile_capture`

#[cfg(not(windows))]
fn main() {
    println!("profile_capture is a Windows-only profiler (GDI + Windows.Graphics.Capture).");
}

#[cfg(windows)]
fn main() {
    win::run();
}

#[cfg(windows)]
mod win {
    use std::time::{Duration, Instant};

    /// Collect per-iteration durations and print min / median / mean.
    struct Stat {
        label: &'static str,
        samples: Vec<Duration>,
    }

    impl Stat {
        fn new(label: &'static str) -> Self {
            Stat {
                label,
                samples: Vec::new(),
            }
        }

        fn record<T>(&mut self, f: impl FnOnce() -> T) -> T {
            let start = Instant::now();
            let out = f();
            self.samples.push(start.elapsed());
            out
        }

        fn report(&self) {
            if self.samples.is_empty() {
                println!("{:<44} (no samples)", self.label);
                return;
            }
            let mut sorted = self.samples.clone();
            sorted.sort();
            let ms = |d: &Duration| d.as_secs_f64() * 1e3;
            let mean = self.samples.iter().map(ms).sum::<f64>() / self.samples.len() as f64;
            println!(
                "{:<44} min {:>8.2} ms   med {:>8.2} ms   mean {:>8.2} ms   (n={})",
                self.label,
                ms(&sorted[0]),
                ms(&sorted[sorted.len() / 2]),
                mean,
                self.samples.len()
            );
        }
    }

    pub fn run() {
        println!("== RustShot capture profile ==\n");
        profile_screenshots_crate();
        profile_gdi_replica();
        profile_wgc();
    }

    // Part A: the production full-screen path (screenshots crate + save)
    fn profile_screenshots_crate() {
        println!("-- Part A: production full-screen path --");
        let mut enumerate = Stat::new("Screen::all() enumeration");
        let mut blit = Stat::new("screen.capture() [screenshots crate]");
        let mut convert = Stat::new("bridge copy into image::RgbaImage");
        let mut save_png = Stat::new("save PNG (production encoder settings)");

        let screens = enumerate
            .record(screenshots::Screen::all)
            .unwrap_or_default();
        for _ in 0..4 {
            let _ = enumerate.record(screenshots::Screen::all);
        }
        let Some(screen) = screens.first() else {
            println!("no screens detected; skipping Part A\n");
            return;
        };

        let dir = std::env::temp_dir().join("rustshot_profile");
        let _ = std::fs::create_dir_all(&dir);
        for i in 0..15 {
            let Ok(captured) = blit.record(|| screen.capture()) else {
                continue;
            };
            let (w, h) = (captured.width(), captured.height());
            let raw = captured.into_raw();
            let img = convert
                .record(|| image::RgbaImage::from_raw(w, h, raw))
                .expect("raw buffer matches dimensions");
            if i < 5 {
                let path = dir.join("profile.png");
                use image::codecs::png::{CompressionType, FilterType, PngEncoder};
                use std::io::BufWriter;
                save_png.record(|| {
                    let file = std::fs::File::create(&path).expect("create profile.png");
                    let encoder = PngEncoder::new_with_quality(
                        BufWriter::new(file),
                        CompressionType::Fast,
                        FilterType::Sub,
                    );
                    img.write_with_encoder(encoder).expect("encode profile.png");
                });
            }
        }
        enumerate.report();
        blit.report();
        convert.report();
        save_png.report();
        println!();
    }

    // Part B: GDI full-screen blit, one timer per API call
    fn profile_gdi_replica() {
        use windows::Win32::Graphics::Gdi::{
            BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
            GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
            DIB_RGB_COLORS, SRCCOPY,
        };
        use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

        println!("-- Part B: GDI blit replica, per-API-call timing --");
        let width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
        let height = unsafe { GetSystemMetrics(SM_CYSCREEN) };
        if width <= 0 || height <= 0 {
            println!("no desktop metrics; skipping Part B\n");
            return;
        }

        let mut get_dc = Stat::new("GetDC + CreateCompatibleDC/Bitmap");
        let mut bitblt = Stat::new("BitBlt (the actual screen copy)");
        let mut getdibits = Stat::new("GetDIBits (GPU->CPU readback)");
        let mut convert = Stat::new("BGRX -> RGBA conversion");
        let mut teardown = Stat::new("GDI teardown (Delete/Release)");

        for _ in 0..15 {
            let (screen_dc, memory_dc, bitmap) = get_dc.record(|| unsafe {
                let screen_dc = GetDC(None);
                let memory_dc = CreateCompatibleDC(Some(screen_dc));
                let bitmap = CreateCompatibleBitmap(screen_dc, width, height);
                SelectObject(memory_dc, bitmap.into());
                (screen_dc, memory_dc, bitmap)
            });

            let blit_ok = bitblt.record(|| unsafe {
                BitBlt(
                    memory_dc,
                    0,
                    0,
                    width,
                    height,
                    Some(screen_dc),
                    0,
                    0,
                    SRCCOPY,
                )
                .is_ok()
            });

            let mut pixels = vec![0u8; width as usize * height as usize * 4];
            if blit_ok {
                let mut info = BITMAPINFO {
                    bmiHeader: BITMAPINFOHEADER {
                        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                        biWidth: width,
                        biHeight: -height, // negative = top-down row order
                        biPlanes: 1,
                        biBitCount: 32,
                        biCompression: BI_RGB.0,
                        ..Default::default()
                    },
                    ..Default::default()
                };
                let copied = getdibits.record(|| unsafe {
                    GetDIBits(
                        memory_dc,
                        bitmap,
                        0,
                        height as u32,
                        Some(pixels.as_mut_ptr().cast()),
                        &mut info,
                        DIB_RGB_COLORS,
                    )
                });
                if copied == height {
                    convert.record(|| {
                        for px in pixels.as_chunks_mut::<4>().0 {
                            px.swap(0, 2);
                            px[3] = 255;
                        }
                    });
                }
            }

            teardown.record(|| unsafe {
                let _ = DeleteObject(bitmap.into());
                let _ = DeleteDC(memory_dc);
                let _ = ReleaseDC(None, screen_dc);
            });
        }

        get_dc.report();
        bitblt.report();
        getdibits.report();
        convert.report();
        teardown.report();
        println!("(screen: {width}x{height})\n");
    }

    // Part C: Windows.Graphics.Capture pipeline, one timer per stage
    fn profile_wgc() {
        use std::sync::mpsc;
        use windows::core::{factory, IInspectable, Interface};
        use windows::Foundation::TypedEventHandler;
        use windows::Graphics::Capture::{
            Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession,
        };
        use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
        use windows::Graphics::DirectX::DirectXPixelFormat;
        use windows::Win32::Foundation::HMODULE;
        use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE;
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
        use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};
        use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

        println!("-- Part C: Windows.Graphics.Capture stage profile --");
        let mut ro_init = Stat::new("RoInitialize (apartment)");
        let mut support = Stat::new("IsSupported + interop factory");
        let mut item_stat = Stat::new("CreateForWindow + Size");
        let mut device_stat = Stat::new("D3D11CreateDevice (hardware)");
        let mut winrt_dev = Stat::new("DXGI cast + WinRT device wrap");
        let mut pool_stat = Stat::new("FramePool CreateFreeThreaded");
        let mut session_stat = Stat::new("CreateCaptureSession + props");
        let mut first_frame = Stat::new("StartCapture -> first FrameArrived");
        let mut read_stat = Stat::new("staging copy + Map + RGBA convert");
        let mut next_frame = Stat::new("[warm] subsequent frame on live session");
        let mut cached_dev = Stat::new("[warm] full capture, D3D device reused");

        let init = ro_init.record(|| unsafe { RoInitialize(RO_INIT_MULTITHREADED) });
        if init.is_err() {
            println!("RoInitialize failed; skipping Part C\n");
            return;
        }

        let supported = support.record(|| {
            GraphicsCaptureSession::IsSupported().unwrap_or(false)
                && factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>().is_ok()
        });
        if !supported {
            println!("WGC unsupported here; skipping Part C\n");
            return;
        }
        let interop = factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>().unwrap();
        let hwnd = unsafe { GetForegroundWindow() };
        if hwnd.is_invalid() {
            println!("no foreground window; skipping Part C\n");
            return;
        }

        // Cold runs: everything created from scratch, like production today.
        let mut shared_device: Option<(ID3D11Device, ID3D11DeviceContext)> = None;
        for round in 0..3 {
            let Ok(item): Result<GraphicsCaptureItem, _> =
                item_stat.record(|| unsafe { interop.CreateForWindow(hwnd) })
            else {
                continue;
            };
            let Ok(size) = item.Size() else { continue };

            let created = device_stat.record(|| {
                let mut device = None;
                let mut context = None;
                unsafe {
                    D3D11CreateDevice(
                        None,
                        D3D_DRIVER_TYPE_HARDWARE,
                        HMODULE::default(),
                        D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                        None,
                        D3D11_SDK_VERSION,
                        Some(&mut device),
                        None,
                        Some(&mut context),
                    )
                }
                .ok()
                .and(device.zip(context))
            });
            let Some((device, context)) = created else {
                continue;
            };

            let winrt_device: Option<IDirect3DDevice> = winrt_dev.record(|| {
                let dxgi: IDXGIDevice = device.cast().ok()?;
                let inspectable = unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi) }.ok()?;
                inspectable.cast().ok()
            });
            let Some(winrt_device) = winrt_device else {
                continue;
            };

            let pool = pool_stat.record(|| {
                Direct3D11CaptureFramePool::CreateFreeThreaded(
                    &winrt_device,
                    DirectXPixelFormat::B8G8R8A8UIntNormalized,
                    1,
                    size,
                )
                .ok()
            });
            let Some(pool) = pool else { continue };

            let session = session_stat.record(|| {
                let session = pool.CreateCaptureSession(&item).ok()?;
                let _ = session.SetIsCursorCaptureEnabled(false);
                let _ = session.SetIsBorderRequired(false);
                Some(session)
            });
            let Some(session) = session else { continue };

            let (sender, receiver) = mpsc::sync_channel(2);
            let handler =
                TypedEventHandler::<Direct3D11CaptureFramePool, IInspectable>::new(move |_, _| {
                    let _ = sender.try_send(());
                    Ok(())
                });
            let Ok(token) = pool.FrameArrived(&handler) else {
                continue;
            };

            let got_frame = first_frame.record(|| {
                session.StartCapture().is_ok()
                    && receiver
                        .recv_timeout(std::time::Duration::from_secs(3))
                        .is_ok()
            });
            if got_frame {
                if let Ok(frame) = pool.TryGetNextFrame() {
                    let _ = read_stat.record(|| read_frame_timed(&frame, &device, &context));
                    let _ = frame.Close();
                }
                // Warm path: more frames from the SAME live session.
                if round == 0 {
                    for _ in 0..5 {
                        let ok = next_frame.record(|| {
                            receiver
                                .recv_timeout(std::time::Duration::from_secs(1))
                                .is_ok()
                                && pool.TryGetNextFrame().is_ok()
                        });
                        if !ok {
                            break;
                        }
                    }
                }
            }

            let _ = pool.RemoveFrameArrived(token);
            let _ = session.Close();
            let _ = pool.Close();
            shared_device = Some((device, context));
        }

        // Warm runs: full item->pool->session->frame cycle with a REUSED device.
        if let Some((device, context)) = shared_device {
            if let Ok(dxgi) = device.cast::<IDXGIDevice>() {
                if let Ok(inspectable) = unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi) } {
                    if let Ok(winrt_device) = inspectable.cast::<IDirect3DDevice>() {
                        for _ in 0..5 {
                            cached_dev.record(|| {
                                capture_once_with_device(
                                    &interop,
                                    hwnd,
                                    &winrt_device,
                                    &device,
                                    &context,
                                )
                            });
                        }
                    }
                }
            }
        }

        ro_init.report();
        support.report();
        item_stat.report();
        device_stat.report();
        winrt_dev.report();
        pool_stat.report();
        session_stat.report();
        first_frame.report();
        read_stat.report();
        next_frame.report();
        cached_dev.report();
        println!();

        // One full cycle with a cached device, for the [warm] row above.
        fn capture_once_with_device(
            interop: &IGraphicsCaptureItemInterop,
            hwnd: windows::Win32::Foundation::HWND,
            winrt_device: &IDirect3DDevice,
            device: &ID3D11Device,
            context: &ID3D11DeviceContext,
        ) -> bool {
            let Ok(item): Result<GraphicsCaptureItem, _> =
                (unsafe { interop.CreateForWindow(hwnd) })
            else {
                return false;
            };
            let Ok(size) = item.Size() else { return false };
            let Ok(pool) = Direct3D11CaptureFramePool::CreateFreeThreaded(
                winrt_device,
                DirectXPixelFormat::B8G8R8A8UIntNormalized,
                1,
                size,
            ) else {
                return false;
            };
            let Ok(session) = pool.CreateCaptureSession(&item) else {
                return false;
            };
            let _ = session.SetIsCursorCaptureEnabled(false);
            let _ = session.SetIsBorderRequired(false);
            let (sender, receiver) = mpsc::sync_channel(1);
            let handler =
                TypedEventHandler::<Direct3D11CaptureFramePool, IInspectable>::new(move |_, _| {
                    let _ = sender.try_send(());
                    Ok(())
                });
            let Ok(token) = pool.FrameArrived(&handler) else {
                return false;
            };
            let ok = session.StartCapture().is_ok()
                && receiver
                    .recv_timeout(std::time::Duration::from_secs(3))
                    .is_ok()
                && pool
                    .TryGetNextFrame()
                    .ok()
                    .map(|frame| {
                        let done = read_frame_timed(&frame, device, context);
                        let _ = frame.Close();
                        done
                    })
                    .unwrap_or(false);
            let _ = pool.RemoveFrameArrived(token);
            let _ = session.Close();
            let _ = pool.Close();
            ok
        }

        // Staging-texture readback, mirroring wgc::read_frame.
        fn read_frame_timed(
            frame: &windows::Graphics::Capture::Direct3D11CaptureFrame,
            device: &ID3D11Device,
            context: &ID3D11DeviceContext,
        ) -> bool {
            let Ok(size) = frame.ContentSize() else {
                return false;
            };
            if size.Width <= 0 || size.Height <= 0 {
                return false;
            }
            let (width, height) = (size.Width as u32, size.Height as u32);
            let Ok(surface) = frame.Surface() else {
                return false;
            };
            let Ok(access) = surface.cast::<IDirect3DDxgiInterfaceAccess>() else {
                return false;
            };
            let Ok(texture): Result<ID3D11Texture2D, _> = (unsafe { access.GetInterface() }) else {
                return false;
            };
            let mut desc = D3D11_TEXTURE2D_DESC::default();
            unsafe { texture.GetDesc(&mut desc) };
            let mut staging_desc = desc;
            staging_desc.Usage = D3D11_USAGE_STAGING;
            staging_desc.BindFlags = 0;
            staging_desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
            staging_desc.MiscFlags = 0;
            let mut staging = None;
            if unsafe { device.CreateTexture2D(&staging_desc, None, Some(&mut staging)) }.is_err() {
                return false;
            }
            let Some(staging) = staging else { return false };
            unsafe { context.CopyResource(&staging, &texture) };
            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
            if unsafe { context.Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped)) }.is_err() {
                return false;
            }
            let ok = if mapped.pData.is_null() {
                false
            } else {
                let pitch = mapped.RowPitch as usize;
                let bytes = unsafe {
                    std::slice::from_raw_parts(mapped.pData.cast::<u8>(), pitch * height as usize)
                };
                let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
                for row in 0..height as usize {
                    let line = &bytes[row * pitch..row * pitch + width as usize * 4];
                    for px in line.as_chunks::<4>().0 {
                        rgba.extend_from_slice(&[px[2], px[1], px[0], 255]);
                    }
                }
                rgba.len() == width as usize * height as usize * 4
            };
            unsafe { context.Unmap(&staging, 0) };
            ok
        }
    }
}
