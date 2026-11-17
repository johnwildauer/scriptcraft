use super::error::{invalid_fdx, io_error, FdxStorageError, StorageOperation};
use super::{FdxBaseline, FdxStorageFileSystem, OpenedFdx, OpenedFdxSource};
use scriptcraft_fdx::{self as fdx, MAX_SOURCE_BYTES};
use std::path::Path;

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
