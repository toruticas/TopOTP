//! CryptoJS/OpenSSL password envelope, solely for upstream backup interop.
//! CBC does not authenticate integrity; never use this format for the vault.
use crate::domain::{AppError, AppResult};
use aes::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit, block_padding::Pkcs7};
use argon2::{
    Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version,
    password_hash::SaltString,
};
use base64::{
    Engine,
    engine::general_purpose::{STANDARD, STANDARD_NO_PAD},
};
use md5::{Digest, Md5};
use rand::{RngCore, rngs::OsRng};
use zeroize::Zeroizing;

pub fn validate_phc(phc: &str) -> AppResult<()> {
    if phc.len() > 512 {
        return Err(AppError::LimitExceeded);
    }
    let hash = PasswordHash::new(phc).map_err(|_| AppError::UnsupportedFormat)?;
    if hash.algorithm.as_str() != "argon2id"
        || hash.version != Some(19)
        || hash.hash.as_ref().map(|h| h.len()) != Some(32)
        || hash.salt.is_none()
    {
        return Err(AppError::UnsupportedFormat);
    }
    let value = |name| {
        hash.params
            .get(name)
            .and_then(|v| v.decimal().ok())
            .ok_or(AppError::UnsupportedFormat)
    };
    let (m, t, p) = (value("m")?, value("t")?, value("p")?);
    if !(8..=65536).contains(&m) || !(1..=6).contains(&t) || !(1..=4).contains(&p) || m < 8 * p {
        return Err(AppError::LimitExceeded);
    }
    Ok(())
}
fn argon() -> AppResult<Argon2<'static>> {
    Ok(Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(19456, 2, 1, Some(32)).map_err(|_| AppError::UnsupportedFormat)?,
    ))
}
pub fn derive(password: &str, salt: &str) -> AppResult<Zeroizing<String>> {
    if password.is_empty() || password.len() > 1024 || !(8..=1024).contains(&salt.len()) {
        return Err(AppError::invalid("password"));
    }
    let mut output = Zeroizing::new([0u8; 32]);
    argon()?
        .hash_password_into(password.as_bytes(), salt.as_bytes(), &mut *output)
        .map_err(|_| AppError::InvalidPassword)?;
    Ok(Zeroizing::new(STANDARD_NO_PAD.encode(*output)))
}
pub fn verify(derived: &str, phc: &str) -> AppResult<()> {
    validate_phc(phc)?;
    Argon2::default()
        .verify_password(
            derived.as_bytes(),
            &PasswordHash::new(phc).map_err(|_| AppError::UnsupportedFormat)?,
        )
        .map_err(|_| AppError::InvalidPassword)
}
pub fn verifier(derived: &str) -> AppResult<String> {
    let mut salt = [0; 16];
    OsRng.fill_bytes(&mut salt);
    let salt = SaltString::encode_b64(&salt).map_err(|_| AppError::StorageUnavailable)?;
    Ok(argon()?
        .hash_password(derived.as_bytes(), &salt)
        .map_err(|_| AppError::StorageUnavailable)?
        .to_string())
}
fn key_iv(passphrase: &str, salt: &[u8]) -> Zeroizing<Vec<u8>> {
    let mut out = Zeroizing::new(Vec::with_capacity(48));
    let mut previous = Zeroizing::new(Vec::new());
    while out.len() < 48 {
        let mut hash = Md5::new();
        hash.update(&*previous);
        hash.update(passphrase.as_bytes());
        hash.update(salt);
        *previous = hash.finalize().to_vec();
        out.extend_from_slice(&previous);
    }
    out
}
pub fn decrypt(envelope: &str, passphrase: &str) -> AppResult<Zeroizing<Vec<u8>>> {
    if envelope.len() > super::MAX_CIPHERTEXT_BYTES {
        return Err(AppError::LimitExceeded);
    }
    let bytes = Zeroizing::new(
        STANDARD
            .decode(envelope)
            .map_err(|_| AppError::UnsupportedFormat)?,
    );
    if bytes.len() < 32 || &bytes[..8] != b"Salted__" || (bytes.len() - 16) % 16 != 0 {
        return Err(AppError::UnsupportedFormat);
    }
    let key = key_iv(passphrase, &bytes[8..16]);
    let cipher = cbc::Decryptor::<aes::Aes256>::new_from_slices(&key[..32], &key[32..])
        .map_err(|_| AppError::UnsupportedFormat)?;
    Ok(Zeroizing::new(
        cipher
            .decrypt_padded_vec_mut::<Pkcs7>(&bytes[16..])
            .map_err(|_| AppError::InvalidPassword)?,
    ))
}
pub fn encrypt(data: &[u8], passphrase: &str) -> AppResult<String> {
    let mut salt = [0; 8];
    OsRng.fill_bytes(&mut salt);
    let key = key_iv(passphrase, &salt);
    let cipher = cbc::Encryptor::<aes::Aes256>::new_from_slices(&key[..32], &key[32..])
        .map_err(|_| AppError::StorageUnavailable)?;
    let mut bytes = b"Salted__".to_vec();
    bytes.extend_from_slice(&salt);
    bytes.extend(cipher.encrypt_padded_vec_mut::<Pkcs7>(data));
    Ok(STANDARD.encode(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hostile_phc_rejected_before_derivation() {
        assert!(validate_phc("$argon2id$v=19$m=999999999,t=2,p=1$c3ludGhldGljLXNhbHQ$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA").is_err());
    }
    #[test]
    fn envelope_roundtrip_and_invalid_password() {
        let e = encrypt(b"synthetic backup only", "synthetic").unwrap();
        assert_eq!(
            &*decrypt(&e, "synthetic").unwrap(),
            b"synthetic backup only"
        );
        assert!(decrypt(&e, "wrong").is_err());
    }
}
