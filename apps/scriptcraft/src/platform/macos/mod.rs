#[cfg(target_os = "macos")]
pub(crate) mod macos_document_open;
#[cfg(target_os = "macos")]
pub(crate) mod macos_lifecycle;
#[cfg(target_os = "macos")]
pub(crate) mod macos_menu;
#[cfg(target_os = "macos")]
pub(crate) mod macos_menu_swatches;
#[cfg(target_os = "macos")]
pub(crate) mod macos_native_dialogs;
#[cfg(target_os = "macos")]
pub(crate) mod macos_open_documents;
#[cfg(target_os = "macos")]
pub(crate) mod macos_pdf_dialog;
#[cfg(target_os = "macos")]
pub(crate) mod macos_print_dialog;
#[cfg(target_os = "macos")]
pub(crate) mod macos_sf_symbols;
#[cfg(target_os = "macos")]
pub(crate) mod macos_unsaved_changes_prompt;

#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
fn request_visible_menu_item_image(item: &objc2_app_kit::NSMenuItem) {
    use objc2::{msg_send, sel};

    let setter = sel!(setPreferredImageVisibility:);
    // SAFETY: `respondsToSelector:` is an NSObject method supported by every
    // NSMenuItem. The guarded setter is sent only when this runtime implements
    // the macOS 27 API; its NSInteger argument is the SDK-defined Visible value.
    unsafe {
        let responds: bool = msg_send![item, respondsToSelector: setter];
        if responds {
            let _: () = msg_send![item, setPreferredImageVisibility: 1isize];
        }
    }
}
