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
    fail_update: bool,
}
struct Storage(Arc<Mutex<Data>>);
impl Store for Storage {
    fn read(&self, id: Uuid) -> AppResult<Option<Zeroizing<Vec<u8>>>> {
        let s = self.0.lock().unwrap();
        s.reads.fetch_add(1, Ordering::SeqCst);
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
