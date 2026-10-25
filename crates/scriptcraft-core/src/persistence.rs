//! Bounded native persistence and explicit recovery candidates.

use crate::{ModelError, Project, MAX_JSON_BYTES};
use std::path::{Path, PathBuf};

pub const MAX_RECOVERY_SNAPSHOTS: usize = 3;
pub const RECOVERY_PREFIX: &str = ".scriptcraft-recovery-";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PersistenceError {
    Io(String),
    Model(ModelError),
    InvalidPath,
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

pub fn serialize(project: &Project) -> Result<Vec<u8>, PersistenceError> {
    let json = project.to_json()?;
    if json.len() > MAX_JSON_BYTES {
        return Err(PersistenceError::Model(ModelError::StringTooLong));
    }
    Ok(json.into_bytes())
}
pub fn deserialize(bytes: &[u8]) -> Result<Project, PersistenceError> {
    if bytes.len() > MAX_JSON_BYTES {
        return Err(PersistenceError::Model(ModelError::StringTooLong));
    }
    let text =
        std::str::from_utf8(bytes).map_err(|_| PersistenceError::Model(ModelError::InvalidJson))?;
    Ok(Project::from_json(text)?)
}

pub fn atomic_save<F: FileSystem>(
    fs: &F,
    destination: &Path,
    project: &Project,
) -> Result<(), PersistenceError> {
    let bytes = serialize(project)?;
    let name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(PersistenceError::InvalidPath)?;
    let temp = destination.with_file_name(format!(".{name}.scriptcraft-tmp"));
    fs.write(&temp, &bytes)?;
    match fs.rename(&temp, destination) {
        Ok(()) => Ok(()),
        Err(error) => {
            let _ = fs.remove(&temp);
            Err(error)
        }
    }
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
    fs: &F,
    directory: &Path,
    project: &Project,
    revision: u64,
) -> Result<PathBuf, PersistenceError> {
    let path = recovery_path(directory, revision);
    fs.write(&path, &serialize(project)?)?;
    let mut paths: Vec<_> = fs
        .list(directory)?
        .into_iter()
        .filter(|path| recovery_revision(path).is_some())
        .collect();
    paths.sort_by_key(|path| recovery_revision(path).unwrap_or(0));
    while paths.len() > MAX_RECOVERY_SNAPSHOTS {
        if let Some(path) = paths.first().cloned() {
            fs.remove(&path)?;
            paths.remove(0);
        }
    }
    Ok(path)
}

pub fn discover_recovery<F: FileSystem>(
    fs: &F,
    directory: &Path,
    saved_revision: u64,
) -> Result<Vec<RecoveryCandidate>, PersistenceError> {
    let mut candidates = Vec::new();
    for path in fs.list(directory)? {
        let Some(revision) = recovery_revision(&path) else {
            continue;
        };
        if revision <= saved_revision {
            continue;
        }
        let Ok(bytes) = fs.read(&path) else { continue };
        let Ok(project) = deserialize(&bytes) else {
            continue;
        };
        candidates.push(RecoveryCandidate {
            path,
            revision,
            project,
        });
    }
    candidates.sort_by_key(|candidate| std::cmp::Reverse(candidate.revision));
    Ok(candidates)
}
fn recovery_revision(path: &Path) -> Option<u64> {
    path.file_name()?
        .to_str()?
        .strip_prefix(RECOVERY_PREFIX)?
        .strip_suffix(".json")?
        .parse()
        .ok()
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
        fail_rename: bool,
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
            if self.fail_rename {
                return Err(PersistenceError::Io("rename".into()));
            }
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
        let mut p = Project::new("p", "Title", "Author").unwrap_or_else(|_| unreachable!());
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
    fn failed_replacement_preserves_destination() {
        let fs = MemFs::default();
        let destination = Path::new("/doc.json");
        atomic_save(&fs, destination, &project("old")).unwrap_or_else(|_| unreachable!());
        let failing = MemFs {
            files: fs.files.clone(),
            fail_rename: true,
        };
        assert!(atomic_save(&failing, destination, &project("new")).is_err());
        assert_eq!(
            deserialize(&fs.read(destination).unwrap_or_default())
                .unwrap_or_else(|_| unreachable!())
                .elements[0]
                .text,
            "old"
        );
    }
    #[test]
    fn recovery_rotates_and_ignores_corruption() {
        let fs = MemFs::default();
        let dir = Path::new("/");
        for revision in 1..=4 {
            save_recovery(&fs, dir, &project(&revision.to_string()), revision)
                .unwrap_or_else(|_| unreachable!());
        }
        fs.files
            .lock()
            .unwrap_or_else(|_| unreachable!())
            .insert(recovery_path(dir, 4), b"bad".to_vec());
        let found = discover_recovery(&fs, dir, 0).unwrap_or_else(|_| unreachable!());
        assert!(found.iter().all(|candidate| candidate.revision != 4));
        assert_eq!(found.len(), 2);
    }
}
