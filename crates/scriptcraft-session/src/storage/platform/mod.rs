mod standard;

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

pub use standard::StdFdxStorageFileSystem;
