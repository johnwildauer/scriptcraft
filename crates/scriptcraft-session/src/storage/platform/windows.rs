use std::fs::{Metadata, OpenOptions, Permissions};
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};

pub(super) fn configure_read(options: &mut OpenOptions) {
    options.custom_flags(windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT);
}

pub(super) fn is_reparse_point(metadata: &Metadata) -> bool {
    metadata.file_attributes()
        & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT
        != 0
}

pub(super) fn is_device(metadata: &Metadata) -> bool {
    metadata.file_attributes() & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_DEVICE
        != 0
}

pub(super) fn refuses_readonly_replacement(permissions: &Permissions) -> bool {
    permissions.readonly()
}
