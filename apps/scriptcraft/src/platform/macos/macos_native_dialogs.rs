//! Shared owner-window and completion coordination for asynchronous AppKit work.
//!
//! File-panel presentation, Print-panel presentation, and Print delivery share
//! one identity, completion slot, owner, and operation lifetime gate.

use eframe::egui;
use block2::RcBlock;
use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSButton, NSControlStateValueOn, NSModalResponse, NSModalResponseCancel, NSModalResponseOK,
    NSOpenPanel, NSSavePanel, NSView, NSWindow,
};
use objc2_foundation::{NSArray, NSString, NSURL};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::marker::PhantomData;
use std::num::NonZeroU64;
use std::ffi::{CStr, OsStr};
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex, MutexGuard};
use scriptcraft_commands::PdfExportOptions;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct NativeOperationId(NonZeroU64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeOperationKind {
    Open,
    SaveAs,
    FountainExport,
    PdfExport,
    PdfExportWithOptions,
    PrintPanel,
    PrintDelivery,
}

/// Only primitive AppKit completion data crosses back from a native callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeOperationOutcome {
    Panel(NativePanelResponse),
    PrintDelivered(bool),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativePanelResponse {
    Accepted,
    Cancelled,
    Unavailable,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeFilePanelContinuation {
    Open,
    SaveAs,
    FountainExport,
    PdfExport,
    PdfExportWithOptions(PdfExportOptions),
}

pub(crate) struct NativeFilePanelRequest<'a> {
    pub(crate) continuation: NativeFilePanelContinuation,
    pub(crate) title: &'a str,
    pub(crate) default_filename: &'a str,
    pub(crate) allowed_extensions: &'a [&'a str],
    pub(crate) include_title_page: bool,
}

pub(crate) enum NativeFilePanelSelection {
    Cancelled,
    Failed,
    Accepted {
        destination: PathBuf,
        include_title_page: Option<bool>,
    },
}

/// Main-thread-owned native resources retained until terminal cleanup.
pub(crate) enum NativeOperationResources {
    FilePanel {
        panel: Option<Retained<NSSavePanel>>,
        accessory: Option<Retained<NSButton>>,
        continuation: NativeFilePanelContinuation,
    },
    PrintPanel(Option<crate::platform::macos::macos_print_dialog::PrintPanelResources>),
    PrintDelivery {
        delivery: Option<crate::platform::macos::macos_print_dialog::PrintDeliveryResources>,
    },
}

pub(crate) struct ActiveNativeOperation {
    pub(crate) identity: NativeOperationId,
    pub(crate) kind: NativeOperationKind,
    // Retain the exact owner used for presentation even if a later frame cannot
    // refresh its raw handle while the native operation is still active.
    _owner: Retained<NSWindow>,
    pub(crate) resources: NativeOperationResources,
}

