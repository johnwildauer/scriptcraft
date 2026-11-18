use super::super::error::{io_error, FdxStorageError, FdxStorageErrorKind, StorageOperation};
use super::super::filesystem::FdxStorageFileSystem;
use super::super::publication::nonempty_parent;
use std::ffi::OsString;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const MAX_TEMP_ATTEMPTS: usize = 128;
static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, Default)]
pub struct StdFdxStorageFileSystem;

impl FdxStorageFileSystem for StdFdxStorageFileSystem {
    type TempFile = std::fs::File;
    type Permissions = std::fs::Permissions;

    fn read_bounded(&self, path: &Path, limit: usize) -> Result<Vec<u8>, FdxStorageError> {
        use std::fs::OpenOptions;

        let mut options = OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        super::unix::configure_read(&mut options)?;
        #[cfg(windows)]
        super::windows::configure_read(&mut options);
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            let _ = &options;
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
        #[cfg(target_os = "windows")]
        {
            if super::windows::is_reparse_point(&metadata) {
                return Err(FdxStorageError::new(
                    FdxStorageErrorKind::SymlinkDestination,
                ));
            }
            if super::windows::is_device(&metadata) {
                return Err(FdxStorageError::new(FdxStorageErrorKind::NotRegularFile));
            }
        }
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
                #[cfg(target_os = "windows")]
                {
                    if super::windows::is_reparse_point(&metadata) {
                        return Err(FdxStorageError::new(
                            FdxStorageErrorKind::SymlinkDestination,
                        ));
                    }
                    if super::windows::is_device(&metadata) {
                        return Err(FdxStorageError::new(FdxStorageErrorKind::NotRegularFile));
                    }
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
            super::unix::configure_temp_permissions(&mut options);
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
        #[cfg(target_os = "windows")]
        if super::windows::refuses_readonly_replacement(permissions) {
            return Err(io_error(StorageOperation::SetPermissions));
        }
        temp.set_permissions(permissions.clone())
            .map_err(|_| io_error(StorageOperation::SetPermissions))
    }

    fn sync_temp(&self, temp: &mut Self::TempFile) -> Result<(), FdxStorageError> {
        temp.sync_all()
            .map_err(|_| io_error(StorageOperation::SyncTemporary))
    }

    fn replace(&self, temp: &Path, destination: &Path) -> Result<(), FdxStorageError> {
        #[cfg(any(unix, target_os = "windows"))]
        {
            std::fs::rename(temp, destination).map_err(|_| io_error(StorageOperation::Replace))
        }
        #[cfg(not(any(unix, target_os = "windows")))]
        {
            let _ = (temp, destination);
            Err(FdxStorageError::new(
                FdxStorageErrorKind::UnsupportedPlatform,
            ))
        }
    }

    fn install_new(&self, temp: &Path, destination: &Path) -> Result<(), FdxStorageError> {
        #[cfg(any(unix, target_os = "windows"))]
        {
            match std::fs::hard_link(temp, destination) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                    Err(FdxStorageError::new(FdxStorageErrorKind::DestinationExists))
                }
                Err(_) => Err(io_error(StorageOperation::CreateDestination)),
            }
        }
        #[cfg(not(any(unix, target_os = "windows")))]
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
