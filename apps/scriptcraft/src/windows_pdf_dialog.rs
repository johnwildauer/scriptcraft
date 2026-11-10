//! Windows common Save-dialog adapter for one-step PDF destination/options selection.
//!
//! The dialog is presented without an owner HWND because this command boundary
//! does not expose a safely borrowed native owner handle. All COM interfaces and
//! their returned path allocation remain alive until destination and checkbox
//! state have been copied into the desktop-local result.

use scriptcraft_commands::PdfExportOptions;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use windows::core::{Interface, PCWSTR, PWSTR, HRESULT};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED,
};
use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
use windows::Win32::UI::Shell::{
    FileSaveDialog, IFileDialogCustomize, IFileSaveDialog, SIGDN_FILESYSPATH,
    FOS_FORCEFILESYSTEM, FOS_NOCHANGEDIR, FOS_OVERWRITEPROMPT, FOS_PATHMUSTEXIST,
    FOS_STRICTFILETYPES,
};

use crate::pdf_export::{PdfExportPickerFailure, PdfExportPickerResult};

const INCLUDE_TITLE_PAGE_CONTROL_ID: u32 = 0x5343;
const PDF_DIALOG_CANCELLED: HRESULT = HRESULT(0x8007_04C7_u32 as i32);

#[derive(Debug, PartialEq, Eq)]
enum DialogResult {
    Accepted {
        destination: Option<PathBuf>,
        include_title_page: bool,
    },
    Cancelled,
    Failed,
}

