use std::path::{Path, PathBuf};

use crate::error::Error;

const KEYRING_SERVICE: &str = "taskstream";

/// Token store: system keyring by profile name, with a 0600 file fallback
/// when no keyring backend is available.
pub struct Secrets {
    fallback_path: PathBuf,
}

impl Secrets {
    pub fn new(fallback_path: PathBuf) -> Self {
        Self { fallback_path }
    }

    pub fn default_fallback_path() -> PathBuf {
        crate::config::default_config_path()
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("secrets.json")
    }

    pub fn set(&self, profile: &str, token: &str) -> Result<(), Error> {
        match keyring::Entry::new(KEYRING_SERVICE, profile) {
            Ok(entry) if entry.set_password(token).is_ok() => Ok(()),
            _ => self.file_set(profile, token),
        }
    }

    pub fn get(&self, profile: &str) -> Result<Option<String>, Error> {
        if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, profile) {
            match entry.get_password() {
                Ok(pw) => return Ok(Some(pw)),
                Err(keyring::Error::NoEntry) => {}
                Err(_) => {}
            }
        }
        self.file_get(profile)
    }

    pub fn delete(&self, profile: &str) -> Result<(), Error> {
        if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, profile) {
            let _ = entry.delete_credential();
        }
        self.file_delete(profile)
    }

    fn file_map(&self) -> Result<std::collections::BTreeMap<String, String>, Error> {
        match std::fs::read_to_string(&self.fallback_path) {
            Ok(t) => {
                serde_json::from_str(&t).map_err(|e| Error::Config(format!("parse secrets: {e}")))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Default::default()),
            Err(e) => Err(Error::Io(format!("read secrets: {e}"))),
        }
    }

    fn file_write(&self, map: &std::collections::BTreeMap<String, String>) -> Result<(), Error> {
        if let Some(dir) = self.fallback_path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| Error::Io(format!("mkdir: {e}")))?;
        }
        let text = serde_json::to_string_pretty(map)
            .map_err(|e| Error::Internal(format!("serialize secrets: {e}")))?;
        std::fs::write(&self.fallback_path, text)
            .map_err(|e| Error::Io(format!("write secrets: {e}")))?;
        set_file_mode_0600(&self.fallback_path)
    }

    fn file_set(&self, profile: &str, token: &str) -> Result<(), Error> {
        let mut m = self.file_map()?;
        m.insert(profile.into(), token.into());
        self.file_write(&m)
    }

    fn file_get(&self, profile: &str) -> Result<Option<String>, Error> {
        Ok(self.file_map()?.get(profile).cloned())
    }

    fn file_delete(&self, profile: &str) -> Result<(), Error> {
        let mut m = self.file_map()?;
        m.remove(profile);
        self.file_write(&m)
    }
}

#[cfg(unix)]
fn set_file_mode_0600(path: &Path) -> Result<(), Error> {
    use std::os::unix::fs::PermissionsExt;
    let perms = std::fs::Permissions::from_mode(0o600);
    std::fs::set_permissions(path, perms).map_err(|e| Error::Io(format!("chmod secrets: {e}")))
}

#[cfg(not(unix))]
fn set_file_mode_0600(_path: &Path) -> Result<(), Error> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_fallback_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let s = Secrets::new(dir.path().join("secrets.json"));
        // Exercise the file layer directly (keyring backend may be absent in CI).
        s.file_set("primary", "tok123").unwrap();
        assert_eq!(s.file_get("primary").unwrap().as_deref(), Some("tok123"));
        s.file_delete("primary").unwrap();
        assert_eq!(s.file_get("primary").unwrap(), None);
    }
}
