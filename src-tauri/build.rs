fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "vault_status",
            "management_ready",
            "pending_preview",
            "unlock_vault",
            "lock_vault",
            "hide_panel",
            "user_interaction",
            "list_accounts",
            "get_totp_codes",
            "save_account",
            "preview_uri_account",
            "delete_account",
            "generate_hotp",
            "copy_code",
            "set_account_order",
            "begin_import",
            "provide_import_passwords",
            "commit_import",
            "cancel_import",
            "export_accounts",
        ]),
    ))
    .expect("Tauri build configuration invalid");
}
