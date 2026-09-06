//! Stage-by-stage profiler for the screenshot capture paths: the `screenshots`
//! crate blit replicated as timed GDI calls, and the Windows.Graphics.Capture
//! pipeline from `capture/platform/wgc.rs` with a timer around every stage,
//! plus warm-reuse variants quantifying device/session caching.
//! Run: `cargo run --release --example profile_capture`

#[cfg(any(windows, test))]
fn mapped_bgrx_length(
    frame_size: (u32, u32),
    texture_size: (u32, u32),
    pitch: usize,
) -> Option<usize> {
    let (width, height) = frame_size;
    if width == 0 || height == 0 || width > texture_size.0 || height > texture_size.1 {
        return None;
    }
    let row_bytes = (width as usize).checked_mul(4)?;
    if pitch < row_bytes {
        return None;
    }
    let length = pitch.checked_mul(height as usize)?;
    (length <= isize::MAX as usize).then_some(length)
}

#[cfg(test)]
#[path = "../tests/unit/profile_capture/mod.rs"]
mod tests;

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
    use std::sync::mpsc;
    use std::time::{Duration, Instant};
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
    use windows::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
        GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
        HBITMAP, HDC, HGDIOBJ, SRCCOPY,
    };
    use windows::Win32::System::WinRT::Direct3D11::{
        CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess,
    };
    use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
    use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN,
    };

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

    pub(crate) fn run() {
        println!("== RustShot capture profile ==\n");
        profile_screenshots_crate();
        profile_gdi_replica();
        profile_wgc();
    }

    fn convert_bgrx_pixels(pixels: &mut [u8]) {
        for pixel in pixels.as_chunks_mut::<4>().0 {
            pixel.swap(0, 2);
            pixel[3] = 255;
        }
    }

    fn convert_bgrx_rows(bytes: &[u8], width: u32, height: u32, pitch: usize) -> Vec<u8> {
        let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
        for row in 0..height as usize {
            let line = &bytes[row * pitch..row * pitch + width as usize * 4];
            for pixel in line.as_chunks::<4>().0 {
                rgba.extend_from_slice(&[pixel[2], pixel[1], pixel[0], 255]);
            }
        }
        rgba
    }

    fn save_profile_png(img: &image::RgbaImage, path: &std::path::Path) {
        use image::codecs::png::{CompressionType, FilterType, PngEncoder};
        use std::io::BufWriter;

        let file = std::fs::File::create(path).expect("create profile.png");
        let encoder = PngEncoder::new_with_quality(
            BufWriter::new(file),
            CompressionType::Fast,
            FilterType::Sub,
        );
        img.write_with_encoder(encoder).expect("encode profile.png");
    }

    /// Keep the selected-object pairing with the DC so readback and deletion can
    /// restore its original bitmap, including when `BitBlt` fails.
    struct GdiCapture {
        screen_dc: HDC,
        memory_dc: HDC,
        bitmap: HBITMAP,
        previous_bitmap: HGDIOBJ,
        width: i32,
        height: i32,
    }

    impl GdiCapture {
        fn create(width: i32, height: i32) -> Self {
            // SAFETY: the positive desktop dimensions come from GetSystemMetrics.
            // The DC/bitmap handles stay on this thread and are released after the sample;
            // null failure handles are tolerated by the subsequent GDI calls.
            unsafe {
                let screen_dc = GetDC(None);
                let memory_dc = CreateCompatibleDC(Some(screen_dc));
                let bitmap = CreateCompatibleBitmap(screen_dc, width, height);
                let previous_bitmap = SelectObject(memory_dc, bitmap.into());
                Self {
                    screen_dc,
                    memory_dc,
                    bitmap,
                    previous_bitmap,
                    width,
                    height,
                }
            }
        }

        fn blit(&self) -> bool {
            // SAFETY: both DCs belong to this sample, the destination bitmap has the
            // requested dimensions, and neither DC is released until after readback.
            unsafe {
                BitBlt(
                    self.memory_dc,
                    0,
                    0,
                    self.width,
                    self.height,
                    Some(self.screen_dc),
                    0,
                    0,
                    SRCCOPY,
                )
                .is_ok()
            }
        }

        fn readback(&self, pixels: &mut [u8], info: &mut BITMAPINFO) -> i32 {
            // SAFETY: restore the object returned by this DC's SelectObject before
            // GetDIBits, whose bitmap must not remain selected in a DC. The caller's
            // 32-bit BI_RGB header matches width/height and pixels has width*height*4
            // bytes; 32-bit rows require no extra alignment padding.
            unsafe {
                SelectObject(self.memory_dc, self.previous_bitmap);
                GetDIBits(
                    self.memory_dc,
                    self.bitmap,
                    0,
                    self.height as u32,
                    Some(pixels.as_mut_ptr().cast()),
                    info,
                    DIB_RGB_COLORS,
                )
            }
        }

        fn release(self) {
            // SAFETY: restore the original bitmap even when readback was skipped;
            // delete only this sample's bitmap and memory DC, then release the desktop
            // DC with the same null HWND used by GetDC. No handle is reused afterwards.
            unsafe {
                SelectObject(self.memory_dc, self.previous_bitmap);
                let _ = DeleteObject(self.bitmap.into());
                let _ = DeleteDC(self.memory_dc);
                let _ = ReleaseDC(None, self.screen_dc);
            }
        }
    }

    fn record_gdi_conversion(convert: &mut Stat, pixels: &mut [u8], fully_copied: bool) {
        if fully_copied {
            convert.record(|| convert_bgrx_pixels(pixels));
        }
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
                save_png.record(|| save_profile_png(&img, &path));
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
        println!("-- Part B: GDI blit replica, per-API-call timing --");
        // SAFETY: these constants query scalar desktop metrics and need no pointers.
        let width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
        // SAFETY: SM_CYSCREEN is a valid scalar metric; no output buffer is supplied.
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
            let capture = get_dc.record(|| GdiCapture::create(width, height));
            let blit_ok = bitblt.record(|| capture.blit());

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
                let copied = getdibits.record(|| capture.readback(&mut pixels, &mut info));
                record_gdi_conversion(&mut convert, &mut pixels, copied == height);
            }

            teardown.record(|| capture.release());
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

        // SAFETY: initialize this profiler thread's WinRT apartment before any WinRT use.
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
        // SAFETY: querying the foreground handle does not dereference application memory.
        let hwnd = unsafe { GetForegroundWindow() };
        if hwnd.is_invalid() {
            println!("no foreground window; skipping Part C\n");
            return;
        }

        // Cold runs: everything created from scratch, like production today.
        let mut shared_device: Option<(ID3D11Device, ID3D11DeviceContext)> = None;
        for round in 0..3 {
            // SAFETY: hwnd was obtained from Windows; if it closes, CreateForWindow
            // returns an error, which skips this sample. interop stays alive for the call.
            let Ok(item): Result<GraphicsCaptureItem, _> =
                item_stat.record(|| unsafe { interop.CreateForWindow(hwnd) })
            else {
                continue;
            };
            let Ok(size) = item.Size() else { continue };

            let created = device_stat.record(create_d3d_device);
            let Some((device, context)) = created else {
                continue;
            };

            let winrt_device = winrt_dev.record(|| wrap_winrt_device(&device));
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
                profile_first_frame(&pool, &device, &context, &mut read_stat);
            }
            // Warm path: more frames from the SAME live session.
            if got_frame && round == 0 {
                profile_subsequent_frames(&receiver, &pool, &mut next_frame);
            }

            let _ = pool.RemoveFrameArrived(token);
            let _ = session.Close();
            let _ = pool.Close();
            shared_device = Some((device, context));
        }

        // Warm runs: full item->pool->session->frame cycle with a REUSED device.
        let shared_device = shared_device.and_then(|(device, context)| {
            let winrt_device = wrap_winrt_device(&device)?;
            Some((device, context, winrt_device))
        });
        if let Some(devices) = shared_device {
            profile_cached_device_runs(&interop, hwnd, devices, &mut cached_dev);
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
    }

    fn create_d3d_device() -> Option<(ID3D11Device, ID3D11DeviceContext)> {
        let mut device = None;
        let mut context = None;
        // SAFETY: hardware creation uses no adapter or software DLL, the SDK
        // version is the supported constant, and both output slots are live Options.
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
        .ok()?;
        device.zip(context)
    }

    fn profile_first_frame(
        pool: &Direct3D11CaptureFramePool,
        device: &ID3D11Device,
        context: &ID3D11DeviceContext,
        read_stat: &mut Stat,
    ) {
        let Ok(frame) = pool.TryGetNextFrame() else {
            return;
        };
        let _ = read_stat.record(|| read_frame_timed(&frame, device, context));
        let _ = frame.Close();
    }

    fn profile_cached_device_runs(
        interop: &IGraphicsCaptureItemInterop,
        hwnd: windows::Win32::Foundation::HWND,
        devices: (ID3D11Device, ID3D11DeviceContext, IDirect3DDevice),
        cached_dev: &mut Stat,
    ) {
        let (device, context, winrt_device) = devices;
        for _ in 0..5 {
            cached_dev.record(|| {
                capture_once_with_device(interop, hwnd, &winrt_device, &device, &context)
            });
        }
    }

    fn wrap_winrt_device(device: &ID3D11Device) -> Option<IDirect3DDevice> {
        let dxgi: IDXGIDevice = device.cast().ok()?;
        // SAFETY: dxgi is a successful QueryInterface result from the live D3D
        // device; the returned inspectable owns its COM reference independently.
        let inspectable = unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi) }.ok()?;
        inspectable.cast().ok()
    }

    fn profile_subsequent_frames(
        receiver: &mpsc::Receiver<()>,
        pool: &Direct3D11CaptureFramePool,
        next_frame: &mut Stat,
    ) {
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

    // One full cycle with a cached device, for the [warm] row above.
    fn capture_once_with_device(
        interop: &IGraphicsCaptureItemInterop,
        hwnd: windows::Win32::Foundation::HWND,
        winrt_device: &IDirect3DDevice,
        device: &ID3D11Device,
        context: &ID3D11DeviceContext,
    ) -> bool {
        // SAFETY: hwnd originated from GetForegroundWindow. Its possible closure
        // is reported by this interop call as an error; the factory remains live.
        let Ok(item): Result<GraphicsCaptureItem, _> = (unsafe { interop.CreateForWindow(hwnd) })
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
        // SAFETY: access comes from this live frame's Direct3D surface; the
        // requested texture IID is checked by GetInterface and its error is handled.
        let Ok(texture): Result<ID3D11Texture2D, _> = (unsafe { access.GetInterface() }) else {
            return false;
        };
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        // SAFETY: texture owns a live COM reference and desc is a writable
        // descriptor of the exact type and size GetDesc fills.
        unsafe { texture.GetDesc(&mut desc) };
        let mut staging_desc = desc;
        staging_desc.Usage = D3D11_USAGE_STAGING;
        staging_desc.BindFlags = 0;
        staging_desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
        staging_desc.MiscFlags = 0;
        let mut staging = None;
        // SAFETY: the descriptor preserves the source texture's dimensions and
        // format, changes its usage to CPU-readable staging, and supplies no initial
        // data pointer. staging is a live output slot; unsupported formats fail.
        if unsafe { device.CreateTexture2D(&staging_desc, None, Some(&mut staging)) }.is_err() {
            return false;
        }
        let Some(staging) = staging else { return false };
        // SAFETY: both live textures belong to the frame pool's D3D device and
        // have identical extents/format; the staging texture is not yet mapped.
        unsafe { context.CopyResource(&staging, &texture) };
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        // SAFETY: subresource zero exists on the CPU-readable staging texture;
        // mapped is a writable descriptor kept alive until the matching Unmap.
        if unsafe { context.Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped)) }.is_err() {
            return false;
        }
        let pitch = mapped.RowPitch as usize;
        let byte_count =
            super::mapped_bgrx_length((width, height), (desc.Width, desc.Height), pitch);
        let ok = if let Some(byte_count) = byte_count.filter(|_| !mapped.pData.is_null()) {
            // SAFETY: Map succeeded and pData is non-null. The checked byte count
            // is within this texture's row-pitched rows and isize::MAX. Staging
            // remains mapped/live until after all reads; this slice never escapes.
            let bytes =
                unsafe { std::slice::from_raw_parts(mapped.pData.cast::<u8>(), byte_count) };
            let rgba = convert_bgrx_rows(bytes, width, height, pitch);
            rgba.len() == width as usize * height as usize * 4
        } else {
            false
        };
        // SAFETY: the matching Map above succeeded for subresource zero, and
        // the borrowed byte slice and its conversion are no longer used.
        unsafe { context.Unmap(&staging, 0) };
        ok
    }
}
