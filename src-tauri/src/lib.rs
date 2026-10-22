mod commands;
mod events;
mod icon;
mod preferences;
mod reminders;
mod system_events;
mod windows;

use preferences::AppState;
use std::sync::{Mutex, atomic::AtomicBool};
use tauri::{
    Manager,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Err(error) = windows::show_panel(app, None) {
                eprintln!("{error}");
            }
        }))
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            app.manage(windows::WidgetLayout::default());
            app.manage(events::initialize(app.handle()).map_err(std::io::Error::other)?);
            let (preferences, load_error) = match preferences::load(app.handle()) {
                Ok(p) => (p, None),
                Err(e) => (preferences::Preferences::default(), Some(e)),
            };
            app.manage(AppState {
                preferences: Mutex::new(preferences.clone()),
                settings_open: AtomicBool::new(false),
                editor_open: AtomicBool::new(false),
                load_error,
            });
            reminders::start(app.handle()).map_err(std::io::Error::other)?;
            let open = MenuItem::with_id(app, "open", "Abrir calendario", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Salir de Calendario", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &quit])?;
            let tray = TrayIconBuilder::with_id("calendar")
                .tooltip("Calendario")
                .icon(icon::calendar_icon())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => {
                        if let Err(e) = windows::show_panel(app, None) {
                            eprintln!("{e}");
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        position,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            // A focus-loss click on the tray may already hide the panel; always open here.
                            if let Err(e) = windows::show_panel(app, Some((position.x, position.y)))
                            {
                                eprintln!("{e}");
                            }
                            let _ = window;
                        }
                    }
                });
            tray.build(app)?;
            windows::apply_panel_size(app.handle(), &preferences)?;
            windows::sync_widget(app.handle(), &preferences, true)?;
            system_events::install(app.handle()).map_err(std::io::Error::other)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }
            match event {
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    if let Err(e) = window.hide() {
                        eprintln!("{e}");
                    }
                }
                tauri::WindowEvent::Focused(false) => {
                    windows::hide_after_focus_loss(window.app_handle().clone());
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_initial_state,
            commands::save_preferences,
            commands::hide_panel,
            commands::set_panel_mode,
            commands::open_calendar,
            commands::open_event_in_calendar,
            commands::start_widget_drag,
            events::list_occurrences,
            events::save_event,
            events::delete_event,
            commands::set_editor_open,
            reminders::get_upcoming,
            reminders::test_notification,
            reminders::notification_status,
            windows::set_widget_expanded
        ])
        .run(tauri::generate_context!())
        .expect("error al ejecutar Calendario");
}
