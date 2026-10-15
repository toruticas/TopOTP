//! Debug-only full-shell test. Real LocalAuthentication/Keychain, public accounts.
use crate::{application::core::Core, domain::AppResult};
use uuid::Uuid;

pub fn seed_if_empty(core: &Core, epoch: &str) -> AppResult<()> {
    if !core.list(epoch)?.is_empty() {
        return Ok(());
    }
    for uri in [
        "otpauth://totp/TopOTP%20Teste:Conta%20sintetica?secret=GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ&issuer=TopOTP%20Teste&digits=8",
        "otpauth://hotp/TopOTP%20Teste:Contador%20sintetico?secret=JBSWY3DPEHPK3PXP&counter=7&issuer=TopOTP%20Teste",
    ] {
        let preview = core.prepare_uri(epoch, uri)?;
        let id = Uuid::parse_str(&preview.rows[0].account.as_ref().unwrap().id).unwrap();
        core.commit_import(
            epoch,
            Uuid::parse_str(&preview.id).unwrap(),
            vec![id],
            false,
        )?;
    }
    Ok(())
}
