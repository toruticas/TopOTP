//! Shared presentation coordinator. Native UI calls use cases, never IPC.
use crate::{
    application::{
        core::Core,
        presentation::{Activation, Presentation},
        session::Phase,
    },
    domain::{AppError, AppResult},
    platform::macos::quick_access::{self, Action, MenuAction, Row},
};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use std::{
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tauri::{Emitter, Manager};

#[derive(Clone, Copy, Serialize, Deserialize, ts_rs::TS)]
#[serde(rename_all = "kebab-case")]
pub enum ManagementRoute {
    General,
    Accounts,
    NewAccount,
    Import,
    Export,
    QrReview,
    Migration,
}
#[derive(Clone, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct ManagementRequest {
    pub route: ManagementRoute,
    pub epoch: String,
}
pub struct Shell {
    pub core: Arc<Core>,
    presentation: Mutex<Presentation>,
    route: Mutex<Option<ManagementRequest>>,
    seen: Mutex<(Phase, String)>,
    reading: AtomicBool,
    last_refresh: Mutex<Instant>,
    scan_cancel: Mutex<Option<Arc<AtomicBool>>>,
}
impl Shell {
    pub fn new(core: Arc<Core>) -> Self {
        Self {
            core,
            presentation: Mutex::new(Presentation::default()),
            route: Mutex::new(None),
            seen: Mutex::new((Phase::Locked, "1".into())),
            reading: AtomicBool::new(false),
            last_refresh: Mutex::new(Instant::now()),
            scan_cancel: Mutex::new(None),
        }
    }
    fn visibility(&self) {
        let visible = self.presentation.lock().unwrap().visible();
        self.core.visibility(visible);
    }
    pub fn take_route(&self) -> Option<ManagementRequest> {
        self.route.lock().unwrap().take()
    }
}
fn state(app: &tauri::AppHandle) -> Arc<Shell> {
    app.state::<Arc<Shell>>().inner().clone()
}
pub fn setup(app: &tauri::AppHandle) {
    let handle = app.clone();
    quick_access::install(Rc::new(move |action| native_action(&handle, action)));
}
pub fn activate(app: &tauri::AppHandle) {
    if objc2_foundation::MainThreadMarker::new().is_none() {
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || activate(&handle));
        return;
    }
    let shell = state(app);
    let Ok(status) = shell.core.status() else {
        return;
    };
    let action = shell.presentation.lock().unwrap().activate(status.phase);
    match action {
        Activation::Ignore => {}
        Activation::Hide => quick_access::hide(),
        Activation::Present(intent) => load(app.clone(), shell, intent, status.epoch, true),
        Activation::Authenticate(intent) => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                let result = shell.core.unlock().await;
                let approved = result.is_ok();
                let current = shell
                    .presentation
                    .lock()
                    .unwrap()
                    .complete(intent, approved);
                match (current, result) {
                    (Some(intent), Ok(status)) => load(app, shell, intent, status.epoch, true),
                    (_, Err(error))
                        if !matches!(error, AppError::Cancelled | AppError::Locked)
                            && shell.presentation.lock().unwrap().current(intent) =>
                    {
                        let _ = app.run_on_main_thread(move || {
                            quick_access::error_message(&error.to_string())
                        });
                    }
                    _ => {}
                }
            });
        }
    }
}
fn rows(core: &Core, epoch: &str) -> AppResult<Vec<Row>> {
    #[cfg(feature = "native-smoke")]
    crate::native_smoke::seed_if_empty(core, epoch)?;
    let crate::application::core::PresentationSnapshot {
        mut accounts,
        codes,
    } = core.presentation_snapshot(epoch)?;
    accounts.sort_by_key(|account| !account.pinned);
    core.authorized(epoch, |_| {
        Ok(accounts
            .into_iter()
            .map(|account| {
                let code = codes.iter().find(|code| code.id == account.id);
                let remaining = code.and_then(|code| {
                    code.valid_until
                        .map(|until| (until - code.server_time).max(0.) as u64)
                });
                Row {
                    account,
                    code: code.map(|code| code.code.clone()),
                    remaining,
                }
            })
            .collect())
    })
}
fn load(app: tauri::AppHandle, shell: Arc<Shell>, intent: u64, epoch: String, show: bool) {
    if !show && shell.reading.swap(true, Ordering::SeqCst) {
        return;
    }
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = rows(&shell.core, &epoch);
        if !show {
            shell.reading.store(false, Ordering::SeqCst);
        }
        let _ = handle.run_on_main_thread(move || {
            if !shell.presentation.lock().unwrap().current(intent) {
                return;
            }
            let authorized = shell
                .core
                .status()
                .is_ok_and(|s| s.phase == Phase::Unlocked && s.epoch == epoch);
            if !authorized {
                if show && matches!(result, Err(AppError::KeychainAccessDenied)) {
                    quick_access::error_message(&AppError::KeychainAccessDenied.to_string());
                }
                return;
            }
            if show
                && shell
                    .core
                    .status()
                    .is_ok_and(|s| s.legacy_migration_required == Some(true))
            {
                let _ = open_management(&app, ManagementRoute::Migration);
                return;
            }
            match result {
                Ok(rows) => {
                    if show {
                        if !shell.presentation.lock().unwrap().shown(intent) {
                            return;
                        }
                        shell.visibility();
                        if let Some(tray) = app.tray_by_id("topotp") {
                            let _ = tray.with_inner_tray_icon(move |tray| {
                                let mtm = objc2_foundation::MainThreadMarker::new().unwrap();
                                if let Some(button) =
                                    tray.ns_status_item().and_then(|item| item.button(mtm))
                                {
                                    quick_access::show(&button, rows, &epoch);
                                }
                            });
                        }
                    } else {
                        quick_access::update(rows, &epoch);
                    }
                }
                Err(error) => {
                    if show {
                        quick_access::error_message(&error.to_string());
                    } else {
                        quick_access::notice(&error.to_string());
                    }
                }
            }
        });
    });
}
pub fn poll(app: &tauri::AppHandle) {
    let _ = app.run_on_main_thread(quick_access::expire_hidden);
    let shell = state(app);
    let Ok(status) = shell.core.status() else {
        return;
    };
    let changed = {
        let mut seen = shell.seen.lock().unwrap();
        let changed = seen.0 != status.phase || seen.1 != status.epoch;
        *seen = (status.phase, status.epoch.clone());
        changed
    };
    if changed {
        if status.phase == Phase::Locked {
            if let Some(cancel) = shell.scan_cancel.lock().unwrap().as_ref() {
                cancel.store(true, Ordering::SeqCst);
            }
            shell.presentation.lock().unwrap().revoke();
            shell.route.lock().unwrap().take();
            let handle = app.clone();
            let core = shell.core.clone();
            let _ = app.run_on_main_thread(move || {
                if core.status().is_ok_and(|s| s.phase != Phase::Unlocked) {
                    crate::platform::macos::scan::selection::cancel();
                    quick_access::reset();
                    let _ = handle.emit("vault-changed", ());
                }
            });
        } else {
            let _ = app.emit("vault-changed", &status);
        }
    }
    let (visible, intent) = {
        let p = shell.presentation.lock().unwrap();
        (p.quick_visible, p.ticket())
    };
    if visible && status.phase == Phase::Unlocked {
        let refresh = {
            let mut at = shell.last_refresh.lock().unwrap();
            if at.elapsed() >= Duration::from_secs(1) {
                *at = Instant::now();
                true
            } else {
                false
            }
        };
        if refresh {
            load(app.clone(), shell, intent, status.epoch, false);
        }
    }
}
fn native_action(app: &tauri::AppHandle, action: Action) {
    let shell = state(app);
    match action {
        Action::Hidden => {
            shell.presentation.lock().unwrap().hidden();
            shell.visibility();
        }
        Action::Dismiss => quick_access::hide(),
        Action::Interact => {
            if let Ok(s) = shell.core.status() {
                let _ = shell.core.interact(&s.epoch);
            }
        }
        Action::Copy(ref id) | Action::Next(ref id) => {
            let next = matches!(&action, Action::Next(_));
            let id = id.clone();
            let Ok(status) = shell.core.status() else {
                return;
            };
            let app = app.clone();
            tauri::async_runtime::spawn_blocking(move || {
                let result = uuid::Uuid::parse_str(&id)
                    .map_err(|_| AppError::invalid("id"))
                    .and_then(|id| {
                        if next {
                            shell.core.hotp(&status.epoch, id).map(|_| ())
                        } else {
                            shell.core.copy(&status.epoch, id)
                        }
                    });
                let message = match result {
                    Ok(()) => {
                        if next {
                            "Código gerado.".into()
                        } else {
                            "Código copiado.".into()
                        }
                    }
                    Err(error) => error.to_string(),
                };
                let _ = app.run_on_main_thread(move || {
                    if shell.core.authorized(&status.epoch, |_| Ok(())).is_ok() {
                        quick_access::notice(&message);
                    }
                });
            });
        }
        Action::Menu(MenuAction::Lock) => lock(app),
        Action::Menu(MenuAction::Quit) => {
            shell.core.lock();
            app.exit(0);
        }
        Action::Menu(MenuAction::NewAccount) => {
            let _ = open_management(app, ManagementRoute::NewAccount);
        }
        Action::Menu(MenuAction::Import) => {
            let _ = open_management(app, ManagementRoute::Import);
        }
        Action::Menu(MenuAction::Export) => {
            let _ = open_management(app, ManagementRoute::Export);
        }
        Action::Menu(MenuAction::Settings) => {
            let _ = open_management(app, ManagementRoute::General);
        }
        Action::Menu(MenuAction::Scan) => {
            start_scan(app);
        }
    }
}
pub fn lock(app: &tauri::AppHandle) {
    let shell = state(app);
    shell.presentation.lock().unwrap().revoke();
    shell.route.lock().unwrap().take();
    if let Some(cancel) = shell.scan_cancel.lock().unwrap().as_ref() {
        cancel.store(true, Ordering::SeqCst);
    }
    shell.core.lock();
    let handle = app.clone();
    let core = shell.core.clone();
    let _ = app.run_on_main_thread(move || {
        if core.status().is_ok_and(|s| s.phase != Phase::Unlocked) {
            crate::platform::macos::scan::selection::cancel();
            quick_access::reset();
            let _ = handle.emit("vault-changed", ());
        }
    });
}
pub fn ensure_management(app: &tauri::AppHandle) -> tauri::Result<tauri::WebviewWindow> {
    if let Some(window) = app.get_webview_window("management") {
        return Ok(window);
    }
    tauri::WebviewWindowBuilder::from_config(app, &app.config().app.windows[0])?
        .on_navigation(|url| {
            url.scheme() == "tauri" && url.host_str() == Some("localhost")
                || cfg!(debug_assertions)
                    && url.scheme() == "http"
                    && url.host_str() == Some("127.0.0.1")
                    && url.port() == Some(1420)
        })
        .build()
}
pub fn open_management(app: &tauri::AppHandle, route: ManagementRoute) -> tauri::Result<()> {
    let shell = state(app);
    let status = shell
        .core
        .status()
        .map_err(|e| tauri::Error::Anyhow(e.into()))?;
    if status.phase != Phase::Unlocked {
        return Err(tauri::Error::Anyhow(AppError::Locked.into()));
    }
    let window = ensure_management(app)?;
    *shell.route.lock().unwrap() = Some(ManagementRequest {
        route,
        epoch: status.epoch,
    });
    shell.presentation.lock().unwrap().management_visible = true;
    shell.visibility();
    window.show()?;
    window.unminimize()?;
    window.set_focus()?;
    quick_access::hide();
    let _ = window.emit("panel-shown", ());
    let _ = window.emit("management-route", ());
    Ok(())
}
pub fn management_visibility(app: &tauri::AppHandle, visible: bool) -> bool {
    let shell = state(app);
    let changed = {
        let mut presentation = shell.presentation.lock().unwrap();
        let changed = presentation.management_visible != visible;
        presentation.management_visible = visible;
        changed
    };
    shell.visibility();
    changed
}
pub fn hide_management(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("management") {
        management_visibility(app, false);
        let shell = state(app);
        if let Ok(status) = shell.core.status() {
            let _ = shell.core.cancel_import(&status.epoch);
        }
        let _ = window.emit("management-closed", ());
        let _ = window.hide();
    }
}

