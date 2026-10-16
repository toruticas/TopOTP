//! Normal management windows remain onscreen when another app becomes active.
use objc2::rc::Retained;
use objc2_app_kit::NSWindow;

pub fn configure(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    let handle = window.clone();
    window.run_on_main_thread(move || {
        if let Ok(pointer) = handle.ns_window() {
            // SAFETY: The Tauri window is alive and this closure runs on the
            // AppKit main thread. Retain it for this synchronous configuration.
            if let Some(window) = unsafe { Retained::retain(pointer.cast::<NSWindow>()) } {
                window.setHidesOnDeactivate(false);
            }
        }
    })
}
