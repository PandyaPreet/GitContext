use crate::model::{AppData, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub fn private_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path).map_err(|_| "Cannot create application directory")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|_| "Cannot secure directory")?;
    }
    Ok(())
}

pub fn read_optional(path: &Path) -> Result<Option<Vec<u8>>> {
    match fs::symlink_metadata(path) {
        Ok(meta) => {
            if meta.file_type().is_symlink() || !meta.is_file() {
                return Err("Refusing configuration symlink or non-regular file".into());
            }
            if meta.len() > 2_097_152 {
                return Err("Configuration file exceeds safe size limit".into());
            }
            fs::read(path)
                .map(Some)
                .map_err(|_| "Cannot read configuration".into())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err("Cannot inspect configuration".into()),
    }
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().ok_or("Missing parent directory")?;
    // Do not follow a destination symlink.
    read_optional(path)?;
    let mut file =
        tempfile::NamedTempFile::new_in(parent).map_err(|_| "Cannot stage configuration")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(path)
            .map(|m| m.permissions().mode() & 0o777)
            .unwrap_or(0o600);
        file.as_file()
            .set_permissions(fs::Permissions::from_mode(mode))
            .map_err(|_| "Cannot set config permissions")?;
    }
    file.write_all(bytes)
        .and_then(|_| file.as_file().sync_all())
        .map_err(|_| "Cannot flush configuration")?;
    file.persist(path)
        .map_err(|_| "Cannot atomically replace configuration")?;
    #[cfg(unix)]
    {
        if let Ok(dir) = fs::File::open(parent) {
            let _ = dir.sync_all();
        }
    }
    Ok(())
}

pub fn load(root: &Path) -> Result<AppData> {
    private_dir(root)?;
    match read_optional(&root.join("state.json"))? {
        None => Ok(AppData::default()),
        Some(bytes) => {
            let mut data: AppData = serde_json::from_slice(&bytes)
                .map_err(|_| "Saved application data is invalid; it has not been overwritten")?;
            if data.schema_version == 1 {
                let backup = root.join("state.v1.backup.json");
                if !backup.exists() {
                    atomic_write(&backup, &bytes)?;
                }
                data.schema_version = 2;
                save(root, &data)?;
            }
            if data.schema_version != 2 {
                return Err("Unsupported data version; update the application".into());
            }
            Ok(data)
        }
    }
}
pub fn save(root: &Path, data: &AppData) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(data).map_err(|_| "Cannot serialize state")?;
    if bytes.len() > 2_097_152 {
        return Err("Application data exceeds its supported size limit".into());
    }
    atomic_write(&root.join("state.json"), &bytes)
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FileChange {
    pub path: PathBuf,
    pub before: Option<Vec<u8>>,
    pub after: Vec<u8>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Journal {
    pub id: String,
    pub files: Vec<FileChange>,
    pub state_before: AppData,
    pub state_after: AppData,
    pub status: String,
}

pub fn save_journal(root: &Path, journal: &Journal) -> Result<()> {
    let dir = root.join("backups");
    private_dir(&dir)?;
    let bytes = serde_json::to_vec(journal).map_err(|_| "Cannot serialize backup")?;
    // Reserve space for status transitions; every saved journal must remain readable.
    if bytes.len() > 2_000_000 {
        return Err("Configuration exceeds the supported rollback backup size; no new configuration writes were performed".into());
    }
    atomic_write(&dir.join(format!("{}.json", journal.id)), &bytes)
}
pub fn restore(change: &FileChange) -> Result<()> {
    match &change.before {
        Some(bytes) => atomic_write(&change.path, bytes),
        None => fs::remove_file(&change.path)
            .map_err(|_| "Cannot remove newly-created config during rollback".into()),
    }
}

// Git respects config.lock. Use the same convention and never overwrite another
// writer's lock. A crash leaves a visible lock requiring manual review.
pub struct ConfigLock {
    path: PathBuf,
}
impl ConfigLock {
    pub fn acquire(path: &Path) -> Result<Self> {
        let mut name = path.as_os_str().to_os_string();
        name.push(".lock");
        let path = PathBuf::from(name);
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        options.open(&path).map_err(|_| "Configuration is locked by another writer or an interrupted operation. Review its .lock file before retrying.")?;
        Ok(Self { path })
    }
}
impl Drop for ConfigLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn migrates_legacy_profiles_with_exact_backup_and_keeps_assignments() {
        let root = tempfile::tempdir().unwrap();
        let original = br#"{"schemaVersion":1,"profiles":[{"id":"p1","name":"Work","githubUser":"alice","gitName":"Alice","gitEmail":"alice@example.com","privateKeyPath":"/tmp/key","publicKeyPath":"/tmp/key.pub","sshAlias":"github-work","organization":"acme","color":"mint"}],"repositories":[{"id":"r1","name":"repo","path":"/tmp/repo","profileId":"p1"}],"activeProfileId":"p1","settings":{"theme":"light","startupView":"tray"}}"#;
        fs::write(root.path().join("state.json"), original).unwrap();
        let migrated = load(root.path()).unwrap();
        assert_eq!(migrated.schema_version, 2);
        assert_eq!(
            migrated.profiles[0].provider,
            crate::provider::GitProvider::Github
        );
        assert_eq!(migrated.profiles[0].username, "alice");
        assert_eq!(migrated.profiles[0].host, "github.com");
        assert_eq!(migrated.profiles[0].ssh_port, 22);
        assert_eq!(migrated.repositories[0].profile_id.as_deref(), Some("p1"));
        assert_eq!(migrated.active_profile_id.as_deref(), Some("p1"));
        assert_eq!(
            fs::read(root.path().join("state.v1.backup.json")).unwrap(),
            original
        );
        load(root.path()).unwrap();
        assert_eq!(
            fs::read(root.path().join("state.v1.backup.json")).unwrap(),
            original
        );
        let legacy: AppData = serde_json::from_slice(original).unwrap();
        let journal = Journal {
            id: "legacy".into(),
            files: vec![],
            state_before: legacy.clone(),
            state_after: legacy,
            status: "applied".into(),
        };
        assert_eq!(
            serde_json::from_slice::<Journal>(&serde_json::to_vec(&journal).unwrap())
                .unwrap()
                .state_before
                .profiles[0]
                .username,
            "alice"
        );
    }
    #[test]
    fn refuses_future_state_without_overwriting_it() {
        let root = tempfile::tempdir().unwrap();
        let data = AppData {
            schema_version: 100,
            ..AppData::default()
        };
        save(root.path(), &data).unwrap();
        let before = fs::read(root.path().join("state.json")).unwrap();
        assert!(load(root.path()).is_err());
        assert_eq!(fs::read(root.path().join("state.json")).unwrap(), before);
    }
}
