use super::core::*;
use super::{ports::*, vault::*};
use crate::domain::{AppError, AppResult};
use std::time::Duration;
use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
};
use uuid::Uuid;
use zeroize::Zeroizing;
#[derive(Default)]
struct Data {
    records: HashMap<Uuid, Vec<u8>>,
    reads: AtomicUsize,
    fail_read: bool,
    fail_update: bool,
    fail_delete: bool,
}
struct Storage(Arc<Mutex<Data>>);
impl Store for Storage {
    fn read(&self, id: Uuid) -> AppResult<Option<Zeroizing<Vec<u8>>>> {
        let s = self.0.lock().unwrap();
        s.reads.fetch_add(1, Ordering::SeqCst);
        if s.fail_read {
            return Err(AppError::StorageUnavailable);
        }
        Ok(s.records.get(&id).cloned().map(Zeroizing::new))
    }
    fn insert(&mut self, id: Uuid, b: &[u8]) -> AppResult<()> {
        self.0.lock().unwrap().records.insert(id, b.to_vec());
        Ok(())
    }
    fn update(&mut self, id: Uuid, b: &[u8]) -> AppResult<()> {
        let mut s = self.0.lock().unwrap();
        if s.fail_update {
            return Err(AppError::StorageUnavailable);
        }
        s.records.insert(id, b.to_vec());
        Ok(())
    }
    fn delete(&mut self, id: Uuid) -> AppResult<()> {
        let mut s = self.0.lock().unwrap();
        if s.fail_delete {
            return Err(AppError::StorageUnavailable);
        }
        s.records.remove(&id);
        Ok(())
    }
}
struct Auth;
impl Authentication for Auth {
    fn authenticate<'a>(
        &'a self,
        _: &'a str,
    ) -> Pin<Box<dyn Future<Output = AppResult<NativeApproval>> + Send + 'a>> {
        Box::pin(async { Err(AppError::Cancelled) })
    }
    fn cancel(&self) {}
}
struct Clip;
impl Clipboard for Clip {
    fn copy(&self, _: &str) -> AppResult<i64> {
        Ok(1)
    }
    fn clear_if_owned(&self, _: i64) {}
}
#[derive(Default)]
struct FakeClock(AtomicU64);
impl Clock for FakeClock {
    fn elapsed(&self) -> Duration {
        Duration::from_secs(self.0.load(Ordering::SeqCst))
    }
    fn unix_seconds(&self) -> u64 {
        59
    }
}
struct Fixture {
    core: Arc<Core>,
    data: Arc<Mutex<Data>>,
    clock: Arc<FakeClock>,
    root: std::path::PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let data = Arc::new(Mutex::new(Data::default()));
        let clock = Arc::new(FakeClock::default());
        let root = std::env::temp_dir().join(format!("topotp-synthetic-{}", Uuid::new_v4()));
        let core = Arc::new(
            Core::new(
                Box::new(Storage(data.clone())),
                DiskIndex::new(root.clone()).unwrap(),
                Arc::new(Auth),
                Arc::new(Clip),
                clock.clone(),
            )
            .unwrap(),
        );
        Self {
            core,
            data,
            clock,
            root,
        }
    }
    fn unlock(&self) -> String {
        self.core.visibility(true);
        let t = self.core.begin_unlock().unwrap();
        self.core
            .finish_unlock(t, NativeApproval::from_native_success())
            .unwrap()
            .epoch
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
#[test]
fn lock_during_calculation_revokes_disclosure_before_mutex_is_released() {
    let f = Fixture::new();
    let e = f.unlock();
    let before = f.core.interruption_generation();
    let (entered, ready) = std::sync::mpsc::channel();
    let (release, wait) = std::sync::mpsc::channel();
    let c = f.core.clone();
    let work = std::thread::spawn(move || {
        c.authorized(&e, |_| {
            entered.send(()).unwrap();
            wait.recv().unwrap();
            Ok("synthetic code")
        })
    });
    ready.recv().unwrap();
    let c = f.core.clone();
    let lock = std::thread::spawn(move || c.lock());
    let start = std::time::Instant::now();
    while f.core.interruption_generation() == before {
        assert!(start.elapsed() < Duration::from_secs(2));
        std::thread::yield_now();
    }
    release.send(()).unwrap();
    assert!(matches!(work.join().unwrap(), Err(AppError::Locked)));
    lock.join().unwrap();
}
#[test]
fn reopening_after_hidden_deadline_cannot_restore_authorization() {
    let f = Fixture::new();
    let e = f.unlock();
    f.core.visibility(false);
    f.clock.0.store(30, Ordering::SeqCst);
    f.core.visibility(true);
    assert!(f.core.list(&e).is_err());
}
#[test]
fn quick_reopen_keeps_preview_but_exactly_thirty_seconds_revokes_it() {
    let f = Fixture::new();
    let e = f.unlock();
    let preview = f
        .core
        .prepare_uri(&e, "otpauth://totp/Synthetic?secret=JBSWY3DPEHPK3PXP")
        .unwrap();
    f.core.visibility(false);
    f.clock.0.store(29, Ordering::SeqCst);
    f.core.visibility(true);
    assert_eq!(f.core.status().unwrap().epoch, e);
    f.core
        .commit_import(
            &e,
            Uuid::parse_str(&preview.id).unwrap(),
            vec![Uuid::parse_str(&preview.rows[0].account.as_ref().unwrap().id).unwrap()],
            false,
        )
        .unwrap();
    let preview = f
        .core
        .prepare_uri(&e, "otpauth://totp/Other?secret=JBSWY3DPEHPK3PXQ")
        .unwrap();
    f.core.visibility(false);
    f.clock.0.store(59, Ordering::SeqCst);
    f.core.visibility(true);
    assert!(f.core.list(&e).is_err());
    assert!(matches!(
        f.core.commit_import(
            &e,
            Uuid::parse_str(&preview.id).unwrap(),
            vec![Uuid::parse_str(&preview.rows[0].account.as_ref().unwrap().id).unwrap()],
            false
        ),
        Err(AppError::Locked)
    ));
}
#[test]
fn import_keeps_hotp_after_label_edit_and_does_not_regress_counter() {
    let f = Fixture::new();
    let e = f.unlock();
    let bytes = include_bytes!("../../../tests/fixtures/authenticator-plain.json");
    let p = f
        .core
        .prepare_import(&e, Zeroizing::new(bytes.to_vec()))
        .unwrap();
    let ids = p
        .rows
        .iter()
        .filter_map(|r| r.account.as_ref())
        .map(|a| Uuid::parse_str(&a.id).unwrap())
        .collect();
    f.core
        .commit_import(&e, Uuid::parse_str(&p.id).unwrap(), ids, false)
        .unwrap();
    let r = f
        .core
        .list(&e)
        .unwrap()
        .into_iter()
        .find(|a| a.kind == crate::domain::account::OtpKind::Hotp)
        .unwrap();
    let id = Uuid::parse_str(&r.id).unwrap();
    f.core.hotp(&e, id).unwrap();
    f.core
        .save(
            &e,
            Some(id),
            crate::domain::account::AccountInput {
                issuer: "Renamed".into(),
                account: "Changed label".into(),
                kind: r.kind,
                algorithm: r.algorithm,
                digits: r.digits,
                period: r.period,
                next_counter: "8".into(),
                secret: None,
            },
        )
        .unwrap();
    let p = f
        .core
        .prepare_import(&e, Zeroizing::new(bytes.to_vec()))
        .unwrap();
    assert!(
        p.rows
            .iter()
            .all(|r| r.issue.as_deref() == Some("DuplicateKept"))
    );
    assert_eq!(
        f.core
            .list(&e)
            .unwrap()
            .into_iter()
            .find(|a| a.id == id.to_string())
            .unwrap()
            .next_counter
            .as_deref(),
        Some("8")
    );
}
#[test]
fn hotp_import_ignores_changed_period_and_preserves_consumed_counter() {
    let f = Fixture::new();
    let e = f.unlock();
    let preview = f
        .core
        .prepare_import(&e, synthetic_hotp_backup("original-id", 30, 7))
        .unwrap();
    let original = Uuid::parse_str(&preview.rows[0].account.as_ref().unwrap().id).unwrap();
    f.core
        .commit_import(&e, preview.id.parse().unwrap(), vec![original], false)
        .unwrap();
    assert_eq!(f.core.hotp(&e, original).unwrap().code, "162583");

    let preview = f
        .core
        .prepare_import(&e, synthetic_hotp_backup("changed-id", 31, 7))
        .unwrap();
    assert_eq!(preview.rows[0].issue.as_deref(), Some("DuplicateKept"));
    let duplicate = Uuid::parse_str(&preview.rows[0].account.as_ref().unwrap().id).unwrap();
    assert!(matches!(
        f.core
            .commit_import(&e, preview.id.parse().unwrap(), vec![duplicate], false),
        Err(AppError::InvalidInput { .. })
    ));
    let accounts = f.core.list(&e).unwrap();
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].id, original.to_string());
    assert_eq!(accounts[0].next_counter.as_deref(), Some("8"));
    assert_eq!(f.core.hotp(&e, original).unwrap().code, "399871");
}

