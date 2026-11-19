use crate::editing::EditError;
use scriptcraft_fdx::FdxError;
use std::fmt;

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

#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FdxStorageError {
    pub kind: FdxStorageErrorKind,
    pub temporary_cleanup_failed: bool,
}

impl FdxStorageError {
    pub(super) fn new(kind: FdxStorageErrorKind) -> Self {
        Self {
            kind,
            temporary_cleanup_failed: false,
        }
    }

    pub(super) fn with_cleanup_failure(mut self) -> Self {
        self.temporary_cleanup_failed = true;
        self
    }
}

impl fmt::Display for FdxStorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match &self.kind {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionalSaveError {
    Session(EditError),
    Codec(FdxError),
    Storage(FdxStorageError),
    MissingBaseline,
}

impl fmt::Display for TransactionalSaveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Session(error) => write!(
                formatter,
                "FDX save candidate could not be prepared: {}",
                session_error_category(error)
            ),
            Self::Codec(error) => write!(
                formatter,
                "FDX save candidate could not be serialized or re-read: {error}"
            ),
            Self::Storage(error) => error.fmt(formatter),
            Self::MissingBaseline => {
                formatter.write_str("FDX Save requires an opened or previously saved target")
            }
        }
    }
}

fn session_error_category(error: &EditError) -> &'static str {
    match error {
        EditError::InvalidElement => "invalid element state",
        EditError::InvalidUtf8Boundary => "invalid text boundary",
        EditError::InvalidSelection => "invalid selection",
        EditError::EmptyHistory => "empty edit history",
        EditError::TextTooLong => "text size limit",
        EditError::TooManyElements => "element count limit",
        EditError::UnsupportedTransition { .. } => "unsupported role transition",
        EditError::InlineAltsRoleChange => "inline Alts role constraint",
        EditError::Preservation(_) => "source preservation constraint",
        EditError::InvalidDefinitionCandidate => "invalid definition replacement candidate",
        EditError::CastList(_) => "cast-list reconciliation constraint",
        EditError::FormattedText(_) => "formatted-text constraint",
        EditError::Model(_) => "model validation constraint",
    }
}

impl std::error::Error for TransactionalSaveError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransactionalSaveOutcome {
    /// Save As installation succeeded, but its temporary hard-link cleanup did
    /// not. The destination and live session are nevertheless successfully saved.
    pub temporary_cleanup_warning: bool,
}

pub(super) fn invalid_fdx(error: FdxError) -> FdxStorageError {
    FdxStorageError::new(FdxStorageErrorKind::InvalidFdx(error))
}

pub(super) fn io_error(operation: StorageOperation) -> FdxStorageError {
    FdxStorageError::new(FdxStorageErrorKind::Io(operation))
}