/// Opens one Windows common Save dialog with the checkbox freshly enabled.
/// The native dialog owns overwrite confirmation; only its selected path and
/// final checkbox state leave this adapter.
#[allow(unsafe_code)]
pub(super) fn show_pdf_save_dialog(default_filename: &str) -> PdfExportPickerResult {
    // SAFETY: This synchronous UI-thread adapter initializes COM as STA before
    // creating or using any Shell dialog interface. S_OK and S_FALSE both
    // require one matching CoUninitialize; every later return is scoped below.
    let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    if initialized.is_err() {
        return PdfExportPickerResult::Error(PdfExportPickerFailure::Unavailable);
    }
    let _apartment = ComApartment;

    let default_filename = wide_null_terminated(default_filename);
    let filter_name = wide_null_terminated("PDF document");
    let filter_spec = wide_null_terminated("*.pdf");
    let extension = wide_null_terminated("pdf");
    let title = wide_null_terminated("Save As PDF");
    let checkbox_label = wide_null_terminated("Include Title Page");
    let filter = COMDLG_FILTERSPEC {
        pszName: PCWSTR(filter_name.as_ptr()),
        pszSpec: PCWSTR(filter_spec.as_ptr()),
    };

    let result = (|| {
        // SAFETY: FileSaveDialog is the system COM class, created in-process
        // inside the initialized STA. The returned interface is owned by its
        // generated COM smart pointer and is dropped before the apartment.
        let dialog: IFileSaveDialog = unsafe {
            CoCreateInstance(&FileSaveDialog, None, CLSCTX_INPROC_SERVER)
        }
        .map_err(|_| DialogResult::Failed)?;

        // SAFETY: Every call targets a live COM interface on its owning STA;
        // all PCWSTR arguments point to NUL-terminated buffers retained through
        // the call, and the filter record points to retained filter buffers.
        unsafe {
            dialog
                .SetTitle(PCWSTR(title.as_ptr()))
                .map_err(|_| DialogResult::Failed)?;
            dialog
                .SetFileTypes(&[filter])
                .map_err(|_| DialogResult::Failed)?;
            dialog
                .SetFileTypeIndex(1)
                .map_err(|_| DialogResult::Failed)?;
            dialog
                .SetDefaultExtension(PCWSTR(extension.as_ptr()))
                .map_err(|_| DialogResult::Failed)?;
            dialog
                .SetFileName(PCWSTR(default_filename.as_ptr()))
                .map_err(|_| DialogResult::Failed)?;
            dialog
                .SetOptions(
                    FOS_FORCEFILESYSTEM
                        | FOS_NOCHANGEDIR
                        | FOS_OVERWRITEPROMPT
                        | FOS_PATHMUSTEXIST
                        | FOS_STRICTFILETYPES,
                )
                .map_err(|_| DialogResult::Failed)?;
        }

        // SAFETY: QueryInterface is performed on the live dialog on its owning
        // STA and the retained customize interface is released before the dialog.
        let customize: IFileDialogCustomize =
            unsafe { dialog.cast() }.map_err(|_| DialogResult::Failed)?;
        let default_include_title_page = PdfExportOptions::default().include_title_page;
        // SAFETY: The control ID is private to this dialog; its UTF-16 label is
        // valid for the call, and both control operations occur before Show.
        unsafe {
            customize
                .AddCheckButton(
                    INCLUDE_TITLE_PAGE_CONTROL_ID,
                    PCWSTR(checkbox_label.as_ptr()),
                    false,
                )
                .map_err(|_| DialogResult::Failed)?;
            customize
                .SetCheckButtonState(
                    INCLUDE_TITLE_PAGE_CONTROL_ID,
                    default_include_title_page,
                )
                .map_err(|_| DialogResult::Failed)?;
        }

        // SAFETY: Show is invoked synchronously with no owner HWND because no
        // safe owner borrow is exposed here. The dialog and custom control stay
        // retained until the result and final checkbox state have been copied.
        if let Err(error) = unsafe { dialog.Show(None) } {
            return if error.code() == PDF_DIALOG_CANCELLED {
                Ok(DialogResult::Cancelled)
            } else {
                Err(DialogResult::Failed)
            };
        }

        // SAFETY: Successful Show leaves a result available on this live dialog.
        let item = unsafe { dialog.GetResult() }.map_err(|_| DialogResult::Failed)?;
        let destination = path_buf_from_shell_item(&item);
        // SAFETY: The checkbox was added before Show and remains owned by this
        // dialog. Read it before releasing either COM interface.
        let checked = unsafe {
            customize
                .GetCheckButtonState(INCLUDE_TITLE_PAGE_CONTROL_ID)
                .map_err(|_| DialogResult::Failed)?
        };

        Ok(DialogResult::Accepted {
            destination,
            include_title_page: checked.0 != 0,
        })
    })();

    match result {
        Ok(result) => picker_result_from_dialog_result(result),
        Err(DialogResult::Failed) => {
            PdfExportPickerResult::Error(PdfExportPickerFailure::Failed)
        }
        Err(DialogResult::Cancelled) => PdfExportPickerResult::Cancel,
        Err(DialogResult::Accepted { .. }) => {
            PdfExportPickerResult::Error(PdfExportPickerFailure::Failed)
        }
    }
}

/// Balances a successful COM apartment initialization, including S_FALSE.
struct ComApartment;

#[allow(unsafe_code)]
impl Drop for ComApartment {
    fn drop(&mut self) {
        // SAFETY: This guard is created exactly once after successful
        // CoInitializeEx on the current thread and drops after all COM objects.
        unsafe { CoUninitialize() };
    }
}