fn synthetic_hotp_backup(source: &str, period: u32, counter: u64) -> Zeroizing<Vec<u8>> {
    Zeroizing::new(
        serde_json::to_vec(&serde_json::json!({
            (source): {
                "dataType": "OTPStorage",
                "type": "hotp",
                "secret": "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ",
                "account": "Synthetic RFC",
                "period": period,
                "counter": counter,
            }
        }))
        .unwrap(),
    )
}

#[test]
fn import_deduplicates_hotp_periods_within_the_backup_and_requires_subset_consent() {
    let f = Fixture::new();
    let e = f.unlock();
    let mut backup: serde_json::Value =
        serde_json::from_slice(&synthetic_hotp_backup("first", 30, 8)).unwrap();
    let second: serde_json::Value =
        serde_json::from_slice(&synthetic_hotp_backup("second", 31, 7)).unwrap();
    backup["first"]["index"] = serde_json::json!(0);
    backup["second"] = second["second"].clone();
    backup["second"]["index"] = serde_json::json!(1);
    let preview = f
        .core
        .prepare_import(&e, Zeroizing::new(serde_json::to_vec(&backup).unwrap()))
        .unwrap();
    assert!(preview.rows[0].issue.is_none());
    assert_eq!(preview.rows[1].issue.as_deref(), Some("DuplicateKept"));
    let first = Uuid::parse_str(&preview.rows[0].account.as_ref().unwrap().id).unwrap();
    let second = Uuid::parse_str(&preview.rows[1].account.as_ref().unwrap().id).unwrap();
    let preview_id = preview.id.parse().unwrap();
    for (ids, acknowledge) in [
        (vec![first, second], true),
        (vec![second], true),
        (vec![first], false),
    ] {
        assert!(matches!(
            f.core.commit_import(&e, preview_id, ids, acknowledge),
            Err(AppError::InvalidInput { .. })
        ));
        assert!(f.core.list(&e).unwrap().is_empty());
    }
    f.core
        .commit_import(&e, preview_id, vec![first], true)
        .unwrap();
    assert_eq!(f.core.list(&e).unwrap().len(), 1);
    assert_eq!(
        f.core.list(&e).unwrap()[0].next_counter.as_deref(),
        Some("8")
    );
}

