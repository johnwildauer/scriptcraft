//! Shared FDX XML declaration, root, and encoding constants.

pub(crate) const UTF8_BOM: &[u8] = b"\xef\xbb\xbf";
pub(crate) const FDX_ROOT: &str = "FinalDraft";
pub(crate) const FDX_VERSION: &str = "6";
pub(crate) const XML_DECLARATION_WITHOUT_TRAILING_LF: &str =
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\" ?>";
pub(crate) const XML_DECLARATION: &str =
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\" ?>\n";
