use super::error::{invalid_fdx, FdxStorageError, FdxStorageErrorKind};
use scriptcraft_fdx::{self as fdx, write_v6, MAX_SOURCE_BYTES};
use scriptcraft_model::Project;
use std::fmt;

/// A fully serialized and codec-validated Version 6 candidate.
#[derive(Clone, PartialEq, Eq)]
pub struct PreparedFdx {
    pub(super) bytes: Vec<u8>,
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
        let bytes = write_v6(project).map_err(invalid_fdx)?;
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

    pub(super) fn from_validated_bytes(bytes: Vec<u8>) -> Result<Self, FdxStorageError> {
        if bytes.len() > MAX_SOURCE_BYTES {
            return Err(FdxStorageError::new(FdxStorageErrorKind::SourceTooLarge));
        }
        fdx::read_v6(&bytes).map_err(invalid_fdx)?;
        Ok(Self { bytes })
    }
}
