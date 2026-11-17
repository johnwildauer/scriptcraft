//! Temporary Phase 3 source-compatibility facade for native FDX storage; remove in Phase 4.

pub use scriptcraft_session::storage::{
    open, open_source, replace, save_as, save_as_overwrite, save_session,
    save_session_with_view_state, save_session_with_window_state, FdxBaseline, FdxSaveMode,
    FdxSaveReceipt, FdxSaveTarget, FdxStorageError, FdxStorageErrorKind,
    FdxStorageFileSystem, OpenedFdx, OpenedFdxSource, PreparedFdx,
    StdFdxStorageFileSystem, StorageOperation, TransactionalSaveError,
    TransactionalSaveOutcome,
};
