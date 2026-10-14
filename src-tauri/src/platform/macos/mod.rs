pub const BUNDLE_IDENTIFIER: &str = "com.rafaelmariano.topotp";

pub mod auth;
pub mod clipboard;
pub mod events;
pub mod keychain;

#[cfg(feature = "native-probe")]
pub mod probe;

pub fn keychain_service() -> &'static str {
    if cfg!(test) {
        "com.rafaelmariano.topotp.test.vault"
    } else if cfg!(debug_assertions) {
        "com.rafaelmariano.topotp.dev.vault"
    } else {
        "com.rafaelmariano.topotp.vault"
    }
}
