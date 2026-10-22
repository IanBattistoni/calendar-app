use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::PathBuf,
    sync::{Mutex, atomic::AtomicBool},
};
use tauri::{AppHandle, Manager};

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Theme {
    #[default]
    Auto,
    Light,
    Dark,
}
#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Accent {
    #[default]
    Blue,
    Green,
    Purple,
    Orange,
}
#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CalendarSize {
    Compact,
    #[default]
    Normal,
    Large,
}
#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WidgetPosition {
    #[default]
    TopRight,
    TopLeft,
    BottomRight,
    BottomLeft,
}
#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WidgetView {
    Agenda,
    #[default]
    Next,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct Preferences {
    pub theme: Theme,
    pub accent: Accent,
    pub calendar_size: CalendarSize,
    pub first_day: u8,
    pub compact: bool,
    pub widget_enabled: bool,
    pub widget_on_top: bool,
    pub widget_position: WidgetPosition,
    pub widget_view: WidgetView,
    pub close_outside: bool,
    pub widget_expand: bool,
    pub widget_upcoming_count: u8,
    pub notifications_enabled: bool,
    pub catch_up_hours: u8,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            theme: Theme::Auto,
            accent: Accent::Blue,
            calendar_size: CalendarSize::Normal,
            first_day: 1,
            compact: false,
            widget_enabled: false,
            widget_on_top: false,
            widget_position: WidgetPosition::TopRight,
            widget_view: WidgetView::Next,
            close_outside: true,
            widget_expand: true,
            widget_upcoming_count: 5,
            notifications_enabled: true,
            catch_up_hours: 24,
        }
    }
}
pub struct AppState {
    pub preferences: Mutex<Preferences>,
    pub settings_open: AtomicBool,
    pub editor_open: AtomicBool,
    pub load_error: Option<String>,
}
fn path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("preferences.json"))
}
pub fn load(app: &AppHandle) -> Result<Preferences, String> {
    let path = path(app)?;
    match fs::read(path) {
        Ok(bytes) => {
            serde_json::from_slice(&bytes).map_err(|e| format!("Preferencias no válidas: {e}"))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Preferences::default()),
        Err(e) => Err(e.to_string()),
    }
}
pub fn save(app: &AppHandle, preferences: &Preferences) -> Result<(), String> {
    if !(3..=10).contains(&preferences.widget_upcoming_count)
        || ![0, 1, 24].contains(&preferences.catch_up_hours)
    {
        return Err("Preferencias de avisos no válidas".into());
    }
    if preferences.first_day > 1 {
        return Err("El primer día debe ser domingo o lunes".into());
    }
    let path = path(app)?;
    fs::create_dir_all(path.parent().ok_or("Ruta de configuración no válida")?)
        .map_err(|e| e.to_string())?;
    let temporary = path.with_extension("tmp");
    let bytes = serde_json::to_vec_pretty(preferences).map_err(|e| e.to_string())?;
    let mut file = fs::File::create(&temporary).map_err(|e| e.to_string())?;
    std::io::Write::write_all(&mut file, &bytes).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    fs::rename(temporary, path).map_err(|e| e.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn widget_is_opt_in_and_preferences_round_trip() {
        let p = Preferences::default();
        assert!(!p.widget_enabled);
        assert!(!p.widget_on_top);
        let restored: Preferences =
            serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
        assert_eq!(restored.first_day, 1);
    }
    #[test]
    fn rejects_unknown_theme() {
        assert!(serde_json::from_str::<Preferences>(r#"{"theme":"unsupported"}"#).is_err());
    }
}
