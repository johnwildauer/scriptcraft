//! Filesystem operations for complete, validated FDX Version 6 candidates.
//!
//! This module is separate from the codec and the legacy JSON persistence path.
//! It uses bounded, no-follow reads; uniquely created same-directory temporary
//! files; file flush/sync before publication; and same-directory replacement.
//! On Unix, replacement uses the platform's same-filesystem rename semantics.
//! The standard backend rejects symlink destinations and non-regular files,
//! preserves basic file permissions on replacement, and uses an atomic hard-link
//! creation for Save As so an existing target is never overwritten. ACLs, owner,
//! extended attributes, and directory-entry power-loss durability are not
//! promised. External modifications are checked immediately before replacement;
//! a non-cooperating writer can still race that final comparison and rename.

use crate::fdx::{self, FdxDocument, FdxError, MAX_SOURCE_BYTES};
use crate::Project;
use std::ffi::OsString;
use std::fmt;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const MAX_TEMP_ATTEMPTS: usize = 128;
static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageOperation {
    Read,
    Inspect,
    CreateTemporary,
    WriteTemporary,
    FlushTemporary,
    SetPermissions,
    SyncTemporary,
    ConflictCheck,
    Replace,
    CreateDestination,
    Cleanup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FdxStorageErrorKind {
    InvalidPath,
    UnsupportedPlatform,
    SymlinkDestination,
    NotRegularFile,
    DestinationExists,
    Conflict,
    BaselinePathMismatch,
    SourceTooLarge,
    InvalidFdx(FdxError),
    Io(StorageOperation),
    TemporaryNameExhausted,
}

/// Safe-to-report storage failure; it never embeds document text or paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FdxStorageError {
    pub kind: FdxStorageErrorKind,
    pub temporary_cleanup_failed: bool,
}

impl FdxStorageError {
    fn new(kind: FdxStorageErrorKind) -> Self {
        Self {
            kind,
            temporary_cleanup_failed: false,
        }
    }

    fn with_cleanup_failure(mut self) -> Self {
        self.temporary_cleanup_failed = true;
        self
    }
}

impl fmt::Display for FdxStorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self.kind {
            FdxStorageErrorKind::InvalidPath => "FDX storage path is invalid",
            FdxStorageErrorKind::UnsupportedPlatform => {
                "atomic FDX storage is unsupported on this platform"
            }
            FdxStorageErrorKind::SymlinkDestination => {
                "FDX storage does not follow symlink destinations"
            }
            FdxStorageErrorKind::NotRegularFile => "FDX storage destination is not a regular file",
            FdxStorageErrorKind::DestinationExists => "Save As destination already exists",
            FdxStorageErrorKind::Conflict => "FDX destination changed since it was read or saved",
            FdxStorageErrorKind::BaselinePathMismatch => {
                "FDX baseline belongs to a different destination"
            }
            FdxStorageErrorKind::SourceTooLarge => "FDX source exceeds the configured size limit",
            FdxStorageErrorKind::InvalidFdx(_) => "FDX candidate is invalid or unsupported",
            FdxStorageErrorKind::Io(operation) => match operation {
                StorageOperation::Read => "FDX file read failed",
                StorageOperation::Inspect => "FDX destination inspection failed",
                StorageOperation::CreateTemporary => "FDX temporary file creation failed",
                StorageOperation::WriteTemporary => "FDX temporary file write failed",
                StorageOperation::FlushTemporary => "FDX temporary file flush failed",
                StorageOperation::SetPermissions => "FDX file permissions could not be preserved",
                StorageOperation::SyncTemporary => "FDX temporary file sync failed",
                StorageOperation::ConflictCheck => "FDX destination conflict check failed",
                StorageOperation::Replace => "FDX destination replacement failed",
                StorageOperation::CreateDestination => "FDX Save As creation failed",
                StorageOperation::Cleanup => "FDX temporary file cleanup failed",
            },
            FdxStorageErrorKind::TemporaryNameExhausted => {
                "no unique FDX temporary filename could be allocated"
            }
        };
        formatter.write_str(message)?;
        if self.temporary_cleanup_failed {
            formatter.write_str("; temporary-file cleanup also failed")?;
        }
        Ok(())
    }
}

impl std::error::Error for FdxStorageError {}

/// A fully serialized and codec-validated Version 6 candidate.
#[derive(Clone, PartialEq, Eq)]
pub struct PreparedFdx {
    bytes: Vec<u8>,
}

