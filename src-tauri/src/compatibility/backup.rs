use super::crypto;
use crate::domain::{
    AppError, AppResult,
    account::{AccountInput, AccountRecord, OtpAlgorithm, OtpKind},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, HashMap},
    time::{Duration, Instant},
};
use uuid::Uuid;
use zeroize::{Zeroize, Zeroizing};

struct SensitiveJson(Value);
fn wipe(v: &mut Value) {
    match v {
        Value::String(s) => s.zeroize(),
        Value::Array(a) => a.iter_mut().for_each(wipe),
        Value::Object(m) => m.values_mut().for_each(wipe),
        _ => {}
    }
}
impl Drop for SensitiveJson {
    fn drop(&mut self) {
        wipe(&mut self.0);
    }
}
pub struct Entry {
    pub source_id: String,
    pub index: usize,
    pub pinned: bool,
    pub record: Option<AccountRecord>,
    pub issue: Option<&'static str>,
}
#[derive(Clone, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct KeyRequest {
    pub id: String,
}
pub struct Decoded {
    pub entries: Vec<Entry>,
    pub keys: Vec<KeyRequest>,
}
#[derive(Deserialize)]
enum BackupKind {
    #[serde(rename = "totp", alias = "1")]
    Totp,
    #[serde(rename = "hotp", alias = "2")]
    Hotp,
}
impl From<BackupKind> for OtpKind {
    fn from(value: BackupKind) -> Self {
        match value {
            BackupKind::Totp => Self::Totp,
            BackupKind::Hotp => Self::Hotp,
        }
    }
}
#[derive(Deserialize)]
enum BackupAlgorithm {
    #[serde(alias = "1")]
    SHA1,
    #[serde(alias = "2")]
    SHA256,
    #[serde(alias = "3")]
    SHA512,
}
impl From<BackupAlgorithm> for OtpAlgorithm {
    fn from(value: BackupAlgorithm) -> Self {
        match value {
            BackupAlgorithm::SHA1 => Self::SHA1,
            BackupAlgorithm::SHA256 => Self::SHA256,
            BackupAlgorithm::SHA512 => Self::SHA512,
        }
    }
}
#[derive(Deserialize)]
struct Raw {
    #[serde(rename = "dataType", default)]
    data_type: Option<String>,
    #[serde(default)]
    hash: Option<String>,
    #[serde(default)]
    encrypted: bool,
    secret: Zeroizing<String>,
    #[serde(rename = "type")]
    kind: BackupKind,
    #[serde(default)]
    issuer: String,
    #[serde(default)]
    account: String,
    #[serde(default)]
    algorithm: Option<BackupAlgorithm>,
    #[serde(default)]
    digits: Option<u8>,
    #[serde(default)]
    period: Option<u32>,
    #[serde(default)]
    counter: Option<u64>,
    #[serde(default)]
    index: usize,
    #[serde(default)]
    pinned: bool,
}
#[derive(Deserialize)]
struct Key {
    id: String,
    salt: String,
    hash: String,
    version: u8,
}

