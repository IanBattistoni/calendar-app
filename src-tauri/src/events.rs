use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::PathBuf,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter, Manager, State};

pub struct EventStore(pub Mutex<PathBuf>);
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    Event,
    Birthday,
    Reminder,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CalendarEvent {
    pub id: String,
    pub title: String,
    pub date: String,
    pub time: Option<String>,
    pub description: String,
    pub kind: Kind,
    pub annual: bool,
    #[serde(default)]
    pub reminders: Vec<i64>,
    #[serde(default = "default_notify_at")]
    pub notify_at: String,
}
pub fn default_notify_at() -> String {
    "09:00".into()
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Occurrence {
    pub event: CalendarEvent,
    pub date: String,
}

fn leap(y: i32) -> bool {
    y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)
}
fn date_parts(s: &str) -> Result<(i32, u32, u32), String> {
    let bytes = s.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !bytes
            .iter()
            .enumerate()
            .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit())
    {
        return Err("Fecha no válida".into());
    }
    let y = s[0..4].parse::<i32>().map_err(|_| "Año no válido")?;
    let m = s[5..7].parse::<u32>().map_err(|_| "Mes no válido")?;
    let d = s[8..10].parse::<u32>().map_err(|_| "Día no válido")?;
    let max = match m {
        2 => {
            if leap(y) {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        _ => 0,
    };
    if !(1900..=9999).contains(&y) || d == 0 || d > max {
        return Err("Fecha no válida (años 1900–9999)".into());
    }
    Ok((y, m, d))
}
fn validate(event: &mut CalendarEvent) -> Result<(), String> {
    const ALLOWED: [i64; 10] = [0, 5, 15, 30, 60, 120, 1440, 2880, 4320, 10080];
    if event.reminders.len() > 10 || event.reminders.iter().any(|v| !ALLOWED.contains(v)) {
        return Err("Anticipación de aviso no válida".into());
    }
    event.reminders.sort_unstable();
    event.reminders.dedup();
    if event.notify_at.len() != 5
        || chrono::NaiveTime::parse_from_str(&event.notify_at, "%H:%M").is_err()
    {
        return Err("Hora de aviso no válida".into());
    }
    event.title = event.title.trim().to_owned();
    if event.title.is_empty() || event.title.chars().count() > 160 {
        return Err("Escribe un título de hasta 160 caracteres".into());
    }
    if event.description.chars().count() > 4000 {
        return Err("La descripción admite hasta 4000 caracteres".into());
    }
    date_parts(&event.date)?;
    if let Some(time) = &event.time {
        let b = time.as_bytes();
        if b.len() != 5
            || b[2] != b':'
            || ![b[0], b[1], b[3], b[4]].iter().all(u8::is_ascii_digit)
            || &time[..2] > "23"
            || &time[3..] > "59"
        {
            return Err("Hora no válida".into());
        }
    }
    if event.kind == Kind::Birthday {
        event.annual = true;
        event.time = None;
    }
    Ok(())
}
pub(crate) fn read(path: &PathBuf) -> Result<Vec<CalendarEvent>, String> {
    match fs::read(path) {
        Ok(bytes) => {
            let mut events: Vec<CalendarEvent> = serde_json::from_slice(&bytes).map_err(|e| {
                format!("No se pudieron leer los eventos: {e}. El archivo se conserva.")
            })?;
            for event in &mut events {
                validate(event)?;
            }
            Ok(events)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e.to_string()),
    }
}
fn write(path: &PathBuf, events: &[CalendarEvent]) -> Result<(), String> {
    fs::create_dir_all(path.parent().ok_or("Ruta de datos no válida")?)
        .map_err(|e| e.to_string())?;
    let temp = path.with_extension("tmp");
    let data = serde_json::to_vec(events).map_err(|e| e.to_string())?;
    let mut file = fs::File::create(&temp).map_err(|e| e.to_string())?;
    std::io::Write::write_all(&mut file, &data).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    fs::rename(temp, path).map_err(|e| e.to_string())
}
pub fn initialize(app: &AppHandle) -> Result<EventStore, String> {
    Ok(EventStore(Mutex::new(
        app.path()
            .app_data_dir()
            .map_err(|e| e.to_string())?
            .join("events.json"),
    )))
}
pub(crate) fn occurrences(
    events: Vec<CalendarEvent>,
    start: &str,
    end: &str,
) -> Result<Vec<Occurrence>, String> {
    let (first_year, _, _) = date_parts(start)?;
    let (last_year, _, _) = date_parts(end)?;
    if start > end || last_year - first_year > 2 {
        return Err("Intervalo de consulta no válido".into());
    }
    let mut result = Vec::new();
    for event in events {
        let (origin_year, m, d) = date_parts(&event.date)?;
        if event.annual {
            for year in first_year.max(origin_year)..=last_year {
                let day = if m == 2 && d == 29 && !leap(year) {
                    28
                } else {
                    d
                };
                let date = format!("{year:04}-{m:02}-{day:02}");
                if date.as_str() >= start && date.as_str() <= end {
                    result.push(Occurrence {
                        event: event.clone(),
                        date,
                    });
                }
            }
        } else if event.date.as_str() >= start && event.date.as_str() <= end {
            result.push(Occurrence {
                date: event.date.clone(),
                event,
            });
        }
    }
    result.sort_by(|a, b| {
        (&a.date, &a.event.time, &a.event.title, &a.event.id).cmp(&(
            &b.date,
            &b.event.time,
            &b.event.title,
            &b.event.id,
        ))
    });
    Ok(result)
}
#[tauri::command]
pub async fn list_occurrences(
    store: State<'_, EventStore>,
    start: String,
    end: String,
) -> Result<Vec<Occurrence>, String> {
    let path = store.0.lock().map_err(|e| e.to_string())?;
    occurrences(read(&path)?, &start, &end)
}
#[tauri::command]
pub async fn save_event(
    app: AppHandle,
    store: State<'_, EventStore>,
    mut event: CalendarEvent,
) -> Result<CalendarEvent, String> {
    validate(&mut event)?;
    {
        let path = store.0.lock().map_err(|e| e.to_string())?;
        let mut events = read(&path)?;
        if event.id.is_empty() {
            let id = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_nanos();
            event.id = format!("{id:x}");
            if events.iter().any(|e| e.id == event.id) {
                return Err("Vuelve a intentar guardar el evento".into());
            }
            events.push(event.clone());
        } else {
            let existing = events
                .iter_mut()
                .find(|e| e.id == event.id)
                .ok_or("El evento ya no existe")?;
            *existing = event.clone();
        }
        write(&path, &events)?;
    }
    let _ = app.emit("events-changed", ());
    crate::reminders::wake(&app);
    Ok(event)
}
#[tauri::command]
pub async fn delete_event(
    app: AppHandle,
    store: State<'_, EventStore>,
    id: String,
) -> Result<(), String> {
    {
        let path = store.0.lock().map_err(|e| e.to_string())?;
        let mut events = read(&path)?;
        let len = events.len();
        events.retain(|e| e.id != id);
        if events.len() == len {
            return Err("El evento ya no existe".into());
        }
        write(&path, &events)?;
    }
    let _ = app.emit("events-changed", ());
    crate::reminders::wake(&app);
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_events_keep_data_and_do_not_enable_unsolicited_alerts() {
        let event: CalendarEvent=serde_json::from_str(r#"{"id":"old","title":"Existente","date":"2026-10-21","time":null,"description":"Datos originales","kind":"event","annual":false}"#).unwrap();
        assert!(event.reminders.is_empty());
        assert_eq!(event.notify_at, "09:00");
        assert_eq!(event.description, "Datos originales");
    }
    fn event(date: &str) -> CalendarEvent {
        CalendarEvent {
            id: "test".into(),
            title: "Prueba".into(),
            date: date.into(),
            time: None,
            description: "".into(),
            kind: Kind::Birthday,
            annual: true,
            reminders: vec![],
            notify_at: default_notify_at(),
        }
    }
    #[test]
    fn validates_dates_times_and_title() {
        for date in ["2026-02-29", "2026-04-31", "2026-13-01", "éé-01-01"] {
            assert!(date_parts(date).is_err());
        }
        assert!(date_parts("2028-02-29").is_ok());
        let mut e = event("2026-10-21");
        e.kind = Kind::Event;
        e.time = Some("24:00".into());
        assert!(validate(&mut e).is_err());
        e.time = None;
        e.title = " ".into();
        assert!(validate(&mut e).is_err());
    }
    #[test]
    fn leap_birthdays_use_february_28_in_common_years() {
        let list = occurrences(vec![event("2024-02-29")], "2027-02-01", "2028-03-01").unwrap();
        assert_eq!(
            list.iter().map(|e| e.date.as_str()).collect::<Vec<_>>(),
            vec!["2027-02-28", "2028-02-29"]
        );
        assert!(
            occurrences(vec![event("2028-02-29")], "2027-01-01", "2027-12-31")
                .unwrap()
                .is_empty()
        );
    }
    #[test]
    fn writes_reopens_updates_and_removes_without_losing_other_events() {
        let dir = std::env::temp_dir().join(format!(
            "calendar-events-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = dir.join("events.json");
        let mut events = vec![event("2026-10-21"), event("2026-10-22")];
        events[1].id = "other".into();
        write(&path, &events).unwrap();
        assert_eq!(read(&path).unwrap().len(), 2);
        events[0].title = "Editado".into();
        write(&path, &events).unwrap();
        assert_eq!(read(&path).unwrap()[0].title, "Editado");
        events.remove(0);
        write(&path, &events).unwrap();
        assert_eq!(read(&path).unwrap()[0].id, "other");
        fs::write(&path, b"broken").unwrap();
        assert!(read(&path).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"broken");
        fs::remove_dir_all(dir).unwrap();
    }
}