impl fmt::Debug for PreparedFdx {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedFdx")
            .field("byte_count", &self.bytes.len())
            .finish()
    }
}

impl PreparedFdx {
    pub fn from_project(project: &Project) -> Result<Self, FdxStorageError> {
        let bytes = fdx::write_v6(project).map_err(invalid_fdx)?;
        Self::from_validated_bytes(bytes)
    }

    /// Validate bytes independently before allowing them into the filesystem API.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, FdxStorageError> {
        Self::from_validated_bytes(bytes)
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    fn from_validated_bytes(bytes: Vec<u8>) -> Result<Self, FdxStorageError> {
        if bytes.len() > MAX_SOURCE_BYTES {
            return Err(FdxStorageError::new(FdxStorageErrorKind::SourceTooLarge));
        }
        fdx::read_v6(&bytes).map_err(invalid_fdx)?;
        Ok(Self { bytes })
    }
}

/// Exact bounded source bytes and path observed by [`open`]. Keeping bytes rather
/// than only a timestamp avoids treating coarse or unstable timestamps as identity.
#[derive(Clone, PartialEq, Eq)]
pub struct FdxBaseline {
    path: PathBuf,
    bytes: Vec<u8>,
}

impl fmt::Debug for FdxBaseline {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FdxBaseline")
            .field("path_bound", &true)
            .field("byte_count", &self.bytes.len())
            .finish()
    }
}

impl FdxBaseline {
    pub fn byte_count(&self) -> usize {
        self.bytes.len()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct OpenedFdx {
    pub document: FdxDocument,
    pub baseline: FdxBaseline,
}

/// Bounded source bytes retained for staged version preflight and import. The
/// source baseline is captured before the caller shows a migration prompt, so a
/// later native save can detect a change made while that prompt was open.
#[derive(PartialEq, Eq)]
pub struct OpenedFdxSource {
    pub baseline: FdxBaseline,
}

impl fmt::Debug for OpenedFdxSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OpenedFdxSource")
            .field("byte_count", &self.baseline.bytes.len())
            .field("baseline", &self.baseline)
            .finish()
    }
}

impl OpenedFdxSource {
    /// Returns the bounded source for secure content and version inspection.
    pub fn bytes(&self) -> &[u8] {
        &self.baseline.bytes
    }
}

impl fmt::Debug for OpenedFdx {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OpenedFdx")
            .field("document", &"<validated FDX document>")
            .field("baseline", &self.baseline)
            .finish()
    }
}

/// Evidence returned only after successful publication. Callers can install this
/// baseline into session state after the filesystem operation has succeeded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FdxSaveReceipt {
    pub baseline: FdxBaseline,
    pub temporary_cleanup_failed: bool,
}

/// Narrow platform seam for deterministic failure injection and filesystem tests.
/// Implementations must not follow a symlink at the destination and must make
/// `install_new` fail atomically if the destination already exists.
pub trait FdxStorageFileSystem {
    type TempFile: Write;
    type Permissions: Clone + PartialEq;

    fn read_bounded(&self, path: &Path, limit: usize) -> Result<Vec<u8>, FdxStorageError>;
    fn inspect_destination(
        &self,
        path: &Path,
    ) -> Result<Option<Self::Permissions>, FdxStorageError>;
    fn create_temp(&self, destination: &Path)
        -> Result<(PathBuf, Self::TempFile), FdxStorageError>;
    fn set_temp_permissions(
        &self,
        temp: &mut Self::TempFile,
        permissions: &Self::Permissions,
    ) -> Result<(), FdxStorageError>;
    fn sync_temp(&self, temp: &mut Self::TempFile) -> Result<(), FdxStorageError>;
    fn replace(&self, temp: &Path, destination: &Path) -> Result<(), FdxStorageError>;
    fn install_new(&self, temp: &Path, destination: &Path) -> Result<(), FdxStorageError>;
    fn remove_temp(&self, temp: &Path) -> Result<(), FdxStorageError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct StdFdxStorageFileSystem;

impl FdxStorageFileSystem for StdFdxStorageFileSystem {
    type TempFile = std::fs::File;
    type Permissions = std::fs::Permissions;

    fn read_bounded(&self, path: &Path, limit: usize) -> Result<Vec<u8>, FdxStorageError> {
        use std::fs::OpenOptions;

        let mut options = OpenOptions::new();
        options.read(true);
        #[cfg(target_os = "macos")]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(0x100); // O_NOFOLLOW
        }
        #[cfg(target_os = "linux")]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(0x20000); // O_NOFOLLOW
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            return Err(FdxStorageError::new(
                FdxStorageErrorKind::UnsupportedPlatform,
            ));
        }

