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
