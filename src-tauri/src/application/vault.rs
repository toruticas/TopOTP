use crate::domain::{
    AppError, AppResult,
    account::{AccountRecord, AccountSummary, MAX_COUNTER},
    otp,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs::{self, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};
use uuid::Uuid;
use zeroize::Zeroizing;

pub trait Store: Send {
    fn read(&self, id: Uuid) -> AppResult<Option<Zeroizing<Vec<u8>>>>;
    fn insert(&mut self, id: Uuid, data: &[u8]) -> AppResult<()>;
    fn update(&mut self, id: Uuid, data: &[u8]) -> AppResult<()>;
    fn delete(&mut self, id: Uuid) -> AppResult<()>;
}
impl<T: Store + ?Sized> Store for Box<T> {
    fn read(&self, id: Uuid) -> AppResult<Option<Zeroizing<Vec<u8>>>> {
        (**self).read(id)
    }
    fn insert(&mut self, id: Uuid, data: &[u8]) -> AppResult<()> {
        (**self).insert(id, data)
    }
    fn update(&mut self, id: Uuid, data: &[u8]) -> AppResult<()> {
        (**self).update(id, data)
    }
    fn delete(&mut self, id: Uuid) -> AppResult<()> {
        (**self).delete(id)
    }
}
pub trait Index: Send {
    fn read(&self, name: &str) -> AppResult<Option<Vec<u8>>>;
    fn write(&mut self, name: &str, data: &[u8]) -> AppResult<()>;
    fn remove(&mut self, name: &str) -> AppResult<()>;
}

pub struct DiskIndex(PathBuf);
impl DiskIndex {
    pub fn new(root: PathBuf) -> AppResult<Self> {
        fs::create_dir_all(&root).map_err(|_| AppError::StorageUnavailable)?;
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700))
            .map_err(|_| AppError::StorageUnavailable)?;
        Ok(Self(root))
    }
}
impl Index for DiskIndex {
    fn read(&self, name: &str) -> AppResult<Option<Vec<u8>>> {
        let mut file = match fs::File::open(self.0.join(name)) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(AppError::StorageUnavailable),
        };
        let mut bytes = Vec::new();
        Read::by_ref(&mut file)
            .take(1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| AppError::StorageUnavailable)?;
        if bytes.len() > 1024 * 1024 {
            return Err(AppError::LimitExceeded);
        }
        Ok(Some(bytes))
    }
    fn write(&mut self, name: &str, data: &[u8]) -> AppResult<()> {
        atomic_write(&self.0.join(name), data)
    }
    fn remove(&mut self, name: &str) -> AppResult<()> {
        match fs::remove_file(self.0.join(name)) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err(AppError::StorageUnavailable),
        }
    }
}
pub fn atomic_write(path: &Path, bytes: &[u8]) -> AppResult<()> {
    let parent = path.parent().ok_or(AppError::StorageUnavailable)?;
    let temp = parent.join(format!(".{}.tmp", Uuid::new_v4()));
    let result = (|| {
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temp)
            .map_err(|_| AppError::StorageUnavailable)?;
        f.write_all(bytes)
            .and_then(|_| f.sync_all())
            .map_err(|_| AppError::StorageUnavailable)?;
        fs::rename(&temp, path).map_err(|_| AppError::StorageUnavailable)?;
        fs::File::open(parent)
            .and_then(|f| f.sync_all())
            .map_err(|_| AppError::StorageUnavailable)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Reference {
    pub id: Uuid,
    pub storage_key: Uuid,
    pub pinned: bool,
}
#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Manifest {
    version: u8,
    references: Vec<Reference>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    staged: Vec<Uuid>,
    retired: Vec<Uuid>,
}

pub struct Vault<S: Store, I: Index> {
    pub store: S,
    index: I,
    references: Vec<Reference>,
}
impl<S: Store, I: Index> Vault<S, I> {
    pub fn open(store: S, index: I) -> AppResult<Self> {
        let manifest = match index.read("manifest.json")? {
            Some(bytes) => serde_json::from_slice::<Manifest>(&bytes)
                .map_err(|_| AppError::StorageUnavailable)?,
            None => Manifest {
                version: 1,
                references: vec![],
            },
        };
        if manifest.version != 1 {
            return Err(AppError::UnsupportedFormat);
        }
        let mut ids = HashSet::new();
        let mut keys = HashSet::new();
        if manifest.references.len() > 1000
            || manifest
                .references
                .iter()
                .any(|r| !ids.insert(r.id) || !keys.insert(r.storage_key))
        {
            return Err(AppError::StorageUnavailable);
        }
        Ok(Self {
            store,
            index,
            references: manifest.references,
        })
    }
    // Called only inside an authenticated coordinator operation.
    pub fn recover(&mut self) -> AppResult<()> {
        if let Some(bytes) = self.index.read("journal.json")? {
            let journal: Journal =
                serde_json::from_slice(&bytes).map_err(|_| AppError::StorageUnavailable)?;
            if journal.staged.len() > 1000 || journal.retired.len() > 1000 {
                return Err(AppError::LimitExceeded);
            }
            for id in journal.staged.into_iter().chain(journal.retired) {
                if !self.references.iter().any(|r| r.storage_key == id) {
                    self.store.delete(id)?;
                }
            }
            self.index.remove("journal.json")?;
        }
        Ok(())
    }
    pub fn record(&self, id: Uuid) -> AppResult<AccountRecord> {
        let reference = self
            .references
            .iter()
            .find(|r| r.id == id)
            .ok_or(AppError::NotFound)?;
        let bytes = self
            .store
            .read(reference.storage_key)?
            .ok_or(AppError::StorageUnavailable)?;
        let record: AccountRecord =
            serde_json::from_slice(&bytes).map_err(|_| AppError::StorageUnavailable)?;
        record.validate()?;
        if record.id != id {
            return Err(AppError::StorageUnavailable);
        }
        Ok(record)
    }
    pub fn summaries(&self) -> AppResult<Vec<AccountSummary>> {
        self.references
            .iter()
            .map(|r| Ok(self.record(r.id)?.summary(r.pinned)))
            .collect()
    }
    pub fn references(&self) -> Vec<Reference> {
        self.references.clone()
    }
    pub fn commit(
        &mut self,
        records: Vec<(AccountRecord, bool)>,
        delete: &[Uuid],
    ) -> AppResult<()> {
        self.commit_with_cleanup(records, delete, false)
    }
    /// Unlike an ordinary commit, erasure succeeds only after secret cleanup.
    /// A failed cleanup retains the journal for the next authenticated recovery.
    pub fn remove_all(&mut self) -> AppResult<()> {
        let ids: Vec<_> = self.references.iter().map(|r| r.id).collect();
        self.commit_with_cleanup(vec![], &ids, true)
    }
    fn commit_with_cleanup(
        &mut self,
        records: Vec<(AccountRecord, bool)>,
        delete: &[Uuid],
        require_cleanup: bool,
    ) -> AppResult<()> {
        self.recover()?;
        let mut next = self.references.clone();
        let mut retired = Vec::new();
        for id in delete {
            let r = next
                .iter()
                .find(|r| r.id == *id)
                .ok_or(AppError::NotFound)?;
            retired.push(r.storage_key);
            next.retain(|r| r.id != *id);
        }
        let mut staged = Vec::new();
        let mut payloads = Vec::new();
        let mut seen = HashSet::new();
        for (record, pinned) in records {
            record.validate()?;
            if !seen.insert(record.id) {
                return Err(AppError::Conflict);
            }
            let storage_key = Uuid::new_v4();
            staged.push(storage_key);
            payloads.push((
                storage_key,
                Zeroizing::new(
                    serde_json::to_vec(&record).map_err(|_| AppError::StorageUnavailable)?,
                ),
            ));
            let reference = Reference {
                id: record.id,
                storage_key,
                pinned,
            };
            if let Some(r) = next.iter_mut().find(|r| r.id == record.id) {
                retired.push(r.storage_key);
                *r = reference;
            } else {
                next.push(reference);
            }
        }
        if next.len() > 1000 {
            return Err(AppError::LimitExceeded);
        }
        let journal = Journal { staged, retired };
        self.index.write(
            "journal.json",
            &serde_json::to_vec(&journal).map_err(|_| AppError::StorageUnavailable)?,
        )?;
        for (id, bytes) in payloads {
            self.store.insert(id, &bytes)?;
        }
        let manifest = Manifest {
            version: 1,
            references: next.clone(),
        };
        if let Err(error) = self.index.write(
            "manifest.json",
            &serde_json::to_vec(&manifest).map_err(|_| AppError::StorageUnavailable)?,
        ) {
            // A write can fail after rename (directory fsync). Reload to avoid
            // treating an already-switched manifest as the previous set.
            let committed = self
                .index
                .read("manifest.json")?
                .and_then(|bytes| serde_json::from_slice::<Manifest>(&bytes).ok())
                .is_some_and(|m| m.version == 1 && m.references == next);
            if !committed {
                return Err(error);
            }
        }
        self.references = next;
        // Ordinary writes are durable even if cleanup fails. Explicit erasure
        // additionally requires secret cleanup; either path retains the journal
        // on failure and never rolls back a committed manifest.
        let cleanup = self.recover();
        if require_cleanup { cleanup } else { Ok(()) }
    }
    pub fn order(&mut self, ids: &[Uuid], pinned: &[Uuid]) -> AppResult<()> {
        if ids.len() != self.references.len()
            || ids.iter().copied().collect::<HashSet<_>>().len() != ids.len()
            || pinned.iter().any(|id| !ids.contains(id))
        {
            return Err(AppError::invalid("order"));
        }
        let next: Vec<_> = ids
            .iter()
            .map(|id| {
                let mut r = self
                    .references
                    .iter()
                    .find(|r| r.id == *id)
                    .ok_or(AppError::NotFound)?
                    .clone();
                r.pinned = pinned.contains(id);
                Ok(r)
            })
            .collect::<AppResult<_>>()?;
        self.index.write(
            "manifest.json",
            &serde_json::to_vec(&Manifest {
                version: 1,
                references: next.clone(),
            })
            .map_err(|_| AppError::StorageUnavailable)?,
        )?;
        self.references = next;
        Ok(())
    }
    pub fn consume_hotp(&mut self, id: Uuid) -> AppResult<String> {
        let mut record = self.record(id)?;
        if record.kind != crate::domain::account::OtpKind::Hotp {
            return Err(AppError::invalid("kind"));
        }
        if record.next_counter >= MAX_COUNTER {
            return Err(AppError::CounterExhausted);
        }
        let code = otp::hotp(
            record.secret.bytes(),
            record.next_counter,
            record.algorithm,
            record.digits,
        );
        record.next_counter += 1;
        let bytes =
            Zeroizing::new(serde_json::to_vec(&record).map_err(|_| AppError::StorageUnavailable)?);
        let r = self
            .references
            .iter()
            .find(|r| r.id == id)
            .ok_or(AppError::NotFound)?;
        self.store.update(r.storage_key, &bytes)?;
        Ok(code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };
    #[derive(Default)]
    struct State {
        data: HashMap<Uuid, Vec<u8>>,
        files: HashMap<String, Vec<u8>>,
        inserts: usize,
        fail_insert: Option<usize>,
        fail_manifest: bool,
        fail_update: bool,
        fail_journal_remove: bool,
    }
    struct MemoryStore(Arc<Mutex<State>>);
    struct MemoryIndex(Arc<Mutex<State>>);
    impl Store for MemoryStore {
        fn read(&self, id: Uuid) -> AppResult<Option<Zeroizing<Vec<u8>>>> {
            Ok(self
                .0
                .lock()
                .unwrap()
                .data
                .get(&id)
                .cloned()
                .map(Zeroizing::new))
        }
        fn insert(&mut self, id: Uuid, data: &[u8]) -> AppResult<()> {
            let mut s = self.0.lock().unwrap();
            s.inserts += 1;
            if s.fail_insert == Some(s.inserts) {
                return Err(AppError::StorageUnavailable);
            }
            s.data.insert(id, data.to_vec());
            Ok(())
        }
        fn update(&mut self, id: Uuid, data: &[u8]) -> AppResult<()> {
            let mut s = self.0.lock().unwrap();
            if s.fail_update {
                return Err(AppError::StorageUnavailable);
            }
            s.data.insert(id, data.to_vec());
            Ok(())
        }
        fn delete(&mut self, id: Uuid) -> AppResult<()> {
            self.0.lock().unwrap().data.remove(&id);
            Ok(())
        }
    }
    impl Index for MemoryIndex {
        fn read(&self, name: &str) -> AppResult<Option<Vec<u8>>> {
            Ok(self.0.lock().unwrap().files.get(name).cloned())
        }
        fn write(&mut self, name: &str, data: &[u8]) -> AppResult<()> {
            let mut s = self.0.lock().unwrap();
            if name == "manifest.json" && s.fail_manifest {
                return Err(AppError::StorageUnavailable);
            }
            s.files.insert(name.into(), data.to_vec());
            Ok(())
        }
        fn remove(&mut self, name: &str) -> AppResult<()> {
            let mut s = self.0.lock().unwrap();
            if name == "journal.json" && s.fail_journal_remove {
                return Err(AppError::StorageUnavailable);
            }
            s.files.remove(name);
            Ok(())
        }
    }
    fn open(s: &Arc<Mutex<State>>) -> Vault<MemoryStore, MemoryIndex> {
        Vault::open(MemoryStore(s.clone()), MemoryIndex(s.clone())).unwrap()
    }
    fn record(name: &str) -> AccountRecord {
        crate::compatibility::uri::parse_uri(&format!(
            "otpauth://hotp/{name}?secret=GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ&counter=0"
        ))
        .unwrap()
    }
    #[test]
    fn staging_failure_and_restart_keep_original_set() {
        let state = Arc::new(Mutex::new(State::default()));
        let mut v = open(&state);
        let r = record("original");
        let id = r.id;
        v.commit(vec![(r, false)], &[]).unwrap();
        state.lock().unwrap().fail_insert = Some(3);
        assert!(
            v.commit(
                vec![(record("staged-one"), false), (record("staged-two"), false)],
                &[]
            )
            .is_err()
        );
        drop(v);
        let mut v = open(&state);
        v.recover().unwrap();
        assert_eq!(v.summaries().unwrap().len(), 1);
        assert_eq!(v.record(id).unwrap().account, "original");
        assert_eq!(state.lock().unwrap().data.len(), 1);
        state.lock().unwrap().fail_insert = None;
        state.lock().unwrap().fail_manifest = true;
        assert!(v.commit(vec![(record("uncommitted"), false)], &[]).is_err());
        state.lock().unwrap().fail_manifest = false;
        drop(v);
        let mut v = open(&state);
        v.recover().unwrap();
        assert_eq!(v.summaries().unwrap().len(), 1);
    }
    #[test]
    fn hotp_write_failure_never_emits_or_consumes_and_restart_preserves_counter() {
        let state = Arc::new(Mutex::new(State::default()));
        let mut v = open(&state);
        let r = record("counter");
        let id = r.id;
        v.commit(vec![(r, false)], &[]).unwrap();
        state.lock().unwrap().fail_update = true;
        assert!(v.consume_hotp(id).is_err());
        assert_eq!(v.record(id).unwrap().next_counter, 0);
        state.lock().unwrap().fail_update = false;
        assert_eq!(v.consume_hotp(id).unwrap(), "755224");
        drop(v);
        let mut v = open(&state);
        assert_eq!(v.consume_hotp(id).unwrap(), "287082");
        let mut r = v.record(id).unwrap();
        r.next_counter = MAX_COUNTER;
        v.commit(vec![(r, false)], &[]).unwrap();
        assert!(matches!(
            v.consume_hotp(id),
            Err(AppError::CounterExhausted)
        ));
    }
    #[test]
    fn index_contains_only_opaque_references_and_pins() {
        let s = Arc::new(Mutex::new(State::default()));
        let mut v = open(&s);
        v.commit(vec![(record("sensitive-label"), true)], &[])
            .unwrap();
        for bytes in s.lock().unwrap().files.values() {
            let text = String::from_utf8_lossy(bytes);
            assert!(!text.contains("sensitive-label"));
            assert!(!text.contains("GEZDG"));
            assert!(!text.contains("counter"));
        }
    }
    #[test]
    fn removal_keeps_accounts_when_empty_manifest_cannot_be_committed() {
        let state = Arc::new(Mutex::new(State::default()));
        let mut v = open(&state);
        v.commit(vec![(record("preserved"), true)], &[]).unwrap();
        state.lock().unwrap().fail_manifest = true;
        assert!(v.remove_all().is_err());
        assert_eq!(v.summaries().unwrap().len(), 1);
        assert_eq!(state.lock().unwrap().data.len(), 1);
        state.lock().unwrap().fail_manifest = false;
        drop(v);
        let mut v = open(&state);
        v.recover().unwrap();
        assert_eq!(v.summaries().unwrap().len(), 1);
        v.remove_all().unwrap();
        assert!(state.lock().unwrap().data.is_empty());
    }
    #[test]
    fn removal_cleans_staged_orphans_and_requires_journal_cleanup_to_succeed() {
        let state = Arc::new(Mutex::new(State::default()));
        let mut v = open(&state);
        v.commit(vec![(record("current"), false)], &[]).unwrap();
        state.lock().unwrap().fail_insert = Some(3);
        assert!(
            v.commit(
                vec![(record("orphan"), false), (record("failed"), false)],
                &[]
            )
            .is_err()
        );
        assert_eq!(state.lock().unwrap().data.len(), 2);
        v.remove_all().unwrap();
        assert!(state.lock().unwrap().data.is_empty());
        state.lock().unwrap().fail_journal_remove = true;
        assert!(v.remove_all().is_err());
        assert!(state.lock().unwrap().files.contains_key("journal.json"));
        state.lock().unwrap().fail_journal_remove = false;
        v.recover().unwrap();
        assert!(v.references().is_empty());
    }
}
