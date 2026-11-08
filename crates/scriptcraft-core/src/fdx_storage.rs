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

use crate::editing::{EditError, EditingSession};
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

    pub fn path(&self) -> &Path {
        &self.path
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

/// Runtime destination state for the headless transactional save preparation.
/// It is intentionally not stored in the FDX document model.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct FdxSaveTarget {
    path: Option<PathBuf>,
    baseline: Option<FdxBaseline>,
}

impl fmt::Debug for FdxSaveTarget {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FdxSaveTarget")
            .field("path_bound", &self.path.is_some())
            .field("baseline", &self.baseline)
            .finish()
    }
}

impl FdxSaveTarget {
    pub fn new_unsaved() -> Self {
        Self::default()
    }

    pub fn from_baseline(baseline: FdxBaseline) -> Self {
        Self {
            path: Some(baseline.path.clone()),
            baseline: Some(baseline),
        }
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    pub fn baseline(&self) -> Option<&FdxBaseline> {
        self.baseline.as_ref()
    }
}

/// Save keeps the opened target and its conflict baseline; ordinary Save As is
/// create-only. Save As with explicit replacement consent may atomically replace
/// an existing regular file after capturing and rechecking its bounded baseline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FdxSaveMode {
    Save,
    SaveAs(PathBuf),
    SaveAsOverwrite(PathBuf),
}

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
        EditError::Preservation(_) => "source preservation constraint",
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

/// Prepares the corrected model, serialized bytes, re-read source baseline and
/// publishable session before any destination write. Live session and target
/// state are changed only after storage reports successful replacement/install.
pub fn save_session<F: FdxStorageFileSystem>(
    fs: &F,
    session: &mut EditingSession,
    target: &mut FdxSaveTarget,
    mode: FdxSaveMode,
) -> Result<TransactionalSaveOutcome, TransactionalSaveError> {
    let text_state = fdx::TextState {
        scaling: 100,
        selection: session.selection(),
    };
    save_session_with_view_state(
        fs,
        session,
        target,
        mode,
        fdx::WindowState::DEFAULT,
        text_state,
    )
}

/// Save using the latest desktop-reported logical native geometry. Geometry is
/// save-scoped disposable FDX view state and never enters the editing session.
pub fn save_session_with_window_state<F: FdxStorageFileSystem>(
    fs: &F,
    session: &mut EditingSession,
    target: &mut FdxSaveTarget,
    mode: FdxSaveMode,
    window_state: fdx::WindowState,
) -> Result<TransactionalSaveOutcome, TransactionalSaveError> {
    let text_state = fdx::TextState {
        scaling: 100,
        selection: session.selection(),
    };
    save_session_with_view_state(fs, session, target, mode, window_state, text_state)
}

/// Save with the current editor selection and persisted whole-percent page zoom.
/// Both values are captured in the detached save candidate only.
pub fn save_session_with_view_state<F: FdxStorageFileSystem>(
    fs: &F,
    session: &mut EditingSession,
    target: &mut FdxSaveTarget,
    mode: FdxSaveMode,
    window_state: fdx::WindowState,
    text_state: fdx::TextState,
) -> Result<TransactionalSaveOutcome, TransactionalSaveError> {
    save_session_with_codec_and_view_state(
        fs,
        session,
        target,
        mode,
        &NativeSaveCandidateCodec,
        window_state,
        text_state,
    )
}

trait SaveCandidateCodec {
    fn serialize(
        &self,
        project: &Project,
        context: &mut fdx::SaveContext,
    ) -> Result<PreparedFdx, TransactionalSaveError>;
    fn reread(&self, bytes: &[u8]) -> Result<Project, FdxError>;
}

struct NativeSaveCandidateCodec;

impl SaveCandidateCodec for NativeSaveCandidateCodec {
    fn serialize(
        &self,
        project: &Project,
        context: &mut fdx::SaveContext,
    ) -> Result<PreparedFdx, TransactionalSaveError> {
        let bytes =
            fdx::write_v6_for_save(project, context).map_err(TransactionalSaveError::Codec)?;
        PreparedFdx::from_validated_bytes(bytes).map_err(|error| match &error.kind {
            FdxStorageErrorKind::InvalidFdx(codec_error) => {
                TransactionalSaveError::Codec(codec_error.clone())
            }
            _ => TransactionalSaveError::Storage(error),
        })
    }

    fn reread(&self, bytes: &[u8]) -> Result<Project, FdxError> {
        fdx::read_v6(bytes)
            .map(|document| document.project)
            .map_err(|error| {
                fdx::save_diagnostic_at(error, fdx::SaveDiagnosticCode::StorageRereadPropagation)
            })
    }
}

