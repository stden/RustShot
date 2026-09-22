//! Owned, changing desktop content for Windows capture process tests.
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use windows::core::{w, PCWSTR};
use windows::Win32::UI::WindowsAndMessaging::*;

pub(crate) struct CaptureWindow {
    id: u64,
    stop: mpsc::Sender<()>,
    worker: Option<JoinHandle<()>>,
}

impl CaptureWindow {
    pub(crate) fn new() -> Self {
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let (stop_tx, stop_rx) = mpsc::channel();
        let worker = thread::spawn(move || {
            let screen =
                screenshots::Screen::all().expect("enumerate test displays")[0].display_info;
            // SAFETY: STATIC is an OS-provided class; strings are static and no
            // borrowed creation parameters are passed. This thread owns the HWND.
            let hwnd = match unsafe {
                CreateWindowExW(
                    WINDOW_EX_STYLE::default(),
                    w!("STATIC"),
                    w!("RustShot capture fixture"),
                    WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                    screen.x.saturating_add(20),
                    screen.y.saturating_add(20),
                    320,
                    220,
                    None,
                    None,
                    None,
                    None,
                )
            } {
                Ok(hwnd) => hwnd,
                Err(error) => {
                    let _ = ready_tx.send(Err(error.to_string()));
                    return;
                }
            };
            let _ = ready_tx.send(Ok(hwnd.0 as u64));
            let mut frame = 0;
            while matches!(stop_rx.try_recv(), Err(mpsc::TryRecvError::Empty)) {
                let mut message = MSG::default();
                // SAFETY: the message is writable and only this thread's queue
                // is drained; DispatchMessageW receives the message Windows filled.
                unsafe {
                    while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                        let _ = TranslateMessage(&message);
                        DispatchMessageW(&message);
                    }
                }
                let title: Vec<u16> = format!("RustShot capture fixture: frame {frame}")
                    .encode_utf16()
                    .chain(Some(0))
                    .collect();
                // SAFETY: hwnd still belongs to this worker; the NUL-terminated
                // title remains alive for the synchronous SetWindowTextW call.
                unsafe { SetWindowTextW(hwnd, PCWSTR(title.as_ptr())) }.unwrap();
                frame += 1;
                thread::sleep(Duration::from_millis(40));
            }
            // SAFETY: only this owning thread destroys the live fixture window.
            let _ = unsafe { DestroyWindow(hwnd) };
        });
        // Create the guard before waiting so failed startup still joins the worker.
        let mut fixture = Self {
            id: 0,
            stop: stop_tx,
            worker: Some(worker),
        };
        fixture.id = ready_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("the fixture window must be ready")
            .expect("create fixture window");
        fixture
    }

    pub(crate) fn id(&self) -> u64 {
        self.id
    }
}

impl Drop for CaptureWindow {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
