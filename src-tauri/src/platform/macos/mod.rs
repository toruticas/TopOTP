pub const BUNDLE_IDENTIFIER: &str = "com.rafaelmariano.topotp";

pub mod auth;
pub mod clipboard;
pub mod events;
pub mod keychain;
pub mod panel;
pub mod quick_access;
pub mod scan;

#[cfg(feature = "native-probe")]
pub mod probe;

pub fn keychain_service() -> &'static str {
    if cfg!(feature = "native-smoke") {
        "com.rafaelmariano.topotp.native-smoke.signed.vault.v1"
    } else if cfg!(test) {
        "com.rafaelmariano.topotp.test.vault"
    } else if cfg!(debug_assertions) {
        "com.rafaelmariano.topotp.dev.signed.vault.v1"
    } else {
        "com.rafaelmariano.topotp.signed.vault.v1"
    }
}
