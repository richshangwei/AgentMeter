//! Current-user protected durable storage for tablet device-pair credentials.
//!
//! The on-disk representation is always DPAPI ciphertext on Windows.  The
//! non-Windows implementation deliberately returns `Unsupported` so a
//! development build cannot silently downgrade to plaintext persistence.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
#[cfg(windows)]
use std::fs::File;
use std::fs::OpenOptions;
use std::io::{self, ErrorKind};
#[cfg(windows)]
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const STORE_SCHEMA_VERSION: u32 = 1;
const MAX_PLAINTEXT_BYTES: usize = 1024 * 1024;
const MAX_CIPHERTEXT_BYTES: usize = 16 * 1024 * 1024;
const TOKEN_HEX_BYTES: usize = 64;
const CSRF_HEX_BYTES: usize = 32;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(crate) struct StoredPair {
    pub(crate) token: String,
    pub(crate) exchange_csrf: String,
    pub(crate) revoked: bool,
}

#[derive(Deserialize, Serialize)]
struct PersistedPairs {
    schema_version: u32,
    pairs: Vec<StoredPair>,
}

pub(crate) struct PairStore {
    path: PathBuf,
    #[cfg(windows)]
    lock: File,
}

impl PairStore {
    pub(crate) fn open(path: &Path) -> io::Result<Self> {
        #[cfg(not(windows))]
        {
            let _ = path;
            return Err(unsupported());
        }

        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;

            let lock_path = lock_path(path);
            let lock = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .share_mode(0)
                .open(lock_path)
                .map_err(|error| io::Error::new(error.kind(), "pair store lock unavailable"))?;
            Ok(Self {
                path: path.to_owned(),
                lock,
            })
        }
    }

    pub(crate) fn load(&self) -> io::Result<Vec<StoredPair>> {
        #[cfg(not(windows))]
        {
            return Err(unsupported());
        }

        #[cfg(windows)]
        {
            let _lock = &self.lock;
            let mut ciphertext = Vec::new();
            let file = match File::open(&self.path) {
                Ok(file) => file,
                Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
                Err(error) => return Err(error),
            };
            file.take((MAX_CIPHERTEXT_BYTES + 1) as u64)
                .read_to_end(&mut ciphertext)?;
            if ciphertext.is_empty() || ciphertext.len() > MAX_CIPHERTEXT_BYTES {
                return Err(invalid_store());
            }
            let plaintext = unprotect(&ciphertext)?;
            if plaintext.is_empty() || plaintext.len() > MAX_PLAINTEXT_BYTES {
                return Err(invalid_store());
            }
            let persisted: PersistedPairs =
                serde_json::from_slice(&plaintext).map_err(|_| invalid_store())?;
            if persisted.schema_version != STORE_SCHEMA_VERSION {
                return Err(invalid_store());
            }
            validate_pairs(&persisted.pairs)?;
            Ok(persisted.pairs)
        }
    }

    pub(crate) fn save(&self, pairs: &[StoredPair]) -> io::Result<()> {
        validate_pairs(pairs)?;

        #[cfg(not(windows))]
        {
            let _ = pairs;
            return Err(unsupported());
        }

        #[cfg(windows)]
        {
            let _lock = &self.lock;
            let plaintext = serde_json::to_vec(&PersistedPairs {
                schema_version: STORE_SCHEMA_VERSION,
                pairs: pairs.to_vec(),
            })
            .map_err(|_| invalid_store())?;
            if plaintext.len() > MAX_PLAINTEXT_BYTES {
                return Err(invalid_store());
            }
            let ciphertext = protect(&plaintext)?;
            if ciphertext.is_empty() || ciphertext.len() > MAX_CIPHERTEXT_BYTES {
                return Err(invalid_store());
            }

            let temporary = temporary_path(&self.path)?;
            let write_result = (|| {
                let mut file = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&temporary)?;
                file.write_all(&ciphertext)?;
                file.flush()?;
                file.sync_all()?;
                drop(file);
                atomic_replace(&temporary, &self.path)
            })();
            if write_result.is_err() {
                let _ = std::fs::remove_file(&temporary);
            }
            write_result
        }
    }
}

fn validate_pairs(pairs: &[StoredPair]) -> io::Result<()> {
    let mut tokens = HashSet::with_capacity(pairs.len());
    for pair in pairs {
        if !valid_hex_secret(&pair.token, TOKEN_HEX_BYTES) || !tokens.insert(&pair.token) {
            return Err(invalid_store());
        }
        if !valid_hex_secret(&pair.exchange_csrf, CSRF_HEX_BYTES) {
            return Err(invalid_store());
        }
    }
    Ok(())
}

fn valid_hex_secret(value: &str, expected_len: usize) -> bool {
    value.len() == expected_len
        && value.len().is_multiple_of(2)
        && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn invalid_store() -> io::Error {
    io::Error::new(ErrorKind::InvalidData, "pair store data is invalid")
}

#[cfg(not(windows))]
fn unsupported() -> io::Error {
    io::Error::new(
        ErrorKind::Unsupported,
        "durable pair storage requires Windows user protection",
    )
}

#[cfg(windows)]
fn lock_path(path: &Path) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(".lock");
    PathBuf::from(value)
}

#[cfg(windows)]
fn temporary_path(path: &Path) -> io::Result<PathBuf> {
    let mut random = [0u8; 8];
    getrandom::fill(&mut random)
        .map_err(|_| io::Error::other("pair store temporary name failed"))?;
    let suffix = random
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let mut value = path.as_os_str().to_os_string();
    value.push(format!(".tmp-{}-{suffix}", std::process::id()));
    Ok(PathBuf::from(value))
}