        let mut file = options
            .open(path)
            .map_err(|_| io_error(StorageOperation::Read))?;
        let metadata = file
            .metadata()
            .map_err(|_| io_error(StorageOperation::Read))?;
        if !metadata.is_file() {
            return Err(FdxStorageError::new(FdxStorageErrorKind::NotRegularFile));
        }
        if metadata.len() > limit as u64 {
            return Err(FdxStorageError::new(FdxStorageErrorKind::SourceTooLarge));
        }
        let mut bytes = Vec::with_capacity(metadata.len().min(limit as u64) as usize);
        Read::take(&mut file, limit.saturating_add(1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| io_error(StorageOperation::Read))?;
        if bytes.len() > limit {
            return Err(FdxStorageError::new(FdxStorageErrorKind::SourceTooLarge));
        }
        Ok(bytes)
    }

    fn inspect_destination(
        &self,
        path: &Path,
    ) -> Result<Option<Self::Permissions>, FdxStorageError> {
        match std::fs::symlink_metadata(path) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    return Err(FdxStorageError::new(
                        FdxStorageErrorKind::SymlinkDestination,
                    ));
                }
                if !metadata.is_file() {
                    return Err(FdxStorageError::new(FdxStorageErrorKind::NotRegularFile));
                }
                Ok(Some(metadata.permissions()))
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(_) => Err(io_error(StorageOperation::Inspect)),
        }
    }

    fn create_temp(
        &self,
        destination: &Path,
    ) -> Result<(PathBuf, Self::TempFile), FdxStorageError> {
        use std::fs::OpenOptions;
        #[cfg(unix)]
        use std::os::unix::fs::OpenOptionsExt;

        let name = destination
            .file_name()
            .ok_or_else(|| FdxStorageError::new(FdxStorageErrorKind::InvalidPath))?;
        let directory = nonempty_parent(destination);
        for _ in 0..MAX_TEMP_ATTEMPTS {
            let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
            let mut temp_name = OsString::from(".");
            temp_name.push(name);
            temp_name.push(format!(".scriptcraft-{}-{id}.tmp", std::process::id()));
            let temp_path = directory.join(temp_name);
            let mut options = OpenOptions::new();
            options.write(true).read(true).create_new(true);
            #[cfg(unix)]
            options.mode(0o600);
            match options.open(&temp_path) {
                Ok(file) => return Ok((temp_path, file)),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(_) => return Err(io_error(StorageOperation::CreateTemporary)),
            }
        }
        Err(FdxStorageError::new(
            FdxStorageErrorKind::TemporaryNameExhausted,
        ))
    }

    fn set_temp_permissions(
        &self,
        temp: &mut Self::TempFile,
        permissions: &Self::Permissions,
    ) -> Result<(), FdxStorageError> {
        temp.set_permissions(permissions.clone())
            .map_err(|_| io_error(StorageOperation::SetPermissions))
    }

    fn sync_temp(&self, temp: &mut Self::TempFile) -> Result<(), FdxStorageError> {
        temp.sync_all()
            .map_err(|_| io_error(StorageOperation::SyncTemporary))
    }

    fn replace(&self, temp: &Path, destination: &Path) -> Result<(), FdxStorageError> {
        #[cfg(unix)]
        {
            std::fs::rename(temp, destination).map_err(|_| io_error(StorageOperation::Replace))
        }
        #[cfg(not(unix))]
        {
            let _ = (temp, destination);
            Err(FdxStorageError::new(
                FdxStorageErrorKind::UnsupportedPlatform,
            ))
        }
    }

    fn install_new(&self, temp: &Path, destination: &Path) -> Result<(), FdxStorageError> {
        #[cfg(unix)]
        {
            match std::fs::hard_link(temp, destination) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                    Err(FdxStorageError::new(FdxStorageErrorKind::DestinationExists))
                }
                Err(_) => Err(io_error(StorageOperation::CreateDestination)),
            }
        }
        #[cfg(not(unix))]
        {
            let _ = (temp, destination);
            Err(FdxStorageError::new(
                FdxStorageErrorKind::UnsupportedPlatform,
            ))
        }
    }

    fn remove_temp(&self, temp: &Path) -> Result<(), FdxStorageError> {
        std::fs::remove_file(temp).map_err(|_| io_error(StorageOperation::Cleanup))
    }
}