pub(crate) struct CompletedNativeOperation {
    pub(crate) operation: ActiveNativeOperation,
    pub(crate) outcome: NativeOperationOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeOperationStartError {
    ActiveOperation,
    OwnerUnavailable,
    IdentityExhausted,
    CompletionPending,
}

#[derive(Default)]
struct CompletionMailbox {
    active_identity: Option<NativeOperationId>,
    completion: Option<QueuedNativeCompletion>,
}

#[derive(Clone, Copy)]
struct QueuedNativeCompletion {
    identity: NativeOperationId,
    outcome: NativeOperationOutcome,
}

/// Cloneable callback handle containing no application/session or AppKit state.
/// The slot accepts one matching completion and drops duplicates and stale IDs.
#[derive(Clone)]
pub(crate) struct NativeOperationCompletionSink {
    identity: NativeOperationId,
    mailbox: Arc<Mutex<CompletionMailbox>>,
    repaint_context: egui::Context,
}

impl NativeOperationCompletionSink {
    pub(crate) fn enqueue(&self, outcome: NativeOperationOutcome) -> bool {
        let accepted = {
            let mut mailbox = lock_recover(&self.mailbox);
            if mailbox.active_identity == Some(self.identity) && mailbox.completion.is_none() {
                mailbox.completion = Some(QueuedNativeCompletion {
                    identity: self.identity,
                    outcome,
                });
                true
            } else {
                false
            }
        };
        if accepted {
            self.repaint_context.request_repaint();
        }
        accepted
    }
}

pub(crate) struct NativeOperationCoordinator {
    owner: Option<Retained<NSWindow>>,
    repaint_context: Option<egui::Context>,
    next_identity: Option<NonZeroU64>,
    active: Option<ActiveNativeOperation>,
    mailbox: Arc<Mutex<CompletionMailbox>>,
    // AppKit objects and their owner are confined to eframe's UI thread.
    _main_thread_only: PhantomData<Rc<()>>,
}

impl NativeOperationCoordinator {
    pub(crate) fn new() -> Self {
        Self {
            owner: None,
            repaint_context: None,
            next_identity: NonZeroU64::new(1),
            active: None,
            mailbox: Arc::new(Mutex::new(CompletionMailbox::default())),
            _main_thread_only: PhantomData,
        }
    }

    /// Refresh only the cached owner and repaint context; an active operation
    /// retains the owner it started with and is never disturbed by acquisition
    /// failure.
    pub(crate) fn refresh_root_owner(
        &mut self,
        frame: &eframe::Frame,
        context: &egui::Context,
    ) {
        self.repaint_context = Some(context.clone());
        self.owner = acquire_root_owner(frame);
    }

    pub(crate) fn is_active(&self) -> bool {
        self.active.is_some()
    }

    pub(crate) fn present_file_panel(
        &mut self,
        request: NativeFilePanelRequest<'_>,
    ) -> Result<(), NativeOperationStartError> {
        let Some(owner) = self.owner.as_ref().cloned() else {
            return Err(NativeOperationStartError::OwnerUnavailable);
        };
        let Some(main_thread) = MainThreadMarker::new() else {
            return Err(NativeOperationStartError::OwnerUnavailable);
        };

        let panel = if request.continuation == NativeFilePanelContinuation::Open {
            let panel = NSOpenPanel::openPanel(main_thread);
            panel.setCanChooseDirectories(false);
            panel.setCanChooseFiles(true);
            panel.setAllowsMultipleSelection(false);
            panel.into_super()
        } else {
            NSSavePanel::savePanel(main_thread)
        };
        let title = NSString::from_str(request.title);
        panel.setTitle(Some(&title));
        let default_filename = NSString::from_str(request.default_filename);
        if request.continuation != NativeFilePanelContinuation::Open {
            panel.setNameFieldStringValue(&default_filename);
        }
        if matches!(
            request.continuation,
            NativeFilePanelContinuation::PdfExport
                | NativeFilePanelContinuation::PdfExportWithOptions(_)
        ) {
            panel.setAllowedContentTypes(&NSArray::arrayWithObject(
                crate::platform::macos::macos_pdf_dialog::pdf_content_type(),
            ));
            panel.setAllowsOtherFileTypes(false);
        } else {
            let extensions = request
                .allowed_extensions
                .iter()
                .map(|extension| NSString::from_str(extension))
                .collect::<Vec<_>>();
            panel.setAllowedFileTypes(Some(&NSArray::from_retained_slice(&extensions)));
            panel.setAllowsOtherFileTypes(false);
        }

        let accessory = request.include_title_page.then(|| {
            let accessory = crate::platform::macos::macos_pdf_dialog::include_title_page_checkbox(
                main_thread,
            );
            panel.setAccessoryView(Some(&accessory));
            accessory
        });
        let operation_kind = match request.continuation {
            NativeFilePanelContinuation::Open => NativeOperationKind::Open,
            NativeFilePanelContinuation::SaveAs => NativeOperationKind::SaveAs,
            NativeFilePanelContinuation::FountainExport => NativeOperationKind::FountainExport,
            NativeFilePanelContinuation::PdfExport => NativeOperationKind::PdfExport,
            NativeFilePanelContinuation::PdfExportWithOptions(_) => {
                NativeOperationKind::PdfExportWithOptions
            }
        };
        let (_identity, sink) = self.begin(
            operation_kind,
            NativeOperationResources::FilePanel {
                panel: Some(panel.clone()),
                accessory,
                continuation: request.continuation,
            },
        )?;
        let completion: RcBlock<dyn Fn(NSModalResponse)> = RcBlock::new(move |response: NSModalResponse| {
            let outcome = if response == NSModalResponseOK {
                NativeOperationOutcome::Panel(NativePanelResponse::Accepted)
            } else if response == NSModalResponseCancel {
                NativeOperationOutcome::Panel(NativePanelResponse::Cancelled)
            } else {
                NativeOperationOutcome::Panel(NativePanelResponse::Failed)
            };
            let _ = sink.enqueue(outcome);
        });
        panel.beginSheetModalForWindow_completionHandler(&owner, &*completion);
        Ok(())
    }

