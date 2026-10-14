use super::{
    ports::{Authentication, Clipboard, Clock, NativeApproval},
    session::{Phase, Session},
    vault::{DiskIndex, Store, Vault},
};
use crate::domain::{
    AppError, AppResult,
    account::{AccountInput, AccountRecord, AccountSummary, OtpKind},
    otp,
};
use serde::Serialize;
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;
use zeroize::Zeroizing;

#[derive(Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct VaultStatus {
    pub phase: Phase,
    pub epoch: String,
}
#[derive(Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct Code {
    pub id: String,
    pub code: String,
    pub valid_until: Option<f64>,
    pub server_time: f64,
}
pub struct Inner {
    pub session: Session,
    pub vault: Vault<Box<dyn Store>, DiskIndex>,
    pub hotp: HashMap<Uuid, Zeroizing<String>>,
    pub clipboard: Option<(i64, Duration)>,
    pub preview: Option<super::transfer::PendingImport>,
    visible: bool,
}
pub struct Core {
    inner: Mutex<Inner>,
    pub auth: Arc<dyn Authentication>,
    clipboard: Arc<dyn Clipboard>,
    clock: Arc<dyn Clock>,
    dialogs: AtomicUsize,
    revocation: AtomicU64,
}
impl Core {
    pub fn new(
        store: Box<dyn Store>,
        index: DiskIndex,
        auth: Arc<dyn Authentication>,
        clipboard: Arc<dyn Clipboard>,
        clock: Arc<dyn Clock>,
    ) -> AppResult<Self> {
        Ok(Self {
            inner: Mutex::new(Inner {
                session: Session::default(),
                vault: Vault::open(store, index)?,
                hotp: HashMap::new(),
                clipboard: None,
                preview: None,
                visible: false,
            }),
            auth,
            clipboard,
            clock,
            dialogs: AtomicUsize::new(0),
            revocation: AtomicU64::new(0),
        })
    }
    pub fn now(&self) -> Duration {
        self.clock.elapsed()
    }
    fn guard(&self) -> AppResult<MutexGuard<'_, Inner>> {
        self.inner.lock().map_err(|_| AppError::StorageUnavailable)
    }
    fn revoke(&self, inner: &mut Inner) {
        self.revocation.fetch_add(1, Ordering::SeqCst);
        inner.session.lock();
        inner.hotp.clear();
        inner.preview = None;
        if let Some((id, _)) = inner.clipboard.take() {
            self.clipboard.clear_if_owned(id);
        }
        self.auth.cancel();
    }
    pub fn lock(&self) {
        // Announce revocation before waiting for an in-progress commit. A
        // calculation/read may finish, but cannot disclose an old result.
        self.revocation.fetch_add(1, Ordering::SeqCst);
        if let Ok(mut inner) = self.guard() {
            self.revoke(&mut inner);
        }
    }
    pub fn tick(&self) -> bool {
        if let Ok(mut inner) = self.guard() {
            if inner.session.expired(self.now()) {
                self.revoke(&mut inner);
                return true;
            }
            if inner
                .clipboard
                .is_some_and(|(_, at)| self.now().saturating_sub(at) >= Duration::from_secs(30))
                && let Some((id, _)) = inner.clipboard.take()
            {
                self.clipboard.clear_if_owned(id);
            }
        }
        false
    }
    pub fn status(&self) -> AppResult<VaultStatus> {
        self.tick();
        let inner = self.guard()?;
        Ok(VaultStatus {
            phase: inner.session.phase,
            epoch: inner.session.epoch.to_string(),
        })
    }
    pub fn begin_unlock(&self) -> AppResult<u64> {
        let mut inner = self.guard()?;
        if inner.session.phase == Phase::Unlocking {
            return Err(AppError::Conflict);
        }
        self.revoke(&mut inner);
        inner.session.begin()
    }
    pub fn finish_unlock(&self, ticket: u64, _proof: NativeApproval) -> AppResult<VaultStatus> {
        let mut inner = self.guard()?;
        inner.session.approve(ticket, self.now())?;
        let visible = inner.visible;
        inner.session.visibility(visible, self.now());
        if let Err(error) = inner.vault.recover() {
            self.revoke(&mut inner);
            return Err(error);
        }
        Ok(VaultStatus {
            phase: inner.session.phase,
            epoch: inner.session.epoch.to_string(),
        })
    }
    pub fn fail_unlock(&self, ticket: u64) {
        if let Ok(mut inner) = self.guard()
            && inner.session.epoch == ticket
        {
            self.revoke(&mut inner);
        }
    }
    pub fn authorized<T>(
        &self,
        epoch: &str,
        work: impl FnOnce(&mut Inner) -> AppResult<T>,
    ) -> AppResult<T> {
        let revocation = self.revocation.load(Ordering::SeqCst);
        let epoch = epoch.parse().map_err(|_| AppError::Locked)?;
        let mut inner = self.guard()?;
        if inner.session.expired(self.now()) {
            self.revoke(&mut inner);
        }
        inner.session.check(epoch, self.now())?;
        if revocation != self.revocation.load(Ordering::SeqCst) {
            return Err(AppError::Locked);
        }
        let result = work(&mut inner)?;
        // Commits are serialized with lock. Duration expiration is checked again
        // before disclosure; a committed HOTP increment is never rolled back.
        if inner.session.expired(self.now()) || revocation != self.revocation.load(Ordering::SeqCst)
        {
            self.revoke(&mut inner);
            return Err(AppError::Locked);
        }
        inner.session.check(epoch, self.now())?;
        Ok(result)
    }
    pub fn interact(&self, epoch: &str) -> AppResult<()> {
        self.authorized(epoch, |i| i.session.interact(i.session.epoch, self.now()))
    }
    pub fn visibility(&self, visible: bool) {
        if let Ok(mut i) = self.guard() {
            if i.session.expired(self.now()) {
                self.revoke(&mut i);
            }
            i.visible = visible;
            i.session.visibility(visible, self.now());
        }
    }
    pub fn list(&self, epoch: &str) -> AppResult<Vec<AccountSummary>> {
        self.authorized(epoch, |i| i.vault.summaries())
    }
    pub fn save(&self, epoch: &str, id: Option<Uuid>, input: AccountInput) -> AppResult<()> {
        self.authorized(epoch, |i| {
            let previous = id.map(|id| i.vault.record(id)).transpose()?;
            let record = AccountRecord::from_input(&input, previous.as_ref())?;
            let pinned =
                id.is_some_and(|id| i.vault.references().iter().any(|r| r.id == id && r.pinned));
            i.vault.commit(vec![(record, pinned)], &[])?;
            if let Some(id) = id {
                i.hotp.remove(&id);
            }
            Ok(())
        })
    }
    pub fn delete(&self, epoch: &str, id: Uuid) -> AppResult<()> {
        self.authorized(epoch, |i| {
            i.vault.commit(vec![], &[id])?;
            i.hotp.remove(&id);
            Ok(())
        })
    }
    pub fn totp(&self, epoch: &str) -> AppResult<Vec<Code>> {
        self.authorized(epoch, |i| {
            let now = self.clock.unix_seconds();
            let mut codes = Vec::new();
            for reference in i.vault.references() {
                let record = i.vault.record(reference.id)?;
                if record.kind == OtpKind::Totp {
                    let (code, until) = otp::totp(&record, now)?;
                    codes.push(Code {
                        id: record.id.to_string(),
                        code,
                        valid_until: Some(until as f64),
                        server_time: now as f64,
                    });
                } else if let Some(code) = i.hotp.get(&record.id) {
                    codes.push(Code {
                        id: record.id.to_string(),
                        code: code.to_string(),
                        valid_until: None,
                        server_time: now as f64,
                    });
                }
            }
            Ok(codes)
        })
    }
    pub fn hotp(&self, epoch: &str, id: Uuid) -> AppResult<Code> {
        self.authorized(epoch, |i| {
            let code = i.vault.consume_hotp(id)?;
            i.hotp.insert(id, Zeroizing::new(code.clone()));
            Ok(Code {
                id: id.to_string(),
                code,
                valid_until: None,
                server_time: self.clock.unix_seconds() as f64,
            })
        })
    }
    pub fn copy(&self, epoch: &str, id: Uuid) -> AppResult<()> {
        self.authorized(epoch, |i| {
            let record = i.vault.record(id)?;
            let code = match record.kind {
                OtpKind::Totp => Zeroizing::new(otp::totp(&record, self.clock.unix_seconds())?.0),
                OtpKind::Hotp => Zeroizing::new(
                    i.hotp
                        .get(&id)
                        .ok_or_else(|| AppError::invalid("generateFirst"))?
                        .to_string(),
                ),
            };
            let identity = self.clipboard.copy(&code)?;
            i.clipboard = Some((identity, self.now()));
            Ok(())
        })
    }
    pub fn dialog(&self) -> DialogGuard<'_> {
        self.dialogs.fetch_add(1, Ordering::SeqCst);
        DialogGuard(self)
    }
    pub fn has_dialog(&self) -> bool {
        self.dialogs.load(Ordering::SeqCst) > 0
    }
    #[cfg(test)]
    pub fn interruption_generation(&self) -> u64 {
        self.revocation.load(Ordering::SeqCst)
    }
}
pub struct DialogGuard<'a>(&'a Core);
impl Drop for DialogGuard<'_> {
    fn drop(&mut self) {
        self.0.dialogs.fetch_sub(1, Ordering::SeqCst);
    }
}

pub struct SystemClock(Instant);
impl Default for SystemClock {
    fn default() -> Self {
        Self(Instant::now())
    }
}
impl Clock for SystemClock {
    fn unix_seconds(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs())
    }
    fn elapsed(&self) -> Duration {
        self.0.elapsed()
    }
}