pub fn decode(
    bytes: &[u8],
    passwords: &HashMap<String, Zeroizing<String>>,
    check: impl FnMut() -> AppResult<()>,
) -> AppResult<Decoded> {
    decode_with_progress(bytes, passwords, check, |_, _| {})
}
pub fn decode_with_progress(
    bytes: &[u8],
    passwords: &HashMap<String, Zeroizing<String>>,
    mut check: impl FnMut() -> AppResult<()>,
    mut progress: impl FnMut(usize, usize),
) -> AppResult<Decoded> {
    if bytes.len() as u64 > super::MAX_BACKUP_BYTES {
        return Err(AppError::LimitExceeded);
    }
    let value =
        SensitiveJson(serde_json::from_slice(bytes).map_err(|_| AppError::UnsupportedFormat)?);
    let object = value.0.as_object().ok_or(AppError::UnsupportedFormat)?;
    if object.len() > super::MAX_OTP_ENTRIES + super::MAX_KEY_ENTRIES {
        return Err(AppError::LimitExceeded);
    }
    let mut keys = HashMap::new();
    let mut account_count = 0;
    for (id, v) in object {
        if id.len() > 128 {
            return Err(AppError::LimitExceeded);
        }
        if v.get("dataType").and_then(Value::as_str) == Some("Key") {
            let key: Key =
                serde_json::from_value(v.clone()).map_err(|_| AppError::UnsupportedFormat)?;
            if key.version != 3 {
                return Err(AppError::UnsupportedFormat);
            }
            if key.id != *id {
                return Err(AppError::UnsupportedFormat);
            }
            if key.id.is_empty() || key.id.len() > 128 || !(8..=1024).contains(&key.salt.len()) {
                return Err(AppError::LimitExceeded);
            }
            crypto::validate_phc(&key.hash)?;
            if keys.insert(key.id.clone(), key).is_some() {
                return Err(AppError::UnsupportedFormat);
            }
        } else {
            account_count += 1;
        }
    }
    if keys.len() > super::MAX_KEY_ENTRIES || account_count > super::MAX_OTP_ENTRIES {
        return Err(AppError::LimitExceeded);
    }
    // Validate all envelopes and password costs before starting the first KDF.
    for v in object.values() {
        if v.get("dataType").and_then(Value::as_str) == Some("EncOTPStorage")
            && (v
                .get("data")
                .and_then(Value::as_str)
                .is_none_or(|s| s.len() > super::MAX_CIPHERTEXT_BYTES)
                || v.get("keyId")
                    .and_then(Value::as_str)
                    .is_none_or(|s| s.len() > 128))
        {
            return Err(AppError::LimitExceeded);
        }
    }
    let start = Instant::now();
    let mut derived = HashMap::new();
    let total = keys.keys().filter(|id| passwords.contains_key(*id)).count();
    let mut completed = 0;
    for (id, key) in &keys {
        check()?;
        if start.elapsed() > Duration::from_secs(30) {
            return Err(AppError::LimitExceeded);
        }
        if let Some(password) = passwords.get(id) {
            let pass = crypto::derive(password, &key.salt)?;
            check()?;
            crypto::verify(&pass, &key.hash)?;
            derived.insert(id.clone(), pass);
            completed += 1;
            check()?;
            progress(completed, total);
        }
    }
    let mut entries = Vec::new();
    let mut needed = BTreeMap::new();
    for (source_id, v) in object {
        check()?;
        if start.elapsed() > Duration::from_secs(30) {
            return Err(AppError::LimitExceeded);
        }
        let data_type = v.get("dataType").and_then(Value::as_str);
        if data_type == Some("Key") {
            continue;
        }
        let index = v
            .get("index")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            .min(1_000_000) as usize;
        let mut entry = Entry {
            source_id: source_id.clone(),
            index,
            pinned: false,
            record: None,
            issue: None,
        };
        let raw_bytes = if data_type == Some("EncOTPStorage") {
            let key_id = v
                .get("keyId")
                .and_then(Value::as_str)
                .ok_or(AppError::UnsupportedFormat)?;
            if !keys.contains_key(key_id) {
                entry.issue = Some("MissingKey");
                entries.push(entry);
                continue;
            }
            let Some(pass) = derived.get(key_id) else {
                needed.insert(
                    key_id.to_owned(),
                    KeyRequest {
                        id: key_id.to_owned(),
                    },
                );
                entry.issue = Some("PasswordRequired");
                entries.push(entry);
                continue;
            };
            match crypto::decrypt(
                v.get("data")
                    .and_then(Value::as_str)
                    .ok_or(AppError::UnsupportedFormat)?,
                pass,
            ) {
                Ok(bytes) => bytes,
                Err(_) => {
                    entry.issue = Some("InvalidEncryptedRecord");
                    entries.push(entry);
                    continue;
                }
            }
        } else if data_type.is_none() || data_type == Some("OTPStorage") {
            Zeroizing::new(serde_json::to_vec(v).map_err(|_| AppError::UnsupportedFormat)?)
        } else {
            entry.issue = Some("UnsupportedFormat");
            entries.push(entry);
            continue;
        };
        match serde_json::from_slice::<Raw>(&raw_bytes) {
            Ok(raw)
                // In v3 the reference marks the *inner* record encrypted even
                // though its secret is plain after the whole record decrypts.
                // A legacy plain envelope with encrypted:true stays rejected.
                if (!raw.encrypted || data_type == Some("EncOTPStorage"))
                    && (data_type != Some("EncOTPStorage")
                        || raw.data_type.as_deref() == Some("OTPStorage")
                            && raw.hash.as_deref() == Some(source_id)) =>
            {
                entry.pinned = raw.pinned;
                entry.index = raw.index;
                let input = AccountInput {
                    issuer: raw.issuer,
                    account: raw.account,
                    kind: raw.kind.into(),
                    algorithm: raw.algorithm.map(Into::into).unwrap_or(OtpAlgorithm::SHA1),
                    digits: raw.digits.unwrap_or(6),
                    period: raw.period.unwrap_or(30),
                    next_counter: raw.counter.unwrap_or(0).to_string(),
                    secret: Some(raw.secret),
                };
                match AccountRecord::from_input(&input, None) {
                    Ok(mut record) => {
                        record.provenance = Some(source_id.clone());
                        entry.record = Some(record);
                    }
                    Err(_) => entry.issue = Some("InvalidParameters"),
                }
            }
            Ok(_) => entry.issue = Some("LegacyEncryptedFormat"),
            Err(_) => entry.issue = Some("UnsupportedOrInvalidRecord"),
        }
        entries.push(entry);
    }
    entries.sort_by_key(|e| e.index);
    Ok(Decoded {
        entries,
        keys: needed.into_values().collect(),
    })
}