fn save_session_with_codec_and_view_state<F: FdxStorageFileSystem, C: SaveCandidateCodec>(
    fs: &F,
    session: &mut EditingSession,
    target: &mut FdxSaveTarget,
    mode: FdxSaveMode,
    codec: &C,
    window_state: fdx::WindowState,
    text_state: fdx::TextState,
) -> Result<TransactionalSaveOutcome, TransactionalSaveError> {
    let operation = match &mode {
        FdxSaveMode::Save => fdx::SaveOperation::Save,
        FdxSaveMode::SaveAs(_) | FdxSaveMode::SaveAsOverwrite(_) => {
            fdx::SaveOperation::SaveAs
        }
    };
    let context = fdx::SaveContext::new(operation)
        .with_session_open_timestamp(session.session_open_timestamp().to_owned())
        .with_window_state(window_state);
    let context = context.with_text_state(text_state);
    save_session_with_context(fs, session, target, mode, context, codec)
}

fn save_session_with_context<F: FdxStorageFileSystem, C: SaveCandidateCodec>(
    fs: &F,
    session: &mut EditingSession,
    target: &mut FdxSaveTarget,
    mode: FdxSaveMode,
    mut context: fdx::SaveContext,
    codec: &C,
) -> Result<TransactionalSaveOutcome, TransactionalSaveError> {
    let (destination, existing_baseline, overwrite_consent) = match mode {
        FdxSaveMode::Save => {
            let path = target
                .path
                .clone()
                .ok_or(TransactionalSaveError::MissingBaseline)?;
            let baseline = target
                .baseline
                .as_ref()
                .ok_or(TransactionalSaveError::MissingBaseline)?
                .clone();
            (path, Some(baseline), false)
        }
        FdxSaveMode::SaveAs(destination) => (destination, None, false),
        FdxSaveMode::SaveAsOverwrite(destination) => (destination, None, true),
    };

    let corrected = session
        .prepare_cast_list_save_candidate()
        .map_err(TransactionalSaveError::Session)?;
    let prepared = codec.serialize(&corrected, &mut context)?;
    let rebased_project = codec
        .reread(&prepared.bytes)
        .map_err(TransactionalSaveError::Codec)?;
    if !same_save_projection(&corrected, &rebased_project) {
        return Err(TransactionalSaveError::Codec(
            fdx::SaveDiagnosticCode::StorageSemanticProjectionMismatch.error(),
        ));
    }
    let publishable_session = session
        .prepare_saved_rebase(rebased_project)
        .map_err(TransactionalSaveError::Session)?;

    // The destination and all session work are now prepared. No fallible
    // application transition follows successful filesystem publication.
    let receipt = match existing_baseline.as_ref() {
        Some(baseline) => replace(fs, &destination, &prepared, baseline),
        None if overwrite_consent => save_as_overwrite(fs, &destination, &prepared),
        None => save_as(fs, &destination, &prepared),
    }
    .map_err(TransactionalSaveError::Storage)?;

    let outcome = TransactionalSaveOutcome {
        temporary_cleanup_warning: receipt.temporary_cleanup_failed,
    };
    target.path = Some(destination);
    target.baseline = Some(receipt.baseline);
    *session = publishable_session;
    Ok(outcome)
}

fn same_save_projection(expected: &Project, actual: &Project) -> bool {
    fn same_elements(
        expected: &[crate::ScreenplayElement],
        actual: &[crate::ScreenplayElement],
    ) -> bool {
        expected.len() == actual.len()
        && expected.iter().zip(actual).all(|(expected, actual)| {
            expected.role == actual.role
                && expected.text == actual.text
                && if expected.runs.is_empty() {
                        actual.runs.iter().all(|run| run.attributes.is_empty())
                    } else {
                        expected.runs == actual.runs
                    }
            })
    }

    same_title_paragraphs(&expected.title_page, &actual.title_page)
        && same_elements(&expected.elements, &actual.elements)
}

