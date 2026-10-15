//! Traditional macOS Keychain. Session authorization lives in application cases.
//! This adapter is never exposed as a generic IPC command.

use objc2_core_foundation::{CFData, CFDictionary, CFRetained, CFString, CFType};
use objc2_security::{SecItemAdd, SecItemCopyMatching, SecItemDelete, SecItemUpdate};
use std::ptr::NonNull;
use zeroize::Zeroizing;

use crate::domain::{AppError, AppResult};

pub struct Keychain {
    service: String,
}

impl Keychain {
    pub fn new(service: String) -> AppResult<Self> {
        if !service.starts_with("com.rafaelmariano.topotp.") {
            return Err(AppError::invalid("service"));
        }
        // SAFETY: The traditional Keychain API controls only Security's own UI,
        // not the separate LocalAuthentication actor. A trust mismatch must fail
        // closed; no item read or automatic refresh may open password dialogs.
        // This legacy API is needed because the app uses the file-based Keychain.
        #[allow(deprecated)]
        let status = unsafe { objc2_security::SecKeychainSetUserInteractionAllowed(false) };
        status_result(status)?;
        Ok(Self { service })
    }

    fn query(
        &self,
        id: &str,
        data: Option<&[u8]>,
        returning: bool,
    ) -> AppResult<CFRetained<CFDictionary<CFString, CFType>>> {
        if id.len() > 128 || id.is_empty() || data.is_some_and(|data| data.len() > 32 * 1024) {
            return Err(AppError::LimitExceeded);
        }
        let service = CFString::from_str(&self.service);
        let account = CFString::from_str(id);
        let payload = data.map(CFData::from_bytes);
        // SAFETY: macOS framework constants are valid. CFDictionary retains all
        // values and the typed dictionary contains only CFString/CFType pairs.
        let query = unsafe {
            use objc2_core_foundation::{kCFBooleanFalse, kCFBooleanTrue};
            use objc2_security::{
                kSecAttrAccount, kSecAttrService, kSecAttrSynchronizable, kSecClass,
                kSecClassGenericPassword, kSecReturnData, kSecUseDataProtectionKeychain,
                kSecValueData,
            };
            let no = kCFBooleanFalse.ok_or(AppError::StorageUnavailable)?;
            let yes = kCFBooleanTrue.ok_or(AppError::StorageUnavailable)?;
            let mut keys = vec![
                kSecClass,
                kSecAttrService,
                kSecAttrAccount,
                kSecUseDataProtectionKeychain,
                kSecAttrSynchronizable,
            ];
            let mut values: Vec<&CFType> =
                vec![kSecClassGenericPassword, &service, &account, no, no];
            if returning {
                keys.push(kSecReturnData);
                values.push(yes);
            }
            if let Some(payload) = &payload {
                keys.push(kSecValueData);
                values.push(payload);
            }
            CFDictionary::from_slices(&keys, &values)
        };
        Ok(query)
    }

    pub fn insert(&self, id: &str, value: &[u8]) -> AppResult<()> {
        let query = self.query(id, Some(value), false)?;
        // SAFETY: Typed query and null optional output; default traditional ACL
        // trusts only this creator. No SecAccess with an unrestricted trust list.
        let status = unsafe { SecItemAdd(query.as_opaque(), std::ptr::null_mut()) };
        status_result(status)
    }

    pub fn read(&self, id: &str) -> AppResult<Option<Zeroizing<Vec<u8>>>> {
        let query = self.query(id, None, true)?;
        let mut result = std::ptr::null();
        // SAFETY: The output points to a local pointer and the query requests
        // CFData only. Successful CopyMatching transfers one retained reference.
        let status = unsafe { SecItemCopyMatching(query.as_opaque(), &mut result) };
        if status == -25300 {
            return Ok(None);
        }
        status_result(status)?;
        let ptr = NonNull::new(result.cast_mut()).ok_or(AppError::StorageUnavailable)?;
        let object = unsafe { CFRetained::from_raw(ptr) };
        let data = object
            .downcast::<CFData>()
            .map_err(|_| AppError::StorageUnavailable)?;
        if data.len() > 32 * 1024 {
            return Err(AppError::LimitExceeded);
        }
        Ok(Some(Zeroizing::new(data.to_vec())))
    }

    pub fn update(&self, id: &str, value: &[u8]) -> AppResult<()> {
        if value.len() > 32 * 1024 {
            return Err(AppError::LimitExceeded);
        }
        let query = self.query(id, None, false)?;
        let data = CFData::from_bytes(value);
        // SAFETY: The update dictionary contains the documented data attribute.
        let attributes = unsafe {
            CFDictionary::<CFString, CFType>::from_slices(
                &[objc2_security::kSecValueData],
                &[&data],
            )
        };
        let status = unsafe { SecItemUpdate(query.as_opaque(), attributes.as_opaque()) };
        status_result(status)
    }

    pub fn delete(&self, id: &str) -> AppResult<()> {
        let query = self.query(id, None, false)?;
        let status = unsafe { SecItemDelete(query.as_opaque()) };
        if status == -25300 {
            return Ok(());
        }
        status_result(status)
    }
}

fn status_result(status: i32) -> AppResult<()> {
    match status {
        0 => Ok(()),
        -128 => Err(AppError::Cancelled),
        -25293 | -25308 => Err(AppError::KeychainAccessDenied),
        _ => Err(AppError::StorageUnavailable),
    }
}

impl crate::application::vault::Store for Keychain {
    fn read(&self, id: uuid::Uuid) -> AppResult<Option<Zeroizing<Vec<u8>>>> {
        self.read(&id.to_string())
    }
    fn insert(&mut self, id: uuid::Uuid, data: &[u8]) -> AppResult<()> {
        Keychain::insert(self, &id.to_string(), data)
    }
    fn update(&mut self, id: uuid::Uuid, data: &[u8]) -> AppResult<()> {
        Keychain::update(self, &id.to_string(), data)
    }
    fn delete(&mut self, id: uuid::Uuid) -> AppResult<()> {
        Keychain::delete(self, &id.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn denied_or_interactive_storage_has_a_distinct_error() {
        for status in [-25293, -25308] {
            assert!(matches!(
                status_result(status),
                Err(AppError::KeychainAccessDenied)
            ));
        }
        assert!(matches!(status_result(-128), Err(AppError::Cancelled)));
    }
}
