use crate::application::core::Core;
use block2::RcBlock;
use objc2_app_kit::{
    NSWorkspace, NSWorkspaceDidWakeNotification, NSWorkspaceScreensDidSleepNotification,
    NSWorkspaceSessionDidResignActiveNotification, NSWorkspaceWillSleepNotification,
};
use objc2_foundation::{NSDistributedNotificationCenter, NSNotification, NSString};
use std::{ptr::NonNull, sync::Arc};
use tauri::Emitter;

pub fn observe(app: tauri::AppHandle, core: Arc<Core>) {
    let center = NSWorkspace::sharedWorkspace().notificationCenter();
    // SAFETY: Framework names are static and callbacks capture only Send data.
    // Notification centers retain tokens/blocks for the process lifetime.
    unsafe {
        for name in [
            NSWorkspaceWillSleepNotification,
            NSWorkspaceDidWakeNotification,
            NSWorkspaceScreensDidSleepNotification,
            NSWorkspaceSessionDidResignActiveNotification,
        ] {
            let core = core.clone();
            let app = app.clone();
            let callback = RcBlock::new(move |_: NonNull<NSNotification>| {
                core.lock();
                let _ = app.emit("vault-changed", ());
            });
            center.addObserverForName_object_queue_usingBlock(Some(name), None, None, &callback);
        }
        // This distributed name is a best-effort macOS signal, not a public
        // documented contract. Public sleep/session signals remain independent.
        let callback = RcBlock::new(move |_: NonNull<NSNotification>| {
            core.lock();
            let _ = app.emit("vault-changed", ());
        });
        NSDistributedNotificationCenter::defaultCenter()
            .addObserverForName_object_queue_usingBlock(
                Some(&NSString::from_str("com.apple.screenIsLocked")),
                None,
                None,
                &callback,
            );
    }
}
