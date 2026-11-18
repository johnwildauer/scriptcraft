use super::super::error::{FdxStorageError, FdxStorageErrorKind};
use std::fs::OpenOptions;
use std::os::unix::fs::OpenOptionsExt;

pub(super) fn configure_read(options: &mut OpenOptions) -> Result<(), FdxStorageError> {
    #[cfg(target_os = "macos")]
    {
        options.custom_flags(0x100); // O_NOFOLLOW
        Ok(())
    }
    #[cfg(target_os = "linux")]
    {
        options.custom_flags(0x20000); // O_NOFOLLOW
        Ok(())
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = options;
        Err(FdxStorageError::new(
            FdxStorageErrorKind::UnsupportedPlatform,
        ))
    }
}

pub(super) fn configure_temp_permissions(options: &mut OpenOptions) {
    options.mode(0o600);
}
