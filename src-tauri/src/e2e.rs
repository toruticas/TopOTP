//! Synthetic adapters compiled only with the explicit e2e feature. No runtime
//! environment variable can enable these in a production build.
use crate::{
    application::{
        core::{Core, SystemClock},
        ports::{Authentication, Clipboard, NativeApproval},
        vault::{DiskIndex, Store},
    },
    domain::AppResult,
};
use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc};
use uuid::Uuid;
use zeroize::Zeroizing;
struct SyntheticStore(HashMap<Uuid, Zeroizing<Vec<u8>>>);
impl Store for SyntheticStore {
    fn read(&self, id: Uuid) -> AppResult<Option<Zeroizing<Vec<u8>>>> {
        Ok(self.0.get(&id).cloned())
    }
    fn insert(&mut self, id: Uuid, b: &[u8]) -> AppResult<()> {
        self.0.insert(id, Zeroizing::new(b.to_vec()));
        Ok(())
    }
    fn update(&mut self, id: Uuid, b: &[u8]) -> AppResult<()> {
        self.insert(id, b)
    }
    fn delete(&mut self, id: Uuid) -> AppResult<()> {
        self.0.remove(&id);
        Ok(())
    }
}
struct SyntheticAuth;
impl Authentication for SyntheticAuth {
    fn authenticate<'a>(
        &'a self,
        _: &'a str,
    ) -> Pin<Box<dyn Future<Output = AppResult<NativeApproval>> + Send + 'a>> {
        Box::pin(async { Ok(NativeApproval::from_native_success()) })
    }
    fn cancel(&self) {}
}
struct SyntheticClipboard;
impl Clipboard for SyntheticClipboard {
    fn copy(&self, _: &str) -> AppResult<i64> {
        Ok(1)
    }
    fn clear_if_owned(&self, _: i64) {}
}
pub fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.artifacts/e2e")
}
pub fn core() -> AppResult<Core> {
    let root = root().join("vault");
    let real_store = std::env::var("TOPOTP_E2E_REAL_STORE").as_deref() == Ok("1");
    let store: Box<dyn Store> = if real_store {
        let value = std::env::var("TOPOTP_E2E_HELPER")
            .map_err(|_| crate::domain::AppError::VaultServiceUnavailable)?;
        let metadata: serde_json::Value = serde_json::from_str(&value)
            .map_err(|_| crate::domain::AppError::VaultServiceUnavailable)?;
        Box::new(crate::platform::macos::vault_helper::HelperStore::connect(
            std::path::Path::new(metadata["binary"].as_str().unwrap()),
            metadata["cdhash"].as_str().unwrap(),
            metadata["certificateSHA1"].as_str().unwrap(),
        )?)
    } else {
        Box::new(SyntheticStore(HashMap::new()))
    };
    let store = if real_store {
        // This feature is debug-only and the signed .e2e peer has a separate
        // Keychain namespace. Clear only its previous public fixture references.
        let mut previous =
            crate::application::vault::Vault::open(store, DiskIndex::new(root.clone())?)?;
        previous.remove_all()?;
        previous.store
    } else {
        store
    };
    if !real_store && root.exists() {
        std::fs::remove_dir_all(&root).map_err(|_| crate::domain::AppError::StorageUnavailable)?;
    }
    Core::new(
        store,
        DiskIndex::new(root)?,
        Arc::new(SyntheticAuth),
        Arc::new(SyntheticClipboard),
        Arc::new(SystemClock::default()),
    )
}
