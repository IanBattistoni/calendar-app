use crate::{
    events::{self, CalendarEvent, EventStore, Occurrence},
    preferences::AppState,
};
use chrono::{
    DateTime, Duration as ChronoDuration, Local, NaiveDate, NaiveDateTime, TimeZone, Utc,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    fs,
    path::PathBuf,
    sync::{
        Mutex,
        mpsc::{self, SyncSender},
    },
    time::Duration,
};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

pub struct ReminderEngine {
    sender: SyncSender<()>,
    pub error: Mutex<Option<String>>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Upcoming {
    pub event: CalendarEvent,
    pub date: String,
    pub event_at: i64,
    pub remind_at: Option<i64>,
    pub expires_at: i64,
}
#[derive(Clone, Debug)]
struct Alert {
    key: String,
    occurrence: String,
    due: i64,
    title: String,
    body: String,
}
#[derive(Clone, Deserialize, Serialize)]
struct Delivery {
    at: i64,
    state: String,
}
type Ledger = BTreeMap<String, Delivery>;

// Ambiguous times use the first occurrence. A skipped local time moves forward
// to the first valid minute, e.g. midnight skipped by a daylight-saving change.
fn local_instant(naive: NaiveDateTime) -> Result<DateTime<Local>, String> {
    for minutes in 0..=180 {
        let value = naive
            .checked_add_signed(ChronoDuration::minutes(minutes))
            .ok_or("Fecha fuera de rango")?;
        if let Some(time) = Local.from_local_datetime(&value).earliest() {
            return Ok(time);
        }
    }
    Err("No se pudo resolver la hora local".into())
}
fn at(date: &str, time: &str) -> Result<DateTime<Local>, String> {
    local_instant(
        NaiveDateTime::parse_from_str(&format!("{date} {time}"), "%Y-%m-%d %H:%M")
            .map_err(|e| e.to_string())?,
    )
}
fn occurrence_times(item: &Occurrence) -> Result<(i64, i64), String> {
    let start = at(
        &item.date,
        item.event.time.as_deref().unwrap_or(&item.event.notify_at),
    )?
    .timestamp_millis();
    let expires = if item.event.time.is_some() {
        start + 60_000
    } else {
        let next = NaiveDate::parse_from_str(&item.date, "%Y-%m-%d")
            .map_err(|e| e.to_string())?
            .succ_opt()
            .ok_or("Fecha fuera de rango")?;
        local_instant(next.and_hms_opt(0, 0, 0).ok_or("Hora no válida")?)?.timestamp_millis()
    };
    Ok((start, expires))
}
fn build_alerts(items: &[Occurrence]) -> Result<Vec<Alert>, String> {
    let mut alerts = Vec::new();
    for item in items {
        let (event_at, _) = occurrence_times(item)?;
        for minutes in &item.event.reminders {
            // Day-sized leads follow civil days, preserving the configured local
            // hour across daylight-saving boundaries. Short leads are durations.
            let due = if *minutes >= 1440 && minutes % 1440 == 0 {
                let date = NaiveDate::parse_from_str(&item.date, "%Y-%m-%d")
                    .map_err(|e| e.to_string())?
                    .checked_sub_signed(ChronoDuration::days(minutes / 1440))
                    .ok_or("Fecha fuera de rango")?;
                at(
                    &date.to_string(),
                    item.event.time.as_deref().unwrap_or(&item.event.notify_at),
                )?
                .timestamp_millis()
            } else {
                event_at - minutes * 60_000
            };
            alerts.push(Alert {
                key: format!("{}|{}|{}|{}", item.event.id, item.date, event_at, minutes),
                occurrence: format!("{}|{}", item.event.id, item.date),
                due,
                title: item.event.title.clone(),
                body: format!(
                    "{} · {}{}",
                    item.date,
                    item.event.time.as_deref().unwrap_or("Todo el día"),
                    if *minutes == 0 {
                        " · Es el momento"
                    } else {
                        " · Recordatorio anticipado"
                    }
                ),
            });
        }
    }
    alerts.sort_by_key(|a| a.due);
    Ok(alerts)
}
fn due_alerts(alerts: &[Alert], ledger: &Ledger, now: i64, catch_up_hours: u8) -> Vec<Alert> {
    let lower = now - (i64::from(catch_up_hours) * 3_600_000).max(60_000);
    let mut latest: HashMap<&str, &Alert> = HashMap::new();
    for alert in alerts.iter().filter(|a| a.due <= now && a.due >= lower) {
        let entry = latest.entry(&alert.occurrence).or_insert(alert);
        if alert.due > entry.due {
            *entry = alert;
        }
    }
    let mut due: Vec<Alert> = latest
        .values()
        .filter(|a| !ledger.contains_key(&a.key))
        .map(|a| (*a).clone())
        .collect();
    due.sort_by_key(|a| a.due);
    due
}
fn ledger_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("notification-history.json"))
}
fn load_ledger(path: &PathBuf) -> Result<Ledger, String> {
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|e| format!("Historial de avisos no válido; se conserva el archivo: {e}")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(BTreeMap::new()),
        Err(e) => Err(e.to_string()),
    }
}
fn save_ledger(path: &PathBuf, ledger: &Ledger) -> Result<(), String> {
    fs::create_dir_all(path.parent().ok_or("Ruta de avisos no válida")?)
        .map_err(|e| e.to_string())?;
    let temporary = path.with_extension("tmp");
    let mut file = fs::File::create(&temporary).map_err(|e| e.to_string())?;
    std::io::Write::write_all(
        &mut file,
        &serde_json::to_vec(ledger).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    fs::rename(temporary, path).map_err(|e| e.to_string())
}
fn query_items(app: &AppHandle, now: DateTime<Local>) -> Result<Vec<Occurrence>, String> {
    let store = app.state::<EventStore>();
    let path = store.0.lock().map_err(|e| e.to_string())?;
    let first = now.date_naive() - ChronoDuration::days(2);
    let last = now.date_naive() + ChronoDuration::days(374);
    events::occurrences(events::read(&path)?, &first.to_string(), &last.to_string())
}
#[tauri::command]
pub async fn get_upcoming(app: AppHandle) -> Result<Vec<Upcoming>, String> {
    let now = Local::now();
    let enabled = app
        .state::<AppState>()
        .preferences
        .lock()
        .map_err(|e| e.to_string())?
        .notifications_enabled;
    let items = query_items(&app, now)?;
    let alerts = build_alerts(&items)?;
    let mut result = Vec::new();
    for item in items {
        let (event_at, expires_at) = occurrence_times(&item)?;
        if expires_at <= now.timestamp_millis() {
            continue;
        }
        let occurrence = format!("{}|{}", item.event.id, item.date);
        let remind_at = alerts
            .iter()
            .find(|a| enabled && a.occurrence == occurrence && a.due > now.timestamp_millis())
            .map(|a| a.due);
        result.push(Upcoming {
            event: item.event,
            date: item.date,
            event_at,
            remind_at,
            expires_at,
        });
    }
    result.sort_by_key(|a| {
        (
            a.remind_at.unwrap_or(a.event_at),
            a.event_at,
            a.event.id.clone(),
        )
    });
    result.truncate(50);
    Ok(result)
}
#[tauri::command]
pub async fn notification_status(app: AppHandle) -> Result<Option<String>, String> {
    match app.try_state::<ReminderEngine>() {
        Some(state) => Ok(state.error.lock().map_err(|e| e.to_string())?.clone()),
        None => Ok(None),
    }
}
#[tauri::command]
pub async fn test_notification(app: AppHandle) -> Result<(), String> {
    app.notification()
        .builder()
        .title("Calendario")
        .body("Los avisos de Calendario están conectados con Windows.")
        .show()
        .map_err(|e| e.to_string())
}
pub fn wake(app: &AppHandle) {
    if let Some(engine) = app.try_state::<ReminderEngine>() {
        let _ = engine.sender.try_send(());
    }
}
fn set_error(app: &AppHandle, error: Option<String>) {
    if let Some(engine) = app.try_state::<ReminderEngine>() {
        if let Ok(mut previous) = engine.error.lock() {
            if *previous != error {
                *previous = error.clone();
                let _ = app.emit("notification-status", &error);
            }
        }
    }
}
fn process(app: &AppHandle) -> Result<i64, String> {
    let now = Local::now();
    let timestamp = now.timestamp_millis();
    let app_state = app.state::<AppState>();
    let preferences = app_state.preferences.lock().map_err(|e| e.to_string())?;
    // Hold the event store through claiming/delivery so a delete/edit cannot race
    // a stale notification after the edit command has returned.
    let store = app.state::<EventStore>();
    let event_path = store.0.lock().map_err(|e| e.to_string())?;
    let items = events::occurrences(
        events::read(&event_path)?,
        &(now.date_naive() - ChronoDuration::days(2)).to_string(),
        &(now.date_naive() + ChronoDuration::days(374)).to_string(),
    )?;
    let alerts = build_alerts(&items)?;
    let next_midnight = local_instant(
        (now.date_naive() + ChronoDuration::days(1))
            .and_hms_opt(0, 0, 0)
            .ok_or("Fecha fuera de rango")?,
    )?
    .timestamp_millis();
    let mut next = items
        .iter()
        .filter_map(|item| occurrence_times(item).ok())
        .flat_map(|(start, end)| [start, end])
        .filter(|time| *time > timestamp)
        .min()
        .unwrap_or(next_midnight)
        .min(next_midnight);
    if preferences.notifications_enabled {
        let path = ledger_path(app)?;
        let mut ledger = load_ledger(&path)?;
        if let Some(time) = alerts
            .iter()
            .filter(|a| a.due > timestamp && !ledger.contains_key(&a.key))
            .map(|a| a.due)
            .min()
        {
            next = next.min(time);
        }
        for alert in due_alerts(&alerts, &ledger, timestamp, preferences.catch_up_hours) {
            // Persist before handing off to Windows. After an abrupt crash an
            // uncertain hand-off is not replayed (at-most-once policy).
            ledger.insert(
                alert.key.clone(),
                Delivery {
                    at: Utc::now().timestamp_millis(),
                    state: "claimed".into(),
                },
            );
            save_ledger(&path, &ledger)?;
            match app
                .notification()
                .builder()
                .title(&alert.title)
                .body(&alert.body)
                .show()
            {
                Ok(()) => {
                    if let Some(delivery) = ledger.get_mut(&alert.key) {
                        delivery.state = "submitted".into();
                    }
                    save_ledger(&path, &ledger)?;
                }
                Err(e) => {
                    ledger.remove(&alert.key);
                    save_ledger(&path, &ledger)?;
                    return Err(format!("Windows no aceptó el aviso: {e}"));
                }
            }
        }
    }
    Ok(next)
}
pub fn start(app: &AppHandle) -> Result<(), String> {
    let (sender, receiver) = mpsc::sync_channel(1);
    app.manage(ReminderEngine {
        sender,
        error: Mutex::new(None),
    });
    let handle = app.clone();
    std::thread::Builder::new()
        .name("calendar-reminders".into())
        .spawn(move || {
            loop {
                let wait = match process(&handle) {
                    Ok(next) => {
                        set_error(&handle, None);
                        Duration::from_millis((next - Utc::now().timestamp_millis()).max(50) as u64)
                    }
                    Err(error) => {
                        set_error(&handle, Some(error));
                        Duration::from_secs(60)
                    }
                };
                let _ = handle.emit("upcoming-changed", ());
                match receiver.recv_timeout(wait) {
                    Ok(()) | Err(mpsc::RecvTimeoutError::Timeout) => {}
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::Kind;
    fn item() -> Occurrence {
        Occurrence {
            date: "2028-02-29".into(),
            event: CalendarEvent {
                id: "a".into(),
                title: "Cumpleaños".into(),
                date: "2000-02-29".into(),
                time: None,
                description: "".into(),
                kind: Kind::Birthday,
                annual: true,
                reminders: vec![4320, 2880, 1440, 0],
                notify_at: "09:00".into(),
            },
        }
    }
    #[test]
    fn offsets_and_latest_catchup_are_not_duplicated() {
        let alerts = build_alerts(&[item()]).unwrap();
        assert_eq!(alerts.len(), 4);
        let now = alerts[3].due + 1000;
        let due = due_alerts(&alerts, &Ledger::new(), now, 24);
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].key, alerts[3].key);
        let mut ledger = Ledger::new();
        ledger.insert(
            due[0].key.clone(),
            Delivery {
                at: now,
                state: "claimed".into(),
            },
        );
        assert!(due_alerts(&alerts, &ledger, now, 24).is_empty());
        assert!(due_alerts(&alerts, &Ledger::new(), now + 25 * 3_600_000, 24).is_empty());
    }
    #[test]
    fn edits_cancel_old_deadlines_and_legacy_events_have_no_alerts() {
        let mut occurrence = item();
        let old = build_alerts(&[occurrence.clone()]).unwrap();
        occurrence.event.time = Some("11:00".into());
        let new = build_alerts(&[occurrence.clone()]).unwrap();
        assert_ne!(old[0].key, new[0].key);
        occurrence.event.reminders.clear();
        assert!(build_alerts(&[occurrence]).unwrap().is_empty());
        assert!(build_alerts(&[]).unwrap().is_empty());
    }
    #[test]
    fn ledger_roundtrip_keeps_claims_after_restart() {
        let path =
            std::env::temp_dir().join(format!("calendar-history-{}.json", std::process::id()));
        let mut ledger = Ledger::new();
        ledger.insert(
            "a|2028-02-29|0".into(),
            Delivery {
                at: 123,
                state: "claimed".into(),
            },
        );
        save_ledger(&path, &ledger).unwrap();
        assert_eq!(
            load_ledger(&path).unwrap()["a|2028-02-29|0"].state,
            "claimed"
        );
        fs::remove_file(path).unwrap();
    }
}
