use crate::{
    application::{
        core::{Code, Core, VaultStatus},
        transfer::ImportPreview,
    },
    domain::{
        AppError, AppResult,
        account::{AccountInput, AccountSummary},
    },
};
use std::{collections::HashMap, sync::Arc};
use tauri::{Emitter, Manager, State, WebviewWindow};
#[cfg(not(feature = "e2e"))]
use tauri_plugin_dialog::DialogExt;
use uuid::Uuid;
use zeroize::Zeroizing;

fn client(window: &WebviewWindow) -> AppResult<()> {
    if window.label() != "panel" {
        return Err(AppError::Locked);
    }
    let url = window.url().map_err(|_| AppError::Locked)?;
    let local = url.scheme() == "tauri" && url.host_str() == Some("localhost")
        || cfg!(debug_assertions)
            && url.scheme() == "http"
            && url.host_str() == Some("127.0.0.1")
            && url.port() == Some(1420);
    if !local {
        return Err(AppError::Locked);
    }
    Ok(())
}
fn id(value: &str) -> AppResult<Uuid> {
    Uuid::parse_str(value).map_err(|_| AppError::invalid("id"))
}
async fn background<T: Send + 'static>(
    work: impl FnOnce() -> AppResult<T> + Send + 'static,
) -> AppResult<T> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|_| AppError::StorageUnavailable)?
}

