//! Legacy JSON codec compatibility and shared filesystem access.
//!
//! JSON is not a native format. The desktop does not call this module for
//! opening or saving documents. JSON disk saves and snapshot recovery are
//! retired because their payload cannot represent complete FDX preservation
//! state; the retired entry points below fail without touching the filesystem.

use crate::{ModelError, Project, MAX_JSON_BYTES};
use std::path::{Path, PathBuf};

pub const RECOVERY_PREFIX: &str = ".scriptcraft-recovery-";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PersistenceError {
    Io(String),
    Model(ModelError),
    LegacyPersistenceRetired,
    RecoveryUnavailable,
}
impl From<ModelError> for PersistenceError {
    fn from(error: ModelError) -> Self {
        Self::Model(error)
    }
}

pub trait FileSystem {
    fn read(&self, path: &Path) -> Result<Vec<u8>, PersistenceError>;
    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), PersistenceError>;
    fn rename(&self, from: &Path, to: &Path) -> Result<(), PersistenceError>;
    fn remove(&self, path: &Path) -> Result<(), PersistenceError>;
    fn list(&self, directory: &Path) -> Result<Vec<PathBuf>, PersistenceError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct StdFileSystem;
impl FileSystem for StdFileSystem {
    fn read(&self, path: &Path) -> Result<Vec<u8>, PersistenceError> {
        std::fs::read(path).map_err(|e| PersistenceError::Io(e.to_string()))
    }
    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), PersistenceError> {
        std::fs::write(path, bytes).map_err(|e| PersistenceError::Io(e.to_string()))
    }
    fn rename(&self, from: &Path, to: &Path) -> Result<(), PersistenceError> {
        std::fs::rename(from, to).map_err(|e| PersistenceError::Io(e.to_string()))
    }
    fn remove(&self, path: &Path) -> Result<(), PersistenceError> {
        std::fs::remove_file(path).map_err(|e| PersistenceError::Io(e.to_string()))
    }
    fn list(&self, directory: &Path) -> Result<Vec<PathBuf>, PersistenceError> {
        std::fs::read_dir(directory)
            .map_err(|e| PersistenceError::Io(e.to_string()))?
            .map(|entry| {
                entry
                    .map(|e| e.path())
                    .map_err(|e| PersistenceError::Io(e.to_string()))
            })
            .collect()
    }
}

/// Encode the historical JSON shape for compatibility fixtures only.
///
/// This is not supported native persistence and must not be used to save a
/// user document or recovery snapshot.
pub fn serialize(project: &Project) -> Result<Vec<u8>, PersistenceError> {
    let json = project.to_json()?;
    if json.len() > MAX_JSON_BYTES {
        return Err(PersistenceError::Model(ModelError::StringTooLong));
    }
    Ok(json.into_bytes())
}
/// Decode the historical JSON shape for compatibility fixtures only.
///
/// This is not a supported document-open or recovery path. The desktop
/// explicitly rejects legacy JSON files.
pub fn deserialize(bytes: &[u8]) -> Result<Project, PersistenceError> {
    if bytes.len() > MAX_JSON_BYTES {
        return Err(PersistenceError::Model(ModelError::StringTooLong));
    }
    let text =
        std::str::from_utf8(bytes).map_err(|_| PersistenceError::Model(ModelError::InvalidJson))?;
    Ok(Project::from_json(text)?)
}

