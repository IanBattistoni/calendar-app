use crate::{
    preferences::{self, AppState, Preferences},
    windows,
};
use serde::Serialize;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};

#[tauri::command]
pub fn set_editor_open(state: State<'_, AppState>, open: bool) {
    state.editor_open.store(open, Ordering::Relaxed);
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitialState {
    preferences: Preferences,
    load_error: Option<String>,
}
#[tauri::command]
pub fn get_initial_state(state: State<'_, AppState>) -> Result<InitialState, String> {
    Ok(InitialState {
        preferences: state.preferences.lock().map_err(|e| e.to_string())?.clone(),
        load_error: state.load_error.clone(),
    })
}
#[tauri::command]
pub async fn save_preferences(app: AppHandle, preferences: Preferences) -> Result<(), String> {
    let state = app.state::<AppState>();
    let previous = {
        let mut stored = state.preferences.lock().map_err(|e| e.to_string())?;
        let previous = stored.clone();
        preferences::save(&app, &preferences)?;
        *stored = preferences.clone();
        previous
    };
    let resize = if previous.calendar_size != preferences.calendar_size {
        windows::apply_panel_size(&app, &preferences)
    } else {
        Ok(())
    };
    if let Err(error) = resize.and_then(|_| {
        windows::sync_widget(
            &app,
            &preferences,
            previous.widget_position != preferences.widget_position
                || previous.widget_view != preferences.widget_view
                || !previous.widget_enabled,
        )
    }) {
        let mut stored = state.preferences.lock().map_err(|e| e.to_string())?;
        preferences::save(&app, &previous)?;
        *stored = previous.clone();
        drop(stored);
        let _ = windows::apply_panel_size(&app, &previous);
        let _ = windows::sync_widget(&app, &previous, true);
        return Err(error);
    }
    app.emit("preferences-changed", &preferences)
        .map_err(|e| e.to_string())?;
    crate::reminders::wake(&app);
    Ok(())
}
#[tauri::command]
pub async fn hide_panel(window: WebviewWindow) -> Result<(), String> {
    if window.label() != "main" {
        return Err("Acción disponible solo en el calendario".into());
    }
    window.hide().map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn set_panel_mode(app: AppHandle, settings: bool) -> Result<(), String> {
    let state = app.state::<AppState>();
    state.settings_open.store(settings, Ordering::Relaxed);
    let preferences = state.preferences.lock().map_err(|e| e.to_string())?.clone();
    windows::apply_panel_size(&app, &preferences)
}
#[tauri::command]
pub async fn open_calendar(app: AppHandle) -> Result<(), String> {
    windows::show_panel(&app, None)
}
#[tauri::command]
pub async fn open_event_in_calendar(app: AppHandle, id: String) -> Result<(), String> {
    let event = {
        let store = app.state::<crate::events::EventStore>();
        let path = store.0.lock().map_err(|e| e.to_string())?;
        crate::events::read(&path)?
            .into_iter()
            .find(|event| event.id == id)
            .ok_or("El evento ya no existe")?
    };
    windows::show_panel(&app, None)?;
    app.get_webview_window("main")
        .ok_or("Calendario no disponible")?
        .emit("edit-event", event)
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub fn start_widget_drag(window: WebviewWindow) -> Result<(), String> {
    if window.label() != "widget" {
        return Err("Acción disponible solo en el widget".into());
    }
    window.start_dragging().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn start_widget_resize(window: WebviewWindow, direction: String) -> Result<(), String> {
    if window.label() != "widget" { return Err("Acción disponible solo en el widget".into()); }
    if !matches!(direction.as_str(), "East" | "West") { return Err("Solo se puede ajustar el ancho".into()); }
    let direction = serde_json::from_value(serde_json::Value::String(direction)).map_err(|e| e.to_string())?;
    window.as_ref().window().start_resize_dragging(direction).map_err(|e| e.to_string())
}
