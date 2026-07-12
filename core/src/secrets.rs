use std::path::{Path, PathBuf};

use crate::error::Error;

const KEYRING_SERVICE: &str = "laba";

/// Token store: system keyring by profile name, with a 0600 file fallback
/// when no keyring backend is available.
pub struct Secrets {
    fallback_path: PathBuf,
    /// When true, use only the fallback file and skip the system keyring, so the
    /// store is fully self-contained (set by [`Secrets::resolve`] when
    /// `OPENPROJECT_SECRETS` is present).
    file_only: bool,
}

impl Secrets {
    pub fn new(fallback_path: PathBuf) -> Self {
        Self {
            fallback_path,
            file_only: false,
        }
    }

    /// Resolve the token store from the environment. When `OPENPROJECT_SECRETS`
    /// is set, that file is the exclusive store (the system keyring is skipped),
    /// so a run — such as a test or a sandboxed invocation — is fully
    /// self-contained and never touches the real keyring. Otherwise the default
    /// fallback path is used with the keyring preferred.
    pub fn resolve() -> Self {
        if let Some(path) = std::env::var_os("OPENPROJECT_SECRETS") {
            return Self {
                fallback_path: PathBuf::from(path),
                file_only: true,
            };
        }
        Self::new(Self::default_fallback_path())
    }

    /// Fallback token file: `$OPENPROJECT_SECRETS` if set, else `secrets.json`
    /// next to the default config. The env override lets a run (e.g. a test or a
    /// sandboxed invocation) point token storage at its own directory, the same
    /// way `OPENPROJECT_CACHE` redirects the cache.
    pub fn default_fallback_path() -> PathBuf {
        if let Some(path) = std::env::var_os("OPENPROJECT_SECRETS") {
            return PathBuf::from(path);
        }
        crate::config::default_config_path()
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("secrets.json")
    }

    pub fn set(&self, profile: &str, token: &str) -> Result<(), Error> {
        if !self.file_only {
            if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, profile) {
                if entry.set_password(token).is_ok() {
                    return Ok(());
                }
            }
        }
        self.file_set(profile, token)
    }

    pub fn get(&self, profile: &str) -> Result<Option<String>, Error> {
        if !self.file_only {
            if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, profile) {
                match entry.get_password() {
                    Ok(pw) => return Ok(Some(pw)),
                    Err(keyring::Error::NoEntry) => {}
                    Err(_) => {}
                }
            }
        }
        self.file_get(profile)
    }

    pub fn delete(&self, profile: &str) -> Result<(), Error> {
        if !self.file_only {
            if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, profile) {
                let _ = entry.delete_credential();
            }
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
        write_private(&self.fallback_path, text.as_bytes())?;
        // Re-assert 0600 to also tighten a file left group/world-readable by an
        // earlier version that wrote before chmod.
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

/// Write `bytes` to `path`, creating the file with 0600 permissions up front so
/// the tokens are never briefly readable by group/other between creation and a
/// later chmod (unix). On other platforms this is a plain write.
#[cfg(unix)]
fn write_private(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)
        .map_err(|e| Error::Io(format!("open secrets: {e}")))?;
    f.write_all(bytes)
        .map_err(|e| Error::Io(format!("write secrets: {e}")))
}

#[cfg(not(unix))]
fn write_private(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    std::fs::write(path, bytes).map_err(|e| Error::Io(format!("write secrets: {e}")))
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

    #[cfg(unix)]
    #[test]
    fn fallback_file_is_created_private() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("secrets.json");
        let s = Secrets::new(path.clone());
        s.file_set("primary", "tok123").unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "secrets file must be owner-only");
    }

    #[cfg(unix)]
    #[test]
    fn fallback_file_tightens_preexisting_loose_perms() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("secrets.json");
        // Simulate a file left world-readable by an earlier version.
        std::fs::write(&path, "{}").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        let s = Secrets::new(path.clone());
        s.file_set("primary", "tok123").unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(
            mode, 0o600,
            "an existing loose secrets file must be tightened"
        );
    }

    #[test]
    fn default_fallback_path_honors_env_override() {
        // nextest runs each test in its own process, so mutating the env is safe.
        let dir = tempfile::tempdir().unwrap();
        let want = dir.path().join("my-secrets.json");
        std::env::set_var("OPENPROJECT_SECRETS", &want);
        assert_eq!(Secrets::default_fallback_path(), want);
        std::env::remove_var("OPENPROJECT_SECRETS");
        // Without the override it falls back next to the config dir.
        assert_eq!(
            Secrets::default_fallback_path().file_name().unwrap(),
            "secrets.json"
        );
    }
}
