//! System tray icon and context menu.
//!
//! Creates the tray icon (a simple 32×32 camera glyph) and a right-click menu
//! with capture, folder, recording, settings, and exit commands.
//!
//! The tray runs inside the winit event loop managed by the app adapter.
//! Menu callbacks are forwarded through a `winit::EventLoopProxy`,
//! which wakes the otherwise sleeping event loop for immediate dispatch.
//!
//! The catalog describes commands; native hotkey registration and exhaustive
//! action dispatch stay in the app adapter, outside the data-only macro.
//!
//! ```mermaid
//! flowchart LR
//!     Catalog[Command catalog] --> Actions[MenuAction and stable IDs]
//!     Catalog --> Menu[Menu captions and order]
//!     Catalog --> Shortcuts[Typed shortcut metadata]
//!     Shortcuts --> Adapter[App hotkey adapter]
//! ```

use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    Icon, TrayIcon, TrayIconBuilder,
};

/// Supported keys, independent of the native registration library.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HotkeyKey {
    PrintScreen,
    R,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HotkeyModifiers {
    None,
    Alt,
    ControlShift,
}

/// A command's shortcut metadata. The app adapter converts it to a native hotkey
/// and uses the same label in registration errors that the menu displays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HotkeySpec {
    pub key: HotkeyKey,
    pub modifiers: HotkeyModifiers,
    pub label: &'static str,
}

// A data-only catalog: one row owns menu ordering, wire id, caption, and shortcut.
// Handlers deliberately stay in the app's ordinary exhaustive Rust match.
macro_rules! command_catalog {
    (
        $(#[$docs:meta])*
        $( $variant:ident => $constant:ident: $id:literal, $caption:literal,
           [$($shortcut:tt)*]; )+
    ) => {
        $(#[$docs])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum MenuAction { $($variant),+ }

        impl MenuAction {
            /// Every action, in its displayed menu order.
            pub const ALL: [MenuAction; [$(stringify!($variant)),+].len()] =
                [$(MenuAction::$variant),+];

            /// The stable id string handed to the OS for this entry.
            pub const fn id(self) -> &'static str {
                match self { $(Self::$variant => $id),+ }
            }

            pub(crate) const fn menu_label(self) -> &'static str {
                match self {
                    $(Self::$variant => command_catalog!(@caption $caption; [$($shortcut)*])),+
                }
            }

            pub(crate) const fn hotkey(self) -> Option<HotkeySpec> {
                match self {
                    $(Self::$variant => command_catalog!(@hotkey [$($shortcut)*])),+
                }
            }
        }

        $(pub const $constant: &str = MenuAction::$variant.id();)+
    };
    (@caption $caption:literal; []) => { $caption };
    (@caption $caption:literal; [$modifiers:ident, $key:ident, $label:literal]) => {
        concat!($caption, " (", $label, ")")
    };
    (@hotkey []) => { None };
    (@hotkey [$modifiers:ident, $key:ident, $label:literal]) => {
        Some(HotkeySpec {
            key: HotkeyKey::$key,
            modifiers: HotkeyModifiers::$modifiers,
            label: $label,
        })
    };
}

command_catalog! {
    /// What a tray menu entry does.
    ///
    /// This enum — not the raw id strings — is what the event loop dispatches on, so
    /// adding an entry is a compile error until it is handled. The `&str` ids below
    /// exist only because `muda` round-trips ids through the OS as strings; they are
    /// the wire format, and they are derived from this enum so the two cannot drift.
    ///
    /// Recording is a SINGLE [`MenuAction::ToggleRecord`] entry. A separate Stop
    /// item used to exist but was removed as a misleading affordance: the tray menu
    /// is built once at startup and cannot reflect live recording state, so a "Stop"
    /// entry appeared enabled even when nothing was recording. The legacy
    /// `"stop_record"` id is still *accepted* by [`MenuAction::from_id`] and routes
    /// to the same toggle.
    Capture => MENU_CAPTURE: "capture", "Capture Screenshot",
        [None, PrintScreen, "PrintScreen"];
    CaptureWindow => MENU_CAPTURE_WINDOW: "capture_window", "Capture Active Window",
        [Alt, PrintScreen, "Alt+PrintScreen"];
    OpenFolder => MENU_OPEN_FOLDER: "open_folder", "Open Screenshots Folder", [];
    ToggleRecord => MENU_START_RECORD: "start_record", "Start or Stop Recording",
        [ControlShift, R, "Ctrl+Shift+R"];
    Settings => MENU_SETTINGS: "settings", "Settings", [];
    Exit => MENU_EXIT: "exit", "Exit", [];
}

impl MenuAction {
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

/// Legacy id from the old two-entry recording menu. Still accepted by
/// [`MenuAction::from_id`], which routes it to [`MenuAction::ToggleRecord`].
pub const MENU_STOP_RECORD: &str = "stop_record";
const DEFAULT_ICON_SIZE: u32 = 32;

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

            if !in_body_y || dist_x > 12 {
                continue; // Outside the camera body stays transparent.
            }
            let dist_sq = dist_x * dist_x + dist_y_sq;
            let color = if dist_sq <= LENS_RADIUS_SQ {
                [255, 255, 255, 255] // White lens.
            } else {
                [41, 128, 185, 255] // Blue camera body.
            };
            rgba[idx..idx + 4].copy_from_slice(&color);
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

    for action in MenuAction::ALL {
        let item = MenuItem::with_id(action.id(), action.menu_label(), true, None);
        menu.append(&item)?;
    }

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
pub fn benchmark_build_default_icon_pixels() -> Vec<u8> {
    build_default_icon_pixels()
}

/// Get the menu event receiver
pub fn menu_receiver() -> &'static tray_icon::menu::MenuEventReceiver {
    MenuEvent::receiver()
}

#[cfg(test)]
#[path = "../tests/unit/tray/mod.rs"]
mod tests;