fn same_title_paragraphs(
    expected: &[crate::TitlePageParagraph],
    actual: &[crate::TitlePageParagraph],
) -> bool {
    expected.len() == actual.len()
        && expected.iter().zip(actual).all(|(expected, actual)| {
            expected.role == actual.role
                && expected.text == actual.text
                && expected.formatting == actual.formatting
                && if expected.runs.is_empty() {
                    actual.runs.iter().all(|run| run.attributes.is_empty())
                } else {
                    expected.runs == actual.runs
                }
        })
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
mod confirmed_save_as_tests {
    use super::*;

    struct FailingCandidateCodec;

    impl SaveCandidateCodec for FailingCandidateCodec {
        fn serialize(
            &self,
            _project: &Project,
            _context: &mut fdx::SaveContext,
        ) -> Result<PreparedFdx, TransactionalSaveError> {
            Err(TransactionalSaveError::Codec(fdx::FdxError::UnsupportedConstruct))
        }

        fn reread(&self, _bytes: &[u8]) -> Result<Project, fdx::FdxError> {
            Err(fdx::FdxError::UnsupportedConstruct)
        }
    }

    struct MismatchedRereadCodec;

    impl SaveCandidateCodec for MismatchedRereadCodec {
        fn serialize(
            &self,
            project: &Project,
            context: &mut fdx::SaveContext,
        ) -> Result<PreparedFdx, TransactionalSaveError> {
            NativeSaveCandidateCodec.serialize(project, context)
        }

        fn reread(&self, bytes: &[u8]) -> Result<Project, fdx::FdxError> {
            let mut project = fdx::read_v6(bytes)?.project;
            if let Some(first) = project.elements.first_mut() {
                first.text.push_str(" injected mismatch");
            }
            Ok(project)
        }
    }

    #[test]
    fn failed_candidate_keeps_existing_destination_and_live_save_state() {
        let directory = std::env::temp_dir().join(format!(
            "scriptcraft-confirmed-save-as-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir(&directory)
            .unwrap_or_else(|_| unreachable!("test directory can be created"));
        let destination = directory.join("existing.fdx");
        std::fs::write(&destination, b"prior destination bytes")
            .unwrap_or_else(|_| unreachable!("test destination can be written"));

        let mut project = Project::new()
            .unwrap_or_else(|_| unreachable!("new project is valid"));
        project.title_page[17].text = "unsaved title page edit".to_owned();
        project.title_page[17].runs[0].text = "unsaved title page edit".to_owned();
        let mut session = EditingSession::new(project)
            .unwrap_or_else(|_| unreachable!("new project session is valid"));
        session
            .insert("unsaved screenplay")
            .unwrap_or_else(|_| unreachable!("test insertion is valid"));
        let before = session.clone();
        let mut target = FdxSaveTarget::new_unsaved();
        let text_state = fdx::TextState {
            scaling: 100,
            selection: session.selection(),
        };

        let error = save_session_with_codec_and_view_state(
            &StdFdxStorageFileSystem,
            &mut session,
            &mut target,
            FdxSaveMode::SaveAsOverwrite(destination.clone()),
            &FailingCandidateCodec,
            fdx::WindowState::DEFAULT,
            text_state,
        )
        .err()
        .unwrap_or_else(|| unreachable!("candidate serialization must fail"));

        assert!(matches!(error, TransactionalSaveError::Codec(_)));
        assert_eq!(
            std::fs::read(&destination).ok().as_deref(),
            Some(&b"prior destination bytes"[..])
        );
        assert_eq!(session, before);
        assert!(session.dirty());
        assert!(target.path().is_none());
        assert!(target.baseline().is_none());

        std::fs::remove_dir_all(directory)
            .unwrap_or_else(|_| unreachable!("test directory can be removed"));
    }

    #[test]
    fn semantic_reread_mismatch_keeps_existing_destination_and_both_surface_state() {
        let directory = std::env::temp_dir().join(format!(
            "scriptcraft-reread-mismatch-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir(&directory)
            .unwrap_or_else(|_| unreachable!("test directory can be created"));
        let destination = directory.join("existing.fdx");
        std::fs::write(&destination, b"prior valid destination")
            .unwrap_or_else(|_| unreachable!("test destination can be written"));

        let mut session = EditingSession::new(
            Project::new().unwrap_or_else(|_| unreachable!("new project is valid")),
        )
        .unwrap_or_else(|_| unreachable!("new project session is valid"));
        session
            .insert("unsaved body")
            .unwrap_or_else(|_| unreachable!("body insertion is valid"));
        session
            .set_title_selection(crate::editing::Selection::caret(
                crate::editing::Position {
                    element: 17,
                    offset: 0,
                },
            ))
            .unwrap_or_else(|_| unreachable!("title caret is valid"));
        session
            .insert_title("unsaved title")
            .unwrap_or_else(|_| unreachable!("title insertion is valid"));
        let before = session.clone();
        let mut target = FdxSaveTarget::new_unsaved();
        let text_state = fdx::TextState {
            scaling: 100,
            selection: session.selection(),
        };

        let error = save_session_with_codec_and_view_state(
            &StdFdxStorageFileSystem,
            &mut session,
            &mut target,
            FdxSaveMode::SaveAsOverwrite(destination.clone()),
            &MismatchedRereadCodec,
            fdx::WindowState::DEFAULT,
            text_state,
        )
        .err()
        .unwrap_or_else(|| unreachable!("semantic reread mismatch must reject candidate"));

        assert!(matches!(error, TransactionalSaveError::Codec(_)));
        assert_eq!(
            std::fs::read(&destination).ok().as_deref(),
            Some(&b"prior valid destination"[..])
        );
        assert_eq!(session, before);
        assert!(session.dirty());
        assert!(target.path().is_none());
        assert!(target.baseline().is_none());

        std::fs::remove_dir_all(directory)
            .unwrap_or_else(|_| unreachable!("test directory can be removed"));
    }
}
