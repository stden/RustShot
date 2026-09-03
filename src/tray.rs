//! System tray icon and context menu.
//!
//! Creates the tray icon (a simple 32×32 camera glyph) and a right-click menu
//! with items for Capture, Open Folder, Toggle Recording, and Exit.
//!
//! The tray runs inside the winit event loop managed by `RustShotApp` in
//! `app.rs`. Menu callbacks are forwarded through a `winit::EventLoopProxy`,
//! which wakes the otherwise sleeping event loop for immediate dispatch.

use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    Icon, TrayIcon, TrayIconBuilder,
};

/// What a tray menu entry does.
///
/// This enum — not the raw id strings — is what the event loop dispatches on, so
/// adding an entry is a compile error until it is handled. The `&str` ids below
/// exist only because `muda` round-trips ids through the OS as strings; they are
/// the wire format, and they are derived from this enum so the two cannot drift.
///
/// Recording is a SINGLE [`MenuAction::ToggleRecord`] entry
/// (`TOGGLE_RECORD_LABEL`). A separate Stop item used to exist but was removed as
/// a misleading affordance: the tray menu is built once at startup and cannot
/// reflect live recording state, so a "Stop" entry appeared enabled even when
/// nothing was recording. The legacy `"stop_record"` id is still *accepted* by
/// [`MenuAction::from_id`] and routes to the same toggle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    Capture,
    CaptureWindow,
    OpenFolder,
    ToggleRecord,
    Settings,
    Exit,
}

impl MenuAction {
    /// Every menu action. Used to resolve an id back to its action without a
    /// second lookup table.
    pub const ALL: [MenuAction; 6] = [
        MenuAction::Capture,
        MenuAction::CaptureWindow,
        MenuAction::OpenFolder,
        MenuAction::ToggleRecord,
        MenuAction::Settings,
        MenuAction::Exit,
    ];

    /// The id string handed to the OS for this entry.
    pub const fn id(self) -> &'static str {
        match self {
            MenuAction::Capture => "capture",
            MenuAction::CaptureWindow => "capture_window",
            MenuAction::OpenFolder => "open_folder",
            MenuAction::ToggleRecord => "start_record",
            MenuAction::Settings => "settings",
            MenuAction::Exit => "exit",
        }
    }

    /// Resolve an id coming back from the OS. Unknown ids yield `None` so the
    /// caller can ignore them rather than guess.
    ///
    /// # Examples
    ///
    /// ```
    /// use rustshot::tray::MenuAction;
    ///
    /// assert_eq!(MenuAction::from_id("capture"), Some(MenuAction::Capture));
    /// // Legacy id from the old two-item recording menu maps to the same toggle:
    /// assert_eq!(MenuAction::from_id("stop_record"), Some(MenuAction::ToggleRecord));
    /// assert_eq!(MenuAction::from_id("nonsense"), None);
    /// ```
    pub fn from_id(id: &str) -> Option<MenuAction> {
        // The legacy id has no variant of its own; it routes to the same toggle.
        if id == MENU_STOP_RECORD {
            return Some(MenuAction::ToggleRecord);
        }
        // Everything else round-trips through `id()`, so there is no second
        // table of strings that could drift out of sync with the first.
        MenuAction::ALL.into_iter().find(|action| action.id() == id)
    }
}

// Menu item ids (the OS-level wire format), derived from `MenuAction`.
pub const MENU_CAPTURE: &str = MenuAction::Capture.id();
pub const MENU_OPEN_FOLDER: &str = MenuAction::OpenFolder.id();
pub const MENU_EXIT: &str = MenuAction::Exit.id();
pub const MENU_CAPTURE_WINDOW: &str = MenuAction::CaptureWindow.id();
pub const MENU_SETTINGS: &str = MenuAction::Settings.id();
pub const MENU_START_RECORD: &str = MenuAction::ToggleRecord.id();
/// Legacy id from the old two-entry recording menu. Still accepted by
/// [`MenuAction::from_id`], which routes it to [`MenuAction::ToggleRecord`].
pub const MENU_STOP_RECORD: &str = "stop_record";
const DEFAULT_ICON_SIZE: u32 = 32;
const TOGGLE_RECORD_LABEL: &str = "Start or Stop Recording (Ctrl+Shift+R)";

/// Generate a 32×32 RGBA pixel buffer for the tray icon (blue camera with white lens).
fn build_default_icon_pixels() -> Vec<u8> {
    let size = DEFAULT_ICON_SIZE;
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    // Pre-compute lens threshold as squared distance to avoid sqrt per pixel
    const LENS_RADIUS_SQ: i32 = 6 * 6; // 36

    for y in 0..size {
        let dist_y = (y as i32 - 16).abs();
        let in_body_y = dist_y <= 10;
        let dist_y_sq = dist_y * dist_y;

        for x in 0..size {
            let idx = ((y * size + x) * 4) as usize;
            let dist_x = (x as i32 - 16).abs();

            if in_body_y && dist_x <= 12 {
                let dist_sq = dist_x * dist_x + dist_y_sq;
                if dist_sq <= LENS_RADIUS_SQ {
                    // Lens - white circle in center
                    rgba[idx] = 255; // R
                    rgba[idx + 1] = 255; // G
                    rgba[idx + 2] = 255; // B
                } else {
                    // Camera body - dark blue
                    rgba[idx] = 41; // R
                    rgba[idx + 1] = 128; // G
                    rgba[idx + 2] = 185; // B
                }
                rgba[idx + 3] = 255; // A
            }
            // else: transparent (already 0)
        }
    }

    rgba
}