fn start_scan(app: &tauri::AppHandle) {
    let shell = state(app);
    let Ok(status) = shell.core.status() else {
        return;
    };
    if status.phase != Phase::Unlocked {
        return;
    }
    {
        let mut slot = shell.scan_cancel.lock().unwrap();
        if slot.is_some() {
            return;
        }
        *slot = Some(Arc::new(AtomicBool::new(false)));
    }
    let cancel = shell.scan_cancel.lock().unwrap().as_ref().unwrap().clone();
    let _ = shell.core.interact(&status.epoch);
    let _ = shell.core.cancel_import(&status.epoch);
    shell.presentation.lock().unwrap().scanning = true;
    shell.visibility();
    quick_access::hide();
    if let Some(window) = app.get_webview_window("management") {
        let _ = window.emit("scan-started", ());
    }
    if let Err(error) = crate::platform::macos::scan::capture::request_permission() {
        finish_scan(app, &cancel, &status.epoch, Err(error));
        return;
    }
    if shell.core.authorized(&status.epoch, |_| Ok(())).is_err() {
        finish_scan(app, &cancel, &status.epoch, Err(AppError::Locked));
        return;
    }
    let handle = app.clone();
    let token = cancel.clone();
    let scan_epoch = status.epoch.clone();
    let selection = crate::platform::macos::scan::selection::start(Rc::new(move |region| {
        let Ok(region) = region else {
            finish_scan(&handle, &token, &status.epoch, Err(region.unwrap_err()));
            return;
        };
        let app = handle.clone();
        let cancel = token.clone();
        let core = shell.core.clone();
        let epoch = status.epoch.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let result =
                crate::platform::macos::scan::capture::capture(region, cancel.clone(), || {
                    core.authorized(&epoch, |_| Ok(())).is_ok()
                })
                .and_then(|payloads| {
                    if cancel.load(Ordering::SeqCst) {
                        return Err(AppError::Cancelled);
                    }
                    core.prepare_qr(&epoch, payloads).map(|_| ())
                });
            let target = app.clone();
            let _ = app.run_on_main_thread(move || finish_scan(&target, &cancel, &epoch, result));
        });
    }));
    if let Err(error) = selection {
        finish_scan(app, &cancel, &scan_epoch, Err(error));
    }
}
fn finish_scan(
    app: &tauri::AppHandle,
    cancel: &Arc<AtomicBool>,
    epoch: &str,
    result: AppResult<()>,
) {
    let shell = state(app);
    {
        let mut slot = shell.scan_cancel.lock().unwrap();
        if !slot
            .as_ref()
            .is_some_and(|current| Arc::ptr_eq(current, cancel))
        {
            return;
        }
        slot.take();
    }
    let cancelled = cancel.swap(true, Ordering::SeqCst);
    shell.presentation.lock().unwrap().scanning = false;
    if let Some(window) = app.get_webview_window("management") {
        let _ = window.emit("scan-finished", ());
    }
    if !cancelled && result.is_ok() && shell.core.authorized(epoch, |_| Ok(())).is_ok() {
        if open_management(app, ManagementRoute::QrReview).is_err() {
            quick_access::error_message("Não foi possível abrir a revisão do QR.");
        }
    } else {
        shell.visibility();
        if !cancelled
            && let Err(error) = result
            && !matches!(error, AppError::Cancelled | AppError::Locked)
        {
            quick_access::error_message(&error.to_string());
        }
    }
}