#[test]
fn import_commit_rechecks_new_vault_duplicates_and_preserves_preview_on_conflict() {
    let f = Fixture::new();
    let e = f.unlock();
    let preview = f
        .core
        .prepare_import(&e, synthetic_hotp_backup("preview-id", 30, 7))
        .unwrap();
    assert!(preview.rows[0].issue.is_none());
    let selected = Uuid::parse_str(&preview.rows[0].account.as_ref().unwrap().id).unwrap();
    let preview_id = preview.id.parse().unwrap();

    f.core
        .save(
            &e,
            None,
            crate::domain::account::AccountInput {
                issuer: "Current vault".into(),
                account: "Synthetic RFC".into(),
                kind: crate::domain::account::OtpKind::Hotp,
                algorithm: crate::domain::account::OtpAlgorithm::SHA1,
                digits: 6,
                period: 30,
                next_counter: "7".into(),
                secret: Some(Zeroizing::new("GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ".into())),
            },
        )
        .unwrap();
    let current = Uuid::parse_str(&f.core.list(&e).unwrap()[0].id).unwrap();
    assert_eq!(f.core.hotp(&e, current).unwrap().code, "162583");
    assert!(matches!(
        f.core.commit_import(&e, preview_id, vec![selected], false),
        Err(AppError::InvalidInput { .. })
    ));
    let accounts = f.core.list(&e).unwrap();
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].id, current.to_string());
    assert_eq!(accounts[0].next_counter.as_deref(), Some("8"));

    // Explicitly removing the conflicting account allows retrying the same preview.
    f.core.delete(&e, current).unwrap();
    f.core
        .commit_import(&e, preview_id, vec![selected], false)
        .unwrap();
    assert_eq!(f.core.list(&e).unwrap()[0].id, selected.to_string());
}