#[tauri::command]
pub fn vault_status(window: WebviewWindow, core: State<'_, Arc<Core>>) -> AppResult<VaultStatus> {
    client(&window)?;
    core.status()
}
#[tauri::command]
pub async fn unlock_vault(
    window: WebviewWindow,
    core: State<'_, Arc<Core>>,
) -> AppResult<VaultStatus> {
    client(&window)?;
    let ticket = core.begin_unlock()?;
    let _dialog = core.dialog();
    match core
        .auth
        .authenticate("Desbloquear suas contas no TopOTP")
        .await
    {
        Ok(proof) => {
            let c = core.inner().clone();
            background(move || c.finish_unlock(ticket, proof)).await
        }
        Err(e) => {
            core.fail_unlock(ticket);
            Err(e)
        }
    }
}
#[tauri::command]
pub fn lock_vault(window: WebviewWindow, core: State<'_, Arc<Core>>) -> AppResult<()> {
    client(&window)?;
    core.lock();
    let _ = window.emit("vault-changed", ());
    Ok(())
}
#[tauri::command]
pub fn hide_panel(window: WebviewWindow, core: State<'_, Arc<Core>>) -> AppResult<()> {
    client(&window)?;
    core.visibility(false);
    let _ = window.emit("panel-hidden", ());
    window.hide().map_err(|_| AppError::StorageUnavailable)
}
#[tauri::command]
pub fn user_interaction(
    window: WebviewWindow,
    core: State<'_, Arc<Core>>,
    epoch: String,
) -> AppResult<()> {
    client(&window)?;
    core.interact(&epoch)
}
#[tauri::command]
pub async fn list_accounts(
    window: WebviewWindow,
    core: State<'_, Arc<Core>>,
    epoch: String,
) -> AppResult<Vec<AccountSummary>> {
    client(&window)?;
    let c = core.inner().clone();
    background(move || c.list(&epoch)).await
}
#[tauri::command]
pub async fn get_totp_codes(
    window: WebviewWindow,
    core: State<'_, Arc<Core>>,
    epoch: String,
) -> AppResult<Vec<Code>> {
    client(&window)?;
    let c = core.inner().clone();
    background(move || c.totp(&epoch)).await
}
#[tauri::command]
pub async fn save_account(
    window: WebviewWindow,
    core: State<'_, Arc<Core>>,
    epoch: String,
    account_id: Option<String>,
    input: AccountInput,
) -> AppResult<()> {
    client(&window)?;
    let c = core.inner().clone();
    let account_id = account_id.map(|v| id(&v)).transpose()?;
    background(move || c.save(&epoch, account_id, input)).await
}
#[tauri::command]
pub async fn preview_uri_account(
    window: WebviewWindow,
    core: State<'_, Arc<Core>>,
    epoch: String,
    uri: Zeroizing<String>,
) -> AppResult<ImportPreview> {
    client(&window)?;
    let c = core.inner().clone();
    background(move || c.prepare_uri(&epoch, &uri)).await
}
#[tauri::command]
pub async fn delete_account(
    window: WebviewWindow,
    core: State<'_, Arc<Core>>,
    epoch: String,
    account_id: String,
) -> AppResult<()> {
    client(&window)?;
    let c = core.inner().clone();
    let account_id = id(&account_id)?;
    background(move || c.delete(&epoch, account_id)).await
}
#[tauri::command]
pub async fn generate_hotp(
    window: WebviewWindow,
    core: State<'_, Arc<Core>>,
    epoch: String,
    account_id: String,
) -> AppResult<Code> {
    client(&window)?;
    let c = core.inner().clone();
    let account_id = id(&account_id)?;
    background(move || c.hotp(&epoch, account_id)).await
}
#[tauri::command]
pub async fn copy_code(
    window: WebviewWindow,
    core: State<'_, Arc<Core>>,
    epoch: String,
    account_id: String,
) -> AppResult<()> {
    client(&window)?;
    let c = core.inner().clone();
    let account_id = id(&account_id)?;
    background(move || c.copy(&epoch, account_id)).await
}
#[tauri::command]
pub async fn set_account_order(
    window: WebviewWindow,
    core: State<'_, Arc<Core>>,
    epoch: String,
    ids: Vec<String>,
    pinned: Vec<String>,
) -> AppResult<()> {
    client(&window)?;
    let c = core.inner().clone();
    let ids = ids.iter().map(|v| id(v)).collect::<AppResult<Vec<_>>>()?;
    let pinned = pinned
        .iter()
        .map(|v| id(v))
        .collect::<AppResult<Vec<_>>>()?;
    background(move || c.authorized(&epoch, |i| i.vault.order(&ids, &pinned))).await
}
#[tauri::command]
pub async fn begin_import(
    window: WebviewWindow,
    core: State<'_, Arc<Core>>,
    epoch: String,
) -> AppResult<ImportPreview> {
    client(&window)?;
    core.authorized(&epoch, |_| Ok(()))?;
    let c = core.inner().clone();
    let app = window.app_handle().clone();
    background(move || {
        let _dialog = c.dialog();
        let bytes = import_bytes(&app)?;
        c.authorized(&epoch, |_| Ok(()))?;
        c.prepare_import(&epoch, bytes)
    })
    .await
}
#[tauri::command]
pub async fn provide_import_passwords(
    window: WebviewWindow,
    core: State<'_, Arc<Core>>,
    epoch: String,
    preview_id: String,
    passwords: HashMap<String, Zeroizing<String>>,
) -> AppResult<ImportPreview> {
    client(&window)?;
    let c = core.inner().clone();
    let preview_id = id(&preview_id)?;
    background(move || {
        c.passwords(&epoch, preview_id, passwords, |completed, total| {
            let _ = window.emit(
                "transfer-progress",
                crate::application::transfer::TransferProgress {
                    epoch: epoch.clone(),
                    preview_id: preview_id.to_string(),
                    completed: completed as u32,
                    total: total as u32,
                },
            );
        })
    })
    .await
}
#[tauri::command]
pub async fn commit_import(
    window: WebviewWindow,
    core: State<'_, Arc<Core>>,
    epoch: String,
    preview_id: String,
    selected: Vec<String>,
    acknowledge_skipped: bool,
) -> AppResult<()> {
    client(&window)?;
    let c = core.inner().clone();
    let preview_id = id(&preview_id)?;
    let selected = selected
        .iter()
        .map(|v| id(v))
        .collect::<AppResult<Vec<_>>>()?;
    background(move || c.commit_import(&epoch, preview_id, selected, acknowledge_skipped)).await
}
#[tauri::command]
pub fn cancel_import(
    window: WebviewWindow,
    core: State<'_, Arc<Core>>,
    epoch: String,
) -> AppResult<()> {
    client(&window)?;
    core.cancel_import(&epoch)
}
#[derive(serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportRequest {
    ids: Vec<String>,
    encrypted: bool,
    #[ts(type = "string")]
    password: Zeroizing<String>,
    #[ts(type = "string")]
    confirmation: Zeroizing<String>,
    acknowledge_plaintext: bool,
}
const MIN_EXPORT_PASSWORD_CHARS: usize = 16;