    pub(crate) fn present_print_panel(
        &mut self,
        snapshot: crate::documents::print::PrintCompositionSnapshot,
    ) -> Result<(), NativeOperationStartError> {
        let Some(owner) = self.owner.as_ref().cloned() else {
            return Err(NativeOperationStartError::OwnerUnavailable);
        };
        let Some(main_thread) = MainThreadMarker::new() else {
            return Err(NativeOperationStartError::OwnerUnavailable);
        };
        let panel = crate::platform::macos::macos_print_dialog::prepare_print_panel(
            snapshot,
            main_thread,
        );
        let (_, completion_sink) = self.begin(
            NativeOperationKind::PrintPanel,
            NativeOperationResources::PrintPanel(Some(panel)),
        )?;
        let panel_is_active = matches!(
            self.active.as_ref().map(|active| &active.resources),
            Some(NativeOperationResources::PrintPanel(Some(_)))
        );
        if !panel_is_active {
            self.discard_unpresented_active();
            return Err(NativeOperationStartError::ActiveOperation);
        }
        if let Some(ActiveNativeOperation {
            resources: NativeOperationResources::PrintPanel(Some(panel)),
            ..
        }) = self.active.as_ref()
        {
            crate::platform::macos::macos_print_dialog::begin_print_panel(
                panel,
                &owner,
                completion_sink,
            );
            return Ok(());
        }
        self.discard_unpresented_active();
        Err(NativeOperationStartError::ActiveOperation)
    }

    /// Prepare exactly one PDFKit-backed operation after panel acceptance and
    /// start its native window-modal lifecycle without blocking this update.
    pub(crate) fn start_print_delivery(
        &mut self,
        settings: crate::platform::macos::macos_print_dialog::AcceptedPrintSettings,
    ) -> Result<(), crate::platform::macos::macos_print_dialog::PrintDeliveryOutcome> {
        use crate::platform::macos::macos_print_dialog::{
            prepare_print_delivery, PrintDeliveryOutcome, PrintDeliveryResult,
        };

        let Some(main_thread) = MainThreadMarker::new() else {
            return Err(PrintDeliveryOutcome {
                result: PrintDeliveryResult::MainThreadUnavailable,
                fallback_summary: None,
            });
        };
        let delivery = prepare_print_delivery(settings, main_thread)
            .map_err(crate::platform::macos::macos_print_dialog::PrintDeliveryResources::failure_outcome)?;
        let fallback_summary = delivery.fallback_summary();
        let start = self.begin(
            NativeOperationKind::PrintDelivery,
            NativeOperationResources::PrintDelivery {
                delivery: Some(delivery),
            },
        );
        let (_, completion_sink) = match start {
            Ok(start) => start,
            Err(_) => {
                return Err(PrintDeliveryOutcome {
                    result: PrintDeliveryResult::PrintOperationUnavailable,
                    fallback_summary: Some(fallback_summary),
                });
            }
        };
        let Some(owner) = self.active.as_ref().map(|active| active._owner.clone()) else {
            self.discard_unpresented_active();
            return Err(PrintDeliveryOutcome {
                result: PrintDeliveryResult::PrintOperationUnavailable,
                fallback_summary: Some(fallback_summary),
            });
        };
        if let Some(ActiveNativeOperation {
            resources: NativeOperationResources::PrintDelivery {
                delivery: Some(delivery),
            },
            ..
        }) = self.active.as_mut()
        {
            crate::platform::macos::macos_print_dialog::begin_print_delivery(
                delivery,
                &owner,
                completion_sink,
                main_thread,
            );
            return Ok(());
        }
        self.discard_unpresented_active();
        Err(PrintDeliveryOutcome {
            result: PrintDeliveryResult::PrintOperationUnavailable,
            fallback_summary: Some(fallback_summary),
        })
    }

