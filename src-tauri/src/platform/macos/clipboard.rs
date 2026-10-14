use crate::domain::{AppError, AppResult};
use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
use objc2_foundation::NSString;

pub fn copy(code: &str) -> AppResult<i64> {
    if !(6..=8).contains(&code.len()) || !code.bytes().all(|c| c.is_ascii_digit()) {
        return Err(AppError::invalid("code"));
    }
    let board = NSPasteboard::generalPasteboard();
    board.clearContents();
    // SAFETY: Public AppKit constant; no caller-controlled pasteboard types.
    if !board.setString_forType(&NSString::from_str(code), unsafe { NSPasteboardTypeString }) {
        return Err(AppError::StorageUnavailable);
    }
    Ok(board.changeCount() as i64)
}
pub fn clear_if_owned(change_count: i64) {
    let board = NSPasteboard::generalPasteboard();
    if owns_write(board.changeCount() as i64, change_count) {
        board.clearContents();
    }
}
pub fn owns_write(current: i64, own: i64) -> bool {
    current == own
}
pub struct NativeClipboard;
impl crate::application::ports::Clipboard for NativeClipboard {
    fn copy(&self, code: &str) -> AppResult<i64> {
        copy(code)
    }
    fn clear_if_owned(&self, id: i64) {
        clear_if_owned(id);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn identical_text_with_new_write_is_not_owned() {
        assert!(!super::owns_write(12, 11));
        assert!(super::owns_write(11, 11));
    }
}
