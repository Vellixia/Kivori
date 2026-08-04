//! Window + tray + single-instance wiring over the pure [`crate::window_lifecycle`] policy (FR-030,
//! SC-007). The policy decides hide-vs-quit / show-on-reactivate (and is unit-tested standalone); this
//! module only performs the resulting OS action and keeps the device thread alive across hide/close.

use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Window, WindowEvent};

use crate::runtime::state::AppState;
use crate::window_lifecycle::{LifecycleAction, LifecycleEvent};

const MAIN: &str = "main";

/// Builder hook: a window close request hides the window (the device thread keeps running); the pure
/// policy makes the decision.
pub fn on_window_event(window: &Window, event: &WindowEvent) {
    if let WindowEvent::CloseRequested { api, .. } = event {
        api.prevent_close();
        perform(window, LifecycleEvent::CloseRequested);
    }
}

/// Installs the tray icon with Open/Quit actions.
///
/// # Errors
/// Propagates Tauri errors while building the menu or tray icon.
pub fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open Kivori", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;
    TrayIconBuilder::new()
        .icon(
            app.default_window_icon()
                .expect("bundled default icon")
                .clone(),
        )
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main(app),
            "quit" => quit_app(app),
            _ => {}
        })
        .build(app)?;
    Ok(())
}

/// Shows + focuses the single main window (dock/taskbar reactivate, tray Open, or a second instance).
pub fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN) {
        let _ = window.show();
        let _ = window.set_focus();
        let _ = window
            .state::<AppState>()
            .lifecycle
            .lock()
            .expect("lifecycle lock")
            .on_event(LifecycleEvent::Reactivated);
    }
}

fn quit_app(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN) {
        window.state::<AppState>().shutdown();
    }
    app.exit(0);
}

fn perform(window: &Window, event: LifecycleEvent) {
    let action = {
        let state = window.state::<AppState>();
        let mut policy = state.lifecycle.lock().expect("lifecycle lock");
        policy.on_event(event)
    };
    match action {
        LifecycleAction::HideWindow => {
            let _ = window.hide();
        }
        LifecycleAction::ShowWindow => {
            let _ = window.show();
            let _ = window.set_focus();
        }
        LifecycleAction::Quit => quit_app(window.app_handle()),
        LifecycleAction::None => {}
    }
}