    fn discard_unpresented_active(&mut self) {
        let Some(active) = self.active.take() else {
            return;
        };
        let mut mailbox = lock_recover(&self.mailbox);
        if mailbox.active_identity == Some(active.identity) {
            mailbox.active_identity = None;
            mailbox.completion = None;
        }
    }

    /// Reserve the sole active slot after confirming a real root owner exists.
    pub(crate) fn begin(
        &mut self,
        kind: NativeOperationKind,
        resources: NativeOperationResources,
    ) -> Result<(NativeOperationId, NativeOperationCompletionSink), NativeOperationStartError> {
        if self.active.is_some() {
            return Err(NativeOperationStartError::ActiveOperation);
        }
        let Some(owner) = self.owner.as_ref() else {
            return Err(NativeOperationStartError::OwnerUnavailable);
        };
        let sequence_value = self
            .next_identity
            .ok_or(NativeOperationStartError::IdentityExhausted)?;
        let identity = NativeOperationId(sequence_value);
        {
            let mut mailbox = lock_recover(&self.mailbox);
            if mailbox.completion.is_some() {
                return Err(NativeOperationStartError::CompletionPending);
            }
            mailbox.active_identity = Some(identity);
        }
        self.next_identity = sequence_value
            .get()
            .checked_add(1)
            .and_then(NonZeroU64::new);
        self.active = Some(ActiveNativeOperation {
            identity,
            kind,
            _owner: owner.clone(),
            resources,
        });
        let sink = NativeOperationCompletionSink {
            identity,
            mailbox: Arc::clone(&self.mailbox),
            repaint_context: self.repaint_context.clone().unwrap_or_default(),
        };
        Ok((identity, sink))
    }