fn wide_null_terminated(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn path_buf_from_wide_path(value: &[u16]) -> Option<PathBuf> {
    if value.is_empty() {
        return None;
    }
    Some(PathBuf::from(OsString::from_wide(value)))
}

/// Owns the Shell-allocated path string until its UTF-16 units are copied.
struct CoTaskMemWide(PWSTR);

#[allow(unsafe_code)]
impl Drop for CoTaskMemWide {
    fn drop(&mut self) {
        // SAFETY: GetDisplayName allocated this pointer with COM task memory;
        // the wrapper owns it uniquely and frees it exactly once.
        unsafe { CoTaskMemFree(Some(self.0.0.cast())) };
    }
}

#[allow(unsafe_code)]
fn path_buf_from_shell_item(item: &windows::Win32::UI::Shell::IShellItem) -> Option<PathBuf> {
    // SAFETY: item is a live shell item returned by this dialog, and the
    // requested filesystem name is a NUL-terminated CoTaskMem allocation.
    let display_name = unsafe { item.GetDisplayName(SIGDN_FILESYSPATH) }.ok()?;
    let allocation = CoTaskMemWide(display_name);
    if allocation.0.0.is_null() {
        return None;
    }

    // SAFETY: IShellItem::GetDisplayName returns a valid NUL-terminated UTF-16
    // string. The allocation guard keeps it alive through the bounded copy.
    let wide = unsafe {
        let mut length = 0usize;
        while *allocation.0.0.add(length) != 0 {
            length += 1;
        }
        std::slice::from_raw_parts(allocation.0.0, length)
    };
    path_buf_from_wide_path(wide)
}

fn picker_result_from_dialog_result(result: DialogResult) -> PdfExportPickerResult {
    match result {
        DialogResult::Cancelled => PdfExportPickerResult::Cancel,
        DialogResult::Failed => PdfExportPickerResult::Error(PdfExportPickerFailure::Failed),
        DialogResult::Accepted {
            destination: Some(destination),
            include_title_page,
        } => PdfExportPickerResult::Accept {
            destination,
            options: PdfExportOptions {
                include_title_page,
            },
        },
        DialogResult::Accepted {
            destination: None, ..
        } => PdfExportPickerResult::Error(PdfExportPickerFailure::Failed),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        path_buf_from_wide_path, picker_result_from_dialog_result, DialogResult,
    };
    use crate::pdf_export::{PdfExportPickerFailure, PdfExportPickerResult};
    use scriptcraft_commands::PdfExportOptions;
    use std::os::windows::ffi::OsStrExt;
    use std::path::PathBuf;

    #[test]
    fn each_accepted_request_returns_destination_and_fresh_checkbox_options_together() {
        let destination = PathBuf::from(r"C:\drafts\Selected output.pdf");
        let first = picker_result_from_dialog_result(DialogResult::Accepted {
            destination: Some(destination.clone()),
            include_title_page: PdfExportOptions::default().include_title_page,
        });
        let toggled_off = picker_result_from_dialog_result(DialogResult::Accepted {
            destination: Some(destination.clone()),
            include_title_page: false,
        });
        let next_request = picker_result_from_dialog_result(DialogResult::Accepted {
            destination: Some(destination.clone()),
            include_title_page: PdfExportOptions::default().include_title_page,
        });

        assert_eq!(
            first,
            PdfExportPickerResult::Accept {
                destination: destination.clone(),
                options: PdfExportOptions {
                    include_title_page: true,
                },
            }
        );
        assert_eq!(
            toggled_off,
            PdfExportPickerResult::Accept {
                destination: destination.clone(),
                options: PdfExportOptions {
                    include_title_page: false,
                },
            }
        );
        assert_eq!(
            next_request,
            PdfExportPickerResult::Accept {
                destination,
                options: PdfExportOptions {
                    include_title_page: true,
                },
            }
        );
    }

    #[test]
    fn cancellation_failure_and_incomplete_acceptance_are_safe_results() {
        assert_eq!(
            picker_result_from_dialog_result(DialogResult::Cancelled),
            PdfExportPickerResult::Cancel
        );
        assert_eq!(
            picker_result_from_dialog_result(DialogResult::Failed),
            PdfExportPickerResult::Error(PdfExportPickerFailure::Failed)
        );
        assert_eq!(
            picker_result_from_dialog_result(DialogResult::Accepted {
                destination: None,
                include_title_page: true,
            }),
            PdfExportPickerResult::Error(PdfExportPickerFailure::Failed)
        );
    }

    #[test]
    fn selected_windows_path_is_copied_without_utf16_loss() {
        let selected = [
            b'C' as u16,
            b':' as u16,
            b'\\' as u16,
            0xD800,
            b'.' as u16,
            b'p' as u16,
            b'd' as u16,
            b'f' as u16,
        ];
        let path = path_buf_from_wide_path(&selected);
        assert!(path.is_some());
        if let Some(path) = path {
            assert_eq!(path.as_os_str().encode_wide().collect::<Vec<_>>(), selected);
        }
        assert_eq!(path_buf_from_wide_path(&[]), None);
    }
}