pub fn encode(
    records: &[(AccountRecord, bool)],
    password: Option<&str>,
    mut check: impl FnMut() -> AppResult<()>,
) -> AppResult<Zeroizing<Vec<u8>>> {
    let mut root = SensitiveJson(Value::Object(Default::default()));
    let key_id = Uuid::new_v4().to_string();
    let salt = Uuid::new_v4().to_string();
    check()?;
    let pass = password.map(|p| crypto::derive(p, &salt)).transpose()?;
    check()?;
    if let Some(pass) = &pass {
        let hash = crypto::verifier(pass)?;
        root.0.as_object_mut().unwrap().insert(
            key_id.clone(),
            json!({"dataType":"Key","id":key_id,"salt":salt,"hash":hash,"version":3}),
        );
    }
    for (index, (r, pinned)) in records.iter().enumerate() {
        check()?;
        r.validate()?;
        let hash = r.id.to_string();
        // The reference FileImport -> EntryStorage.import path uses parseInt
        // for these fields. Its named-enum exports lose HOTP, algorithms and
        // custom periods on import. Numeric strings preserve all three while
        // remaining within its RawOTPStorage string field contract.
        let kind = match r.kind {
            OtpKind::Totp => "1",
            OtpKind::Hotp => "2",
        };
        let algorithm = match r.algorithm {
            OtpAlgorithm::SHA1 => "1",
            OtpAlgorithm::SHA256 => "2",
            OtpAlgorithm::SHA512 => "3",
        };
        let raw = SensitiveJson(
            json!({"dataType":"OTPStorage","encrypted":false,"secret":&*r.secret.encoded(),"type":kind,"hash":hash,"index":index,"issuer":r.issuer,"account":r.account,"algorithm":algorithm,"digits":r.digits,"period":r.period,"counter":r.next_counter,"pinned":pinned}),
        );
        let v = if let Some(pass) = &pass {
            let bytes = Zeroizing::new(
                serde_json::to_vec(&raw.0).map_err(|_| AppError::StorageUnavailable)?,
            );
            json!({"dataType":"EncOTPStorage","keyId":key_id,"data":crypto::encrypt(&bytes,pass)?,"index":index})
        } else {
            raw.0.clone()
        };
        root.0.as_object_mut().unwrap().insert(hash, v);
    }
    check()?;
    Ok(Zeroizing::new(
        serde_json::to_vec_pretty(&root.0).map_err(|_| AppError::StorageUnavailable)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn upstream_reference_plain_defaults_and_v3_multiple_keys() {
        let plain = decode(
            include_bytes!("../../../tests/fixtures/authenticator-plain.json"),
            &HashMap::new(),
            || Ok(()),
        )
        .unwrap();
        assert_eq!(plain.entries.len(), 3);
        assert_eq!(plain.entries[0].record.as_ref().unwrap().next_counter, 7);
        assert_eq!(plain.entries[2].record.as_ref().unwrap().period, 30);
        let bytes = include_bytes!("../../../tests/fixtures/authenticator-v3.json");
        let locked = decode(bytes, &HashMap::new(), || Ok(())).unwrap();
        assert_eq!(locked.keys.len(), 2);
        let p = HashMap::from([
            (
                "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa0".into(),
                Zeroizing::new("synthetic-password-a".into()),
            ),
            (
                "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa1".into(),
                Zeroizing::new("synthetic-password-b".into()),
            ),
        ]);
        let decoded = decode(bytes, &p, || Ok(())).unwrap();
        assert!(decoded.keys.is_empty());
        for (a, b) in plain.entries.iter().zip(&decoded.entries) {
            assert_eq!(a.source_id, b.source_id);
            assert_eq!(a.index, b.index);
            assert_eq!(a.pinned, b.pinned);
            assert!(a.issue.is_none() && b.issue.is_none());
            let a = a.record.as_ref().unwrap();
            let b = b.record.as_ref().unwrap();
            assert_eq!(a.kind, b.kind);
            assert_eq!(a.algorithm, b.algorithm);
            assert_eq!(a.digits, b.digits);
            assert_eq!(a.period, b.period);
            assert_eq!(a.issuer, b.issuer);
            assert_eq!(a.account, b.account);
            assert_eq!(a.secret.bytes(), b.secret.bytes());
            assert_eq!(a.next_counter, b.next_counter);
        }
        let wrong = HashMap::from([(
            "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa0".into(),
            Zeroizing::new("wrong".into()),
        )]);
        assert!(matches!(
            decode(bytes, &wrong, || Ok(())),
            Err(AppError::InvalidPassword)
        ));
    }
    #[test]
    fn mixed_backup_keeps_plain_entries_and_requests_only_remaining_keys() {
        let mut mixed: Value = serde_json::from_slice(include_bytes!(
            "../../../tests/fixtures/authenticator-v3.json"
        ))
        .unwrap();
        mixed.as_object_mut().unwrap().insert(
            "44444444-4444-4444-8444-444444444444".into(),
            json!({"type":"totp","secret":"JBSWY3DPEHPK3PXP","index":3}),
        );
        let bytes = serde_json::to_vec(&mixed).unwrap();
        let locked = decode(&bytes, &HashMap::new(), || Ok(())).unwrap();
        assert_eq!(locked.keys.len(), 2);
        assert_eq!(
            locked.entries.iter().filter(|e| e.record.is_some()).count(),
            1
        );
        let partial = HashMap::from([(
            "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa0".into(),
            Zeroizing::new("synthetic-password-a".into()),
        )]);
        let decoded = decode(&bytes, &partial, || Ok(())).unwrap();
        assert_eq!(decoded.keys.len(), 1);
        assert_eq!(decoded.keys[0].id, "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa1");
        assert_eq!(
            decoded
                .entries
                .iter()
                .filter(|e| e.record.is_some())
                .count(),
            3
        );
        assert_eq!(decoded.entries[0].issue, Some("PasswordRequired"));
    }
    #[test]
    fn key_object_id_must_match_its_map_key_as_required_by_original_import() {
        let mut value: Value = serde_json::from_slice(include_bytes!(
            "../../../tests/fixtures/authenticator-v3.json"
        ))
        .unwrap();
        value["aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa0"]["id"] = json!("different-key-id");
        assert!(matches!(
            decode(
                &serde_json::to_vec(&value).unwrap(),
                &HashMap::new(),
                || Ok(())
            ),
            Err(AppError::UnsupportedFormat)
        ));
    }
    #[test]
    fn encrypted_export_roundtrip_preserves_counter_without_consuming() {
        let record = crate::compatibility::uri::parse_uri(
            "otpauth://hotp/Synthetic?secret=JBSWY3DPEHPK3PXP&counter=42",
        )
        .unwrap();
        let bytes = encode(&[(record, true)], Some("synthetic-export-password"), || {
            Ok(())
        })
        .unwrap();
        let preview = decode(&bytes, &HashMap::new(), || Ok(())).unwrap();
        let p = HashMap::from([(
            preview.keys[0].id.clone(),
            Zeroizing::new("synthetic-export-password".into()),
        )]);
        let data = decode(&bytes, &p, || Ok(())).unwrap();
        assert_eq!(data.entries[0].record.as_ref().unwrap().next_counter, 42);
        assert!(data.entries[0].pinned);
    }
    #[test]
    fn unsupported_and_missing_keys_are_visible_and_cancellation_stops_work() {
        let bytes=br#"{"a":{"dataType":"EncOTPStorage","keyId":"absent","data":"Salted-invalid"},"b":{"type":"steam","secret":"SYNTHETIC"},"c":{"type":"totp","secret":"JBSWY3DPEHPK3PXP","encrypted":true}}"#;
        let decoded = decode(bytes, &HashMap::new(), || Ok(())).unwrap();
        assert_eq!(decoded.entries.len(), 3);
        assert!(
            decoded
                .entries
                .iter()
                .all(|e| e.record.is_none() && e.issue.is_some())
        );
        assert!(matches!(
            decode(bytes, &HashMap::new(), || Err(AppError::Cancelled)),
            Err(AppError::Cancelled)
        ));
    }
}
