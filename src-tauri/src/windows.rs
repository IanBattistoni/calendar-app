use crate::preferences::{AppState, CalendarSize, Preferences, WidgetPosition, WidgetView};
use std::sync::Mutex;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, LogicalSize, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder};

// Matches the transparent inset around #app in the widget stylesheet.
const WIDGET_SHADOW_INSET: f64 = 12.0;

#[derive(Default)]
pub struct WidgetLayout {
    collapsed: Mutex<Option<tauri::PhysicalSize<u32>>>,
    pub dragging: std::sync::atomic::AtomicBool,
}
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetMetrics {
    collapsed_height: f64,
    expanded_height: f64,
    viewport_height: f64,
}
fn anchored_height(
    requested: u32,
    collapsed: u32,
    y: i32,
    bottom: i32,
    border: u32,
    margin: i32,
) -> u32 {
    let available = (bottom as i64 - y as i64 - border as i64 - margin as i64).max(0) as u32;
    requested.min(available).max(collapsed)
}
#[tauri::command]
pub async fn set_widget_expanded(
    window: tauri::WebviewWindow,
    expanded: bool,
    metrics: Option<WidgetMetrics>,
) -> Result<(), String> {
    if window.label() != "widget" {
        return Err("Acción disponible solo en el widget".into());
    }
    tauri::async_runtime::spawn_blocking(move || resize_widget(&window, expanded, metrics))
        .await
        .map_err(|e| e.to_string())?
}
// A single native bounds update avoids intermediate WebView resizes and flicker.
fn place_window(
    window: &tauri::WebviewWindow,
    position: PhysicalPosition<i32>,
    size: tauri::PhysicalSize<u32>,
) -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            SWP_NOACTIVATE, SWP_NOZORDER, SetWindowPos,
        };
        let outer = window.outer_size().map_err(|e| e.to_string())?;
        let inner = window.inner_size().map_err(|e| e.to_string())?;
        let hwnd = window.hwnd().map_err(|e| e.to_string())?;
        let ok = unsafe {
            SetWindowPos(
                hwnd.0 as _,
                std::ptr::null_mut(),
                position.x,
                position.y,
                (size.width + outer.width.saturating_sub(inner.width)) as i32,
                (size.height + outer.height.saturating_sub(inner.height)) as i32,
                SWP_NOACTIVATE | SWP_NOZORDER,
            )
        };
        if ok == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
    }
    #[cfg(not(windows))]
    {
        window.set_size(size).map_err(|e| e.to_string())?;
        window.set_position(position).map_err(|e| e.to_string())?;
    }
    Ok(())
}
// Hover changes size only. In particular, SWP_NOMOVE prevents Windows from
// moving the card when the dropdown is close to a monitor edge.
fn resize_widget(
    window: &tauri::WebviewWindow,
    expanded: bool,
    metrics: Option<WidgetMetrics>,
) -> Result<(), String> {
    let app = window.app_handle();
    let p = app
        .state::<AppState>()
        .preferences
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    let expanded = expanded && p.widget_expand && matches!(p.widget_view, WidgetView::Next);
    let layout = app.state::<WidgetLayout>();
    if layout.dragging.load(Ordering::Relaxed) {
        return Ok(());
    }
    let mut saved = layout.collapsed.lock().map_err(|e| e.to_string())?;
    let current = window.inner_size().map_err(|e| e.to_string())?;
    let outer = window.outer_size().map_err(|e| e.to_string())?;
    let (collapsed_height, requested) = if let Some(metrics) = metrics {
        if !metrics.collapsed_height.is_finite()
            || !metrics.expanded_height.is_finite()
            || !metrics.viewport_height.is_finite()
            || !(1.0..=20000.0).contains(&metrics.viewport_height)
            || !(24.0..=2000.0).contains(&metrics.collapsed_height)
            || !(metrics.collapsed_height..=4000.0).contains(&metrics.expanded_height)
        {
            return Err("Medidas del widget no válidas".into());
        }
        // The WebView's CSS scale can differ from the monitor scale (zoom,
        // accessibility text sizing). Measure it rather than guess a fixed height.
        let ratio = current.height as f64 / metrics.viewport_height;
        let collapsed = (metrics.collapsed_height * ratio).ceil() as u32;
        let requested = (if expanded {
            metrics.expanded_height
        } else {
            metrics.collapsed_height
        } * ratio)
            .ceil() as u32;
        (collapsed, requested)
    } else {
        let collapsed = saved.unwrap_or(current).height;
        (collapsed, collapsed)
    };
    let height = if expanded {
        *saved = Some(tauri::PhysicalSize::new(current.width, collapsed_height));
        let position = window.outer_position().map_err(|e| e.to_string())?;
        if let Some(monitor) = window.current_monitor().map_err(|e| e.to_string())? {
            anchored_height(
                requested,
                collapsed_height,
                position.y,
                monitor.work_area().position.y + monitor.work_area().size.height as i32,
                outer.height.saturating_sub(current.height),
                0,
            )
        } else {
            requested
        }
    } else {
        *saved = None;
        collapsed_height
    };
    if height == current.height {
        return Ok(());
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOZORDER, SetWindowPos,
        };
        let hwnd = window.hwnd().map_err(|e| e.to_string())?;
        let ok = unsafe {
            SetWindowPos(
                hwnd.0 as _,
                std::ptr::null_mut(),
                0,
                0,
                outer.width as i32,
                (height + outer.height.saturating_sub(current.height)) as i32,
                SWP_NOMOVE | SWP_NOACTIVATE | SWP_NOZORDER,
            )
        };
        if ok == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
    }
    #[cfg(not(windows))]
    {
        window
            .set_size(tauri::PhysicalSize::new(current.width, height))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn panel_size(p: &Preferences) -> (f64, f64) {
    match p.calendar_size {
        CalendarSize::Compact => (336.0, 540.0),
        CalendarSize::Normal => (380.0, 590.0),
        CalendarSize::Large => (428.0, 640.0),
    }
}
// Calculate against the chosen monitor before changing the window. Resizing
// first can make Windows relocate it and change current_monitor mid-transition.
fn panel_bounds(
    area: &tauri::PhysicalRect<i32, u32>,
    scale: f64,
    logical_size: (f64, f64),
    border: tauri::PhysicalSize<u32>,
) -> (PhysicalPosition<i32>, tauri::PhysicalSize<u32>) {
    let margin = (12.0 * scale).round() as u32;
    let available_width = area
        .size
        .width
        .saturating_sub(2 * margin + border.width)
        .max(1);
    let available_height = area
        .size
        .height
        .saturating_sub(2 * margin + border.height)
        .max(1);
    let inner = tauri::PhysicalSize::new(
        ((logical_size.0 * scale).round() as u32).min(available_width),
        ((logical_size.1 * scale).round() as u32).min(available_height),
    );
    let x = area.position.x
        + (area.size.width as i64 - inner.width as i64 - border.width as i64 - margin as i64).max(0)
            as i32;
    let y = area.position.y
        + (area.size.height as i64 - inner.height as i64 - border.height as i64 - margin as i64)
            .max(0) as i32;
    (PhysicalPosition::new(x, y), inner)
}
fn layout_panel(
    app: &AppHandle,
    p: &Preferences,
    monitor_point: Option<(f64, f64)>,
) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or("No se encontró el calendario")?;
    let monitor = match monitor_point {
        Some((x, y)) => app.monitor_from_point(x, y).map_err(|e| e.to_string())?,
        None => window.current_monitor().map_err(|e| e.to_string())?,
    }
    .or(app.primary_monitor().map_err(|e| e.to_string())?);
    if let Some(monitor) = monitor {
        let settings = app
            .state::<AppState>()
            .settings_open
            .load(Ordering::Relaxed);
        let size = if settings {
            (720.0, 620.0)
        } else {
            panel_size(p)
        };
        let outer = window.outer_size().map_err(|e| e.to_string())?;
        let inner = window.inner_size().map_err(|e| e.to_string())?;
        let border = tauri::PhysicalSize::new(
            outer.width.saturating_sub(inner.width),
            outer.height.saturating_sub(inner.height),
        );
        let (position, size) =
            panel_bounds(monitor.work_area(), monitor.scale_factor(), size, border);
        place_window(&window, position, size)?;
    }
    Ok(())
}
pub fn show_panel(app: &AppHandle, anchor: Option<(f64, f64)>) -> Result<(), String> {
    let preferences = app
        .state::<AppState>()
        .preferences
        .lock()
        .map_err(|e| e.to_string())?
        .clone();
    layout_panel(app, &preferences, anchor)?;
    let window = app
        .get_webview_window("main")
        .ok_or("No se encontró el calendario")?;
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())
}
pub fn apply_panel_size(app: &AppHandle, p: &Preferences) -> Result<(), String> {
    layout_panel(app, p, None)
}
fn show_widget_without_focus(window: &tauri::WebviewWindow) -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{SW_SHOWNOACTIVATE, ShowWindow};
        let hwnd = window.hwnd().map_err(|e| e.to_string())?;
        unsafe {
            ShowWindow(hwnd.0 as _, SW_SHOWNOACTIVATE);
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        window.show().map_err(|e| e.to_string())
    }
}
fn panel_has_focus(window: &tauri::WebviewWindow) -> bool {
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GA_ROOTOWNER, GetAncestor, GetForegroundWindow, IsChild,
        };
        let Ok(hwnd) = window.hwnd() else {
            return false;
        };
        let hwnd = hwnd.0 as _;
        // Native select popups belong to this panel; they are not an outside click.
        unsafe {
            let foreground = GetForegroundWindow();
            foreground == hwnd
                || IsChild(hwnd, foreground) != 0
                || GetAncestor(foreground, GA_ROOTOWNER) == hwnd
        }
    }
    #[cfg(not(windows))]
    {
        window.is_focused().unwrap_or(false)
    }
}
fn should_hide_panel(
    close_outside: bool,
    editor_open: bool,
    visible: bool,
    has_focus: bool,
) -> bool {
    close_outside && !editor_open && visible && !has_focus
}
pub fn hide_after_focus_loss(app: AppHandle) {
    // Recheck once after focus settles, including when a widget or an owned
    // native control is being created. This is not a recurring polling loop.
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(100));
        let Some(window) = app.get_webview_window("main") else {
            return;
        };
        let state = app.state::<AppState>();
        let close_outside = state
            .preferences
            .lock()
            .map(|p| p.close_outside)
            .unwrap_or(false);
        if should_hide_panel(
            close_outside,
            state.editor_open.load(Ordering::Relaxed),
            window.is_visible().unwrap_or(false),
            panel_has_focus(&window),
        ) {
            let _ = window.hide();
        }
    });
}
pub fn sync_widget(app: &AppHandle, p: &Preferences, reposition: bool) -> Result<(), String> {
    if !p.widget_enabled {
        if let Some(window) = app.get_webview_window("widget") {
            window.destroy().map_err(|e| e.to_string())?;
        }
        *app.state::<WidgetLayout>()
            .collapsed
            .lock()
            .map_err(|e| e.to_string())? = None;
        return Ok(());
    }
    let (content_width, content_height) = match p.widget_view {
        WidgetView::Agenda => (420.0, 500.0),
        WidgetView::Next => (403.0, 116.0),
    };
    let width = content_width + WIDGET_SHADOW_INSET * 2.0;
    let height = content_height + WIDGET_SHADOW_INSET * 2.0;
    let existed = app.get_webview_window("widget").is_some();
    let window = match app.get_webview_window("widget") {
        Some(window) => window,
        None => WebviewWindowBuilder::new(
            app,
            "widget",
            WebviewUrl::App("index.html?view=widget".into()),
        )
        .title("Calendario · Widget")
        .inner_size(width, height)
        .min_inner_size(280.0, 80.0)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .resizable(false)
        .skip_taskbar(true)
        .visible(false)
        .focused(false)
        .build()
        .map_err(|e| e.to_string())?,
    };
    #[cfg(windows)]
    if !existed {
        // Recalculate the borderless client frame before measuring it. Without
        // this, Windows can report caption insets left over from creation and
        // enlarge the transparent widget on its first resize.
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SetWindowPos,
        };
        let hwnd = window.hwnd().map_err(|e| e.to_string())?;
        if unsafe {
            SetWindowPos(
                hwnd.0 as _,
                std::ptr::null_mut(),
                0,
                0,
                0,
                0,
                SWP_FRAMECHANGED | SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER,
            )
        } == 0
        {
            return Err(std::io::Error::last_os_error().to_string());
        }
    }
    window
        .set_always_on_top(p.widget_on_top)
        .map_err(|e| e.to_string())?;
    if !reposition && existed {
        if !p.widget_expand || matches!(p.widget_view, WidgetView::Agenda) {
            resize_widget(&window, false, None)?;
        }
        return Ok(());
    }
    resize_widget(&window, false, None)?;
    window
        .set_size(LogicalSize::new(width, height))
        .map_err(|e| e.to_string())?;
    // Windows can retain its standard minimum tracking height on a newly
    // created borderless window. Apply the actual compact bounds atomically.
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    place_window(
        &window,
        window.outer_position().map_err(|e| e.to_string())?,
        tauri::PhysicalSize::new((width * scale) as u32, (height * scale) as u32),
    )?;
    if let Some(monitor) = window
        .current_monitor()
        .map_err(|e| e.to_string())?
        .or(app.primary_monitor().map_err(|e| e.to_string())?)
    {
        let area = monitor.work_area();
        let size = window.outer_size().map_err(|e| e.to_string())?;
        let margin = ((16.0 - WIDGET_SHADOW_INSET) * monitor.scale_factor()) as i32;
        let right = matches!(
            p.widget_position,
            WidgetPosition::TopRight | WidgetPosition::BottomRight
        );
        let bottom = matches!(
            p.widget_position,
            WidgetPosition::BottomRight | WidgetPosition::BottomLeft
        );
        let x = area.position.x
            + if right {
                (area.size.width as i32 - size.width as i32 - margin).max(0)
            } else {
                margin
            };
        let y = area.position.y
            + if bottom {
                (area.size.height as i32 - size.height as i32 - margin).max(0)
            } else {
                margin
            };
        window
            .set_position(PhysicalPosition::new(x, y))
            .map_err(|e| e.to_string())?;
    }
    if !existed {
        crate::system_events::install_widget(&window)?;
    }
    show_widget_without_focus(&window)
}

