use super::*;
use cucumber::{then, when, World};

#[derive(Default, World)]
struct TrayRuntimeWorld {
    tray: Option<tray_icon::TrayIcon>,
    #[cfg(target_os = "linux")]
    dispatched: std::rc::Rc<std::cell::Cell<bool>>,
}

impl std::fmt::Debug for TrayRuntimeWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrayRuntimeWorld")
            .field("tray_created", &self.tray.is_some())
            .finish()
    }
}

#[when("the native tray icon and menu are created")]
fn create_native_tray(world: &mut TrayRuntimeWorld) {
    world.tray = Some(create_tray().expect("create the native tray and its menu"));
    #[cfg(target_os = "linux")]
    {
        let dispatched = std::rc::Rc::clone(&world.dispatched);
        gtk::glib::idle_add_local_once(move || dispatched.set(true));
    }
}

#[then("the tray is available and queued native events are dispatched")]
fn assert_tray_runtime(world: &mut TrayRuntimeWorld) {
    assert!(world.tray.is_some());
    #[cfg(target_os = "linux")]
    {
        pump_platform_events();
        assert!(
            world.dispatched.get(),
            "GTK menu callbacks must not remain queued"
        );
    }
    assert!(std::ptr::eq(menu_receiver(), menu_receiver()));
}

pub(super) fn run() {
    // The existing native tray test calls this on its one GTK-owning thread.
    futures::executor::block_on(
        TrayRuntimeWorld::cucumber()
            .with_default_cli()
            .max_concurrent_scenarios(1)
            .fail_on_skipped()
            .run_and_exit("tests/features/in_process/tray_runtime.feature"),
    );
}