pub fn atomic_save<F: FileSystem>(
    _fs: &F,
    _destination: &Path,
    _project: &Project,
) -> Result<(), PersistenceError> {
    Err(PersistenceError::LegacyPersistenceRetired)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryCandidate {
    pub path: PathBuf,
    pub revision: u64,
    pub project: Project,
}
pub fn recovery_path(directory: &Path, revision: u64) -> PathBuf {
    directory.join(format!("{RECOVERY_PREFIX}{revision}.json"))
}

pub fn save_recovery<F: FileSystem>(
    _fs: &F,
    _directory: &Path,
    _project: &Project,
    _revision: u64,
) -> Result<PathBuf, PersistenceError> {
    Err(PersistenceError::RecoveryUnavailable)
}

pub fn discover_recovery<F: FileSystem>(
    _fs: &F,
    _directory: &Path,
    _saved_revision: u64,
) -> Result<Vec<RecoveryCandidate>, PersistenceError> {
    Err(PersistenceError::RecoveryUnavailable)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{editing::EditingSession, Project, Role};
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};
    #[derive(Clone, Default)]
    struct MemFs {
        files: Arc<Mutex<BTreeMap<PathBuf, Vec<u8>>>>,
    }
    impl FileSystem for MemFs {
        fn read(&self, path: &Path) -> Result<Vec<u8>, PersistenceError> {
            self.files
                .lock()
                .map_err(|_| PersistenceError::Io("lock".into()))?
                .get(path)
                .cloned()
                .ok_or_else(|| PersistenceError::Io("missing".into()))
        }
        fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), PersistenceError> {
            self.files
                .lock()
                .map_err(|_| PersistenceError::Io("lock".into()))?
                .insert(path.to_owned(), bytes.to_vec());
            Ok(())
        }
        fn rename(&self, from: &Path, to: &Path) -> Result<(), PersistenceError> {
            let mut files = self
                .files
                .lock()
                .map_err(|_| PersistenceError::Io("lock".into()))?;
            let bytes = files
                .remove(from)
                .ok_or_else(|| PersistenceError::Io("missing".into()))?;
            files.insert(to.to_owned(), bytes);
            Ok(())
        }
        fn remove(&self, path: &Path) -> Result<(), PersistenceError> {
            self.files
                .lock()
                .map_err(|_| PersistenceError::Io("lock".into()))?
                .remove(path);
            Ok(())
        }
        fn list(&self, _directory: &Path) -> Result<Vec<PathBuf>, PersistenceError> {
            Ok(self
                .files
                .lock()
                .map_err(|_| PersistenceError::Io("lock".into()))?
                .keys()
                .cloned()
                .collect())
        }
    }
    fn project(text: &str) -> Project {
        let mut p = Project::from_json(include_str!("../tests/fixtures/legacy_project.json"))
            .unwrap_or_else(|_| unreachable!());
        p.elements[0].role = Role::Dialogue;
        p.elements[0].text = text.into();
        p
    }
    #[test]
    fn round_trip_does_not_mutate_session() {
        let session = EditingSession::new(project("hello")).unwrap_or_else(|_| unreachable!());
        let before = session.clone();
        let bytes = serialize(session.project()).unwrap_or_else(|_| unreachable!());
        assert_eq!(
            deserialize(&bytes).unwrap_or_else(|_| unreachable!()),
            *session.project()
        );
        assert_eq!(session, before);
    }
    #[test]
    fn malformed_and_unknown_versions_rejected() {
        assert!(deserialize(b"not json").is_err());
        let json = String::from_utf8(serialize(&project("x")).unwrap_or_default())
            .unwrap_or_default()
            .replacen("\"schema_version\":1", "\"schema_version\":999", 1);
        assert_eq!(
            deserialize(json.as_bytes()),
            Err(PersistenceError::Model(ModelError::InvalidSchemaVersion))
        );
    }
    #[test]
    fn retired_json_save_preserves_existing_destination() {
        let fs = MemFs::default();
        let destination = Path::new("/doc.json");
        fs.write(destination, b"existing user JSON")
            .unwrap_or_else(|_| unreachable!());
        assert_eq!(
            atomic_save(&fs, destination, &project("new")),
            Err(PersistenceError::LegacyPersistenceRetired)
        );
        assert_eq!(fs.read(destination), Ok(b"existing user JSON".to_vec()));
    }
    #[test]
    fn recovery_is_unavailable_without_reading_or_mutating_old_snapshots() {
        let fs = MemFs::default();
        let dir = Path::new("/");
        let old_snapshot = recovery_path(dir, 4);
        fs.files
            .lock()
            .unwrap_or_else(|_| unreachable!())
            .insert(old_snapshot.clone(), b"legacy incomplete snapshot".to_vec());
        let before = fs.files.lock().unwrap_or_else(|_| unreachable!()).clone();

        assert_eq!(
            save_recovery(&fs, dir, &project("new"), 5),
            Err(PersistenceError::RecoveryUnavailable)
        );
        assert_eq!(
            discover_recovery(&fs, dir, 0),
            Err(PersistenceError::RecoveryUnavailable)
        );
        assert_eq!(
            *fs.files.lock().unwrap_or_else(|_| unreachable!()),
            before
        );
        assert_eq!(fs.read(&old_snapshot), Ok(b"legacy incomplete snapshot".to_vec()));
    }
}
