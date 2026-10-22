#[cfg(windows)]
pub fn install(app: &tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    use windows_sys::Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        UI::{
            Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
            WindowsAndMessaging::{
                WM_NCDESTROY, WM_POWERBROADCAST, WM_SETTINGCHANGE, WM_TIMECHANGE,
            },
        },
    };
    unsafe extern "system" fn callback(
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        id: usize,
        data: usize,
    ) -> LRESULT {
        // Installed on the Tauri main window's owning thread; the Box lives
        // until WM_NCDESTROY and is never shared as a raw pointer elsewhere.
        let app = unsafe { &*(data as *const tauri::AppHandle) };
        if message == WM_TIMECHANGE
            || message == WM_SETTINGCHANGE
            || (message == WM_POWERBROADCAST && matches!(wparam, 7 | 18))
        {
            crate::reminders::wake(app);
            let _ = tauri::Emitter::emit(app, "clock-changed", ());
        }
        if message == WM_NCDESTROY {
            unsafe {
                RemoveWindowSubclass(hwnd, Some(callback), id);
                drop(Box::from_raw(data as *mut tauri::AppHandle));
            }
        }
        unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
    }
    let window = app
        .get_webview_window("main")
        .ok_or("Ventana principal no disponible")?;
    let hwnd = window.hwnd().map_err(|e| e.to_string())?;
    let pointer = Box::into_raw(Box::new(app.clone()));
    if unsafe { SetWindowSubclass(hwnd.0 as HWND, Some(callback), 0x43414C, pointer as usize) } == 0
    {
        unsafe {
            drop(Box::from_raw(pointer));
        }
        return Err("No se pudo registrar la reanudación y cambios de hora de Windows".into());
    }
    Ok(())
}
#[cfg(not(windows))]
pub fn install(_app: &tauri::AppHandle) -> Result<(), String> {
    Ok(())
}

#[cfg(windows)]
pub fn install_widget(window: &tauri::WebviewWindow) -> Result<(), String> {
    let owned = window.clone();
    window
        .run_on_main_thread(move || {
            if let Err(error) = install_widget_on_thread(&owned) {
                eprintln!("{error}");
            }
        })
        .map_err(|e| e.to_string())
}
#[cfg(windows)]
fn install_widget_on_thread(window: &tauri::WebviewWindow) -> Result<(), String> {
    use tauri::Manager;
    use windows_sys::Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        UI::{
            Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
            WindowsAndMessaging::{WM_ENTERSIZEMOVE, WM_EXITSIZEMOVE, WM_NCDESTROY, WM_SIZING, WM_NCHITTEST, GetWindowRect},
        },
    };
    unsafe extern "system" fn callback(
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        id: usize,
        data: usize,
    ) -> LRESULT {
        let app = unsafe { &*(data as *const tauri::AppHandle) };
        if message == WM_NCHITTEST {
            let hit = unsafe { DefSubclassProc(hwnd, message, wparam, lparam) };
            // Native top/bottom grips must not bypass the horizontal-only UI.
            return match hit { 12 | 15 => 1, 13 | 16 => 10, 14 | 17 => 11, _ => hit };
        }
        if message == WM_ENTERSIZEMOVE {
            let mut rect = windows_sys::Win32::Foundation::RECT::default();
            if unsafe { GetWindowRect(hwnd, &mut rect) } != 0 {
                if let Ok(mut bounds) = app.state::<crate::windows::WidgetLayout>().resize_vertical_bounds.lock() {
                    *bounds = Some((rect.top, rect.bottom));
                }
            }
        }
        if message == WM_SIZING {
            app.state::<crate::windows::WidgetLayout>().resizing.store(true, std::sync::atomic::Ordering::Relaxed);
            if let Ok(bounds) = app.state::<crate::windows::WidgetLayout>().resize_vertical_bounds.lock() {
                if let Some((top, bottom)) = *bounds {
                    let rect = unsafe { &mut *(lparam as *mut windows_sys::Win32::Foundation::RECT) };
                    rect.top = top;
                    rect.bottom = bottom;
                }
            }
            return 1;
        }
        if message == WM_EXITSIZEMOVE && app.state::<crate::windows::WidgetLayout>().resizing.swap(false, std::sync::atomic::Ordering::Relaxed) {
            if let Some(window) = app.get_webview_window("widget") {
                if let (Ok(size), Ok(scale)) = (window.inner_size(), window.scale_factor()) {
                    let state = app.state::<crate::preferences::AppState>();
                    if let Ok(mut stored) = state.preferences.lock() {
                        if matches!(stored.widget_view, crate::preferences::WidgetView::Next) {
                            let mut next = stored.clone();
                            next.widget_width = (size.width as f64 / scale).clamp(304.0, 1600.0);
                            // Discard legacy manual heights; hover owns vertical layout.
                            next.widget_height = 0.0;
                            next.widget_dropdown_height = 280.0;
                            match crate::preferences::save(app, &next) {
                                Ok(()) => { *stored = next.clone(); let _ = tauri::Emitter::emit(app, "preferences-changed", next); }
                                Err(error) => eprintln!("No se pudo guardar el tamaño: {error}"),
                            }
                        }
                    }
                }
            }
        }
        if message == WM_ENTERSIZEMOVE || message == WM_EXITSIZEMOVE {
            use tauri::Manager;
            app.state::<crate::windows::WidgetLayout>().dragging.store(
                message == WM_ENTERSIZEMOVE,
                std::sync::atomic::Ordering::Relaxed,
            );
            let _ = tauri::Emitter::emit(app, "widget-dragging", message == WM_ENTERSIZEMOVE);
        }
        if message == WM_NCDESTROY {
            unsafe {
                RemoveWindowSubclass(hwnd, Some(callback), id);
                drop(Box::from_raw(data as *mut tauri::AppHandle));
            }
        }
        unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
    }
    let hwnd = window.hwnd().map_err(|e| e.to_string())?;
    let pointer = Box::into_raw(Box::new(window.app_handle().clone()));
    if unsafe { SetWindowSubclass(hwnd.0 as _, Some(callback), 0x574944, pointer as usize) } == 0 {
        unsafe {
            drop(Box::from_raw(pointer));
        }
        return Err("No se pudo registrar el arrastre del widget".into());
    }
    Ok(())
}
#[cfg(not(windows))]
pub fn install_widget(_window: &tauri::WebviewWindow) -> Result<(), String> {
    Ok(())
}
