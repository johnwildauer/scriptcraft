use super::error::FdxStorageError;
use std::io::Write;
use std::path::{Path, PathBuf};

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
    fn create_temp(
        &self,
        destination: &Path,
    ) -> Result<(PathBuf, Self::TempFile), FdxStorageError>;
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
