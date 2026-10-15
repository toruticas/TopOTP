use super::core::Core;
use crate::{
    compatibility::{
        MAX_BACKUP_BYTES,
        backup::{self, Decoded, KeyRequest},
    },
    domain::{AppError, AppResult, account::AccountSummary},
};
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    fs::{File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::OpenOptionsExt,
    path::Path,
};
use uuid::Uuid;
use zeroize::Zeroizing;

pub struct PendingImport {
    pub origin: &'static str,
    pub id: Uuid,
    pub bytes: Zeroizing<Vec<u8>>,
    pub decoded: Decoded,
    pub revision: u64,
    pub duplicates: HashSet<Uuid>,
}
#[derive(Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct ImportRow {
    pub source_id: String,
    pub account: Option<AccountSummary>,
    pub issue: Option<String>,
}
#[derive(Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub origin: String,
    pub id: String,
    pub rows: Vec<ImportRow>,
    pub keys: Vec<KeyRequest>,
}
#[derive(Clone, Serialize, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct TransferProgress {
    pub epoch: String,
    pub preview_id: String,
    pub completed: u32,
    pub total: u32,
}
impl PendingImport {
    pub fn preview(&self) -> ImportPreview {
        ImportPreview {
            origin: self.origin.to_owned(),
            id: self.id.to_string(),
            keys: self.decoded.keys.clone(),
            rows: self
                .decoded
                .entries
                .iter()
                .map(|e| ImportRow {
                    source_id: e.source_id.clone(),
                    account: e.record.as_ref().map(|r| r.summary(e.pinned)),
                    issue: if e
                        .record
                        .as_ref()
                        .is_some_and(|r| self.duplicates.contains(&r.id))
                    {
                        Some("DuplicateKept".into())
                    } else {
                        e.issue.map(str::to_owned)
                    },
                })
                .collect(),
        }
    }
}
pub fn read_bounded(path: &Path) -> AppResult<Zeroizing<Vec<u8>>> {
    let file = File::open(path).map_err(|_| AppError::StorageUnavailable)?;
    if !file
        .metadata()
        .map_err(|_| AppError::StorageUnavailable)?
        .is_file()
    {
        return Err(AppError::UnsupportedFormat);
    }
    let mut bytes = Zeroizing::new(Vec::new());
    file.take(MAX_BACKUP_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| AppError::StorageUnavailable)?;
    if bytes.len() as u64 > MAX_BACKUP_BYTES {
        return Err(AppError::LimitExceeded);
    }
    Ok(bytes)
}
impl Core {
    pub fn prepare_uri(&self, epoch: &str, uri: &str) -> AppResult<ImportPreview> {
        let record = crate::compatibility::uri::parse_uri(uri)?;
        self.authorized(epoch, |inner| {
            let mut pending = PendingImport {
                origin: "uri",
                id: Uuid::new_v4(),
                bytes: Zeroizing::new(vec![]),
                decoded: Decoded {
                    keys: vec![],
                    entries: vec![backup::Entry {
                        source_id: record.id.to_string(),
                        index: 0,
                        pinned: false,
                        record: Some(record),
                        issue: None,
                    }],
                },
                revision: 0,
                duplicates: HashSet::new(),
            };
            detect_duplicates(inner, &mut pending)?;
            let out = pending.preview();
            inner.preview = Some(pending);
            Ok(out)
        })
    }