#[cfg(test)]
mod widget_tests {
    use super::{anchored_height, panel_bounds, should_hide_panel};
    use tauri::PhysicalPosition;
    #[test]
    fn settings_bounds_stay_on_the_selected_negative_coordinate_monitor() {
        let area = tauri::PhysicalRect {
            position: PhysicalPosition::new(-1920, 0),
            size: tauri::PhysicalSize::new(1920, 1040),
        };
        let (position, size) =
            panel_bounds(&area, 1.0, (720.0, 620.0), tauri::PhysicalSize::new(2, 2));
        assert_eq!(position, PhysicalPosition::new(-734, 406));
        assert_eq!(size, tauri::PhysicalSize::new(720, 620));
        let (position, size) =
            panel_bounds(&area, 2.0, (720.0, 620.0), tauri::PhysicalSize::new(2, 2));
        assert!(position.x >= -1920 && position.x + size.width as i32 + 2 <= 0);
        assert!(position.y >= 0 && position.y + size.height as i32 + 2 <= 1040);
    }
    #[test]
    fn outside_click_policy_applies_to_both_panel_modes_and_protects_editors() {
        assert!(should_hide_panel(true, false, true, false));
        assert!(!should_hide_panel(false, false, true, false));
        assert!(!should_hide_panel(true, true, true, false));
        assert!(!should_hide_panel(true, false, true, true));
        assert!(!should_hide_panel(true, false, false, false));
    }
    #[test]
    fn dropdown_uses_space_below_without_repositioning_even_at_negative_coordinates() {
        assert_eq!(anchored_height(480, 136, 30, 1080, 0, 0), 480);
        assert_eq!(anchored_height(480, 136, 800, 1080, 0, 0), 280);
        assert_eq!(anchored_height(480, 136, 950, 1080, 0, 0), 136);
        assert_eq!(anchored_height(480, 136, -200, 0, 0, 0), 200);
    }
}
