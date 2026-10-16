//! Private native transport to the immutable Keychain owner. Never exposed to JS.
use crate::{
    application::vault::Store,
    domain::{AppError, AppResult},
};
use objc2_core_foundation::{CFDictionary, CFNumber, CFRetained, CFString, CFType};
use objc2_security::{SecCSFlags, SecCode, SecRequirement};
use std::{
    io::{Read, Write},
    net::Shutdown,
    os::unix::{io::OwnedFd, net::UnixStream},
    path::Path,
    process::{Child, Command, Stdio},
    ptr::NonNull,
    sync::Mutex,
    time::Duration,
};
use uuid::Uuid;
use zeroize::Zeroizing;

const LIMIT: usize = 32 * 1024;
const HELLO: &[u8; 8] = b"TOPOTP01";
pub struct HelperStore(Mutex<Channel>);
struct Channel {
    socket: UnixStream,
    child: Option<Child>,
    failed: bool,
}
impl HelperStore {
    pub fn packaged(path: &Path) -> AppResult<Self> {
        Self::connect(
            path,
            option_env!("TOPOTP_VAULT_HELPER_CDHASH").ok_or(AppError::VaultServiceUnavailable)?,
            option_env!("TOPOTP_VAULT_HELPER_CERT_SHA1")
                .ok_or(AppError::VaultServiceUnavailable)?,
        )
    }
    pub fn connect(path: &Path, cdhash: &str, certificate: &str) -> AppResult<Self> {
        if [cdhash, certificate]
            .iter()
            .any(|value| value.len() != 40 || !value.bytes().all(|c| c.is_ascii_hexdigit()))
        {
            return Err(AppError::VaultServiceUnavailable);
        }
        let (socket, peer) = UnixStream::pair().map_err(unavailable)?;
        socket
            .set_read_timeout(Some(Duration::from_secs(10)))
            .map_err(unavailable)?;
        socket
            .set_write_timeout(Some(Duration::from_secs(10)))
            .map_err(unavailable)?;
        let fd: OwnedFd = peer.into();
        let child = Command::new(path)
            .stdin(Stdio::from(fd))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(unavailable)?;
        let mut channel = Channel {
            socket,
            child: Some(child),
            failed: false,
        };
        let mut hello = [0; 8];
        channel.socket.read_exact(&mut hello).map_err(unavailable)?;
        if &hello != HELLO {
            return Err(AppError::VaultServiceUnavailable);
        }
        // The owned Child has not been reaped, so its PID cannot be reused.
        verify_child(channel.child.as_ref().unwrap().id(), cdhash, certificate)?;
        let (status, bytes) = channel.response()?;
        if status != 0 || !bytes.is_empty() {
            return Err(AppError::VaultServiceUnavailable);
        }
        Ok(Self(Mutex::new(channel)))
    }
    fn request(
        &self,
        operation: u8,
        id: Uuid,
        payload: &[u8],
    ) -> AppResult<(i32, Zeroizing<Vec<u8>>)> {
        if payload.len() > LIMIT || ((operation == 2 || operation == 3) && payload.is_empty()) {
            return Err(AppError::LimitExceeded);
        }
        let mut channel = self.0.lock().map_err(unavailable)?;
        if channel.failed {
            return Err(AppError::VaultServiceUnavailable);
        }
        let result = channel.exchange(operation, id, payload);
        if matches!(result, Err(AppError::VaultServiceUnavailable)) {
            channel.stop();
        }
        result
    }
}
fn unavailable<T>(_: T) -> AppError {
    AppError::VaultServiceUnavailable
}
fn verify_child(pid: u32, cdhash: &str, certificate: &str) -> AppResult<()> {
    let pid = CFNumber::new_i32(pid.try_into().map_err(unavailable)?);
    let requirement = CFString::from_str(&format!(
        "identifier \"com.rafaelmariano.topotp.vault-helper.v1\" and certificate leaf = H\"{certificate}\" and cdhash H\"{cdhash}\""
    ));
    // SAFETY: Framework constants and owned output pointers; each successful
    // Copy/Create transfers one reference. Only the live owned child is checked.
    unsafe {
        let attributes = CFDictionary::<CFString, CFType>::from_slices(
            &[objc2_security::kSecGuestAttributePid],
            &[&pid],
        );
        let mut code = std::ptr::null_mut();
        if SecCode::copy_guest_with_attributes(
            None,
            Some(attributes.as_opaque()),
            SecCSFlags::DefaultFlags,
            NonNull::from(&mut code),
        ) != 0
        {
            return Err(AppError::VaultServiceUnavailable);
        }
        let code =
            CFRetained::from_raw(NonNull::new(code).ok_or(AppError::VaultServiceUnavailable)?);
        let mut parsed = std::ptr::null_mut();
        if SecRequirement::create_with_string(
            &requirement,
            SecCSFlags::DefaultFlags,
            NonNull::from(&mut parsed),
        ) != 0
        {
            return Err(AppError::VaultServiceUnavailable);
        }
        let parsed =
            CFRetained::from_raw(NonNull::new(parsed).ok_or(AppError::VaultServiceUnavailable)?);
        // kSecCSStrictValidate is 1 << 4 in Security/CSCommon.h.
        if code.check_validity(
            SecCSFlags(1 << 4) | SecCSFlags::NoNetworkAccess,
            Some(&parsed),
        ) != 0
        {
            return Err(AppError::VaultServiceUnavailable);
        }
    }
    Ok(())
}
impl Channel {
    fn response(&mut self) -> AppResult<(i32, Zeroizing<Vec<u8>>)> {
        let mut header = [0; 8];
        self.socket.read_exact(&mut header).map_err(unavailable)?;
        let status = i32::from_be_bytes(header[..4].try_into().unwrap());
        let length = u32::from_be_bytes(header[4..].try_into().unwrap()) as usize;
        if length > LIMIT {
            return Err(AppError::VaultServiceUnavailable);
        }
        let mut bytes = Zeroizing::new(vec![0; length]);
        self.socket.read_exact(&mut bytes).map_err(unavailable)?;
        if status != 0 && !bytes.is_empty() {
            return Err(AppError::VaultServiceUnavailable);
        }
        Ok((status, bytes))
    }
    fn exchange(
        &mut self,
        operation: u8,
        id: Uuid,
        payload: &[u8],
    ) -> AppResult<(i32, Zeroizing<Vec<u8>>)> {
        let mut header = [0; 21];
        header[0] = operation;
        header[1..17].copy_from_slice(id.as_bytes());
        header[17..].copy_from_slice(&(payload.len() as u32).to_be_bytes());
        self.socket
            .write_all(&header)
            .and_then(|_| self.socket.write_all(payload))
            .map_err(unavailable)?;
        self.response()
    }
    fn stop(&mut self) {
        self.failed = true;
        let _ = self.socket.shutdown(Shutdown::Both);
        if let Some(mut child) = self.child.take() {
            if !matches!(child.try_wait(), Ok(Some(_))) {
                let _ = child.kill();
            }
            let _ = child.wait();
        }
    }
}
impl Drop for Channel {
    fn drop(&mut self) {
        self.stop();
    }
}
fn stored(status: i32) -> AppResult<()> {
    match status {
        0 => Ok(()),
        -25293 | -25308 => Err(AppError::KeychainAccessDenied),
        -50 | -67050 => Err(AppError::VaultServiceUnavailable),
        _ => Err(AppError::StorageUnavailable),
    }
}
impl Store for HelperStore {
    fn read(&self, id: Uuid) -> AppResult<Option<Zeroizing<Vec<u8>>>> {
        let (status, bytes) = self.request(1, id, &[])?;
        if status == -25300 {
            return Ok(None);
        }
        stored(status)?;
        Ok(Some(bytes))
    }
    fn insert(&mut self, id: Uuid, value: &[u8]) -> AppResult<()> {
        stored(self.request(2, id, value)?.0)
    }
    fn update(&mut self, id: Uuid, value: &[u8]) -> AppResult<()> {
        stored(self.request(3, id, value)?.0)
    }
    fn delete(&mut self, id: Uuid) -> AppResult<()> {
        let status = self.request(4, id, &[])?.0;
        if status == -25300 {
            Ok(())
        } else {
            stored(status)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn pair() -> (HelperStore, UnixStream) {
        let (socket, peer) = UnixStream::pair().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_millis(100)))
            .unwrap();
        (
            HelperStore(Mutex::new(Channel {
                socket,
                child: None,
                failed: false,
            })),
            peer,
        )
    }
    #[test]
    fn native_transport_preserves_uuid_and_bounded_payload() {
        let (mut store, mut peer) = pair();
        let id = Uuid::new_v4();
        let server = std::thread::spawn(move || {
            let mut header = [0; 21];
            peer.read_exact(&mut header).unwrap();
            assert_eq!(header[0], 2);
            assert_eq!(&header[1..17], id.as_bytes());
            let count = u32::from_be_bytes(header[17..].try_into().unwrap());
            assert_eq!(count, 7);
            let mut value = [0; 7];
            peer.read_exact(&mut value).unwrap();
            assert_eq!(&value, b"fixture");
            peer.write_all(&[0; 8]).unwrap();
            peer.read_exact(&mut header).unwrap();
            assert_eq!(header[0], 1);
            assert_eq!(&header[1..17], id.as_bytes());
            peer.write_all(&[0, 0, 0, 0, 0, 0, 0, 7]).unwrap();
            peer.write_all(b"fixture").unwrap();
        });
        store.insert(id, b"fixture").unwrap();
        assert_eq!(&*store.read(id).unwrap().unwrap(), b"fixture");
        server.join().unwrap();
    }
    #[test]
    fn oversized_response_stops_the_channel_without_another_request() {
        let (store, mut peer) = pair();
        let server = std::thread::spawn(move || {
            let mut header = [0; 21];
            peer.read_exact(&mut header).unwrap();
            peer.write_all(&[0, 0, 0, 0, 0, 1, 0, 0]).unwrap();
            assert_eq!(peer.read(&mut header).unwrap(), 0);
        });
        for _ in 0..2 {
            assert!(matches!(
                store.read(Uuid::new_v4()),
                Err(AppError::VaultServiceUnavailable)
            ));
        }
        server.join().unwrap();
    }
    #[test]
    fn timeout_is_latched_instead_of_reconnecting() {
        let (store, mut peer) = pair();
        assert!(matches!(
            store.read(Uuid::new_v4()),
            Err(AppError::VaultServiceUnavailable)
        ));
        assert!(matches!(
            store.read(Uuid::new_v4()),
            Err(AppError::VaultServiceUnavailable)
        ));
        let mut header = [0; 21];
        peer.read_exact(&mut header).unwrap();
        assert_eq!(peer.read(&mut header).unwrap(), 0);
    }
}
