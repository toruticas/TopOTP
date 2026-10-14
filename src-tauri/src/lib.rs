pub mod application;
#[cfg(all(feature = "e2e", not(debug_assertions)))]
compile_error!("The synthetic e2e feature is forbidden in release builds");
pub mod bindings;
pub mod commands;
pub mod compatibility;
pub mod domain;
#[cfg(feature = "e2e")]
mod e2e;
pub mod platform;

use application::core::Core;
use std::sync::Arc;
use tauri::{
    Emitter, Manager,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent},
};

fn hidden(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("panel") {
        app.state::<Arc<Core>>().visibility(false);
        let _ = window.emit("panel-hidden", ());
        let _ = window.hide();
    }
}
fn show(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("panel") {
        if let Some(tray) = app.tray_by_id("topotp") {
            position(&window, &tray);
        }
        app.state::<Arc<Core>>().visibility(true);
        let _ = window.show();
        let _ = window.set_focus();
        let _ = window.emit("panel-shown", ());
    }
}
fn position(window: &tauri::WebviewWindow, tray: &TrayIcon) {
    let _ = platform::macos::panel::position(window, tray);
}
pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| show(app)))
        .plugin(tauri_plugin_dialog::init());
    #[cfg(feature = "e2e")]
    let builder = builder.plugin(tauri_plugin_wdio_webdriver::init());
    let app = builder
        .setup(|app| {
            #[cfg(feature = "e2e")]
            if app.config().identifier != "com.rafaelmariano.topotp.e2e" {
                return Err("e2e requires tauri.e2e.conf.json".into());
            }
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            #[cfg(not(feature = "e2e"))]
            let core = Arc::new(Core::new(
                Box::new(platform::macos::keychain::Keychain::new(
                    platform::macos::keychain_service().into(),
                )?),
                application::vault::DiskIndex::new(app.path().app_data_dir()?.join(
                    if cfg!(debug_assertions) {
                        "dev"
                    } else {
                        "vault"
                    },
                ))?,
                Arc::new(platform::macos::auth::NativeAuthenticator::new()),
                Arc::new(platform::macos::clipboard::NativeClipboard),
                Arc::new(application::core::SystemClock::default()),
            )?);
            #[cfg(feature = "e2e")]
            let core = Arc::new(e2e::core()?);
            app.manage(core.clone());
            tauri::WebviewWindowBuilder::from_config(app, &app.config().app.windows[0])?
                .on_navigation(|url| {
                    url.scheme() == "tauri" && url.host_str() == Some("localhost")
                        || cfg!(debug_assertions)
                            && url.scheme() == "http"
                            && url.host_str() == Some("127.0.0.1")
                            && url.port() == Some(1420)
                })
                .build()?;
            core.visibility(app.config().app.windows[0].visible);
            platform::macos::events::observe(app.handle().clone(), core.clone());
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    if core.tick() {
                        let _ = handle.emit("vault-changed", ());
                    }
                }
            });
            let open = MenuItem::with_id(app, "open", "Abrir", true, None::<&str>)?;
            let lock = MenuItem::with_id(app, "lock", "Bloquear", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Sair", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &lock, &quit])?;
            TrayIconBuilder::with_id("topotp")
                .icon(tauri::image::Image::from_bytes(include_bytes!(
                    "../icons/tray/20x20.png"
                ))?)
                .icon_as_template(true)
                .tooltip("TopOTP")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => show(app),
                    "lock" => {
                        app.state::<Arc<Core>>().lock();
                        let _ = app.emit("vault-changed", ());
                        hidden(app);
                    }
                    "quit" => {
                        app.state::<Arc<Core>>().lock();
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if matches!(
                        event,
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        }
                    ) {
                        let app = tray.app_handle();
                        if app
                            .get_webview_window("panel")
                            .is_some_and(|w| w.is_visible().unwrap_or(false))
                        {
                            hidden(app);
                        } else {
                            show(app);
                        }
                    }
                })
                .build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                api.prevent_close();
                hidden(window.app_handle());
            }
            tauri::WindowEvent::Focused(false) if !window.state::<Arc<Core>>().has_dialog() => {
                hidden(window.app_handle())
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            commands::vault_status,
            commands::unlock_vault,
            commands::lock_vault,
            commands::hide_panel,
            commands::user_interaction,
            commands::list_accounts,
            commands::get_totp_codes,
            commands::save_account,
            commands::preview_uri_account,
            commands::delete_account,
            commands::generate_hotp,
            commands::copy_code,
            commands::set_account_order,
            commands::begin_import,
            commands::provide_import_passwords,
            commands::commit_import,
            commands::cancel_import,
            commands::export_accounts
        ])
        .build(tauri::generate_context!())
        .expect("TopOTP could not start");
    app.run(|app, event| match event {
        tauri::RunEvent::Exit | tauri::RunEvent::ExitRequested { .. } => {
            app.state::<Arc<Core>>().lock()
        }
        tauri::RunEvent::Reopen { .. } => show(app),
        _ => {}
    });
}
