//! QR enrollment reuses the validated URI parser and the existing commit path.
use super::{
    core::Core,
    transfer::{ImportPreview, PendingImport},
};
use crate::{
    compatibility::{
        backup::{Decoded, Entry},
        uri,
    },
    domain::{AppError, AppResult},
};
use std::collections::HashSet;
use uuid::Uuid;
use zeroize::Zeroizing;

pub const MAX_QR_CODES: usize = 16;
pub const MAX_QR_PAYLOAD: usize = 8192;
impl Core {
    pub fn prepare_qr(
        &self,
        epoch: &str,
        payloads: Vec<Zeroizing<String>>,
    ) -> AppResult<ImportPreview> {
        self.authorized(epoch, |inner| {
            if inner.preview.is_some() {
                Err(AppError::Conflict)
            } else {
                Ok(())
            }
        })?;
        if payloads.is_empty() {
            return Err(AppError::NoQrCode);
        }
        if payloads.len() > MAX_QR_CODES || payloads.iter().any(|p| p.len() > MAX_QR_PAYLOAD) {
            return Err(AppError::LimitExceeded);
        }
        let mut entries = Vec::new();
        for (index, payload) in payloads.iter().enumerate() {
            let (record, issue) = match uri::parse_uri(payload) {
                Ok(record) => (Some(record), None),
                Err(AppError::UnsupportedFormat) => (None, Some("UnsupportedFormat")),
                Err(_) => (None, Some("InvalidParameters")),
            };
            entries.push(Entry {
                source_id: record.as_ref().map_or_else(
                    || Uuid::new_v4().to_string(),
                    |record| record.id.to_string(),
                ),
                index,
                pinned: false,
                record,
                issue,
            });
        }
        if entries.iter().all(|e| e.record.is_none()) {
            return Err(
                if entries.iter().all(|e| e.issue == Some("UnsupportedFormat")) {
                    AppError::UnsupportedFormat
                } else {
                    AppError::InvalidQrCode
                },
            );
        }
        self.authorized(epoch, |inner| {
            if inner.preview.is_some() {
                return Err(AppError::Conflict);
            }
            let mut pending = PendingImport {
                origin: "qr",
                id: Uuid::new_v4(),
                bytes: Zeroizing::new(vec![]),
                decoded: Decoded {
                    keys: vec![],
                    entries,
                },
                revision: 0,
                duplicates: HashSet::new(),
            };
            super::transfer::detect_duplicates(inner, &mut pending)?;
            let result = pending.preview();
            inner.preview = Some(pending);
            Ok(result)
        })
    }
}