    pub fn prepare_import(
        &self,
        epoch: &str,
        bytes: Zeroizing<Vec<u8>>,
    ) -> AppResult<ImportPreview> {
        let decoded = backup::decode(&bytes, &HashMap::new(), || {
            self.authorized(epoch, |_| Ok(()))
        })?;
        self.authorized(epoch, |inner| {
            let mut pending = PendingImport {
                origin: "backup",
                id: Uuid::new_v4(),
                bytes,
                decoded,
                revision: 0,
                duplicates: HashSet::new(),
            };
            detect_duplicates(inner, &mut pending)?;
            let preview = pending.preview();
            inner.preview = Some(pending);
            Ok(preview)
        })
    }
    pub fn passwords(
        &self,
        epoch: &str,
        id: Uuid,
        passwords: HashMap<String, Zeroizing<String>>,
        progress: impl FnMut(usize, usize),
    ) -> AppResult<ImportPreview> {
        if passwords.len() > 32 {
            return Err(AppError::LimitExceeded);
        }
        let (bytes, revision) = self.authorized(epoch, |i| {
            let p = i
                .preview
                .as_mut()
                .filter(|p| p.id == id)
                .ok_or(AppError::NotFound)?;
            p.revision += 1;
            Ok((p.bytes.clone(), p.revision))
        })?;
        let decoded = backup::decode_with_progress(
            &bytes,
            &passwords,
            || {
                self.authorized(epoch, |i| {
                    if i.preview
                        .as_ref()
                        .is_some_and(|p| p.id == id && p.revision == revision)
                    {
                        Ok(())
                    } else {
                        Err(AppError::Cancelled)
                    }
                })
            },
            progress,
        )?;
        self.authorized(epoch, |i| {
            let mut pending = i
                .preview
                .take()
                .filter(|p| p.id == id && p.revision == revision)
                .ok_or(AppError::Cancelled)?;
            pending.decoded = decoded;
            detect_duplicates(i, &mut pending)?;
            let out = pending.preview();
            i.preview = Some(pending);
            Ok(out)
        })
    }
    pub fn commit_import(
        &self,
        epoch: &str,
        id: Uuid,
        selected: Vec<Uuid>,
        acknowledge_skipped: bool,
    ) -> AppResult<()> {
        self.authorized(epoch, |i| {
            let p = i
                .preview
                .as_ref()
                .filter(|p| p.id == id)
                .ok_or(AppError::NotFound)?;
            if p.origin == "qr" && selected.len() != 1 {
                return Err(AppError::invalid("selection"));
            }
            if selected.is_empty()
                || selected.iter().copied().collect::<HashSet<_>>().len() != selected.len()
            {
                return Err(AppError::invalid("selection"));
            }
            for id in &selected {
                if !p
                    .decoded
                    .entries
                    .iter()
                    .any(|e| e.record.as_ref().is_some_and(|r| r.id == *id))
                    || p.duplicates.contains(id)
                {
                    return Err(AppError::invalid("selection"));
                }
            }
            if selected.len() != p.decoded.entries.len() && !acknowledge_skipped {
                return Err(AppError::invalid("acknowledgeSkipped"));
            }
            let p = i.preview.take().unwrap();
            let records = p
                .decoded
                .entries
                .into_iter()
                .filter_map(|e| {
                    e.record
                        .filter(|r| selected.contains(&r.id))
                        .map(|r| (r, e.pinned))
                })
                .collect();
            i.vault.commit(records, &[])
        })
    }
    pub fn cancel_import(&self, epoch: &str) -> AppResult<()> {
        self.authorized(epoch, |i| {
            i.preview = None;
            Ok(())
        })
    }
    pub fn export_bytes(
        &self,
        epoch: &str,
        ids: &[Uuid],
        password: Option<&str>,
        _proof: super::ports::NativeApproval,
    ) -> AppResult<Zeroizing<Vec<u8>>> {
        if ids.is_empty() || ids.iter().copied().collect::<HashSet<_>>().len() != ids.len() {
            return Err(AppError::invalid("selection"));
        }
        let records = self.authorized(epoch, |i| {
            ids.iter()
                .map(|id| {
                    let record = i.vault.record(*id)?;
                    let pinned = i.vault.references().iter().any(|r| r.id == *id && r.pinned);
                    Ok((record, pinned))
                })
                .collect::<AppResult<Vec<_>>>()
        })?;
        let start = std::time::Instant::now();
        backup::encode(&records, password, || {
            if start.elapsed() > std::time::Duration::from_secs(30) {
                return Err(AppError::LimitExceeded);
            }
            self.authorized(epoch, |_| Ok(()))
        })
    }
    pub fn write_export(
        &self,
        epoch: &str,
        path: &Path,
        bytes: &[u8],
        encrypted: bool,
    ) -> AppResult<()> {
        self.authorized(epoch, |_| {
            if encrypted {
                super::vault::atomic_write(path, bytes)
            } else {
                let mut file = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .mode(0o600)
                    .open(path)
                    .map_err(|_| AppError::StorageUnavailable)?;
                let result = file
                    .write_all(bytes)
                    .and_then(|_| file.sync_all())
                    .map_err(|_| AppError::StorageUnavailable);
                if result.is_err() {
                    drop(file);
                    let _ = std::fs::remove_file(path);
                }
                result
            }
        })
    }
}
pub(super) fn detect_duplicates(
    inner: &super::core::Inner,
    pending: &mut PendingImport,
) -> AppResult<()> {
    pending.duplicates.clear();
    let mut known = Vec::new();
    for r in inner.vault.references() {
        known.push(inner.vault.record(r.id)?);
    }
    for e in &pending.decoded.entries {
        if let Some(record) = &e.record
            && (known.iter().any(|r| r.same_identity(record))
                || pending
                    .decoded
                    .entries
                    .iter()
                    .take_while(|other| other.source_id != e.source_id)
                    .any(|other| {
                        other
                            .record
                            .as_ref()
                            .is_some_and(|r| r.same_identity(record))
                    }))
        {
            pending.duplicates.insert(record.id);
        }
    }
    Ok(())
}
