//! Shared filesystem access for native storage and export adapters.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PersistenceError {
    Io(String),
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
