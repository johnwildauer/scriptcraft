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

/// Save keeps the opened target and its conflict baseline; Save As installs a
/// new destination without overwriting an existing file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FdxSaveMode {
    Save,
    SaveAs(PathBuf),
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
    let text_state = fdx::TextState { scaling: 100, selection: session.selection() };
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
    let text_state = fdx::TextState { scaling: 100, selection: session.selection() };
    save_session_with_view_state(
        fs,
        session,
        target,
        mode,
        window_state,
        text_state,
    )
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
        let bytes = fdx::write_v6_for_save(project, context).map_err(TransactionalSaveError::Codec)?;
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
                fdx::save_diagnostic_at(
                    error,
                    fdx::SaveDiagnosticCode::StorageRereadPropagation,
                )
            })
    }
}

fn save_session_with_codec<F: FdxStorageFileSystem, C: SaveCandidateCodec>(
    fs: &F,
    session: &mut EditingSession,
    target: &mut FdxSaveTarget,
    mode: FdxSaveMode,
    codec: &C,
) -> Result<TransactionalSaveOutcome, TransactionalSaveError> {
    let text_state = fdx::TextState { scaling: 100, selection: session.selection() };
    save_session_with_codec_and_view_state(
        fs,
        session,
        target,
        mode,
        codec,
        fdx::WindowState::DEFAULT,
        text_state,
    )
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
        FdxSaveMode::SaveAs(_) => fdx::SaveOperation::SaveAs,
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
    let (destination, existing_baseline) = match mode {
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
            (path, Some(baseline))
        }
        FdxSaveMode::SaveAs(destination) => (destination, None),
    };

    let corrected = session
        .prepare_cast_list_save_candidate()
        .map_err(TransactionalSaveError::Session)?;
    let prepared = codec.serialize(&corrected, &mut context)?;
    let mut rebased_project = codec
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

    same_elements(&expected.title_page, &actual.title_page)
        && same_elements(&expected.elements, &actual.elements)
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
mod tests {
    use super::*;
    use crate::{Role, ScreenplayElement};
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};

    #[test]
    fn transactional_save_error_display_preserves_safe_categories_only() {
        let session = TransactionalSaveError::Session(EditError::InvalidElement).to_string();
        let codec = TransactionalSaveError::Codec(FdxError::UnsupportedConstruct).to_string();
        let storage = TransactionalSaveError::Storage(FdxStorageError::new(
            FdxStorageErrorKind::Io(StorageOperation::Replace),
        ))
        .to_string();
        let missing_baseline = TransactionalSaveError::MissingBaseline.to_string();

        assert_eq!(
            session,
            "FDX save candidate could not be prepared: invalid element state"
        );
        assert_eq!(
            codec,
            "FDX save candidate could not be serialized or re-read: FDX contains a construct that cannot be projected safely"
        );
        assert_eq!(storage, "FDX destination replacement failed");
        assert_eq!(
            missing_baseline,
            "FDX Save requires an opened or previously saved target"
        );

        for detail in [session, codec, storage, missing_baseline] {
            assert!(!detail.contains("EXT. PRIVATE LOCATION"));
            assert!(!detail.contains("screenplay dialogue"));
            assert!(!detail.contains("/Users/"));
        }
    }

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
    enum Fault {
        #[default]
        None,
        CreateTemporary,
        WriteAfter(usize),
        WriteAfterAndCleanup(usize),
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
            if let Fault::WriteAfter(limit) | Fault::WriteAfterAndCleanup(limit) = fault {
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
            if *self
                .fault
                .lock()
                .map_err(|_| io_error(StorageOperation::CreateTemporary))?
                == Fault::CreateTemporary
            {
                return Err(io_error(StorageOperation::CreateTemporary));
            }
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
            let fault = *self
                .fault
                .lock()
                .map_err(|_| io_error(StorageOperation::Cleanup))?;
            if matches!(fault, Fault::Cleanup | Fault::WriteAfterAndCleanup(_)) {
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

    fn save_test_session(source: &[u8]) -> EditingSession {
        let project = fdx::read_v6(source)
            .unwrap_or_else(|_| unreachable!())
            .project;
        EditingSession::new(project).unwrap_or_else(|_| unreachable!())
    }

    fn assert_saved_character_role(project: &Project) {
        assert_eq!(project.elements[0].role, Role::Character);
        assert_eq!(
            project.elements[0]
                .source_provenance
                .as_ref()
                .and_then(|provenance| provenance.foreign_role.as_deref()),
            Some("Character")
        );
        let paragraph = match &project
            .preservation
            .as_ref()
            .unwrap_or_else(|| unreachable!("reopened import retains preservation"))
            .sections[0]
            .children[0]
        {
            crate::preservation::PreservedNode::Element(paragraph) => paragraph,
            _ => unreachable!("first content child is the managed paragraph"),
        };
        assert_eq!(paragraph.source_type.as_deref(), Some("Character"));
        assert_eq!(
            paragraph
                .attributes
                .iter()
                .find(|attribute| attribute.name.local_name == "Type")
                .map(|attribute| attribute.value.as_str()),
            Some("Character")
        );
    }

    fn set_caret(session: &mut EditingSession, element: usize, offset: usize) {
        session
            .set_selection(crate::editing::Selection::caret(crate::editing::Position {
                element,
                offset,
            }))
            .unwrap_or_else(|_| unreachable!());
    }

    #[test]
    fn imported_action_tab_can_be_saved_and_saved_as_before_reopen() {
        let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Action" ID="source-action"><Text>line</Text></Paragraph></Content></FinalDraft>"#;
        let fs = MemFs::default();

        let save_path = Path::new("/mem/imported-role-save.fdx");
        fs.put(save_path, source.to_vec());
        let opened = open_source(&fs, save_path).unwrap_or_else(|_| unreachable!());
        let mut save_target = FdxSaveTarget::from_baseline(opened.baseline);
        let mut session_to_save = save_test_session(source);
        set_caret(&mut session_to_save, 0, "line".len());
        session_to_save.tab().unwrap_or_else(|_| unreachable!());
        assert_eq!(session_to_save.project().elements[0].role, Role::Character);
        save_session(&fs, &mut session_to_save, &mut save_target, FdxSaveMode::Save)
            .unwrap_or_else(|_| unreachable!("synchronized role must Save"));
        let saved = fs.bytes(save_path).unwrap_or_else(|| unreachable!());
        let reopened = fdx::read_v6(&saved)
            .unwrap_or_else(|_| unreachable!("saved Action-to-Character file must reopen"))
            .project;
        assert_saved_character_role(&reopened);

        let copy_source_path = Path::new("/mem/imported-role-copy-source.fdx");
        let copy_path = Path::new("/mem/imported-role-copy.fdx");
        fs.put(copy_source_path, source.to_vec());
        let mut copy_session = save_test_session(source);
        set_caret(&mut copy_session, 0, "line".len());
        copy_session.tab().unwrap_or_else(|_| unreachable!());
        let mut copy_target = FdxSaveTarget::new_unsaved();
        save_session(&fs, &mut copy_session, &mut copy_target, FdxSaveMode::SaveAs(copy_path.to_owned()))
            .unwrap_or_else(|_| unreachable!("synchronized role must Save As"));
        assert_eq!(fs.bytes(copy_source_path), Some(source.to_vec()));
        let copied = fs.bytes(copy_path).unwrap_or_else(|| unreachable!());
        let reopened_copy = fdx::read_v6(&copied)
            .unwrap_or_else(|_| unreachable!("Save As Action-to-Character file must reopen"))
            .project;
        assert_saved_character_role(&reopened_copy);
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
    fn transactional_save_rebases_source_syncs_cast_and_preserves_history() {
        let fs = MemFs::default();
        let path = Path::new("/mem/rebase.fdx");
        let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Scene Heading" ID="scene"><Text>INT. ROOM - DAY</Text></Paragraph><Paragraph Type="Character" ID="cue"><Text>ALICE</Text></Paragraph><Paragraph Type="Cast List" ID="cast"><Text>(STALE)</Text></Paragraph></Content><VendorData keep="yes"/></FinalDraft>"#;
        fs.put(path, source.to_vec());
        let opened = open_source(&fs, path).unwrap_or_else(|_| unreachable!());
        let mut target = FdxSaveTarget::from_baseline(opened.baseline);
        let mut session = save_test_session(source);
        set_caret(&mut session, 1, 5);
        session.insert("X").unwrap_or_else(|_| unreachable!());
        let original_selection = session.selection();
        assert!(session.dirty());
        assert_eq!(session.project().elements[2].text, "(STALE)");

        let outcome = save_session(&fs, &mut session, &mut target, FdxSaveMode::Save)
            .unwrap_or_else(|_| unreachable!());
        assert!(!outcome.temporary_cleanup_warning);
        assert!(!session.dirty());
        assert!(session.can_undo());
        assert_eq!(session.selection(), original_selection);
        assert_eq!(session.project().elements[1].text, "ALICEX");
        assert_eq!(session.project().elements[2].text, "(ALICEX)");
        let saved = fs.bytes(path).unwrap_or_default();
        assert!(std::str::from_utf8(&saved)
            .unwrap_or("")
            .contains("VendorData keep=\"yes\""));
        assert_eq!(
            session
                .project()
                .preservation
                .as_ref()
                .unwrap_or_else(|| unreachable!())
                .source
                .original_bytes,
            saved
        );
        assert_eq!(target.path(), Some(path));
        assert_eq!(
            target.baseline(),
            Some(
                &open_source(&fs, path)
                    .unwrap_or_else(|_| unreachable!())
                    .baseline
            )
        );

        // A second edit/save is based on the just-published preservation source,
        // not the bytes that were present at initial open.
        set_caret(&mut session, 1, 6);
        session.insert("Y").unwrap_or_else(|_| unreachable!());
        save_session(&fs, &mut session, &mut target, FdxSaveMode::Save)
            .unwrap_or_else(|_| unreachable!());
        assert!(!session.dirty());
        assert_eq!(session.project().elements[1].text, "ALICEXY");
        assert_eq!(session.project().elements[2].text, "(ALICEXY)");
        let latest = fs.bytes(path).unwrap_or_default();
        let reopened = fdx::read_v6(&latest).unwrap_or_else(|_| unreachable!());
        assert_eq!(reopened.project.elements[1].text, "ALICEXY");
        assert_eq!(reopened.project.elements[2].text, "(ALICEXY)");
        assert_eq!(
            session
                .project()
                .preservation
                .as_ref()
                .unwrap_or_else(|| unreachable!())
                .source
                .original_bytes,
            latest
        );
        session.undo().unwrap_or_else(|_| unreachable!());
        assert_eq!(session.project().elements[1].text, "ALICEX");
        assert!(session.dirty());
        session.redo().unwrap_or_else(|_| unreachable!());
        assert_eq!(session.project().elements[1].text, "ALICEXY");
        assert!(!session.dirty());
    }

    #[test]
    fn transactional_save_failures_preserve_session_target_and_destination() {
        let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Action" ID="p1"><Text>original</Text></Paragraph></Content><VendorData keep="yes"/></FinalDraft>"#;
        for fault in [
            Fault::CreateTemporary,
            Fault::WriteAfter(7),
            Fault::Flush,
            Fault::Permissions,
            Fault::Sync,
            Fault::Replace,
        ] {
            let fs = MemFs::default();
            let path = Path::new("/mem/rollback.fdx");
            fs.put(path, source.to_vec());
            let opened = open_source(&fs, path).unwrap_or_else(|_| unreachable!());
            let mut target = FdxSaveTarget::from_baseline(opened.baseline);
            let before_target = target.clone();
            let mut session = save_test_session(source);
            set_caret(&mut session, 0, 8);
            session.insert(" edited").unwrap_or_else(|_| unreachable!());
            let before_session = session.clone();
            fs.set_fault(fault);

            let error = save_session(&fs, &mut session, &mut target, FdxSaveMode::Save)
                .err()
                .unwrap_or_else(|| unreachable!());
            assert_eq!(session, before_session, "fault: {fault:?}");
            assert_eq!(target, before_target, "fault: {fault:?}");
            assert_eq!(fs.bytes(path), Some(source.to_vec()), "fault: {fault:?}");
            let diagnostic = error.to_string();
            assert!(!diagnostic.contains("original"));
            assert!(!diagnostic.contains("/mem/rollback.fdx"));
        }

        // A failed pre-publication write can also report its separate cleanup
        // failure without changing the destination or candidate publication.
        let fs = MemFs::default();
        let path = Path::new("/mem/rollback-cleanup.fdx");
        fs.put(path, source.to_vec());
        let opened = open_source(&fs, path).unwrap_or_else(|_| unreachable!());
        let mut target = FdxSaveTarget::from_baseline(opened.baseline);
        let mut session = save_test_session(source);
        set_caret(&mut session, 0, 8);
        session.insert(" edited").unwrap_or_else(|_| unreachable!());
        let before = session.clone();
        fs.set_fault(Fault::WriteAfterAndCleanup(2));
        let error = save_session(&fs, &mut session, &mut target, FdxSaveMode::Save)
            .err()
            .unwrap_or_else(|| unreachable!());
        assert_eq!(session, before);
        assert_eq!(fs.bytes(path), Some(source.to_vec()));
        assert!(matches!(
            error,
            TransactionalSaveError::Storage(FdxStorageError {
                temporary_cleanup_failed: true,
                ..
            })
        ));
    }

    #[test]
    fn candidate_codec_and_reread_failures_publish_nothing() {
        struct InjectedCodec {
            fail_serialization: bool,
            fail_reread: bool,
            mismatch_reread: bool,
        }

        impl SaveCandidateCodec for InjectedCodec {
            fn serialize(
                &self,
                project: &Project,
                _context: &mut fdx::SaveContext,
            ) -> Result<PreparedFdx, TransactionalSaveError> {
                if self.fail_serialization {
                    return Err(TransactionalSaveError::Codec(FdxError::InvalidValue));
                }
                PreparedFdx::from_project(project).map_err(|error| match &error.kind {
                    FdxStorageErrorKind::InvalidFdx(codec_error) => {
                        TransactionalSaveError::Codec(codec_error.clone())
                    }
                    _ => TransactionalSaveError::Storage(error),
                })
            }

            fn reread(&self, bytes: &[u8]) -> Result<Project, FdxError> {
                if self.fail_reread {
                    Err(FdxError::MalformedXml)
                } else {
                    let mut project = fdx::read_v6(bytes)?.project;
                    if self.mismatch_reread {
                        let element = project
                            .elements
                            .first_mut()
                            .ok_or(FdxError::MissingContent)?;
                        element.text.push_str(" unexpected");
                        if let Some(run) = element.runs.first_mut() {
                            run.text.push_str(" unexpected");
                        }
                    }
                    Ok(project)
                }
            }
        }

        let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Action" ID="p1"><Text>original</Text></Paragraph></Content></FinalDraft>"#;
        for codec in [
            InjectedCodec {
                fail_serialization: true,
                fail_reread: false,
                mismatch_reread: false,
            },
            InjectedCodec {
                fail_serialization: false,
                fail_reread: true,
                mismatch_reread: false,
            },
            InjectedCodec {
                fail_serialization: false,
                fail_reread: false,
                mismatch_reread: true,
            },
        ] {
            let fs = MemFs::default();
            let path = PathBuf::from("/mem/preparation.fdx");
            fs.put(&path, source.to_vec());
            let opened = open_source(&fs, &path).unwrap_or_else(|_| unreachable!());
            let mut target = FdxSaveTarget::from_baseline(opened.baseline);
            let before_target = target.clone();
            let mut session = save_test_session(source);
            set_caret(&mut session, 0, 8);
            session.insert(" edited").unwrap_or_else(|_| unreachable!());
            let before_session = session.clone();

            assert!(save_session_with_codec(
                &fs,
                &mut session,
                &mut target,
                FdxSaveMode::Save,
                &codec
            )
            .is_err());
            assert_eq!(session, before_session);
            assert_eq!(target, before_target);
            assert_eq!(fs.bytes(&path), Some(source.to_vec()));
        }

        let formatted_cast = br#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Scene Heading" ID="scene"><Text>INT. ROOM</Text></Paragraph><Paragraph Type="Character" ID="cue"><Text>ALICE</Text></Paragraph><Paragraph Type="Cast List" ID="cast"><Text Style="Bold">(STALE)</Text></Paragraph></Content></FinalDraft>"#;
        let fs = MemFs::default();
        let mut session = save_test_session(formatted_cast);
        let before_session = session.clone();
        let mut target = FdxSaveTarget::new_unsaved();
        let error = save_session(
            &fs,
            &mut session,
            &mut target,
            FdxSaveMode::SaveAs(PathBuf::from("/mem/candidate.fdx")),
        )
        .err()
        .unwrap_or_else(|| unreachable!());
        assert!(matches!(error, TransactionalSaveError::Session(_)));
        assert_eq!(session, before_session);
        assert_eq!(target, FdxSaveTarget::new_unsaved());
        assert!(fs.bytes(Path::new("/mem/candidate.fdx")).is_none());
    }

    #[test]
    fn injected_save_as_failure_does_not_publish_window_text_state_or_lineage_candidate() {
        let source = br#"<FinalDraft DocumentType="Script" Version="6"><DocumentRef DateTime="20261001T120000" id="doc"/><Content><Paragraph Type="Action" ID="p1"><Text>original</Text></Paragraph></Content></FinalDraft>"#;
        let fs = MemFs::default();
        fs.set_fault(Fault::WriteAfter(0));
        let mut session = save_test_session(source);
        let before_session = session.clone();
        let mut target = FdxSaveTarget::new_unsaved();
        let before_target = target.clone();
        let destination = PathBuf::from("/mem/window-lineage-failure.fdx");
        let context = fdx::SaveContext::with_timestamp(
            fdx::SaveOperation::SaveAs,
            "20261103T101112".to_owned(),
        )
        .with_session_open_timestamp("20261001T120000".to_owned())
        .with_test_lineage_ids(vec!["aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa".to_owned()])
        .with_window_state(fdx::WindowState {
            left: -1440,
            top: 95,
            width: 1200,
            height: 900,
        })
        .with_text_state(fdx::TextState {
            scaling: 188,
            selection: session.selection(),
        });

        assert!(save_session_with_context(
            &fs,
            &mut session,
            &mut target,
            FdxSaveMode::SaveAs(destination.clone()),
            context,
            &NativeSaveCandidateCodec,
        )
        .is_err());
        assert_eq!(session, before_session);
        assert_eq!(target, before_target);
        assert!(fs.bytes(&destination).is_none());
    }

    #[test]
    fn transactional_conflict_and_clean_session_are_coherent() {
        let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Action" ID="p1"><Text>original</Text></Paragraph></Content></FinalDraft>"#;

        let fs = MemFs::default();
        let path = Path::new("/mem/conflict-session.fdx");
        fs.put(path, source.to_vec());
        let opened = open_source(&fs, path).unwrap_or_else(|_| unreachable!());
        let mut target = FdxSaveTarget::from_baseline(opened.baseline);
        let before_target = target.clone();
        let mut session = save_test_session(source);
        set_caret(&mut session, 0, 8);
        session
            .insert(" candidate")
            .unwrap_or_else(|_| unreachable!());
        let before_session = session.clone();
        let external = candidate("external replacement").bytes;
        fs.change_destination_on_sync(path, external.clone());

        assert!(matches!(
            save_session(&fs, &mut session, &mut target, FdxSaveMode::Save),
            Err(TransactionalSaveError::Storage(FdxStorageError {
                kind: FdxStorageErrorKind::Conflict,
                ..
            }))
        ));
        assert_eq!(session, before_session);
        assert_eq!(target, before_target);
        assert_eq!(fs.bytes(path), Some(external));

        let clean_fs = MemFs::default();
        let clean_path = Path::new("/mem/clean-session.fdx");
        clean_fs.put(clean_path, source.to_vec());
        let clean_opened = open_source(&clean_fs, clean_path).unwrap_or_else(|_| unreachable!());
        let mut clean_target = FdxSaveTarget::from_baseline(clean_opened.baseline);
        let mut clean_session = save_test_session(source);
        let original_revision = clean_session.revision();
        assert!(!clean_session.dirty());
        save_session(
            &clean_fs,
            &mut clean_session,
            &mut clean_target,
            FdxSaveMode::Save,
        )
        .unwrap_or_else(|_| unreachable!());
        assert!(!clean_session.dirty());
        assert_eq!(
            clean_session.revision(),
            original_revision.saturating_add(1)
        );
        assert_eq!(
            clean_session
                .project()
                .preservation
                .as_ref()
                .unwrap_or_else(|| unreachable!())
                .source
                .original_bytes,
            clean_fs.bytes(clean_path).unwrap_or_default()
        );
    }

    #[test]
    fn save_as_cleanup_warning_is_successfully_published_to_session_and_target() {
        let fs = MemFs::default();
        let destination = PathBuf::from("/mem/new-save-as.fdx");
        let mut session = EditingSession::new(Project::new().unwrap_or_else(|_| unreachable!()))
            .unwrap_or_else(|_| unreachable!());
        session
            .insert("new screenplay")
            .unwrap_or_else(|_| unreachable!());
        let before_session = session.clone();
        let mut target = FdxSaveTarget::new_unsaved();
        fs.set_fault(Fault::Cleanup);

        let result = save_session(
            &fs,
            &mut session,
            &mut target,
            FdxSaveMode::SaveAs(destination.clone()),
        );
        assert_eq!(
            result,
            Ok(TransactionalSaveOutcome {
                temporary_cleanup_warning: true
            })
        );
        assert_eq!(target.path(), Some(destination.as_path()));
        assert_eq!(
            target.baseline().map(FdxBaseline::path),
            Some(destination.as_path())
        );
        assert_eq!(
            fs.bytes(&destination),
            Some(
                session
                    .project()
                    .preservation
                    .as_ref()
                    .unwrap_or_else(|| unreachable!())
                    .source
                    .original_bytes
                    .as_slice()
                    .to_vec()
            )
        );
        assert!(!session.dirty());
        assert_eq!(
            session
                .project()
                .elements
                .iter()
                .map(|element| (element.role, element.text.as_str()))
                .collect::<Vec<_>>(),
            before_session
                .project()
                .elements
                .iter()
                .map(|element| (element.role, element.text.as_str()))
                .collect::<Vec<_>>()
        );
        assert!(session.revision() > before_session.revision());
    }

    #[test]
    fn transactional_save_as_refuses_existing_destination_without_publication() {
        let fs = MemFs::default();
        let source_path = Path::new("/mem/save-as-source.fdx");
        let destination = Path::new("/mem/save-as-existing.fdx");
        let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Action" ID="source"><Text>source</Text></Paragraph></Content></FinalDraft>"#.to_vec();
        let existing = candidate("existing destination").bytes;
        fs.put(source_path, source.clone());
        fs.put(destination, existing.clone());
        let opened = open_source(&fs, source_path).unwrap_or_else(|_| unreachable!());
        let mut target = FdxSaveTarget::from_baseline(opened.baseline);
        let before_target = target.clone();
        let mut session = save_test_session(&source);
        set_caret(&mut session, 0, 6);
        session
            .insert(" changed")
            .unwrap_or_else(|_| unreachable!());
        let before_session = session.clone();

        assert!(matches!(
            save_session(
                &fs,
                &mut session,
                &mut target,
                FdxSaveMode::SaveAs(destination.to_owned())
            ),
            Err(TransactionalSaveError::Storage(FdxStorageError {
                kind: FdxStorageErrorKind::DestinationExists,
                ..
            }))
        ));
        assert_eq!(session, before_session);
        assert_eq!(target, before_target);
        assert_eq!(fs.bytes(source_path), Some(source));
        assert_eq!(fs.bytes(destination), Some(existing));
    }

    #[test]
    fn save_as_does_not_modify_source_and_older_conversion_replaces_only_on_save() {
        let fs = MemFs::default();
        let source_path = Path::new("/mem/older-source.fdx");
        let older = br#"<FinalDraft DocumentType="Script" Version="5"><TitlePage><Paragraph><Text>Legacy title</Text></Paragraph></TitlePage><Content><Paragraph Type="Vendor Paragraph"><Text>legacy body</Text></Paragraph></Content></FinalDraft>"#;
        fs.put(source_path, older.to_vec());
        let opened_source = open_source(&fs, source_path).unwrap_or_else(|_| unreachable!());
        let target = FdxSaveTarget::from_baseline(opened_source.baseline);
        let imported = fdx::import_fdx(older, fdx::FdxMigrationConsent::Continue)
            .unwrap_or_else(|_| unreachable!());
        let fdx::FdxImportOutcome::Candidate(imported) = imported else {
            unreachable!();
        };
        let mut session =
            EditingSession::new(imported.document.project).unwrap_or_else(|_| unreachable!());
        let mut save_as_target = FdxSaveTarget::new_unsaved();
        let destination = PathBuf::from("/mem/converted-copy.fdx");
        save_session(
            &fs,
            &mut session,
            &mut save_as_target,
            FdxSaveMode::SaveAs(destination.clone()),
        )
        .unwrap_or_else(|_| unreachable!());
        assert_eq!(fs.bytes(source_path), Some(older.to_vec()));
        assert!(fs
            .bytes(&destination)
            .unwrap_or_default()
            .windows(b"Version=\"6\"".len())
            .any(|window| window == b"Version=\"6\""));

        // Explicit in-place save is also supported for a migrated source and
        // writes nothing until this save transaction is requested.
        let mut in_place_target = target;
        let migrated = fdx::import_fdx(older, fdx::FdxMigrationConsent::Continue)
            .unwrap_or_else(|_| unreachable!());
        let fdx::FdxImportOutcome::Candidate(migrated) = migrated else {
            unreachable!();
        };
        let mut in_place_session =
            EditingSession::new(migrated.document.project).unwrap_or_else(|_| unreachable!());
        assert_eq!(fs.bytes(source_path), Some(older.to_vec()));
        save_session(
            &fs,
            &mut in_place_session,
            &mut in_place_target,
            FdxSaveMode::Save,
        )
        .unwrap_or_else(|_| unreachable!());
        assert!(fs
            .bytes(source_path)
            .unwrap_or_default()
            .windows(b"Version=\"6\"".len())
            .any(|window| window == b"Version=\"6\""));
    }

    #[test]
    fn transactional_save_and_save_as_reopen_with_established_paragraph_ids() {
        let fs = MemFs::default();
        let source_path = Path::new("/mem/uuid-lifecycle-source.fdx");
        let copy_path = PathBuf::from("/mem/uuid-lifecycle-copy.fdx");
        let source = br#"<FinalDraft DocumentType="Script" Version="6"><TitlePage>
<Paragraph Type="Action" id="title"><Text>TITLE</Text></Paragraph>
</TitlePage><Content>
<Paragraph Type="Scene Heading" id="scene"><Text>INT. ROOM</Text></Paragraph>
<Paragraph Type="Character" id="character"><Text>ALPHA</Text></Paragraph>
</Content></FinalDraft>"#;
        fs.put(source_path, source.to_vec());
        let opened = open(&fs, source_path).unwrap_or_else(|_| unreachable!());
        let mut target = FdxSaveTarget::from_baseline(opened.baseline);
        let mut session = EditingSession::new(opened.document.project)
            .unwrap_or_else(|_| unreachable!());

        save_session(&fs, &mut session, &mut target, FdxSaveMode::Save)
            .unwrap_or_else(|_| unreachable!("identity-preserving Save must publish"));
        let saved = fs
            .bytes(source_path)
            .unwrap_or_else(|| unreachable!("Save destination must exist"));
        let saved_ids = paragraph_ids_for_storage_test(
            &fdx::read_v6(&saved)
                .unwrap_or_else(|_| unreachable!("saved document must reopen"))
                .project,
        );
        assert!(saved_ids.contains("scene"));
        assert!(saved_ids.contains("character"));
        assert!(saved_ids.contains("title"));
        assert_eq!(
            session
                .project()
                .preservation
                .as_ref()
                .map(|state| {
                    state
                        .identities
                        .iter()
                        .filter(|identity| identity.domain == crate::preservation::ForeignIdDomain::Paragraph)
                        .map(|identity| identity.value.clone())
                        .collect::<std::collections::BTreeSet<_>>()
                }),
            Some(saved_ids.clone())
        );

        let original_source = fs.bytes(source_path);
        let mut copy_target = FdxSaveTarget::new_unsaved();
        save_session(
            &fs,
            &mut session,
            &mut copy_target,
            FdxSaveMode::SaveAs(copy_path.clone()),
        )
        .unwrap_or_else(|_| unreachable!("identity-preserving Save As must publish"));
        assert_eq!(fs.bytes(source_path), original_source);
        let copied = fs
            .bytes(&copy_path)
            .unwrap_or_else(|| unreachable!("Save As destination must exist"));
        let copied_project = fdx::read_v6(&copied)
            .unwrap_or_else(|_| unreachable!("Save As output must reopen"))
            .project;
        assert_eq!(paragraph_ids_for_storage_test(&copied_project), saved_ids);
    }

    fn paragraph_ids_for_storage_test(project: &Project) -> std::collections::BTreeSet<String> {
        project
            .preservation
            .as_ref()
            .map(|state| {
                state
                    .identities
                    .iter()
                    .filter(|identity| {
                        identity.domain == crate::preservation::ForeignIdDomain::Paragraph
                    })
                    .map(|identity| identity.value.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    #[test]
    fn empty_managed_paragraph_save_and_save_as_are_atomic_on_injected_failure() {
        let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Action" ID="blank"/></Content></FinalDraft>"#;
        let fs = MemFs::default();
        let source_path = Path::new("/mem/empty-managed-source.fdx");
        let copy_path = Path::new("/mem/empty-managed-copy.fdx");
        let failed_copy_path = Path::new("/mem/empty-managed-failed-copy.fdx");
        fs.put(source_path, source.to_vec());
        let opened = open(&fs, source_path).unwrap_or_else(|_| unreachable!());
        let mut session = EditingSession::new(opened.document.project)
            .unwrap_or_else(|_| unreachable!("childless managed paragraph forms a session"));
        let mut target = FdxSaveTarget::from_baseline(opened.baseline);

        save_session(&fs, &mut session, &mut target, FdxSaveMode::Save)
            .unwrap_or_else(|_| unreachable!("native empty paragraph Save succeeds"));
        let saved = fs
            .bytes(source_path)
            .unwrap_or_else(|| unreachable!("Save publishes source destination"));
        assert!(!std::str::from_utf8(&saved).unwrap_or("").contains("<Text"));
        let reopened = fdx::read_v6(&saved)
            .unwrap_or_else(|_| unreachable!("empty paragraph Save output reopens"))
            .project;
        assert!(reopened.elements[0].text.is_empty());
        assert!(reopened.elements[0].runs.is_empty());

        let mut copy_target = FdxSaveTarget::new_unsaved();
        save_session(
            &fs,
            &mut session,
            &mut copy_target,
            FdxSaveMode::SaveAs(copy_path.to_owned()),
        )
        .unwrap_or_else(|_| unreachable!("native empty paragraph Save As succeeds"));
        let copied = fs
            .bytes(copy_path)
            .unwrap_or_else(|| unreachable!("Save As publishes destination"));
        assert!(!std::str::from_utf8(&copied).unwrap_or("").contains("<Text"));
        save_session(&fs, &mut session, &mut copy_target, FdxSaveMode::Save)
            .unwrap_or_else(|_| unreachable!("second Save after Save As succeeds"));

        let session_before_failure = session.clone();
        let target_before_failure = target.clone();
        let destination_before_failure = fs.bytes(source_path);
        fs.set_fault(Fault::WriteAfter(1));
        assert!(save_session(&fs, &mut session, &mut target, FdxSaveMode::Save).is_err());
        assert_eq!(session, session_before_failure);
        assert_eq!(target, target_before_failure);
        assert_eq!(fs.bytes(source_path), destination_before_failure);

        let mut failed_target = FdxSaveTarget::new_unsaved();
        let failed_target_before = failed_target.clone();
        let session_before_save_as_failure = session.clone();
        fs.set_fault(Fault::WriteAfter(1));
        assert!(save_session(
            &fs,
            &mut session,
            &mut failed_target,
            FdxSaveMode::SaveAs(failed_copy_path.to_owned()),
        )
        .is_err());
        assert_eq!(session, session_before_save_as_failure);
        assert_eq!(failed_target, failed_target_before);
        assert_eq!(fs.bytes(failed_copy_path), None);
    }

    #[test]
    fn multi_run_plain_cast_list_empty_later_run_survives_repeated_save() {
        let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Scene Heading" ID="heading"><Text>INT. ROOM</Text></Paragraph><Paragraph Type="Character" ID="cue"><Text>ALPHA</Text></Paragraph><Paragraph Type="Cast List" ID="cast"><Text>(stale</Text><Text>)</Text></Paragraph></Content></FinalDraft>"#;
        let fs = MemFs::default();
        let path = Path::new("/mem/multi-run-cast-list-empty-run.fdx");
        fs.put(path, source.to_vec());
        let opened = open(&fs, path).unwrap_or_else(|_| unreachable!());
        let mut session = EditingSession::new(opened.document.project)
            .unwrap_or_else(|_| unreachable!("plain multi-run Cast List imports"));
        let mut target = FdxSaveTarget::from_baseline(opened.baseline);

        save_session(&fs, &mut session, &mut target, FdxSaveMode::Save)
            .unwrap_or_else(|_| unreachable!("first corrected Cast List Save succeeds"));
        let first = fs
            .bytes(path)
            .unwrap_or_else(|| unreachable!("first Save publishes candidate"));
        let reopened = fdx::read_v6(&first)
            .unwrap_or_else(|_| unreachable!("corrected Cast List reopens"))
            .project;
        assert!(reopened
            .elements
            .iter()
            .any(|element| element.role == Role::CastList && element.text == "(ALPHA)"));

        save_session(&fs, &mut session, &mut target, FdxSaveMode::Save)
            .unwrap_or_else(|_| unreachable!("second corrected Cast List Save succeeds"));
        let second = fs
            .bytes(path)
            .unwrap_or_else(|| unreachable!("second Save publishes candidate"));
        let reopened_again = fdx::read_v6(&second)
            .unwrap_or_else(|_| unreachable!("second corrected Cast List output reopens"))
            .project;
        assert!(reopened_again
            .elements
            .iter()
            .any(|element| element.role == Role::CastList && element.text == "(ALPHA)"));
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

    #[test]
    fn ordinary_save_initializes_empty_xref_history_once_with_session_open_time() {
        let source = br#"<FinalDraft DocumentType="Script" Version="6"><DocumentRef id="document-original"/><Content><Paragraph Type="Action" id="p1"><Text>lineage</Text></Paragraph></Content></FinalDraft>"#;
        let fs = MemFs::default();
        let source_path = Path::new("/mem/lineage-empty-history.fdx");
        fs.put(source_path, source.to_vec());
        let opened = open(&fs, source_path).unwrap_or_else(|_| unreachable!());
        let session_open_timestamp = "20261103T090807".to_owned();
        let mut session = EditingSession::with_session_open_timestamp_for_test(
            opened.document.project,
            session_open_timestamp.clone(),
        )
        .unwrap_or_else(|_| unreachable!());
        let mut target = FdxSaveTarget::from_baseline(opened.baseline);
        let xref_id = "b8e1f570-3e16-4a90-8d6a-7e7f0d0a51aa".to_owned();
        let first_context = fdx::SaveContext::with_timestamp(
            fdx::SaveOperation::Save,
            "20261103T101112".to_owned(),
        )
        .with_session_open_timestamp(session_open_timestamp.clone())
        .with_test_lineage_ids(vec![xref_id.clone()]);
        save_session_with_context(
            &fs,
            &mut session,
            &mut target,
            FdxSaveMode::Save,
            first_context,
            &NativeSaveCandidateCodec,
        )
        .unwrap_or_else(|_| unreachable!("first Save writes initial XRef history"));
        let first_xml = String::from_utf8(
            fs.bytes(source_path)
                .unwrap_or_else(|| unreachable!("first Save output exists")),
        )
        .unwrap_or_else(|_| unreachable!("writer emits UTF-8"));
        assert!(first_xml.contains("DateTime=\"20261103T101112\" id=\"document-original\""));
        assert!(first_xml.contains(&format!("DateTime=\"{session_open_timestamp}\" id=\"{xref_id}\"")));
        assert_eq!(first_xml.matches("<XRef").count(), 1);

        let second_context = fdx::SaveContext::with_timestamp(
            fdx::SaveOperation::Save,
            "20261103T121314".to_owned(),
        )
        .with_session_open_timestamp(session_open_timestamp.clone());
        save_session_with_context(
            &fs,
            &mut session,
            &mut target,
            FdxSaveMode::Save,
            second_context,
            &NativeSaveCandidateCodec,
        )
        .unwrap_or_else(|_| unreachable!("subsequent Save preserves initialized history"));
        let second_xml = String::from_utf8(
            fs.bytes(source_path)
                .unwrap_or_else(|| unreachable!("second Save output exists")),
        )
        .unwrap_or_else(|_| unreachable!("writer emits UTF-8"));
        assert!(second_xml.contains("DateTime=\"20261103T121314\" id=\"document-original\""));
        assert!(second_xml.contains(&format!("DateTime=\"{session_open_timestamp}\" id=\"{xref_id}\"")));
        assert_eq!(second_xml.matches("<XRef").count(), 1);
        assert_eq!(session.session_open_timestamp(), session_open_timestamp);
    }

    #[test]
    fn lineage_save_context_uses_fixed_clock_and_ids_across_session_rebases() {
        let source = br#"<FinalDraft DocumentType="Script" Version="6"><DocumentRef DateTime="20261001T120000" id="document-original"><XRef DateTime="20260930T120000" id="xref-original"/></DocumentRef><Content><Paragraph Type="Action" id="p1"><Text>lineage</Text></Paragraph></Content></FinalDraft>"#;
        let fs = MemFs::default();
        let source_path = Path::new("/mem/lineage-open.fdx");
        fs.put(source_path, source.to_vec());
        let opened = open(&fs, source_path).unwrap_or_else(|_| unreachable!());
        let mut session = EditingSession::with_session_open_timestamp_for_test(
            opened.document.project,
            "20261103T090807".to_owned(),
        )
        .unwrap_or_else(|_| unreachable!());
        let session_open_timestamp = session.session_open_timestamp().to_owned();
        let mut target = FdxSaveTarget::from_baseline(opened.baseline);
        let first_id = "b8e1f570-3e16-4a90-8d6a-7e7f0d0a51aa".to_owned();
        let second_id = "17ebd8dd-7497-4e6d-99fd-a88fb7a2c620".to_owned();
        let first_destination = PathBuf::from("/mem/lineage-copy-1.fdx");
        let first_context = fdx::SaveContext::with_timestamp(
            fdx::SaveOperation::SaveAs,
            "20261103T101112".to_owned(),
        )
        .with_session_open_timestamp(session_open_timestamp.clone())
        .with_test_lineage_ids(vec![first_id.clone()]);
        save_session_with_context(
            &fs,
            &mut session,
            &mut target,
            FdxSaveMode::SaveAs(first_destination.clone()),
            first_context,
            &NativeSaveCandidateCodec,
        )
        .unwrap_or_else(|_| unreachable!("fixed-context Save As succeeds"));
        let first_bytes = fs
            .bytes(&first_destination)
            .unwrap_or_else(|| unreachable!("first fixed-context output exists"));
        let first_xml = String::from_utf8(first_bytes)
            .unwrap_or_else(|_| unreachable!("writer emits UTF-8"));
        assert!(first_xml.contains("DateTime=\"20261103T101112\" id=\"document-original\""));
        assert!(first_xml.contains(&format!("DateTime=\"{session_open_timestamp}\" id=\"b8e1f570-3e16-4a90-8d6a-7e7f0d0a51aa\"")));
        assert_eq!(session.session_open_timestamp(), session_open_timestamp);

        let second_destination = PathBuf::from("/mem/lineage-copy-2.fdx");
        let second_context = fdx::SaveContext::with_timestamp(
            fdx::SaveOperation::SaveAs,
            "20261103T121314".to_owned(),
        )
        .with_session_open_timestamp(session_open_timestamp.clone())
        .with_test_lineage_ids(vec![second_id.clone()]);
        save_session_with_context(
            &fs,
            &mut session,
            &mut target,
            FdxSaveMode::SaveAs(second_destination.clone()),
            second_context,
            &NativeSaveCandidateCodec,
        )
        .unwrap_or_else(|_| unreachable!("repeated fixed-context Save As succeeds"));
        let second_bytes = fs
            .bytes(&second_destination)
            .unwrap_or_else(|| unreachable!("second fixed-context output exists"));
        let second_xml = String::from_utf8(second_bytes)
            .unwrap_or_else(|_| unreachable!("writer emits UTF-8"));
        assert!(second_xml.contains("DateTime=\"20261103T121314\" id=\"document-original\""));
        assert!(second_xml.contains(&format!("DateTime=\"{session_open_timestamp}\" id=\"b8e1f570-3e16-4a90-8d6a-7e7f0d0a51aa\"")));
        assert!(second_xml.contains(&format!("DateTime=\"{session_open_timestamp}\" id=\"17ebd8dd-7497-4e6d-99fd-a88fb7a2c620\"")));
        assert_eq!(session.session_open_timestamp(), session_open_timestamp);
        assert_ne!(first_id, second_id);

        let reopened = open(&fs, &second_destination).unwrap_or_else(|_| unreachable!());
        let reopened_timestamp = "20261103T151617".to_owned();
        let mut reopened_session = EditingSession::with_session_open_timestamp_for_test(
            reopened.document.project,
            reopened_timestamp.clone(),
        )
        .unwrap_or_else(|_| unreachable!());
        let mut reopened_target = FdxSaveTarget::from_baseline(reopened.baseline);
        let third_id = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa".to_owned();
        let third_destination = PathBuf::from("/mem/lineage-copy-reopened.fdx");
        let third_context = fdx::SaveContext::with_timestamp(
            fdx::SaveOperation::SaveAs,
            "20261103T171819".to_owned(),
        )
        .with_session_open_timestamp(reopened_timestamp.clone())
        .with_test_lineage_ids(vec![third_id.clone()]);
        save_session_with_context(
            &fs,
            &mut reopened_session,
            &mut reopened_target,
            FdxSaveMode::SaveAs(third_destination.clone()),
            third_context,
            &NativeSaveCandidateCodec,
        )
        .unwrap_or_else(|_| unreachable!("reopened session Save As succeeds"));
        let third_xml = String::from_utf8(
            fs.bytes(&third_destination)
                .unwrap_or_else(|| unreachable!("reopened output exists")),
        )
        .unwrap_or_else(|_| unreachable!("writer emits UTF-8"));
        assert!(third_xml.contains(&format!("DateTime=\"{reopened_timestamp}\" id=\"{third_id}\"")));
        assert!(third_xml.contains("DateTime=\"20261103T171819\" id=\"document-original\""));
    }

    #[test]
    fn lineage_publication_failure_keeps_live_tree_anchor_path_and_destination_unchanged() {
        let source = br#"<FinalDraft DocumentType="Script" Version="6"><DocumentRef DateTime="20261001T120000" id="document-original"><XRef DateTime="20260930T120000" id="xref-original"/></DocumentRef><Content><Paragraph Type="Action" id="p1"><Text>lineage</Text></Paragraph></Content></FinalDraft>"#;
        let fs = MemFs::default();
        let source_path = Path::new("/mem/lineage-failure.fdx");
        fs.put(source_path, source.to_vec());
        let opened = open(&fs, source_path).unwrap_or_else(|_| unreachable!());
        let mut session = EditingSession::new(opened.document.project)
            .unwrap_or_else(|_| unreachable!());
        let before_session = session.clone();
        let mut target = FdxSaveTarget::from_baseline(opened.baseline);
        let before_target = target.clone();
        fs.set_fault(Fault::Replace);

        let context = fdx::SaveContext::with_timestamp(
            fdx::SaveOperation::Save,
            "20261103T101112".to_owned(),
        )
        .with_session_open_timestamp(session.session_open_timestamp().to_owned());
        let result = save_session_with_context(
            &fs,
            &mut session,
            &mut target,
            FdxSaveMode::Save,
            context,
            &NativeSaveCandidateCodec,
        );

        assert!(matches!(
            result,
            Err(TransactionalSaveError::Storage(FdxStorageError {
                kind: FdxStorageErrorKind::Io(StorageOperation::Replace),
                ..
            }))
        ));
        assert_eq!(session, before_session);
        assert_eq!(target, before_target);
        assert_eq!(fs.bytes(source_path), Some(source.to_vec()));
    }

    #[test]
    fn lineage_id_allocation_retries_registered_collisions_and_keeps_failures_transactional() {
        let collision = "b8e1f570-3e16-4a90-8d6a-7e7f0d0a51aa".to_owned();
        let allocated = "17ebd8dd-7497-4e6d-99fd-a88fb7a2c620".to_owned();
        let source = format!(
            "<FinalDraft DocumentType=\"Script\" Version=\"6\"><DocumentRef DateTime=\"20261001T120000\" id=\"document-original\"/><Vendor id=\"{collision}\"/><Content><Paragraph Type=\"Action\" id=\"p1\"><Text>lineage</Text></Paragraph></Content></FinalDraft>"
        )
        .into_bytes();
        let fs = MemFs::default();
        let source_path = Path::new("/mem/lineage-collision.fdx");
        fs.put(source_path, source.clone());
        let opened = open(&fs, source_path).unwrap_or_else(|_| unreachable!());
        let mut session = EditingSession::new(opened.document.project)
            .unwrap_or_else(|_| unreachable!());
        let mut target = FdxSaveTarget::from_baseline(opened.baseline);
        let destination = PathBuf::from("/mem/lineage-collision-copy.fdx");
        let context = fdx::SaveContext::with_timestamp(
            fdx::SaveOperation::SaveAs,
            "20261103T101112".to_owned(),
        )
        .with_session_open_timestamp("20261001T120000".to_owned())
        .with_test_lineage_ids(vec![collision.clone(), allocated.clone()]);
        save_session_with_context(
            &fs,
            &mut session,
            &mut target,
            FdxSaveMode::SaveAs(destination.clone()),
            context,
            &NativeSaveCandidateCodec,
        )
        .unwrap_or_else(|_| unreachable!("allocator retries a registered ID collision"));
        let output = fs
            .bytes(&destination)
            .unwrap_or_else(|| unreachable!("collision retry output exists"));
        let output = String::from_utf8(output)
            .unwrap_or_else(|_| unreachable!("writer emits UTF-8"));
        assert!(output.contains(&format!("id=\"{allocated}\"")));
        assert!(!output.contains(&format!("DateTime=\"20261001T120000\" id=\"{collision}\"")));

        let fs = MemFs::default();
        let source_path = Path::new("/mem/lineage-collision-limit.fdx");
        fs.put(source_path, source.clone());
        let opened = open(&fs, source_path).unwrap_or_else(|_| unreachable!());
        let mut session = EditingSession::new(opened.document.project)
            .unwrap_or_else(|_| unreachable!());
        let before_session = session.clone();
        let mut target = FdxSaveTarget::from_baseline(opened.baseline);
        let before_target = target.clone();
        let destination = PathBuf::from("/mem/lineage-collision-limit-copy.fdx");
        let context = fdx::SaveContext::with_timestamp(
            fdx::SaveOperation::SaveAs,
            "20261103T101112".to_owned(),
        )
        .with_session_open_timestamp("20261001T120000".to_owned())
        .with_test_lineage_ids(vec![collision.clone(); 16]);
        let result = save_session_with_context(
            &fs,
            &mut session,
            &mut target,
            FdxSaveMode::SaveAs(destination.clone()),
            context,
            &NativeSaveCandidateCodec,
        );
        assert!(matches!(
            result,
            Err(TransactionalSaveError::Codec(
                FdxError::SaveDiagnosticUnsupportedConstruct(38)
            ))
        ));
        assert_eq!(session, before_session);
        assert_eq!(target, before_target);
        assert_eq!(fs.bytes(source_path), Some(source));
        assert!(fs.bytes(&destination).is_none());
    }

    #[test]
    fn lineage_entropy_failure_does_not_publish_a_new_document() {
        let fs = MemFs::default();
        let mut session = EditingSession::new(Project::new().unwrap_or_else(|_| unreachable!()))
            .unwrap_or_else(|_| unreachable!());
        let before_session = session.clone();
        let mut target = FdxSaveTarget::new_unsaved();
        let destination = PathBuf::from("/mem/lineage-entropy-failure.fdx");
        let context = fdx::SaveContext::with_timestamp(
            fdx::SaveOperation::SaveAs,
            "20261103T101112".to_owned(),
        )
        .with_session_open_timestamp("20261103T090807".to_owned())
        .with_test_lineage_entropy_failure();

        let result = save_session_with_context(
            &fs,
            &mut session,
            &mut target,
            FdxSaveMode::SaveAs(destination.clone()),
            context,
            &NativeSaveCandidateCodec,
        );
        assert!(matches!(
            result,
            Err(TransactionalSaveError::Codec(
                FdxError::SaveDiagnosticUnsupportedConstruct(37)
            ))
        ));
        assert_eq!(session, before_session);
        assert_eq!(target, FdxSaveTarget::new_unsaved());
        assert!(fs.bytes(&destination).is_none());
    }

    #[test]
    fn first_new_document_save_as_uses_separate_session_and_save_timestamps() {
        let fs = MemFs::default();
        let project = Project::new().unwrap_or_else(|_| unreachable!());
        let mut session = EditingSession::with_session_open_timestamp_for_test(
            project,
            "20261103T090807".to_owned(),
        )
        .unwrap_or_else(|_| unreachable!());
        let mut target = FdxSaveTarget::new_unsaved();
        let destination = PathBuf::from("/mem/lineage-new-project.fdx");
        let document_id = "b8e1f570-3e16-4a90-8d6a-7e7f0d0a51aa".to_owned();
        let xref_id = "17ebd8dd-7497-4e6d-99fd-a88fb7a2c620".to_owned();
        let context = fdx::SaveContext::with_timestamp(
            fdx::SaveOperation::SaveAs,
            "20261103T101112".to_owned(),
        )
        .with_session_open_timestamp("20261103T090807".to_owned())
        .with_test_lineage_ids(vec![document_id.clone(), xref_id.clone()]);

        save_session_with_context(
            &fs,
            &mut session,
            &mut target,
            FdxSaveMode::SaveAs(destination.clone()),
            context,
            &NativeSaveCandidateCodec,
        )
        .unwrap_or_else(|_| unreachable!("new fixed-context Save As succeeds"));
        let output = fs
            .bytes(&destination)
            .unwrap_or_else(|| unreachable!("new project Save As output exists"));
        let output = String::from_utf8(output)
            .unwrap_or_else(|_| unreachable!("writer emits UTF-8"));
        assert!(output.contains("<DocumentRef DateTime=\"20261103T101112\" id=\"b8e1f570-3e16-4a90-8d6a-7e7f0d0a51aa\">"));
        assert!(output.contains("<XRef DateTime=\"20261103T090807\" id=\"17ebd8dd-7497-4e6d-99fd-a88fb7a2c620\"/>"));
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