/// Prepare the platform GUI toolkit that [`create_tray`] builds on.
///
/// Linux only, and the whole reason this function exists. `tray-icon` reaches the
/// Linux tray through GTK and appindicator, and every gtk-rs widget constructor
/// opens with an assertion that `gtk::init` has already run on the calling
/// thread. Nothing else in RustShot uses GTK -- winit talks to X11/Wayland
/// directly -- so without this call the first `gtk::Menu::new()` PANICKED with
/// "GTK has not been initialized. Call `gtk::init` first.". That is a panic, not
/// an `Err`, so it escaped [`create_tray`] entirely and killed the process from
/// inside `resumed`, whose error arm can only log. It is the failure the Linux CI
/// job reported once the appindicator library was installed.
///
/// Note the panic came from `TrayIconBuilder::build`, not from the `Menu::new()`
/// below: muda builds its real `gtk::Menu` lazily, when tray-icon asks for the
/// context menu. So the initialisation has to happen before the BUILD, and doing
/// it first here covers both.
///
/// Initialising through the thin `gtk-sys` bindings would not work: the assertion
/// reads gtk-rs's own `INITIALIZED`/`IS_MAIN_THREAD` state, which only the `gtk`
/// crate itself sets.
///
/// `gtk::init` returns `Ok` immediately when the calling thread already
/// initialised it, so this is safe to call on every [`create_tray`] -- `resumed`
/// may rebuild a tray that was dropped.
#[cfg(target_os = "linux")]
fn init_platform_gui() -> Result<(), String> {
    gtk::init().map_err(|error| {
        format!(
            "Failed to initialise GTK for the tray icon: {error}. The Linux tray \
             needs a display server (DISPLAY or WAYLAND_DISPLAY) and the GTK 3 \
             runtime libraries."
        )
    })
}

/// Nothing to initialise on Windows or macOS: those tray backends need only the
/// event loop [`create_tray`]'s caller already runs. A named function rather than
/// a `cfg` block inside [`create_tray`] so the Linux requirement has one
/// documented home instead of an unexplained conditional mid-function.
#[cfg(not(target_os = "linux"))]
fn init_platform_gui() -> Result<(), String> {
    Ok(())
}

/// Create the system tray icon with context menu
pub fn create_tray() -> Result<TrayIcon, Box<dyn std::error::Error>> {
    // Before any widget exists: on Linux an uninitialised toolkit turns the whole
    // function into a panic instead of the `Err` its signature promises.
    init_platform_gui()?;

    let menu = Menu::new();

    let capture_item =
        MenuItem::with_id(MENU_CAPTURE, "Capture Screenshot (PrintScreen)", true, None);
    let capture_window_item = MenuItem::with_id(
        MENU_CAPTURE_WINDOW,
        "Capture Active Window (Alt+PrintScreen)",
        true,
        None,
    );
    let open_folder_item =
        MenuItem::with_id(MENU_OPEN_FOLDER, "Open Screenshots Folder", true, None);
    let settings_item = MenuItem::with_id(MENU_SETTINGS, "Settings", true, None);
    let exit_item = MenuItem::with_id(MENU_EXIT, "Exit", true, None);
    let start_record_item = MenuItem::with_id(MENU_START_RECORD, TOGGLE_RECORD_LABEL, true, None);

    menu.append(&capture_item)?;
    menu.append(&capture_window_item)?;
    menu.append(&open_folder_item)?;
    menu.append(&start_record_item)?;
    menu.append(&settings_item)?;
    menu.append(&exit_item)?;

    let icon = create_default_icon()?;

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("RustShot - Press PrintScreen to capture")
        .with_icon(icon)
        .build()?;

    Ok(tray)
}

/// Create a simple default icon (blue camera-like icon)
fn create_default_icon() -> Result<Icon, Box<dyn std::error::Error>> {
    // Create a simple 32x32 RGBA icon (blue square with white center)
    let rgba = build_default_icon_pixels();
    Icon::from_rgba(rgba, DEFAULT_ICON_SIZE, DEFAULT_ICON_SIZE).map_err(|e| e.into())
}

/// Expose icon pixel generation for deterministic benchmarks.
#[doc(hidden)]
#[allow(dead_code)]
pub fn benchmark_build_default_icon_pixels() -> Vec<u8> {
    build_default_icon_pixels()
}

/// Get the menu event receiver
pub fn menu_receiver() -> &'static tray_icon::menu::MenuEventReceiver {
    MenuEvent::receiver()
}

#[cfg(test)]
mod tests;
