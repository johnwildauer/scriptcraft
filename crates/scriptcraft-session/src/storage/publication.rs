use super::candidate::PreparedFdx;
use super::error::{io_error, FdxStorageError, FdxStorageErrorKind, StorageOperation};
use super::filesystem::FdxStorageFileSystem;
use super::target::{FdxBaseline, FdxSaveReceipt};
use scriptcraft_fdx::MAX_SOURCE_BYTES;
use std::io::Write;
use std::path::{Path, PathBuf};

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

/// Save As with explicit native replacement consent. A pre-existing destination
/// must be a readable regular file and is read through the bounded no-follow
/// adapter before using the same conflict-checked atomic replacement path as
/// ordinary Save. If absent, installation remains create-only, so a racing
/// creator cannot be overwritten without a baseline.
pub fn save_as_overwrite<F: FdxStorageFileSystem>(
    fs: &F,
    destination: &Path,
    candidate: &PreparedFdx,
) -> Result<FdxSaveReceipt, FdxStorageError> {
    match fs.inspect_destination(destination)? {
        Some(_) => {
            let baseline = FdxBaseline {
                path: destination.to_owned(),
                bytes: fs.read_bounded(destination, MAX_SOURCE_BYTES)?,
            };
            replace(fs, destination, candidate, &baseline)
        }
        None => save_as(fs, destination, candidate),
    }
}

fn publish_temp<F: FdxStorageFileSystem>(
    fs: &F,
    destination: &Path,
    candidate: &PreparedFdx,
    expected: Option<(&F::Permissions, &FdxBaseline)>,
) -> Result<FdxSaveReceipt, FdxStorageError> {
    // Allocate the receipt's new baseline before publication. Once replacement
    // or no-overwrite installation succeeds, returning the receipt requires
    // only moving already-prepared state (cleanup remains a separate warning).
    let published_baseline = FdxBaseline {
        path: destination.to_owned(),
        bytes: candidate.bytes.clone(),
    };
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
        baseline: published_baseline,
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

pub(super) fn nonempty_parent(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}
