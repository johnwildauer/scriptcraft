use scriptcraft_fdx::FdxDocument;
use std::fmt;
use std::path::{Path, PathBuf};

/// Exact bounded source bytes and path observed by [`open`]. Keeping bytes rather
/// than only a timestamp avoids treating coarse or unstable timestamps as identity.
#[derive(Clone, PartialEq, Eq)]
pub struct FdxBaseline {
    pub(super) path: PathBuf,
    pub(super) bytes: Vec<u8>,
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
    pub(super) path: Option<PathBuf>,
    pub(super) baseline: Option<FdxBaseline>,
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
