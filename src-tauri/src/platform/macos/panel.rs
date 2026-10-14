//! Anchor in AppKit screen points on the main thread. Tray/Tauri physical
//! coordinates and CoreGraphics monitor lookup use different units on Retina.
use objc2::rc::Retained;
use objc2_app_kit::NSWindow;
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize};
use tauri::{WebviewWindow, tray::TrayIcon};

fn origin_below_icon(icon: NSRect, visible: NSRect, panel: NSSize) -> NSPoint {
    NSPoint::new(
        (icon.origin.x + icon.size.width / 2.0 - panel.width / 2.0).clamp(
            visible.origin.x,
            (visible.origin.x + visible.size.width - panel.width).max(visible.origin.x),
        ),
        (icon.origin.y - 4.0 - panel.height).clamp(
            visible.origin.y,
            (visible.origin.y + visible.size.height - panel.height).max(visible.origin.y),
        ),
    )
}

pub fn position(window: &WebviewWindow, tray: &TrayIcon) -> tauri::Result<bool> {
    let window = window.clone();
    tray.with_inner_tray_icon(move |tray| {
        let Some(mtm) = MainThreadMarker::new() else {
            return false;
        };
        let Some(item) = tray.ns_status_item() else {
            return false;
        };
        // SAFETY: Tauri executes this closure on the main thread. Its window
        // pointer is a live NSWindow, retained here for the synchronous move.
        unsafe {
            let Some(anchor) = item.button(mtm).and_then(|button| button.window()) else {
                return false;
            };
            let Some(screen) = anchor.screen() else {
                return false;
            };
            let Ok(pointer) = window.ns_window() else {
                return false;
            };
            let Some(panel) = Retained::retain(pointer.cast::<NSWindow>()) else {
                return false;
            };
            let origin =
                origin_below_icon(anchor.frame(), screen.visibleFrame(), panel.frame().size);
            // AppKit moves synchronously before show; no frame at the old
            // centered position and no DPI conversion using the old monitor.
            panel.setFrameOrigin(origin);
            true
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rect(x: f64, y: f64, width: f64, height: f64) -> NSRect {
        NSRect::new(NSPoint::new(x, y), NSSize::new(width, height))
    }
    #[test]
    fn aligns_center_and_top_edge_below_retina_icon_in_screen_points() {
        let icon = rect(1000.0, 958.0, 24.0, 24.0);
        let size = NSSize::new(380.0, 480.0);
        let origin = origin_below_icon(icon, rect(0.0, 38.0, 1512.0, 920.0), size);
        assert_eq!(
            origin.x + size.width / 2.0,
            icon.origin.x + icon.size.width / 2.0
        );
        assert_eq!(origin.y + size.height, icon.origin.y - 4.0);
    }
    #[test]
    fn fits_at_both_edges_and_on_secondary_displays_with_negative_origins() {
        for visible in [
            rect(0.0, 38.0, 1512.0, 920.0),
            rect(-1920.0, -300.0, 1920.0, 1056.0),
        ] {
            let size = NSSize::new(380.0, 480.0);
            for x in [
                visible.origin.x,
                visible.origin.x + visible.size.width - 24.0,
            ] {
                let icon = rect(x, visible.origin.y + visible.size.height, 24.0, 24.0);
                let origin = origin_below_icon(icon, visible, size);
                assert!(origin.x >= visible.origin.x);
                assert!(origin.x + size.width <= visible.origin.x + visible.size.width);
                assert!(origin.y >= visible.origin.y);
                assert!(origin.y + size.height <= icon.origin.y);
            }
        }
    }
}