#[test]
fn import_commit_keeps_preview_when_duplicate_revalidation_cannot_read_storage() {
    let f = Fixture::new();
    let e = f.unlock();
    let existing =
        crate::compatibility::uri::parse_uri("otpauth://totp/Existing?secret=JBSWY3DPEHPK3PXP")
            .unwrap();
    f.core
        .authorized(&e, |i| i.vault.commit(vec![(existing, false)], &[]))
        .unwrap();
    let preview = f
        .core
        .prepare_import(&e, synthetic_hotp_backup("new-id", 30, 7))
        .unwrap();
    let selected = Uuid::parse_str(&preview.rows[0].account.as_ref().unwrap().id).unwrap();
    let preview_id = preview.id.parse().unwrap();
    f.data.lock().unwrap().fail_read = true;
    assert!(matches!(
        f.core.commit_import(&e, preview_id, vec![selected], false),
        Err(AppError::StorageUnavailable)
    ));
    f.data.lock().unwrap().fail_read = false;
    assert_eq!(f.core.list(&e).unwrap().len(), 1);
    f.core
        .commit_import(&e, preview_id, vec![selected], false)
        .unwrap();
    assert_eq!(f.core.list(&e).unwrap().len(), 2);
}

#[test]
fn import_password_retry_keeps_preview_after_duplicate_storage_read_failure() {
    let f = Fixture::new();
    let e = f.unlock();
    let existing =
        crate::compatibility::uri::parse_uri("otpauth://totp/Existing?secret=JBSWY3DPEHPK3PXP")
            .unwrap();
    f.core
        .authorized(&e, |i| i.vault.commit(vec![(existing, false)], &[]))
        .unwrap();
    let imported = crate::compatibility::uri::parse_uri(
        "otpauth://hotp/Synthetic?secret=GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ&counter=7",
    )
    .unwrap();
    let bytes = crate::compatibility::backup::encode(
        &[(imported, false)],
        Some("synthetic-backup-password"),
        || Ok(()),
    )
    .unwrap();
    let preview = f.core.prepare_import(&e, bytes).unwrap();
    let preview_id = preview.id.parse().unwrap();
    let passwords = || {
        HashMap::from([(
            preview.keys[0].id.clone(),
            Zeroizing::new("synthetic-backup-password".into()),
        )])
    };
    f.data.lock().unwrap().fail_read = true;
    assert!(matches!(
        f.core.passwords(&e, preview_id, passwords(), |_, _| {}),
        Err(AppError::StorageUnavailable)
    ));
    f.data.lock().unwrap().fail_read = false;
    let decrypted = f
        .core
        .passwords(&e, preview_id, passwords(), |_, _| {})
        .unwrap();
    assert_eq!(decrypted.id, preview.id);
    assert!(decrypted.rows[0].issue.is_none());
    assert_eq!(f.core.list(&e).unwrap().len(), 1);
    f.core
        .commit_import(
            &e,
            preview_id,
            vec![
                decrypted.rows[0]
                    .account
                    .as_ref()
                    .unwrap()
                    .id
                    .parse()
                    .unwrap(),
            ],
            false,
        )
        .unwrap();
    assert_eq!(f.core.list(&e).unwrap().len(), 2);
}

