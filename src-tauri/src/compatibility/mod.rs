//! Boundaries applied before processing untrusted backup payloads.

pub mod backup;
pub mod crypto;
pub mod uri;

pub const MAX_BACKUP_BYTES: u64 = 5 * 1024 * 1024;
pub const MAX_OTP_ENTRIES: usize = 1000;
pub const MAX_KEY_ENTRIES: usize = 32;
pub const MAX_TEXT_BYTES: usize = 1024;
pub const MAX_CIPHERTEXT_BYTES: usize = 16 * 1024;
