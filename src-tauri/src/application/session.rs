use crate::domain::{AppError, AppResult};
use serde::Serialize;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, ts_rs::TS)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Locked,
    Unlocking,
    Unlocked,
}

pub struct Session {
    pub epoch: u64,
    pub phase: Phase,
    started: Duration,
    interaction: Duration,
    hidden: Option<Duration>,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            epoch: 1,
            phase: Phase::Locked,
            started: Duration::ZERO,
            interaction: Duration::ZERO,
            hidden: None,
        }
    }
}

impl Session {
    pub fn lock(&mut self) {
        self.epoch += 1;
        self.phase = Phase::Locked;
        self.hidden = None;
    }
    pub fn begin(&mut self) -> AppResult<u64> {
        if self.phase == Phase::Unlocking {
            return Err(AppError::Conflict);
        }
        self.lock();
        self.phase = Phase::Unlocking;
        Ok(self.epoch)
    }
    // Only the native coordinator calls this after consuming NativeApproval.
    pub(super) fn approve(&mut self, ticket: u64, now: Duration) -> AppResult<()> {
        if self.epoch != ticket || self.phase != Phase::Unlocking {
            return Err(AppError::Locked);
        }
        self.phase = Phase::Unlocked;
        self.started = now;
        self.interaction = now;
        self.hidden = None;
        Ok(())
    }
    pub fn check(&self, epoch: u64, now: Duration) -> AppResult<()> {
        if self.epoch != epoch || self.phase != Phase::Unlocked || self.expired(now) {
            Err(AppError::Locked)
        } else {
            Ok(())
        }
    }
    pub fn expired(&self, now: Duration) -> bool {
        self.phase == Phase::Unlocked
            && (now.saturating_sub(self.started) >= Duration::from_secs(300)
                || now.saturating_sub(self.interaction) >= Duration::from_secs(120)
                || self
                    .hidden
                    .is_some_and(|hidden| now.saturating_sub(hidden) >= Duration::from_secs(15)))
    }
    pub fn interact(&mut self, epoch: u64, now: Duration) -> AppResult<()> {
        self.check(epoch, now)?;
        self.interaction = now;
        Ok(())
    }
    pub fn visibility(&mut self, visible: bool, now: Duration) {
        self.hidden = if visible { None } else { Some(now) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn s(n: u64) -> Duration {
        Duration::from_secs(n)
    }
    #[test]
    fn late_approval_cannot_unlock() {
        let mut v = Session::default();
        let ticket = v.begin().unwrap();
        v.lock();
        assert!(v.approve(ticket, s(0)).is_err());
        assert_eq!(v.phase, Phase::Locked);
    }
    #[test]
    fn polling_and_interaction_cannot_extend_absolute_deadline() {
        let mut v = Session::default();
        let e = v.begin().unwrap();
        v.approve(e, s(0)).unwrap();
        for n in 0..120 {
            v.check(e, s(n)).unwrap();
        }
        assert!(v.check(e, s(120)).is_err());
        v.interact(e, s(119)).unwrap();
        v.interact(e, s(238)).unwrap();
        assert!(v.check(e, s(300)).is_err());
    }
    #[test]
    fn hidden_deadline_and_old_generation() {
        let mut v = Session::default();
        let e = v.begin().unwrap();
        v.approve(e, s(0)).unwrap();
        v.visibility(false, s(3));
        assert!(v.check(e, s(18)).is_err());
        v.visibility(true, s(17));
        v.check(e, s(18)).unwrap();
        v.lock();
        let e2 = v.begin().unwrap();
        v.approve(e2, s(20)).unwrap();
        assert!(v.check(e, s(20)).is_err());
    }
}