#[cfg(windows)]
fn protect(plaintext: &[u8]) -> io::Result<Vec<u8>> {
    use std::ptr;
    use std::slice;
    use windows_sys::Win32::Foundation::{HLOCAL, LocalFree};
    use windows_sys::Win32::Security::Cryptography::{
        CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData,
    };

    let input = CRYPT_INTEGER_BLOB {
        cbData: u32::try_from(plaintext.len()).map_err(|_| invalid_store())?,
        pbData: plaintext.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    // No CRYPTPROTECT_LOCAL_MACHINE flag: DPAPI binds this ciphertext to the
    // current Windows user profile, not to the machine or all users.
    let success = unsafe {
        CryptProtectData(
            &input,
            ptr::null(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    if success == 0 || output.pbData.is_null() || output.cbData == 0 {
        return Err(crypto_failure());
    }
    let bytes = unsafe { slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
    unsafe {
        let _ = LocalFree(output.pbData.cast::<core::ffi::c_void>() as HLOCAL);
    }
    Ok(bytes)
}

#[cfg(windows)]
fn unprotect(ciphertext: &[u8]) -> io::Result<Vec<u8>> {
    use std::ptr;
    use std::slice;
    use windows_sys::Win32::Foundation::{HLOCAL, LocalFree};
    use windows_sys::Win32::Security::Cryptography::{
        CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptUnprotectData,
    };

    let input = CRYPT_INTEGER_BLOB {
        cbData: u32::try_from(ciphertext.len()).map_err(|_| invalid_store())?,
        pbData: ciphertext.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    let success = unsafe {
        CryptUnprotectData(
            &input,
            ptr::null_mut(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    if success == 0 || output.pbData.is_null() || output.cbData == 0 {
        return Err(crypto_failure());
    }
    let bytes = unsafe { slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
    unsafe {
        let _ = LocalFree(output.pbData.cast::<core::ffi::c_void>() as HLOCAL);
    }
    Ok(bytes)
}

#[cfg(windows)]
fn crypto_failure() -> io::Error {
    io::Error::new(ErrorKind::PermissionDenied, "pair store protection failed")
}

#[cfg(windows)]
fn atomic_replace(source: &Path, destination: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };

    let mut source_wide = source.as_os_str().encode_wide().collect::<Vec<_>>();
    source_wide.push(0);
    let mut destination_wide = destination.as_os_str().encode_wide().collect::<Vec<_>>();
    destination_wide.push(0);
    let success = unsafe {
        MoveFileExW(
            source_wide.as_ptr(),
            destination_wide.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if success == 0 {
        Err(io::Error::other("pair store atomic replace failed"))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn sample() -> StoredPair {
        StoredPair {
            token: "a1".repeat(32),
            exchange_csrf: "b2".repeat(16),
            revoked: false,
        }
    }

    fn test_path(label: &str) -> (PathBuf, PathBuf) {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "agentmeter-pair-store-{label}-{}-{stamp}",
            std::process::id()
        ));
        std::fs::create_dir(&directory).unwrap();
        (directory.clone(), directory.join("pairs.bin"))
    }

    #[cfg(windows)]
    #[test]
    fn round_trip_is_dpapi_ciphertext_and_validates_schema() {
        let (directory, path) = test_path("roundtrip");
        let store = PairStore::open(&path).unwrap();
        let pair = sample();
        store.save(std::slice::from_ref(&pair)).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert!(
            !bytes
                .windows(pair.token.len())
                .any(|window| window == pair.token.as_bytes())
        );
        assert_eq!(store.load().unwrap(), vec![pair]);
        drop(store);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn independent_process_can_reload_current_user_ciphertext() {
        const CHILD_STORE_PATH: &str = "AGENTMETER_PAIR_STORE_CHILD_PATH";
        let (directory, path) = test_path("process");
        let pair = sample();
        let store = PairStore::open(&path).unwrap();
        store.save(std::slice::from_ref(&pair)).unwrap();
        drop(store);

        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "pair_store::tests::child_process_load",
                "--nocapture",
            ])
            .env(CHILD_STORE_PATH, &path)
            .output()
            .unwrap();
        assert!(output.status.success());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn child_process_load() {
        let Ok(path) = std::env::var("AGENTMETER_PAIR_STORE_CHILD_PATH") else {
            return;
        };
        let store = PairStore::open(Path::new(&path)).unwrap();
        assert_eq!(store.load().unwrap(), vec![sample()]);
    }

    #[cfg(windows)]
    #[test]
    fn lock_is_exclusive_for_store_lifetime() {
        let (directory, path) = test_path("lock");
        let first = PairStore::open(&path).unwrap();
        let second = PairStore::open(&path);
        assert!(second.is_err());
        drop(first);
        assert!(PairStore::open(&path).is_ok());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn corrupt_ciphertext_fails_closed_without_plaintext_fallback() {
        let (directory, path) = test_path("corrupt");
        let store = PairStore::open(&path).unwrap();
        std::fs::write(&path, b"not dpapi ciphertext").unwrap();
        let error = store.load().unwrap_err();
        assert_eq!(error.kind(), ErrorKind::PermissionDenied);
        assert_eq!(error.to_string(), "pair store protection failed");
        drop(store);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn invalid_hex_secrets_are_rejected_before_persistence() {
        let mut pair = sample();
        pair.token = "not-hex".into();
        assert_eq!(
            validate_pairs(&[pair]).unwrap_err().kind(),
            ErrorKind::InvalidData
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn non_windows_is_explicitly_unsupported() {
        let path = Path::new("pairs.bin");
        assert!(matches!(
            PairStore::open(path),
            Err(error) if error.kind() == ErrorKind::Unsupported
        ));
    }
}
