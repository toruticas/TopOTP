use super::core::*;
use super::session::Phase;
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
    fail_update: bool,
    fail_insert: bool,
    deny_read: bool,
}
struct Storage(Arc<Mutex<Data>>);
impl Store for Storage {
    fn read(&self, id: Uuid) -> AppResult<Option<Zeroizing<Vec<u8>>>> {
        let s = self.0.lock().unwrap();
        s.reads.fetch_add(1, Ordering::SeqCst);
        if s.deny_read {
            return Err(AppError::KeychainAccessDenied);
        }
        Ok(s.records.get(&id).cloned().map(Zeroizing::new))
    }
    fn insert(&mut self, id: Uuid, b: &[u8]) -> AppResult<()> {
        if self.0.lock().unwrap().fail_insert {
            return Err(AppError::StorageUnavailable);
        }
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
        self.0.lock().unwrap().records.remove(&id);
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
        Self::with_auth(Arc::new(Auth))
    }
    fn with_auth(auth: Arc<dyn Authentication>) -> Self {
        let data = Arc::new(Mutex::new(Data::default()));
        let clock = Arc::new(FakeClock::default());
        let root = std::env::temp_dir().join(format!("topotp-synthetic-{}", Uuid::new_v4()));
        let core = Arc::new(
            Core::new(
                Box::new(Storage(data.clone())),
                DiskIndex::new(root.clone()).unwrap(),
                auth,
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
fn storage_denial_revokes_session_and_background_work_never_retries_reads() {
    let f = Fixture::new();
    let epoch = f.unlock();
    let preview = f
        .core
        .prepare_uri(
            &epoch,
            "otpauth://hotp/Synthetic?secret=JBSWY3DPEHPK3PXP&counter=7",
        )
        .unwrap();
    let id = Uuid::parse_str(&preview.rows[0].account.as_ref().unwrap().id).unwrap();
    f.core
        .commit_import(
            &epoch,
            Uuid::parse_str(&preview.id).unwrap(),
            vec![id],
            false,
        )
        .unwrap();
    f.core.hotp(&epoch, id).unwrap();
    f.core
        .prepare_uri(&epoch, "otpauth://totp/Pending?secret=GEZDGNBVGY3TQOJQ")
        .unwrap();
    f.data.lock().unwrap().deny_read = true;
    assert!(matches!(
        f.core.presentation_snapshot(&epoch),
        Err(AppError::KeychainAccessDenied)
    ));
    let reads = f.data.lock().unwrap().reads.load(Ordering::SeqCst);
    for _ in 0..20 {
        f.core.tick();
        assert_eq!(f.core.status().unwrap().phase, Phase::Locked);
        assert!(matches!(
            f.core.presentation_snapshot(&epoch),
            Err(AppError::Locked)
        ));
        assert!(matches!(f.core.list(&epoch), Err(AppError::Locked)));
    }
    assert_eq!(f.data.lock().unwrap().reads.load(Ordering::SeqCst), reads);
    f.data.lock().unwrap().deny_read = false;
    let next_epoch = f.unlock();
    f.core
        .authorized(&next_epoch, |inner| {
            assert!(inner.hotp.is_empty());
            assert!(inner.preview.is_none());
            Ok(())
        })
        .unwrap();
}

#[test]
fn migration_waits_for_a_committed_import_and_preserves_current_hotp_counter() {
    let f = Fixture::new();
    f.core.require_legacy_migration();
    let epoch = f.unlock();
    assert_eq!(
        f.core.status().unwrap().legacy_migration_required,
        Some(true)
    );
    let preview = f
        .core
        .prepare_uri(
            &epoch,
            "otpauth://hotp/Synthetic?secret=JBSWY3DPEHPK3PXP&counter=123",
        )
        .unwrap();
    let id = Uuid::parse_str(&preview.rows[0].account.as_ref().unwrap().id).unwrap();
    assert_eq!(
        f.core.status().unwrap().legacy_migration_required,
        Some(true)
    );
    f.data.lock().unwrap().fail_insert = true;
    assert!(
        f.core
            .commit_import(
                &epoch,
                Uuid::parse_str(&preview.id).unwrap(),
                vec![id],
                false
            )
            .is_err()
    );
    assert_eq!(
        f.core.status().unwrap().legacy_migration_required,
        Some(true)
    );
    f.data.lock().unwrap().fail_insert = false;
    let preview = f
        .core
        .prepare_uri(
            &epoch,
            "otpauth://hotp/Synthetic?secret=JBSWY3DPEHPK3PXP&counter=123",
        )
        .unwrap();
    let id = Uuid::parse_str(&preview.rows[0].account.as_ref().unwrap().id).unwrap();
    f.core
        .commit_import(
            &epoch,
            Uuid::parse_str(&preview.id).unwrap(),
            vec![id],
            false,
        )
        .unwrap();
    assert_eq!(f.core.status().unwrap().legacy_migration_required, None);
    assert_eq!(
        f.core.list(&epoch).unwrap()[0].next_counter.as_deref(),
        Some("123")
    );
    f.core.delete(&epoch, id).unwrap();
    assert_eq!(f.core.status().unwrap().legacy_migration_required, None);
}

#[test]
fn encrypted_migration_between_vaults_preserves_accounts_order_pins_and_latest_counters() {
    let old = Fixture::new();
    let old_epoch = old.unlock();
    let preview = old
        .core
        .prepare_import(
            &old_epoch,
            Zeroizing::new(
                include_bytes!("../../../tests/fixtures/authenticator-plain.json").to_vec(),
            ),
        )
        .unwrap();
    let mut ids: Vec<_> = preview
        .rows
        .iter()
        .filter_map(|row| row.account.as_ref())
        .map(|account| Uuid::parse_str(&account.id).unwrap())
        .collect();
    old.core
        .commit_import(
            &old_epoch,
            Uuid::parse_str(&preview.id).unwrap(),
            ids.clone(),
            false,
        )
        .unwrap();
    ids.reverse();
    old.core
        .authorized(&old_epoch, |inner| inner.vault.order(&ids, &ids[..1]))
        .unwrap();
    let hotp = old
        .core
        .list(&old_epoch)
        .unwrap()
        .into_iter()
        .find(|account| account.kind == crate::domain::account::OtpKind::Hotp)
        .unwrap();
    let hotp_id = Uuid::parse_str(&hotp.id).unwrap();
    old.core.hotp(&old_epoch, hotp_id).unwrap();
    old.core.hotp(&old_epoch, hotp_id).unwrap();
    let before = serde_json::to_value(old.core.list(&old_epoch).unwrap()).unwrap();
    let bytes = old
        .core
        .export_bytes(
            &old_epoch,
            &ids,
            Some("synthetic-migration-password"),
            NativeApproval::from_native_success(),
        )
        .unwrap();
    let new = Fixture::new();
    new.core.require_legacy_migration();
    let epoch = new.unlock();
    let preview = new.core.prepare_import(&epoch, bytes).unwrap();
    let passwords = preview
        .keys
        .iter()
        .map(|key| {
            (
                key.id.clone(),
                Zeroizing::new("synthetic-migration-password".to_owned()),
            )
        })
        .collect();
    let preview = new
        .core
        .passwords(
            &epoch,
            Uuid::parse_str(&preview.id).unwrap(),
            passwords,
            |_, _| {},
        )
        .unwrap();
    assert!(preview.rows.iter().all(|row| row.issue.is_none()));
    assert!(
        preview
            .rows
            .iter()
            .all(|row| ids.iter().any(|id| row.source_id == id.to_string()))
    );
    let imported_ids = preview
        .rows
        .iter()
        .map(|row| Uuid::parse_str(&row.account.as_ref().unwrap().id).unwrap())
        .collect();
    new.core
        .commit_import(
            &epoch,
            Uuid::parse_str(&preview.id).unwrap(),
            imported_ids,
            false,
        )
        .unwrap();
    assert_eq!(new.core.status().unwrap().legacy_migration_required, None);
    let metadata = |mut value: serde_json::Value| {
        for account in value.as_array_mut().unwrap() {
            account.as_object_mut().unwrap().remove("id");
        }
        value
    };
    assert_eq!(
        metadata(serde_json::to_value(new.core.list(&epoch).unwrap()).unwrap()),
        metadata(before.clone())
    );
    assert_eq!(
        serde_json::to_value(old.core.list(&old_epoch).unwrap()).unwrap(),
        before
    );
    let expected = old.core.hotp(&old_epoch, hotp_id).unwrap().code;
    let imported_hotp = new
        .core
        .list(&epoch)
        .unwrap()
        .into_iter()
        .find(|account| account.kind == crate::domain::account::OtpKind::Hotp)
        .unwrap();
    assert_eq!(
        new.core
            .hotp(&epoch, Uuid::parse_str(&imported_hotp.id).unwrap())
            .unwrap()
            .code,
        expected
    );
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

struct DeferredAuth {
    entered: std::sync::mpsc::Sender<()>,
    result: Mutex<Option<std::sync::mpsc::Receiver<AppResult<NativeApproval>>>>,
    attempts: AtomicUsize,
}
impl Authentication for DeferredAuth {
    fn authenticate<'a>(
        &'a self,
        _: &'a str,
    ) -> Pin<Box<dyn Future<Output = AppResult<NativeApproval>> + Send + 'a>> {
        self.attempts.fetch_add(1, Ordering::SeqCst);
        let result = self.result.lock().unwrap().take().unwrap();
        let _ = self.entered.send(());
        Box::pin(async move { result.recv().unwrap() })
    }
    fn cancel(&self) {}
}
#[test]
fn shared_unlock_is_single_flight_and_lock_rejects_late_native_approval() {
    let (entered, started) = std::sync::mpsc::channel();
    let (finish, result) = std::sync::mpsc::channel();
    let auth = Arc::new(DeferredAuth {
        entered,
        result: Mutex::new(Some(result)),
        attempts: AtomicUsize::new(0),
    });
    let f = Fixture::with_auth(auth.clone());
    let core = f.core.clone();
    let worker = std::thread::spawn(move || tauri::async_runtime::block_on(core.unlock()));
    started.recv_timeout(Duration::from_secs(3)).unwrap();
    assert!(matches!(
        tauri::async_runtime::block_on(f.core.unlock()),
        Err(AppError::Conflict)
    ));
    assert_eq!(auth.attempts.load(Ordering::SeqCst), 1);
    assert_eq!(f.data.lock().unwrap().reads.load(Ordering::SeqCst), 0);
    f.core.lock();
    finish
        .send(Ok(NativeApproval::from_native_success()))
        .unwrap();
    assert!(matches!(worker.join().unwrap(), Err(AppError::Locked)));
    assert_eq!(
        f.core.status().unwrap().phase,
        super::session::Phase::Locked
    );
    assert_eq!(f.data.lock().unwrap().reads.load(Ordering::SeqCst), 0);
}
#[test]
fn rejected_shared_unlock_does_not_retry_on_status_or_tick() {
    let (entered, started) = std::sync::mpsc::channel();
    let (finish, result) = std::sync::mpsc::channel();
    let auth = Arc::new(DeferredAuth {
        entered,
        result: Mutex::new(Some(result)),
        attempts: AtomicUsize::new(0),
    });
    let f = Fixture::with_auth(auth.clone());
    let core = f.core.clone();
    let worker = std::thread::spawn(move || tauri::async_runtime::block_on(core.unlock()));
    started.recv_timeout(Duration::from_secs(3)).unwrap();
    finish.send(Err(AppError::Cancelled)).unwrap();
    assert!(matches!(worker.join().unwrap(), Err(AppError::Cancelled)));
    for _ in 0..10 {
        f.core.tick();
        assert_eq!(
            f.core.status().unwrap().phase,
            super::session::Phase::Locked
        );
    }
    assert_eq!(auth.attempts.load(Ordering::SeqCst), 1);
    assert_eq!(f.data.lock().unwrap().reads.load(Ordering::SeqCst), 0);
}

#[test]
fn aggregate_visibility_handoff_preserves_session_until_all_surfaces_hide() {
    let f = Fixture::new();
    let epoch = f.unlock();
    let mut presentation = super::presentation::Presentation::default();
    let super::presentation::Activation::Present(intent) =
        presentation.activate(super::session::Phase::Unlocked)
    else {
        panic!()
    };
    presentation.shown(intent);
    presentation.management_visible = true;
    presentation.hidden();
    f.core.visibility(presentation.visible());
    f.clock.0.store(31, Ordering::SeqCst);
    assert!(!f.core.tick());
    assert!(f.core.list(&epoch).is_ok());
    presentation.management_visible = false;
    f.core.visibility(presentation.visible());
    f.clock.0.store(50, Ordering::SeqCst);
    f.core.visibility(presentation.visible());
    f.clock.0.store(60, Ordering::SeqCst);
    assert!(f.core.list(&epoch).is_ok());
    f.clock.0.store(61, Ordering::SeqCst);
    assert!(f.core.list(&epoch).is_err());
}

#[test]
fn native_snapshot_reads_each_record_once_and_never_consumes_hotp() {
    let f = Fixture::new();
    let epoch = f.unlock();
    let record = crate::compatibility::uri::parse_uri(
        "otpauth://hotp/Synthetic?secret=GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ&counter=0",
    )
    .unwrap();
    let id = record.id;
    f.core
        .authorized(&epoch, |inner| {
            inner.vault.commit(vec![(record, false)], &[])
        })
        .unwrap();
    let before = f.data.lock().unwrap().reads.load(Ordering::SeqCst);
    let snapshot = f.core.presentation_snapshot(&epoch).unwrap();
    assert_eq!(
        f.data.lock().unwrap().reads.load(Ordering::SeqCst) - before,
        1
    );
    assert_eq!(snapshot.accounts.len(), 1);
    assert!(snapshot.codes.is_empty());
    assert_eq!(f.core.hotp(&epoch, id).unwrap().code, "755224");
    for _ in 0..3 {
        let snapshot = f.core.presentation_snapshot(&epoch).unwrap();
        assert_eq!(snapshot.accounts[0].next_counter.as_deref(), Some("1"));
        assert_eq!(snapshot.codes[0].code, "755224");
    }
    f.core.lock();
    let before = f.data.lock().unwrap().reads.load(Ordering::SeqCst);
    assert!(f.core.presentation_snapshot(&epoch).is_err());
    assert_eq!(f.data.lock().unwrap().reads.load(Ordering::SeqCst), before);
}

#[test]
fn copy_recalculates_totp_when_the_visible_code_has_expired() {
    struct BoundaryClock(AtomicU64);
    impl Clock for BoundaryClock {
        fn elapsed(&self) -> Duration {
            Duration::ZERO
        }
        fn unix_seconds(&self) -> u64 {
            self.0.load(Ordering::SeqCst)
        }
    }
    #[derive(Default)]
    struct RecordingClipboard(Mutex<String>);
    impl Clipboard for RecordingClipboard {
        fn copy(&self, value: &str) -> AppResult<i64> {
            *self.0.lock().unwrap() = value.into();
            Ok(1)
        }
        fn clear_if_owned(&self, _: i64) {}
    }
    let f = Fixture::new();
    let clock = Arc::new(BoundaryClock(AtomicU64::new(59)));
    let clipboard = Arc::new(RecordingClipboard::default());
    let core = Core::new(
        Box::new(Storage(f.data.clone())),
        DiskIndex::new(f.root.join("boundary")).unwrap(),
        Arc::new(Auth),
        clipboard.clone(),
        clock.clone(),
    )
    .unwrap();
    core.visibility(true);
    let ticket = core.begin_unlock().unwrap();
    let epoch = core
        .finish_unlock(ticket, NativeApproval::from_native_success())
        .unwrap()
        .epoch;
    let preview = core
        .prepare_uri(
            &epoch,
            "otpauth://totp/RFC?secret=GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ",
        )
        .unwrap();
    let id = Uuid::parse_str(&preview.rows[0].account.as_ref().unwrap().id).unwrap();
    core.commit_import(
        &epoch,
        Uuid::parse_str(&preview.id).unwrap(),
        vec![id],
        false,
    )
    .unwrap();
    assert_eq!(
        core.presentation_snapshot(&epoch).unwrap().codes[0].code,
        "287082"
    );
    clock.0.store(60, Ordering::SeqCst);
    core.copy(&epoch, id).unwrap();
    assert_eq!(*clipboard.0.lock().unwrap(), "359152");
}
fn qr(value: &str) -> Zeroizing<String> {
    Zeroizing::new(value.to_owned())
}
#[test]
fn qr_review_exposes_only_metadata_and_commits_exactly_one_candidate() {
    let f = Fixture::new();
    let epoch = f.unlock();
    let preview=f.core.prepare_qr(&epoch,vec![qr("otpauth://totp/Sint%C3%A9tico:alice?secret=JBSWY3DPEHPK3PXP&issuer=Sint%C3%A9tico"),qr("otpauth://hotp/RFC?secret=GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ&counter=7")]).unwrap();
    assert_eq!(preview.origin, "qr");
    assert_eq!(preview.rows.len(), 2);
    assert!(f.core.list(&epoch).unwrap().is_empty());
    let json = serde_json::to_string(&preview).unwrap();
    for secret in [
        "secret",
        "otpauth://",
        "JBSWY3DPEHPK3PXP",
        "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ",
    ] {
        assert!(!json.contains(secret));
    }
    let ids: Vec<_> = preview
        .rows
        .iter()
        .map(|row| Uuid::parse_str(&row.account.as_ref().unwrap().id).unwrap())
        .collect();
    let preview_id = Uuid::parse_str(&preview.id).unwrap();
    assert!(
        f.core
            .commit_import(&epoch, preview_id, ids.clone(), true)
            .is_err()
    );
    f.core
        .commit_import(&epoch, preview_id, vec![ids[1]], true)
        .unwrap();
    let accounts = f.core.list(&epoch).unwrap();
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].next_counter.as_deref(), Some("7"));
    let duplicate = f
        .core
        .prepare_qr(
            &epoch,
            vec![qr(
                "otpauth://hotp/RFC?secret=GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ&counter=0",
            )],
        )
        .unwrap();
    assert_eq!(duplicate.rows[0].issue.as_deref(), Some("DuplicateKept"));
    assert_eq!(
        f.core.list(&epoch).unwrap()[0].next_counter.as_deref(),
        Some("7")
    );
}
#[test]
fn qr_invalid_payloads_limits_and_pending_transfers_do_not_change_the_vault() {
    let f = Fixture::new();
    let epoch = f.unlock();
    assert!(matches!(
        f.core.prepare_qr(&epoch, vec![]),
        Err(AppError::NoQrCode)
    ));
    assert!(matches!(
        f.core
            .prepare_qr(&epoch, vec![qr("https://example.invalid")]),
        Err(AppError::UnsupportedFormat)
    ));
    assert!(matches!(
        f.core
            .prepare_qr(&epoch, vec![qr("otpauth://totp/test?secret=INVALID!")]),
        Err(AppError::InvalidQrCode)
    ));
    assert!(matches!(
        f.core.prepare_qr(&epoch, vec![qr(&"x".repeat(8193))]),
        Err(AppError::LimitExceeded)
    ));
    assert!(matches!(
        f.core
            .prepare_qr(&epoch, (0..17).map(|_| qr("invalid")).collect()),
        Err(AppError::LimitExceeded)
    ));
    let uri = "otpauth://totp/test?secret=JBSWY3DPEHPK3PXP";
    let pending = f.core.prepare_uri(&epoch, uri).unwrap();
    assert!(matches!(
        f.core.prepare_qr(&epoch, vec![qr(uri)]),
        Err(AppError::Conflict)
    ));
    assert_eq!(
        f.core
            .authorized(&epoch, |inner| Ok(inner
                .preview
                .as_ref()
                .unwrap()
                .id
                .to_string()))
            .unwrap(),
        pending.id
    );
    f.core.cancel_import(&epoch).unwrap();
    let mixed = f
        .core
        .prepare_qr(
            &epoch,
            vec![
                qr(uri),
                qr("https://example.invalid"),
                qr("otpauth://totp/test?secret=INVALID!"),
            ],
        )
        .unwrap();
    assert_eq!(mixed.rows[1].issue.as_deref(), Some("UnsupportedFormat"));
    assert_eq!(mixed.rows[2].issue.as_deref(), Some("InvalidParameters"));
    assert!(f.core.list(&epoch).unwrap().is_empty());
}
#[test]
fn qr_revocation_and_storage_failure_never_report_a_saved_account() {
    let f = Fixture::new();
    let epoch = f.unlock();
    let uri = "otpauth://totp/test?secret=JBSWY3DPEHPK3PXP";
    let p = f.core.prepare_qr(&epoch, vec![qr(uri)]).unwrap();
    let id = Uuid::parse_str(&p.rows[0].account.as_ref().unwrap().id).unwrap();
    f.core.lock();
    assert!(
        f.core
            .commit_import(&epoch, Uuid::parse_str(&p.id).unwrap(), vec![id], true)
            .is_err()
    );
    assert!(matches!(
        f.core.prepare_qr(&epoch, vec![qr(uri)]),
        Err(AppError::Locked)
    ));
    let epoch = f.unlock();
    assert!(f.core.list(&epoch).unwrap().is_empty());
    let p = f.core.prepare_qr(&epoch, vec![qr(uri)]).unwrap();
    let id = Uuid::parse_str(&p.rows[0].account.as_ref().unwrap().id).unwrap();
    f.data.lock().unwrap().fail_insert = true;
    assert!(
        f.core
            .commit_import(&epoch, Uuid::parse_str(&p.id).unwrap(), vec![id], true)
            .is_err()
    );
    assert!(f.core.list(&epoch).unwrap().is_empty());
}