#[test]
fn import_keeps_distinct_totp_periods_and_their_codes() {
    let f = Fixture::new();
    let e = f.unlock();
    let mut backup: serde_json::Value =
        serde_json::from_slice(&synthetic_hotp_backup("period-30", 30, 0)).unwrap();
    backup["period-30"]["type"] = serde_json::json!("totp");
    backup["period-60"] = backup["period-30"].clone();
    backup["period-60"]["period"] = serde_json::json!(60);
    let preview = f
        .core
        .prepare_import(&e, Zeroizing::new(serde_json::to_vec(&backup).unwrap()))
        .unwrap();
    assert_eq!(preview.rows.len(), 2);
    assert!(preview.rows.iter().all(|r| r.issue.is_none()));
    let selected = preview
        .rows
        .iter()
        .map(|r| r.account.as_ref().unwrap().id.parse().unwrap())
        .collect();
    f.core
        .commit_import(&e, preview.id.parse().unwrap(), selected, false)
        .unwrap();
    let mut codes: Vec<_> = f
        .core
        .totp(&e)
        .unwrap()
        .into_iter()
        .map(|c| c.code)
        .collect();
    codes.sort();
    assert_eq!(codes, ["287082", "755224"]);
}

#[test]
fn plaintext_export_never_overwrites_and_lock_prevents_destination_creation() {
    let f = Fixture::new();
    let e = f.unlock();
    let path = f.root.join("synthetic.json");
    f.core
        .write_export(&e, &path, b"synthetic plaintext", false)
        .unwrap();
    assert!(
        f.core
            .write_export(&e, &path, b"replacement", false)
            .is_err()
    );
    assert_eq!(std::fs::read(&path).unwrap(), b"synthetic plaintext");
    f.core.lock();
    let cancelled = f.root.join("cancelled.json");
    assert!(
        f.core
            .write_export(&e, &cancelled, b"synthetic", true)
            .is_err()
    );
    assert!(!cancelled.exists());
}
#[test]
fn no_credential_reads_before_auth_or_after_late_success() {
    let f = Fixture::new();
    assert!(f.core.list("1").is_err());
    let ticket = f.core.begin_unlock().unwrap();
    assert!(f.core.list(&ticket.to_string()).is_err());
    f.core.lock();
    assert!(
        f.core
            .finish_unlock(ticket, NativeApproval::from_native_success())
            .is_err()
    );
    assert_eq!(f.data.lock().unwrap().reads.load(Ordering::SeqCst), 0);
}
#[test]
fn uri_preview_has_no_secret_and_requires_commit() {
    let f = Fixture::new();
    let e = f.unlock();
    let p = f
        .core
        .prepare_uri(&e, "otpauth://totp/URI?secret=JBSWY3DPEHPK3PXP")
        .unwrap();
    assert!(
        !serde_json::to_string(&p)
            .unwrap()
            .contains("JBSWY3DPEHPK3PXP")
    );
    assert!(f.core.list(&e).unwrap().is_empty());
    let id = Uuid::parse_str(&p.rows[0].account.as_ref().unwrap().id).unwrap();
    f.core
        .commit_import(&e, Uuid::parse_str(&p.id).unwrap(), vec![id], false)
        .unwrap();
    assert_eq!(f.core.list(&e).unwrap().len(), 1);
}
#[test]
fn timers_and_lock_reject_old_epochs_without_reads() {
    let f = Fixture::new();
    let e = f.unlock();
    f.core.list(&e).unwrap();
    f.clock.0.store(120, Ordering::SeqCst);
    assert!(f.core.tick());
    assert!(f.core.list(&e).is_err());
    let e2 = f.unlock();
    assert!(f.core.list(&e).is_err());
    f.core.list(&e2).unwrap();
}
#[test]
fn concurrent_hotp_consumption_is_serial_and_failure_discloses_nothing() {
    let f = Fixture::new();
    let e = f.unlock();
    let r = crate::compatibility::uri::parse_uri(
        "otpauth://hotp/Synthetic?secret=GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ&counter=0",
    )
    .unwrap();
    let id = r.id;
    f.core
        .authorized(&e, |i| i.vault.commit(vec![(r, false)], &[]))
        .unwrap();
    let workers: Vec<_> = (0..10)
        .map(|_| {
            let c = f.core.clone();
            let e = e.clone();
            std::thread::spawn(move || c.hotp(&e, id).unwrap().code)
        })
        .collect();
    let mut codes: Vec<_> = workers.into_iter().map(|w| w.join().unwrap()).collect();
    codes.sort();
    codes.dedup();
    assert_eq!(codes.len(), 10);
    assert_eq!(
        f.core.list(&e).unwrap()[0].next_counter.as_deref(),
        Some("10")
    );
    f.data.lock().unwrap().fail_update = true;
    assert!(f.core.hotp(&e, id).is_err());
    assert_eq!(
        f.core.list(&e).unwrap()[0].next_counter.as_deref(),
        Some("10")
    );
    f.core.lock();
    assert!(f.core.copy(&e, id).is_err());
}
#[test]
fn import_preview_is_revoked_before_commit() {
    let f = Fixture::new();
    let e = f.unlock();
    let p = f
        .core
        .prepare_import(
            &e,
            Zeroizing::new(
                include_bytes!("../../../tests/fixtures/authenticator-plain.json").to_vec(),
            ),
        )
        .unwrap();
    let selected = p
        .rows
        .iter()
        .filter_map(|r| r.account.as_ref().map(|a| Uuid::parse_str(&a.id).unwrap()))
        .collect();
    f.core.lock();
    assert!(
        f.core
            .commit_import(&e, Uuid::parse_str(&p.id).unwrap(), selected, false)
            .is_err()
    );
    let e2 = f.unlock();
    assert!(f.core.list(&e2).unwrap().is_empty());
}

