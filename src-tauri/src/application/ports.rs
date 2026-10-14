use crate::domain::AppResult;
use std::{future::Future, pin::Pin, time::Duration};
pub trait Clock: Send + Sync {
    fn unix_seconds(&self) -> u64;
    fn elapsed(&self) -> Duration;
}
// No serde implementation: callers cannot manufacture this receipt via IPC.
pub struct NativeApproval {
    _native: (),
}
impl NativeApproval {
    pub(crate) fn from_native_success() -> Self {
        Self { _native: () }
    }
}
pub trait Authentication: Send + Sync {
    fn authenticate<'a>(
        &'a self,
        reason: &'a str,
    ) -> Pin<Box<dyn Future<Output = AppResult<NativeApproval>> + Send + 'a>>;
    fn cancel(&self);
}
pub trait Clipboard: Send + Sync {
    fn copy(&self, code: &str) -> AppResult<i64>;
    fn clear_if_owned(&self, write_id: i64);
}