fn validate_backup_password(password: &str, confirmation: &str) -> AppResult<()> {
    if password.trim().chars().count() < MIN_EXPORT_PASSWORD_CHARS
        || password.len() > 1024
        || password != confirmation
    {
        return Err(AppError::invalid("passwordConfirmation"));
    }
    Ok(())
}

#[cfg(test)]
mod export_password_tests {
    use super::validate_backup_password;

    #[test]
    fn accepts_long_backup_passwords_and_matching_confirmation() {
        let password = "lago-violeta-porto-sinal";
        assert!(validate_backup_password(password, password).is_ok());
        assert!(validate_backup_password("🔐abcdefghijklmno", "🔐abcdefghijklmno").is_ok());
        assert!(validate_backup_password(password, "senha diferente").is_err());
        assert!(validate_backup_password("senha curta", "senha curta").is_err());
        let padded = "           curta       ";
        assert!(validate_backup_password(padded, padded).is_err());
        let oversized = "🔐".repeat(257);
        assert!(validate_backup_password(&oversized, &oversized).is_err());
    }
}

#[tauri::command]
pub async fn export_accounts(
    window: WebviewWindow,
    core: State<'_, Arc<Core>>,
    epoch: String,
    request: ExportRequest,
) -> AppResult<()> {
    let ExportRequest {
        ids,
        encrypted,
        password,
        confirmation,
        acknowledge_plaintext,
    } = request;
    client(&window)?;
    core.authorized(&epoch, |_| Ok(()))?;
    if encrypted {
        validate_backup_password(&password, &confirmation)?;
    }
    if !encrypted && !acknowledge_plaintext {
        return Err(AppError::invalid("acknowledgePlaintext"));
    }
    if ids.is_empty() || ids.len() > 1000 {
        return Err(AppError::invalid("selection"));
    }
    let ids = ids.iter().map(|v| id(v)).collect::<AppResult<Vec<_>>>()?;
    let _dialog = core.dialog();
    let proof = core
        .auth
        .authenticate("Autorizar a exportação das contas do TopOTP")
        .await?;
    core.authorized(&epoch, |_| Ok(()))?;
    let c = core.inner().clone();
    let app = window.app_handle().clone();
    background(move || {
        let _dialog = c.dialog();
        let path = export_destination(&app)?;
        let bytes = c.export_bytes(&epoch, &ids, encrypted.then_some(password.as_str()), proof)?;
        c.write_export(&epoch, &path, &bytes, encrypted)
    })
    .await
}

fn import_bytes(_app: &tauri::AppHandle) -> AppResult<Zeroizing<Vec<u8>>> {
    #[cfg(feature = "e2e")]
    {
        crate::application::transfer::read_bounded(&crate::e2e::root().join("import-source.json"))
    }
    #[cfg(not(feature = "e2e"))]
    {
        let path = _app
            .dialog()
            .file()
            .add_filter("Backup Authenticator", &["json", "txt"])
            .blocking_pick_file()
            .ok_or(AppError::Cancelled)?
            .into_path()
            .map_err(|_| AppError::StorageUnavailable)?;
        crate::application::transfer::read_bounded(&path)
    }
}
fn export_destination(_app: &tauri::AppHandle) -> AppResult<std::path::PathBuf> {
    #[cfg(feature = "e2e")]
    {
        Ok(crate::e2e::root().join("topotp-backup.json"))
    }
    #[cfg(not(feature = "e2e"))]
    {
        _app.dialog()
            .file()
            .add_filter("Backup JSON", &["json"])
            .set_file_name("topotp-backup.json")
            .blocking_save_file()
            .ok_or(AppError::Cancelled)?
            .into_path()
            .map_err(|_| AppError::StorageUnavailable)
    }
}
