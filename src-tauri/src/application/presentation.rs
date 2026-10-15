//! Intent and visibility bookkeeping. It cannot authorize vault access.
use super::session::Phase;

#[derive(Debug, PartialEq, Eq)]
pub enum Activation {
    Ignore,
    Hide,
    Authenticate(u64),
    Present(u64),
}
#[derive(Default)]
pub struct Presentation {
    serial: u64,
    pending: Option<u64>,
    pub quick_visible: bool,
    pub management_visible: bool,
    pub scanning: bool,
}
impl Presentation {
    pub fn activate(&mut self, phase: Phase) -> Activation {
        if self.pending.is_some() || self.scanning || phase == Phase::Unlocking {
            return Activation::Ignore;
        }
        if self.quick_visible {
            return Activation::Hide;
        }
        self.serial += 1;
        if phase == Phase::Unlocked {
            Activation::Present(self.serial)
        } else {
            self.pending = Some(self.serial);
            Activation::Authenticate(self.serial)
        }
    }
    pub fn complete(&mut self, ticket: u64, approved: bool) -> Option<u64> {
        if self.pending != Some(ticket) || !self.current(ticket) {
            return None;
        }
        self.pending = None;
        approved.then_some(ticket)
    }
    pub fn current(&self, ticket: u64) -> bool {
        self.serial == ticket
    }
    pub fn ticket(&self) -> u64 {
        self.serial
    }
    pub fn shown(&mut self, ticket: u64) -> bool {
        if !self.current(ticket) {
            return false;
        }
        self.quick_visible = true;
        true
    }
    pub fn hidden(&mut self) {
        // Repeated notifications don't invalidate newly started work.
        if self.quick_visible {
            self.quick_visible = false;
            self.serial += 1;
        }
    }
    pub fn revoke(&mut self) {
        self.serial += 1;
        self.pending = None;
        self.quick_visible = false;
        self.scanning = false;
    }
    pub fn visible(&self) -> bool {
        self.quick_visible || self.management_visible || self.scanning
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repeated_clicks_never_start_multiple_prompts() {
        let mut state = Presentation::default();
        let Activation::Authenticate(ticket) = state.activate(Phase::Locked) else {
            panic!()
        };
        for _ in 0..10 {
            assert_eq!(state.activate(Phase::Unlocking), Activation::Ignore);
        }
        assert_eq!(state.complete(ticket, true), Some(ticket));
        assert!(state.shown(ticket));
        assert_eq!(state.activate(Phase::Unlocked), Activation::Hide);
    }
    #[test]
    fn cancelled_attempt_ends_without_automatic_retry() {
        let mut state = Presentation::default();
        let Activation::Authenticate(ticket) = state.activate(Phase::Locked) else {
            panic!()
        };
        assert_eq!(state.complete(ticket, false), None);
        assert_eq!(state.complete(ticket, true), None);
        assert!(!state.quick_visible);
        // Only another explicit activate call can produce Authenticate.
        assert!(
            matches!(state.activate(Phase::Locked),Activation::Authenticate(next) if next>ticket)
        );
    }
    #[test]
    fn revocation_rejects_old_approval_and_results() {
        let mut state = Presentation::default();
        let Activation::Authenticate(ticket) = state.activate(Phase::Locked) else {
            panic!()
        };
        state.revoke();
        assert_eq!(state.complete(ticket, true), None);
        assert!(!state.shown(ticket));
    }
    #[test]
    fn management_handoff_keeps_aggregate_visibility() {
        let mut state = Presentation::default();
        let Activation::Present(ticket) = state.activate(Phase::Unlocked) else {
            panic!()
        };
        state.shown(ticket);
        state.management_visible = true;
        state.hidden();
        assert!(state.visible());
        let serial = state.ticket();
        state.hidden();
        assert_eq!(state.ticket(), serial);
        state.management_visible = false;
        assert!(!state.visible());
    }
    #[test]
    fn hiding_rejects_pending_render_without_consuming_new_activation() {
        let mut state = Presentation::default();
        let Activation::Present(old) = state.activate(Phase::Unlocked) else {
            panic!()
        };
        state.shown(old);
        state.hidden();
        assert!(!state.current(old));
        let Activation::Present(new) = state.activate(Phase::Unlocked) else {
            panic!()
        };
        state.hidden();
        assert!(state.shown(new));
    }
}
