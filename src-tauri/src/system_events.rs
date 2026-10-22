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
            WindowsAndMessaging::{WM_ENTERSIZEMOVE, WM_EXITSIZEMOVE, WM_NCDESTROY},
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