    /// Drain at most one completion. A mismatched completion is discarded but
    /// cannot release or replace the currently active operation.
    pub(crate) fn take_matching_completion(&mut self) -> Option<CompletedNativeOperation> {
        let completion = {
            let mut mailbox = lock_recover(&self.mailbox);
            let completion = mailbox.completion.take()?;
            let Some(active) = self.active.as_ref() else {
                return None;
            };
            if completion.identity != active.identity
                || mailbox.active_identity != Some(active.identity)
            {
                return None;
            }
            mailbox.active_identity = None;
            completion
        };
        let operation = self.active.take()?;
        Some(CompletedNativeOperation {
            operation,
            outcome: completion.outcome,
        })
    }
}

impl CompletedNativeOperation {
    pub(crate) fn take_file_panel_selection(
        &mut self,
    ) -> Option<(NativeFilePanelContinuation, NativeFilePanelSelection)> {
        let NativeOperationResources::FilePanel {
            panel,
            accessory,
            continuation,
        } = &mut self.operation.resources
        else {
            return None;
        };
        let result = match self.outcome {
            NativeOperationOutcome::Panel(NativePanelResponse::Cancelled) => {
                NativeFilePanelSelection::Cancelled
            }
            NativeOperationOutcome::Panel(NativePanelResponse::Accepted) => {
                let destination = panel
                    .as_ref()
                    .and_then(|panel| panel.URL())
                    .filter(|url| url.isFileURL())
                    .and_then(|url| path_buf_from_file_url(&url));
                match destination {
                    Some(destination) => NativeFilePanelSelection::Accepted {
                        destination,
                        include_title_page: accessory
                            .as_ref()
                            .map(|checkbox| checkbox.state() == NSControlStateValueOn),
                    },
                    None => NativeFilePanelSelection::Failed,
                }
            }
            NativeOperationOutcome::Panel(
                NativePanelResponse::Unavailable | NativePanelResponse::Failed,
            )
            | NativeOperationOutcome::PrintDelivered(_) => NativeFilePanelSelection::Failed,
        };
        Some((*continuation, result))
    }

    pub(crate) fn take_print_panel_completion(
        &mut self,
    ) -> Option<crate::platform::macos::macos_print_dialog::PrintPanelCompletion> {
        let NativeOperationResources::PrintPanel(panel) = &mut self.operation.resources else {
            return None;
        };
        let panel = panel.take()?;
        let response = match self.outcome {
            NativeOperationOutcome::Panel(response) => response,
            NativeOperationOutcome::PrintDelivered(_) => NativePanelResponse::Failed,
        };
        Some(crate::platform::macos::macos_print_dialog::complete_print_panel(
            panel, response,
        ))
    }

    pub(crate) fn take_print_delivery_outcome(
        &mut self,
    ) -> Option<crate::platform::macos::macos_print_dialog::PrintDeliveryOutcome> {
        let NativeOperationResources::PrintDelivery { delivery } = &mut self.operation.resources
        else {
            return None;
        };
        let delivery = delivery.take()?;
        let reported_success = match self.outcome {
            NativeOperationOutcome::PrintDelivered(success) => success,
            NativeOperationOutcome::Panel(_) => false,
        };
        Some(crate::platform::macos::macos_print_dialog::complete_print_delivery(
            delivery,
            reported_success,
        ))
    }
}

/// Copies the filesystem representation without lossy Unicode conversion so
/// each typed continuation can reject invalid UTF-8 before domain work begins.
#[allow(unsafe_code)]
fn path_buf_from_file_url(url: &NSURL) -> Option<PathBuf> {
    let representation = url.fileSystemRepresentation();
    // SAFETY: NSURL supplies a non-null, NUL-terminated filesystem path and the
    // retained URL stays alive until this immediate byte copy has completed.
    let bytes = unsafe { CStr::from_ptr(representation.as_ptr()).to_bytes() };
    if bytes.is_empty() {
        return None;
    }
    Some(PathBuf::from(OsStr::from_bytes(bytes)))
}

#[allow(unsafe_code)]
fn acquire_root_owner(frame: &eframe::Frame) -> Option<Retained<NSWindow>> {
    let handle = frame.window_handle().ok()?;
    let RawWindowHandle::AppKit(appkit_handle) = handle.as_raw() else {
        return None;
    };
    let _main_thread = MainThreadMarker::new()?;
    let view_pointer = appkit_handle.ns_view.cast::<NSView>();

    // SAFETY: `window_handle()` is borrowed from the live eframe frame for this
    // entire conversion, and its AppKit variant guarantees `ns_view` is the
    // root viewport's valid NSView pointer. The update callback runs on the
    // AppKit main thread (also checked above). We borrow the view only long
    // enough to ask AppKit for its owning window, which is returned retained.
    let view: &NSView = unsafe { view_pointer.as_ref() };
    view.window()
}

fn lock_recover<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}
