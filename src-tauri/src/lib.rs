pub mod application;
#[cfg(all(
    any(
        feature = "e2e",
        feature = "native-ui-probe",
        feature = "native-smoke",
        feature = "native-probe"
    ),
    not(debug_assertions)
))]
compile_error!("Synthetic test features are forbidden in release builds");
#[cfg(all(feature = "e2e", feature = "native-smoke"))]
compile_error!("Native smoke and E2E adapters must not be combined");
pub mod bindings;
pub mod commands;
pub mod compatibility;
pub mod domain;
#[cfg(feature = "e2e")]
mod e2e;
#[cfg(feature = "native-smoke")]
mod native_smoke;
pub mod platform;
pub mod shell;

use application::core::Core;
use std::sync::Arc;
use tauri::{
    Emitter, Manager,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

pub fn run() {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            shell::activate(app)
        }))
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
            #[cfg(feature = "native-smoke")]
            if app.config().identifier != "com.rafaelmariano.topotp.native-smoke" {
                return Err("native-smoke requires tauri.native-smoke.conf.json".into());
            }
            #[cfg(not(feature = "e2e"))]
            let store: Box<dyn application::vault::Store> =
                if !cfg!(debug_assertions) || cfg!(feature = "native-smoke") {
                    match platform::macos::vault_helper::HelperStore::packaged(
                        &app.path().resource_dir()?.join("vault-helper"),
                    ) {
                        Ok(store) => Box::new(store),
                        Err(error) => {
                            platform::macos::quick_access::error_message(&error.to_string());
                            return Err(error.into());
                        }
                    }
                } else {
                    Box::new(platform::macos::keychain::Keychain::new(
                        platform::macos::keychain_service().into(),
                    )?)
                };
            #[cfg(not(feature = "e2e"))]
            let core = Arc::new(Core::new(
                store,
                application::vault::DiskIndex::new(app.path().app_data_dir()?.join(
                    if !cfg!(debug_assertions) || cfg!(feature = "native-smoke") {
                        "vault-broker-v1"
                    } else {
                        "dev-signed-v1"
                    },
                ))?,
                Arc::new(platform::macos::auth::NativeAuthenticator::new()),
                Arc::new(platform::macos::clipboard::NativeClipboard),
                Arc::new(application::core::SystemClock::default()),
            )?);
            #[cfg(not(any(feature = "e2e", feature = "native-smoke")))]
            {
                let root = app.path().app_data_dir()?;
                let (legacy, previous, current) = if cfg!(debug_assertions) {
                    ("dev", "dev", "dev-signed-v1")
                } else {
                    ("vault", "vault-signed-v1", "vault-broker-v1")
                };
                if (root.join(legacy).join("manifest.json").is_file()
                    || root.join(previous).join("manifest.json").is_file())
                    && !root.join(current).join("manifest.json").exists()
                {
                    core.require_legacy_migration();
                }
            }
            #[cfg(feature = "e2e")]
            let core = Arc::new(e2e::core()?);
            app.manage(core.clone());
            app.manage(Arc::new(shell::Shell::new(core.clone())));
            shell::setup(app.handle());
            #[cfg(feature = "e2e")]
            {
                let window = shell::ensure_management(app.handle())?;
                window.show()?;
                shell::management_visibility(app.handle(), true);
            }
            platform::macos::events::observe(app.handle().clone(), core.clone());
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    core.tick();
                    shell::poll(&handle);
                }
            });
            let open = MenuItem::with_id(app, "open", "Abrir", true, None::<&str>)?;
            let lock = MenuItem::with_id(app, "lock", "Bloquear", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Sair", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &lock, &quit])?;
            let _tray = TrayIconBuilder::with_id("topotp")
                .icon(tauri::image::Image::from_bytes(include_bytes!(
                    "../icons/tray/20x20.png"
                ))?)
                .icon_as_template(true)
                .tooltip(app.config().product_name.as_deref().unwrap_or("TopOTP"))
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => shell::activate(app),
                    "lock" => shell::lock(app),
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
                        shell::activate(app);
                    }
                })
                .build(app)?;
            #[cfg(feature = "native-smoke")]
            _tray.set_title(Some("Teste"))?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != "management" {
                return;
            }
            match event {
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    shell::hide_management(window.app_handle());
                }
                tauri::WindowEvent::Resized(_) | tauri::WindowEvent::Focused(true) => {
                    let visible = window.is_visible().unwrap_or(false)
                        && !window.is_minimized().unwrap_or(false);
                    if shell::management_visibility(window.app_handle(), visible) {
                        let _ = window.emit(
                            if visible {
                                "panel-shown"
                            } else {
                                "panel-hidden"
                            },
                            (),
                        );
                    }
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::vault_status,
            commands::management_ready,
            commands::pending_preview,
            commands::unlock_vault,
            commands::lock_vault,
            commands::hide_panel,
            commands::user_interaction,
            commands::list_accounts,
            commands::get_totp_codes,
            commands::save_account,
            commands::preview_uri_account,
            commands::delete_account,
            commands::remove_all_data,
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
        tauri::RunEvent::Reopen { .. } => shell::activate(app),
        _ => {}
    });
}