/// Boundedly read and validate an FDX file; the returned baseline is tied to the
/// exact path and bytes used for later conflict detection.
pub fn open<F: FdxStorageFileSystem>(fs: &F, path: &Path) -> Result<OpenedFdx, FdxStorageError> {
    let source = open_source(fs, path)?;
    let document = fdx::read_v6(source.bytes()).map_err(invalid_fdx)?;
    Ok(OpenedFdx {
        document,
        baseline: source.baseline,
    })
}

/// Boundedly read an FDX source without assuming its declared version. The
/// caller can preflight and stage migration before constructing an active
/// document, while retaining the exact original bytes for conflict detection.
pub fn open_source<F: FdxStorageFileSystem>(
    fs: &F,
    path: &Path,
) -> Result<OpenedFdxSource, FdxStorageError> {
    fs.inspect_destination(path)?
        .ok_or_else(|| io_error(StorageOperation::Read))?;
    let bytes = fs.read_bounded(path, MAX_SOURCE_BYTES)?;
    Ok(OpenedFdxSource {
        baseline: FdxBaseline {
            path: path.to_owned(),
            bytes,
        },
    })
}

/// Publish a prepared candidate to an existing file only if the file still
/// exactly matches the baseline obtained from [`open`] (or a prior receipt).
pub fn replace<F: FdxStorageFileSystem>(
    fs: &F,
    destination: &Path,
    candidate: &PreparedFdx,
    baseline: &FdxBaseline,
) -> Result<FdxSaveReceipt, FdxStorageError> {
    if baseline.path != destination {
        return Err(FdxStorageError::new(
            FdxStorageErrorKind::BaselinePathMismatch,
        ));
    }
    let permissions = fs
        .inspect_destination(destination)?
        .ok_or_else(|| FdxStorageError::new(FdxStorageErrorKind::Conflict))?;
    let current = fs.read_bounded(destination, MAX_SOURCE_BYTES)?;
    if current != baseline.bytes {
        return Err(FdxStorageError::new(FdxStorageErrorKind::Conflict));
    }
    publish_temp(fs, destination, candidate, Some((&permissions, baseline)))
}

/// Save As creates a new path and never replaces an existing file or follows a
/// symlink. The source document is outside this operation and remains untouched.
pub fn save_as<F: FdxStorageFileSystem>(
    fs: &F,
    destination: &Path,
    candidate: &PreparedFdx,
) -> Result<FdxSaveReceipt, FdxStorageError> {
    if fs.inspect_destination(destination)?.is_some() {
        return Err(FdxStorageError::new(FdxStorageErrorKind::DestinationExists));
    }
    publish_temp(fs, destination, candidate, None)
}

fn publish_temp<F: FdxStorageFileSystem>(
    fs: &F,
    destination: &Path,
    candidate: &PreparedFdx,
    expected: Option<(&F::Permissions, &FdxBaseline)>,
) -> Result<FdxSaveReceipt, FdxStorageError> {
    let (temp_path, mut temp) = fs.create_temp(destination)?;
    let stage_result = (|| {
        temp.write_all(&candidate.bytes)
            .map_err(|_| io_error(StorageOperation::WriteTemporary))?;
        temp.flush()
            .map_err(|_| io_error(StorageOperation::FlushTemporary))?;
        if let Some((permissions, _)) = expected {
            fs.set_temp_permissions(&mut temp, permissions)?;
        }
        fs.sync_temp(&mut temp)?;
        Ok::<(), FdxStorageError>(())
    })();
    drop(temp);
    if let Err(error) = stage_result {
        return Err(cleanup_after(fs, &temp_path, error));
    }

    let check_result = (|| match expected {
        Some((permissions, baseline)) => {
            if fs.inspect_destination(destination)?.as_ref() != Some(permissions)
                || fs.read_bounded(destination, MAX_SOURCE_BYTES)? != baseline.bytes
            {
                Err(FdxStorageError::new(FdxStorageErrorKind::Conflict))
            } else {
                Ok(())
            }
        }
        None => match fs.inspect_destination(destination)? {
            Some(_) => Err(FdxStorageError::new(FdxStorageErrorKind::DestinationExists)),
            None => Ok(()),
        },
    })();
    if let Err(error) = check_result {
        return Err(cleanup_after(fs, &temp_path, error));
    }

    let install_result = match expected {
        Some(_) => fs.replace(&temp_path, destination),
        None => fs.install_new(&temp_path, destination),
    };
    if let Err(error) = install_result {
        return Err(cleanup_after(fs, &temp_path, error));
    }

    let cleanup_failed = if expected.is_none() {
        fs.remove_temp(&temp_path).is_err()
    } else {
        false
    };
    Ok(FdxSaveReceipt {
        baseline: FdxBaseline {
            path: destination.to_owned(),
            bytes: candidate.bytes.clone(),
        },
        temporary_cleanup_failed: cleanup_failed,
    })
}

