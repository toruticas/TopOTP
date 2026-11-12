use crate::domain::{AppError, AppResult};
use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
use objc2_foundation::NSString;

pub fn copy(code: &str) -> AppResult<i64> {
    if !(6..=8).contains(&code.len()) || !code.bytes().all(|c| c.is_ascii_digit()) {
        return Err(AppError::invalid("code"));
    }
    let board = NSPasteboard::generalPasteboard();
    write_owned(&board, || {
        // SAFETY: Public AppKit constant; no caller-controlled pasteboard types.
        board.setString_forType(&NSString::from_str(code), unsafe { NSPasteboardTypeString })
    })
}
fn write_owned(board: &NSPasteboard, write: impl FnOnce() -> bool) -> AppResult<i64> {
    // Capture our ownership, not whichever application's write is current after
    // setString returns. Another process may replace the contents immediately.
    let change_count = board.clearContents() as i64;
    if !write() {
        return Err(AppError::StorageUnavailable);
    }
    Ok(change_count)
}
pub fn clear_if_owned(change_count: i64) {
    clear_owned(&NSPasteboard::generalPasteboard(), change_count);
}
fn clear_owned(board: &NSPasteboard, change_count: i64) {
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
    use super::*;
    use objc2::{
        msg_send,
        rc::{Retained, autoreleasepool},
    };

    struct IsolatedPasteboard(Retained<NSPasteboard>);
    impl Drop for IsolatedPasteboard {
        fn drop(&mut self) {
            // SAFETY: Public NSPasteboard cleanup for this unique test board.
            unsafe { msg_send![&*self.0, releaseGlobally] }
        }
    }

    #[test]
    fn replacement_between_writing_and_returning_is_preserved() {
        autoreleasepool(|_| {
            let isolated = IsolatedPasteboard(NSPasteboard::pasteboardWithUniqueName());
            let board = &isolated.0;
            let other = NSPasteboard::pasteboardWithName(&board.name());
            // SAFETY: Public AppKit constant; only synthetic test strings.
            let kind = unsafe { NSPasteboardTypeString };
            for replacement in ["synthetic-other-app", "123456"] {
                let own = write_owned(board, || {
                    assert!(board.setString_forType(&NSString::from_str("123456"), kind));
                    other.clearContents();
                    assert!(other.setString_forType(&NSString::from_str(replacement), kind));
                    true
                })
                .unwrap();
                assert_ne!(own, board.changeCount() as i64);
                clear_owned(board, own);
                assert_eq!(other.stringForType(kind).unwrap().to_string(), replacement);
            }
            let own = write_owned(board, || {
                board.setString_forType(&NSString::from_str("123456"), kind)
            })
            .unwrap();
            clear_owned(board, own);
            assert!(board.stringForType(kind).is_none());
        });
    }

    #[test]
    fn identical_text_with_new_write_is_not_owned() {
        assert!(!super::owns_write(12, 11));
        assert!(super::owns_write(11, 11));
    }
}
