//! Diagnostic-only native API probe, excluded from regular/release builds.

use std::time::{SystemTime, UNIX_EPOCH};

use objc2_core_foundation::{CFData, CFDictionary, CFString, CFType};
use objc2_local_authentication::{LAContext, LAPolicy};
use objc2_security::{SecAccessControl, SecAccessControlCreateFlags, SecItemAdd, SecItemDelete};
use serde::Serialize;

use crate::domain::{AppError, AppResult};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeProbeReport {
    pub synthetic_only: bool,
    pub device_owner_authentication_available: bool,
    pub touch_id_available: bool,
    pub add_status: i32,
    pub data_protection_keychain: bool,
    pub read_status: Option<i32>,
    pub update_status: Option<i32>,
    pub cleanup_status: Option<i32>,
    pub blocked_by_missing_entitlement: bool,
    pub authenticated_read_verified: bool,
}

pub fn probe() -> AppResult<NativeProbeReport> {
    let protected = std::env::args().any(|arg| arg == "--data-protection");
    // SAFETY: Framework constructors and policy preflight have no callbacks or
    // borrowed buffers. Preflight does not authenticate or disclose credentials.
    let context = unsafe { LAContext::new() };
    let owner_available = unsafe {
        context
            .canEvaluatePolicy_error(LAPolicy::DeviceOwnerAuthentication)
            .is_ok()
    };
    let touch_id_available = unsafe {
        context
            .canEvaluatePolicy_error(LAPolicy::DeviceOwnerAuthenticationWithBiometrics)
            .is_ok()
    };
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| AppError::StorageUnavailable)?
        .as_nanos();
    let service = CFString::from_str(&format!("com.rafaelmariano.topotp.probe.{suffix}"));
    let account = CFString::from_str("synthetic-control");
    let data = CFData::from_bytes(b"TopOTP public synthetic Rust probe");

    // SAFETY: All extern constants are provided by Security/CoreFoundation on
    // supported macOS versions. Dictionaries retain their keys/values. The
    // protection is a documented CFString, and result/error pointers are null.
    let (add_status, cleanup_status, read_status, update_status) = unsafe {
        use objc2_core_foundation::{kCFBooleanFalse, kCFBooleanTrue};
        use objc2_security::{
            kSecAttrAccessControl, kSecAttrAccessibleWhenUnlockedThisDeviceOnly, kSecAttrAccount,
            kSecAttrService, kSecAttrSynchronizable, kSecClass, kSecClassGenericPassword,
            kSecUseDataProtectionKeychain, kSecValueData,
        };
        let access = SecAccessControl::with_flags(
            None,
            kSecAttrAccessibleWhenUnlockedThisDeviceOnly,
            SecAccessControlCreateFlags::UserPresence,
            std::ptr::null_mut(),
        )
        .ok_or(AppError::StorageUnavailable)?;
        let yes = kCFBooleanTrue.ok_or(AppError::StorageUnavailable)?;
        let no = kCFBooleanFalse.ok_or(AppError::StorageUnavailable)?;
        let mut keys = vec![
            kSecClass,
            kSecAttrService,
            kSecAttrAccount,
            kSecUseDataProtectionKeychain,
            kSecAttrSynchronizable,
            kSecValueData,
        ];
        let mut values: Vec<&CFType> = vec![
            kSecClassGenericPassword,
            &service,
            &account,
            if protected { yes } else { no },
            no,
            &data,
        ];
        if protected {
            keys.push(kSecAttrAccessControl);
            values.push(&access);
        }
        let attributes = CFDictionary::from_slices(&keys, &values);
        let status = SecItemAdd(attributes.as_opaque(), std::ptr::null_mut());
        let mut read_status = None;
        let mut update_status = None;
        let cleanup = if status == 0 {
            if !protected {
                let store = super::keychain::Keychain::new(service.to_string())?;
                read_status = Some(
                    if store.read("synthetic-control").is_ok_and(|value| {
                        value.is_some_and(|bytes| {
                            bytes.as_slice() == b"TopOTP public synthetic Rust probe"
                        })
                    }) {
                        0
                    } else {
                        -1
                    },
                );
                update_status = Some(
                    if store
                        .update(
                            "synthetic-control",
                            b"TopOTP public synthetic updated Rust probe",
                        )
                        .is_ok()
                    {
                        0
                    } else {
                        -1
                    },
                );
            }
            let identity = CFDictionary::from_slices(&keys[..5], &values[..5]);
            Some(SecItemDelete(identity.as_opaque()))
        } else {
            None
        };
        (status, cleanup, read_status, update_status)
    };
    Ok(NativeProbeReport {
        synthetic_only: true,
        device_owner_authentication_available: owner_available,
        touch_id_available,
        add_status,
        data_protection_keychain: protected,
        read_status,
        update_status,
        cleanup_status,
        blocked_by_missing_entitlement: add_status == -34018,
        authenticated_read_verified: false,
    })
}
