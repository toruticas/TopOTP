//! Cross-language regression: the real Rust codecs against unmodified pinned
//! Authenticator import/export/OTP routines, with synthetic browser adapters.
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    io::Write,
    path::Path,
    process::{Command, Stdio},
};
use topotp_lib::{
    compatibility::backup,
    domain::{
        account::{AccountInput, AccountRecord, OtpAlgorithm, OtpKind, Secret},
        otp,
    },
};
use zeroize::Zeroizing;

#[test]
fn plain_and_v3_roundtrip_through_original_file_import_and_export() {
    let mut records = Vec::new();
    for (index, algorithm) in [
        OtpAlgorithm::SHA1,
        OtpAlgorithm::SHA256,
        OtpAlgorithm::SHA512,
    ]
    .into_iter()
    .enumerate()
    {
        for kind in [OtpKind::Totp, OtpKind::Hotp] {
            let input = AccountInput {
                issuer: "Sintético & <teste> 🔑".into(),
                account: format!("{kind:?}-{algorithm:?}@example.test"),
                kind,
                algorithm,
                digits: 6 + index as u8,
                period: if kind == OtpKind::Totp { 45 } else { 30 },
                next_counter: "123456".into(),
                secret: Some(
                    Secret::parse("GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ")
                        .unwrap()
                        .encoded(),
                ),
            };
            records.push((AccountRecord::from_input(&input, None).unwrap(), index == 1));
        }
    }
    let expected: Vec<Value> = records
        .iter()
        .map(|(r, pinned)| {
            let codes: Vec<_> = [0, 44, 45, 59, 1111111109, 1234567890, 20000000000]
                .into_iter()
                .map(|time| json!({"time":time,"code":otp::totp(r,time).unwrap().0}))
                .collect();
            json!({"issuer":r.issuer,"account":r.account,"kind":r.kind,"algorithm":r.algorithm,
                "digits":r.digits,"period":r.period,"counter":r.next_counter,"pinned":pinned,
                "secret":&*r.secret.encoded(),"codes":codes,
                "code":otp::hotp(r.secret.bytes(),r.next_counter,r.algorithm,r.digits)})
        })
        .collect();
    let plain = backup::encode(&records, None, || Ok(())).unwrap();
    let password = "Senha sintética 🔐 com espaços";
    let encrypted = backup::encode(&records, Some(password), || Ok(())).unwrap();
    let request = json!({
        "plain":serde_json::from_slice::<Value>(&plain).unwrap(),
        "encrypted":serde_json::from_slice::<Value>(&encrypted).unwrap(),
        "password":password,"expected":expected,
    });
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let mut child = Command::new("node")
        .arg("tests/compatibility/reference-roundtrip.mjs")
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Install the pinned Node/npm test dependencies before cargo test");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&request).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "Original Authenticator rejected the backup:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    for format in ["plain", "encrypted", "reorderedEncrypted"] {
        let bytes = serde_json::to_vec(&response[format]).unwrap();
        let preview = backup::decode(&bytes, &HashMap::new(), || Ok(())).unwrap();
        let passwords = preview
            .keys
            .iter()
            .map(|key| (key.id.clone(), Zeroizing::new(password.to_owned())))
            .collect();
        let decoded = backup::decode(&bytes, &passwords, || Ok(())).unwrap();
        assert!(decoded.keys.is_empty());
        assert_eq!(decoded.entries.len(), records.len());
        let mut original_records: Vec<_> = records.iter().collect();
        if format == "reorderedEncrypted" {
            original_records.reverse();
        }
        for (entry, (original, pinned)) in decoded.entries.iter().zip(original_records) {
            assert!(entry.issue.is_none(), "{format}: {:?}", entry.issue);
            let restored = entry.record.as_ref().unwrap();
            assert_eq!(restored.kind, original.kind);
            assert_eq!(restored.algorithm, original.algorithm);
            assert_eq!(restored.digits, original.digits);
            assert_eq!(restored.period, original.period);
            assert_eq!(restored.next_counter, original.next_counter);
            assert_eq!(restored.issuer, original.issuer);
            assert_eq!(restored.account, original.account);
            assert_eq!(entry.pinned, *pinned);
            assert_eq!(restored.secret.bytes(), original.secret.bytes());
        }
    }
}
