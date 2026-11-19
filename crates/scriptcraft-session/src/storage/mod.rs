//! Filesystem operations for complete, validated FDX Version 6 candidates.
//!
//! This module is separate from the codec and the legacy JSON persistence path.
//! It uses bounded, no-follow reads; uniquely created same-directory temporary
//! files; file flush/sync before publication; and same-directory replacement.
//! On Unix, replacement uses the platform's same-filesystem rename semantics;
//! on Windows, it uses the standard library's replacement rename operation.
//! The standard backend rejects final-component symlink/reparse destinations and
//! non-regular files, preserves basic file permissions on replacement, and uses
//! atomic hard-link creation for Save As so an existing target is never
//! overwritten. On Windows, `Permissions` copies the standard-library file
//! attributes for replaceable targets, but does not preserve Windows ACLs or
//! owner. Read-only targets fail closed before publication: the verified native
//! rename probe did not replace a read-only destination when the staged file
//! also carried its read-only attribute; this avoids silently dropping the
//! attribute or attempting a delete-then-rename fallback. ACLs, owner, extended
//! attributes, and directory-entry power-loss durability are not promised.
//! External modifications are checked immediately before replacement; a
//! non-cooperating writer can still race that final comparison and rename.

mod candidate;
mod error;
mod filesystem;
mod native_session;
mod open;
mod platform;
mod publication;
mod save;
mod target;

pub use candidate::PreparedFdx;
pub use error::{
    FdxStorageError, FdxStorageErrorKind, StorageOperation, TransactionalSaveError,
    TransactionalSaveOutcome,
};
pub use filesystem::FdxStorageFileSystem;
pub use open::{open, open_source};
pub use platform::StdFdxStorageFileSystem;
pub use publication::{replace, save_as, save_as_overwrite};
pub use save::{save_session, save_session_with_view_state, save_session_with_window_state};
pub use target::{
    FdxBaseline, FdxSaveMode, FdxSaveReceipt, FdxSaveTarget, OpenedFdx, OpenedFdxSource,
};
