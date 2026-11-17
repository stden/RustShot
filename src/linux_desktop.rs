//! GTK startup and event servicing for the Linux tray on the winit main thread.

/// Initialize GTK before constructing the tray menu or icon.
pub fn initialize() -> Result<(), String> {
    gtk::init().map_err(|error| format!("Failed to initialize GTK for the system tray: {error}"))
}

/// Process queued GTK events without blocking winit or its hotkey handling.
pub fn service_events() {
    while gtk::events_pending() {
        gtk::main_iteration_do(false);
    }
}