#[test]
fn failed_secret_deletion_is_reported_and_recovered_on_next_unlock() {
    let f = Fixture::new();
    let e = f.unlock();
    let record = crate::compatibility::uri::parse_uri(
        "otpauth://hotp/Synthetic?secret=JBSWY3DPEHPK3PXP&counter=0",
    )
    .unwrap();
    let id = record.id;
    f.core
        .authorized(&e, |i| i.vault.commit(vec![(record, false)], &[]))
        .unwrap();
    f.core.hotp(&e, id).unwrap();
    f.data.lock().unwrap().fail_delete = true;

    let error = f.core.delete(&e, id).unwrap_err();
    assert_eq!(
        serde_json::to_value(error).unwrap()["code"],
        "DeletionPending"
    );
    assert!(f.core.list(&e).unwrap().is_empty());
    assert!(
        !f.core
            .authorized(&e, |i| Ok(i.hotp.contains_key(&id)))
            .unwrap()
    );
    assert_eq!(f.data.lock().unwrap().records.len(), 1);
    assert!(f.root.join("journal.json").exists());

    f.core.lock();
    f.data.lock().unwrap().fail_delete = false;
    let e = f.unlock();
    assert!(f.core.list(&e).unwrap().is_empty());
    assert!(f.data.lock().unwrap().records.is_empty());
    assert!(!f.root.join("journal.json").exists());
}