fn cleanup_after<F: FdxStorageFileSystem>(
    fs: &F,
    path: &Path,
    error: FdxStorageError,
) -> FdxStorageError {
    if fs.remove_temp(path).is_err() {
        error.with_cleanup_failure()
    } else {
        error
    }
}

fn invalid_fdx(error: FdxError) -> FdxStorageError {
    FdxStorageError::new(FdxStorageErrorKind::InvalidFdx(error))
}

fn io_error(operation: StorageOperation) -> FdxStorageError {
    FdxStorageError::new(FdxStorageErrorKind::Io(operation))
}

fn nonempty_parent(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Role, ScreenplayElement};
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
    enum Fault {
        #[default]
        None,
        WriteAfter(usize),
        Flush,
        Permissions,
        Sync,
        Replace,
        Cleanup,
    }

    #[derive(Clone, Default)]
    struct MemFs {
        files: Arc<Mutex<BTreeMap<PathBuf, Vec<u8>>>>,
        permissions: Arc<Mutex<BTreeMap<PathBuf, u32>>>,
        fault: Arc<Mutex<Fault>>,
        change_on_sync: Arc<Mutex<SyncChange>>,
        temp_serial: Arc<AtomicU64>,
    }

    type SyncChange = Option<(PathBuf, Vec<u8>)>;

    struct MemTemp {
        fs: MemFs,
        path: PathBuf,
        bytes: Vec<u8>,
    }

    impl Write for MemTemp {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            let fault = *self
                .fs
                .fault
                .lock()
                .map_err(|_| io::Error::other("poisoned"))?;
            if let Fault::WriteAfter(limit) = fault {
                if self.bytes.len() >= limit {
                    return Err(io::Error::other("injected partial write failure"));
                }
                let count = bytes.len().min(limit - self.bytes.len());
                self.bytes.extend_from_slice(&bytes[..count]);
                return Ok(count);
            }
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            if *self
                .fs
                .fault
                .lock()
                .map_err(|_| io::Error::other("poisoned"))?
                == Fault::Flush
            {
                return Err(io::Error::other("injected flush failure"));
            }
            Ok(())
        }
    }

    impl MemFs {
        fn put(&self, path: &Path, bytes: Vec<u8>) {
            if let Ok(mut files) = self.files.lock() {
                files.insert(path.to_owned(), bytes);
            }
            if let Ok(mut permissions) = self.permissions.lock() {
                permissions.insert(path.to_owned(), 0o640);
            }
        }

        fn bytes(&self, path: &Path) -> Option<Vec<u8>> {
            self.files.lock().ok()?.get(path).cloned()
        }

        fn set_fault(&self, fault: Fault) {
            if let Ok(mut current) = self.fault.lock() {
                *current = fault;
            }
        }

        fn change_destination_on_sync(&self, path: &Path, bytes: Vec<u8>) {
            if let Ok(mut change) = self.change_on_sync.lock() {
                *change = Some((path.to_owned(), bytes));
            }
        }
    }

    impl FdxStorageFileSystem for MemFs {
        type TempFile = MemTemp;
        type Permissions = u32;

        fn read_bounded(&self, path: &Path, limit: usize) -> Result<Vec<u8>, FdxStorageError> {
            let bytes = self
                .files
                .lock()
                .map_err(|_| io_error(StorageOperation::Read))?
                .get(path)
                .cloned()
                .ok_or_else(|| io_error(StorageOperation::Read))?;
            if bytes.len() > limit {
                return Err(FdxStorageError::new(FdxStorageErrorKind::SourceTooLarge));
            }
            Ok(bytes)
        }

        fn inspect_destination(
            &self,
            path: &Path,
        ) -> Result<Option<Self::Permissions>, FdxStorageError> {
            if !self
                .files
                .lock()
                .map_err(|_| io_error(StorageOperation::Inspect))?
                .contains_key(path)
            {
                return Ok(None);
            }
            Ok(Some(
                *self
                    .permissions
                    .lock()
                    .map_err(|_| io_error(StorageOperation::Inspect))?
                    .get(path)
                    .unwrap_or(&0o600),
            ))
        }

        fn create_temp(
            &self,
            destination: &Path,
        ) -> Result<(PathBuf, Self::TempFile), FdxStorageError> {
            for _ in 0..MAX_TEMP_ATTEMPTS {
                let serial = self.temp_serial.fetch_add(1, Ordering::Relaxed);
                let path =
                    nonempty_parent(destination).join(format!(".scriptcraft-test-{serial}.tmp"));
                let mut files = self
                    .files
                    .lock()
                    .map_err(|_| io_error(StorageOperation::CreateTemporary))?;
                if files.contains_key(&path) {
                    continue;
                }
                files.insert(path.clone(), Vec::new());
                drop(files);
                return Ok((
                    path.clone(),
                    MemTemp {
                        fs: self.clone(),
                        path,
                        bytes: Vec::new(),
                    },
                ));
            }
            Err(FdxStorageError::new(
                FdxStorageErrorKind::TemporaryNameExhausted,
            ))
        }

        fn set_temp_permissions(
            &self,
            _temp: &mut Self::TempFile,
            _permissions: &Self::Permissions,
        ) -> Result<(), FdxStorageError> {
            if *self
                .fault
                .lock()
                .map_err(|_| io_error(StorageOperation::SetPermissions))?
                == Fault::Permissions
            {
                Err(io_error(StorageOperation::SetPermissions))
            } else {
                Ok(())
            }
        }

        fn sync_temp(&self, temp: &mut Self::TempFile) -> Result<(), FdxStorageError> {
            if *self
                .fault
                .lock()
                .map_err(|_| io_error(StorageOperation::SyncTemporary))?
                == Fault::Sync
            {
                return Err(io_error(StorageOperation::SyncTemporary));
            }
            self.files
                .lock()
                .map_err(|_| io_error(StorageOperation::SyncTemporary))?
                .insert(temp.path.clone(), temp.bytes.clone());
            if let Some((path, bytes)) = self
                .change_on_sync
                .lock()
                .map_err(|_| io_error(StorageOperation::SyncTemporary))?
                .take()
            {
                self.files
                    .lock()
                    .map_err(|_| io_error(StorageOperation::SyncTemporary))?
                    .insert(path, bytes);
            }
            Ok(())
        }

        fn replace(&self, temp: &Path, destination: &Path) -> Result<(), FdxStorageError> {
            if *self
                .fault
                .lock()
                .map_err(|_| io_error(StorageOperation::Replace))?
                == Fault::Replace
            {
                return Err(io_error(StorageOperation::Replace));
            }
            let mut files = self
                .files
                .lock()
                .map_err(|_| io_error(StorageOperation::Replace))?;
            let bytes = files
                .remove(temp)
                .ok_or_else(|| io_error(StorageOperation::Replace))?;
            files.insert(destination.to_owned(), bytes);
            let mut permissions = self
                .permissions
                .lock()
                .map_err(|_| io_error(StorageOperation::Replace))?;
            let mode = permissions.remove(temp).unwrap_or(0o600);
            permissions.insert(destination.to_owned(), mode);
            Ok(())
        }

        fn install_new(&self, temp: &Path, destination: &Path) -> Result<(), FdxStorageError> {
            let mut files = self
                .files
                .lock()
                .map_err(|_| io_error(StorageOperation::CreateDestination))?;
            if files.contains_key(destination) {
                return Err(FdxStorageError::new(FdxStorageErrorKind::DestinationExists));
            }
            let bytes = files
                .get(temp)
                .cloned()
                .ok_or_else(|| io_error(StorageOperation::CreateDestination))?;
            files.insert(destination.to_owned(), bytes);
            Ok(())
        }

        fn remove_temp(&self, temp: &Path) -> Result<(), FdxStorageError> {
            if *self
                .fault
                .lock()
                .map_err(|_| io_error(StorageOperation::Cleanup))?
                == Fault::Cleanup
            {
                return Err(io_error(StorageOperation::Cleanup));
            }
            self.files
                .lock()
                .map_err(|_| io_error(StorageOperation::Cleanup))?
                .remove(temp);
            self.permissions
                .lock()
                .map_err(|_| io_error(StorageOperation::Cleanup))?
                .remove(temp);
            Ok(())
        }
    }

    fn candidate(text: &str) -> PreparedFdx {
        let mut project = Project::new().unwrap_or_else(|_| unreachable!());
        project.elements = vec![ScreenplayElement::new("scene-1", Role::SceneHeading, text)
            .unwrap_or_else(|_| unreachable!())];
        PreparedFdx::from_project(&project).unwrap_or_else(|_| unreachable!())
    }

    #[test]
    fn replace_detects_external_changes_and_success_returns_new_baseline() {
        let fs = MemFs::default();
        let path = Path::new("/mem/story.fdx");
        let original = candidate("INT. OLD - DAY");
        fs.put(path, original.bytes.clone());
        let opened = open(&fs, path).unwrap_or_else(|_| unreachable!());
        let updated = candidate("INT. NEW - DAY");

        fs.put(path, candidate("INT. OTHER - DAY").bytes);
        assert_eq!(
            replace(&fs, path, &updated, &opened.baseline).map(|_| ()),
            Err(FdxStorageError::new(FdxStorageErrorKind::Conflict))
        );
        let externally_modified = fs.bytes(path);
        assert_eq!(
            externally_modified,
            Some(candidate("INT. OTHER - DAY").bytes)
        );

        fs.put(path, original.bytes);
        let reopened = open(&fs, path).unwrap_or_else(|_| unreachable!());
        let receipt =
            replace(&fs, path, &updated, &reopened.baseline).unwrap_or_else(|_| unreachable!());
        assert_eq!(fs.bytes(path), Some(updated.bytes.clone()));
        assert_eq!(receipt.baseline.bytes, updated.bytes);
        assert_eq!(receipt.baseline.path, path);
    }

    #[test]
    fn change_during_temp_preparation_is_detected_before_replacement() {
        let fs = MemFs::default();
        let path = Path::new("/mem/story.fdx");
        let original = candidate("INT. OLD - DAY");
        let external = candidate("INT. EXTERNAL - DAY");
        fs.put(path, original.bytes);
        let opened = open(&fs, path).unwrap_or_else(|_| unreachable!());
        fs.change_destination_on_sync(path, external.bytes.clone());

        assert_eq!(
            replace(
                &fs,
                path,
                &candidate("INT. CANDIDATE - DAY"),
                &opened.baseline
            )
            .map(|_| ()),
            Err(FdxStorageError::new(FdxStorageErrorKind::Conflict))
        );
        assert_eq!(fs.bytes(path), Some(external.bytes));
    }

    #[test]
    fn save_as_refuses_existing_destination_without_touching_source_or_target() {
        let fs = MemFs::default();
        let source = Path::new("/mem/source.fdx");
        let destination = Path::new("/mem/destination.fdx");
        let source_bytes = candidate("INT. SOURCE - DAY").bytes;
        let destination_bytes = candidate("INT. EXISTING - DAY").bytes;
        fs.put(source, source_bytes.clone());
        fs.put(destination, destination_bytes.clone());

        assert_eq!(
            save_as(&fs, destination, &candidate("INT. NEW - DAY")).map(|_| ()),
            Err(FdxStorageError::new(FdxStorageErrorKind::DestinationExists))
        );
        assert_eq!(fs.bytes(source), Some(source_bytes));
        assert_eq!(fs.bytes(destination), Some(destination_bytes));
    }

    #[test]
    fn partial_write_flush_sync_and_replace_failures_preserve_destination() {
        for fault in [
            Fault::WriteAfter(7),
            Fault::Flush,
            Fault::Permissions,
            Fault::Sync,
            Fault::Replace,
        ] {
            let fs = MemFs::default();
            let path = Path::new("/mem/story.fdx");
            let original = candidate("INT. ORIGINAL - DAY").bytes;
            fs.put(path, original.clone());
            let opened = open(&fs, path).unwrap_or_else(|_| unreachable!());
            fs.set_fault(fault);
            assert!(replace(
                &fs,
                path,
                &candidate("INT. REPLACEMENT - DAY"),
                &opened.baseline
            )
            .is_err());
            assert_eq!(fs.bytes(path), Some(original));
            let files = fs.files.lock().unwrap_or_else(|_| unreachable!());
            assert_eq!(
                files
                    .keys()
                    .filter(|path| {
                        path.file_name()
                            .is_some_and(|name| name.to_string_lossy().ends_with(".tmp"))
                    })
                    .count(),
                0
            );
        }
    }

    #[test]
    fn save_as_uses_no_replace_install_and_returns_cleanup_state() {
        let fs = MemFs::default();
        let destination = Path::new("/mem/new.fdx");
        let output = candidate("INT. NEW - DAY");
        let receipt = save_as(&fs, destination, &output).unwrap_or_else(|_| unreachable!());
        assert_eq!(fs.bytes(destination), Some(output.bytes.clone()));
        assert_eq!(receipt.baseline.bytes, output.bytes);
        assert!(!receipt.temporary_cleanup_failed);
    }

    #[test]
    fn cleanup_failure_is_reported_without_claiming_failed_publication() {
        let fs = MemFs::default();
        let destination = Path::new("/mem/new.fdx");
        fs.set_fault(Fault::Cleanup);
        let output = candidate("INT. NEW - DAY");

        let receipt = save_as(&fs, destination, &output).unwrap_or_else(|_| unreachable!());

        assert!(receipt.temporary_cleanup_failed);
        assert_eq!(fs.bytes(destination), Some(output.bytes));
        assert_eq!(
            fs.files
                .lock()
                .unwrap_or_else(|_| unreachable!())
                .keys()
                .filter(|path| {
                    path.file_name()
                        .is_some_and(|name| name.to_string_lossy().ends_with(".tmp"))
                })
                .count(),
            1
        );
    }

    #[test]
    fn temporary_name_collision_retries_without_truncating_the_colliding_file() {
        let fs = MemFs::default();
        let destination = Path::new("/mem/new.fdx");
        let collision = Path::new("/mem/.scriptcraft-test-0.tmp");
        fs.put(collision, b"unrelated temporary data".to_vec());
        let output = candidate("INT. NEW - DAY");

        save_as(&fs, destination, &output).unwrap_or_else(|_| unreachable!());

        assert_eq!(
            fs.bytes(collision),
            Some(b"unrelated temporary data".to_vec())
        );
        assert_eq!(fs.bytes(destination), Some(output.bytes));
    }

    #[test]
    fn prepared_bytes_must_be_valid_and_bounded_fdx() {
        assert!(matches!(
            PreparedFdx::from_bytes(b"not FDX".to_vec()).map(|_| ()),
            Err(FdxStorageError {
                kind: FdxStorageErrorKind::InvalidFdx(_),
                ..
            })
        ));
        assert_eq!(
            PreparedFdx::from_bytes(vec![b'x'; MAX_SOURCE_BYTES + 1]).map(|_| ()),
            Err(FdxStorageError::new(FdxStorageErrorKind::SourceTooLarge))
        );
    }

    #[cfg(unix)]
    #[test]
    fn standard_backend_rejects_symlink_and_preserves_mode_bits() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let root = std::env::temp_dir().join(format!(
            "scriptcraft-fdx-storage-{}-{}",
            std::process::id(),
            NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).unwrap_or_else(|_| unreachable!());
        let source = root.join("source.fdx");
        let destination = root.join("destination.fdx");
        let symlink_path = root.join("linked.fdx");
        let old = candidate("INT. OLD - DAY");
        let new = candidate("INT. NEW - DAY");
        std::fs::write(&source, &old.bytes).unwrap_or_else(|_| unreachable!());
        std::fs::write(&destination, &old.bytes).unwrap_or_else(|_| unreachable!());
        std::fs::set_permissions(&destination, std::fs::Permissions::from_mode(0o640))
            .unwrap_or_else(|_| unreachable!());
        symlink(&destination, &symlink_path).unwrap_or_else(|_| unreachable!());
        let backend = StdFdxStorageFileSystem;

        assert_eq!(
            open(&backend, &symlink_path).map(|_| ()),
            Err(FdxStorageError::new(
                FdxStorageErrorKind::SymlinkDestination
            ))
        );
        assert_eq!(
            save_as(&backend, &symlink_path, &new).map(|_| ()),
            Err(FdxStorageError::new(
                FdxStorageErrorKind::SymlinkDestination
            ))
        );
        let opened = open(&backend, &destination).unwrap_or_else(|_| unreachable!());
        replace(&backend, &destination, &new, &opened.baseline).unwrap_or_else(|_| unreachable!());
        let mode = std::fs::metadata(&destination)
            .unwrap_or_else(|_| unreachable!())
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o640);
        assert_eq!(std::fs::read(source).unwrap_or_default(), old.bytes);
        std::fs::remove_dir_all(root).unwrap_or_else(|_| unreachable!());
    }
}
