//! Narrow AppKit adapter for the native Print panel and Include Title Page option.
//!
//! The panel has no safely borrowed owner window at the typed-command boundary,
//! so this adapter uses native modal presentation. It deliberately omits the
//! panel preview option until a Print-composed document preview can be supplied.

use objc2::rc::Retained;
use objc2::{define_class, msg_send, AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSButton, NSControlStateValueOff, NSControlStateValueOn, NSPrintInfo, NSPrintOperation,
    NSPrintPanel, NSPrintPanelAccessorizing, NSPrintPanelAccessorySummaryKey,
    NSPrintPanelAccessorySummaryItemDescriptionKey, NSPrintPanelAccessorySummaryItemNameKey,
    NSPrintPanelOptions, NSPrintPanelResult, NSView, NSViewController,
};
use objc2_foundation::{
    NSArray, NSData, NSDictionary, NSObjectProtocol, NSPoint, NSSet, NSSize, NSString,
};
use objc2_pdf_kit::{PDFDocument, PDFPrintScalingMode};
use scriptcraft_commands::PdfExportOptions;

use crate::print_export::PrintCompositionSnapshot;

const ACCESSORY_WIDTH: f64 = 300.0;
const ACCESSORY_HEIGHT: f64 = 28.0;
const PRINT_OPERATION_SHOWS_PANEL: bool = false;
const PDF_PRINT_AUTO_ROTATE: bool = true;
const PDF_PRINT_SCALING_MODE: PDFPrintScalingMode = PDFPrintScalingMode::PageScaleDownToFit;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PanelResponse {
    Accepted,
    Cancelled,
    Failed,
}

/// The one owned accepted-panel result. The snapshot is taken before display;
/// accepted AppKit settings and the title-page choice travel with it.
pub(super) struct AcceptedPrintSettings {
    print_info: Retained<NSPrintInfo>,
    options: PdfExportOptions,
    snapshot: PrintCompositionSnapshot,
}

impl AcceptedPrintSettings {
    pub(super) fn print_info(&self) -> &NSPrintInfo {
        &self.print_info
    }

    pub(super) fn options(&self) -> PdfExportOptions {
        self.options
    }

    pub(super) fn snapshot(&self) -> &PrintCompositionSnapshot {
        &self.snapshot
    }
}

pub(super) enum PrintPanelResult {
    Cancelled,
    Accepted(AcceptedPrintSettings),
    Unavailable,
    Failed,
}

pub(super) enum PrintPanelDispatch<T> {
    Cancelled,
    Accepted(T),
    Unavailable,
    Failed,
}

pub(super) fn dispatch_panel_result<T>(
    result: PrintPanelResult,
    deliver: impl FnOnce(AcceptedPrintSettings) -> T,
) -> PrintPanelDispatch<T> {
    match result {
        PrintPanelResult::Cancelled => PrintPanelDispatch::Cancelled,
        PrintPanelResult::Accepted(settings) => PrintPanelDispatch::Accepted(deliver(settings)),
        PrintPanelResult::Unavailable => PrintPanelDispatch::Unavailable,
        PrintPanelResult::Failed => PrintPanelDispatch::Failed,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PrintDeliveryResult {
    OperationReportedSuccess,
    OperationCancelledOrFailed,
    PdfBuildFailed,
    PdfDocumentInvalid,
    MainThreadUnavailable,
    PrintOperationUnavailable,
}

struct PrintOperationResources<Settings, Request, Data, Document, Operation> {
    _settings: Settings,
    _request: Request,
    _data: Data,
    _document: Document,
    operation: Operation,
}

fn run_retaining_resources<Settings, Request, Data, Document, Operation, Result>(
    resources: PrintOperationResources<Settings, Request, Data, Document, Operation>,
    run: impl FnOnce(&Operation) -> Result,
) -> Result {
    let result = run(&resources.operation);
    drop(resources);
    result
}

fn run_once_after_preparation<Prepared>(
    prepared: Result<Prepared, PrintDeliveryResult>,
    submit: impl FnOnce(Prepared) -> bool,
) -> Result<PrintDeliveryResult, PrintDeliveryResult> {
    prepared.map(|prepared| {
        if submit(prepared) {
            PrintDeliveryResult::OperationReportedSuccess
        } else {
            PrintDeliveryResult::OperationCancelledOrFailed
        }
    })
}

/// Builds and submits one PDFKit-backed operation from the accepted settings
/// and request-time snapshot. A false AppKit Boolean is intentionally not
/// classified further: the API does not distinguish cancellation from failure.
pub(super) fn deliver_print(settings: AcceptedPrintSettings) -> PrintDeliveryResult {
    let Some(main_thread) = MainThreadMarker::new() else {
        return PrintDeliveryResult::MainThreadUnavailable;
    };

    let prepared = prepare_print_operation(settings, main_thread);
    run_once_after_preparation(prepared, |resources| {
        run_retaining_resources(resources, |operation| operation.runOperation())
    })
    .unwrap_or_else(|error| error)
}

fn prepare_print_operation(
    settings: AcceptedPrintSettings,
    main_thread: MainThreadMarker,
) -> Result<
    PrintOperationResources<
        AcceptedPrintSettings,
        crate::print_export::PrintRequest,
        Retained<NSData>,
        Retained<PDFDocument>,
        Retained<NSPrintOperation>,
    >,
    PrintDeliveryResult,
> {
    let request = settings
        .snapshot()
        .build_request(settings.options())
        .map_err(|_| PrintDeliveryResult::PdfBuildFailed)?;
    if request.pdf_data().is_empty() {
        return Err(PrintDeliveryResult::PdfDocumentInvalid);
    }

    // SAFETY: The pointer addresses the live immutable Vec for this call and
    // NSData copies the complete byte range before returning. The request is
    // also retained for the full native operation lifetime below.
    #[allow(unsafe_code)]
    let data = unsafe {
        NSData::dataWithBytes_length(
            request.pdf_data().as_ptr().cast(),
            request.pdf_data().len() as _,
        )
    };
    // SAFETY: PDFDocument is created and used on the marked main thread; `data`
    // is retained for the complete operation lifetime. The binding marks the
    // framework initializer unsafe, so this local call is narrowly allowed.
    #[allow(unsafe_code)]
    let Some(document) = (unsafe { PDFDocument::initWithData(PDFDocument::alloc(), &data) }) else {
        return Err(PrintDeliveryResult::PdfDocumentInvalid);
    };
    // SAFETY: PDFKit's operation factory is invoked on the marked main thread
    // with retained document data and the accepted, live NSPrintInfo. Its unsafe
    // binding is confined to this call; all returned objects remain retained.
    #[allow(unsafe_code)]
    let Some(operation) = (unsafe {
        document.printOperationForPrintInfo_scalingMode_autoRotate(
            Some(settings.print_info()),
            PDF_PRINT_SCALING_MODE,
            PDF_PRINT_AUTO_ROTATE,
            main_thread,
        )
    }) else {
        return Err(PrintDeliveryResult::PrintOperationUnavailable);
    };

    let job_title = NSString::from_str(request.job_title());
    operation.setJobTitle(Some(&job_title));
    operation.setShowsPrintPanel(PRINT_OPERATION_SHOWS_PANEL);

    Ok(PrintOperationResources {
        _settings: settings,
        _request: request,
        _data: data,
        _document: document,
        operation,
    })
}

/// Opens one native Print panel with fresh defaults and returns panel settings,
/// option, and pre-panel composition snapshot together. Cancellation does not
/// construct the PDF bytes.
pub(super) fn show_print_panel(snapshot: PrintCompositionSnapshot) -> PrintPanelResult {
    let Some(main_thread) = MainThreadMarker::new() else {
        return PrintPanelResult::Unavailable;
    };

    let include_title_page = include_title_page_checkbox(main_thread);
    let accessory = TitlePageAccessoryController::new(include_title_page.clone(), main_thread);
    let print_info = NSPrintInfo::new();
    let panel = NSPrintPanel::printPanel(main_thread);
    let options = NSPrintPanelOptions::ShowsCopies
        | NSPrintPanelOptions::ShowsPageRange
        | NSPrintPanelOptions::ShowsPaperSize
        | NSPrintPanelOptions::ShowsOrientation
        | NSPrintPanelOptions::ShowsScaling;
    panel.setOptions(options);

    // SAFETY: `accessory` is a retained main-thread NSViewController subclass
    // implementing NSPrintPanelAccessorizing and remains alive through the
    // complete synchronous panel lifetime.
    #[allow(unsafe_code)]
    unsafe {
        panel.addAccessoryController(&accessory);
    }

    let response = panel.runModalWithPrintInfo(&print_info);
    let response = if response == NSPrintPanelResult::Printed.0 {
        PanelResponse::Accepted
    } else if response == NSPrintPanelResult::Cancelled.0 {
        PanelResponse::Cancelled
    } else {
        PanelResponse::Failed
    };

    let accepted_info = (response == PanelResponse::Accepted).then(|| panel.printInfo());
    match map_panel_response(
        response,
        accepted_info,
        include_title_page.state() == NSControlStateValueOn,
        snapshot,
    ) {
        PanelMapping::Cancelled => PrintPanelResult::Cancelled,
        PanelMapping::Failed => PrintPanelResult::Failed,
        PanelMapping::Accepted { print_info, options, snapshot } => {
            PrintPanelResult::Accepted(AcceptedPrintSettings {
                print_info,
                options,
                snapshot,
            })
        }
    }
}

enum PanelMapping<T> {
    Cancelled,
    Accepted {
        print_info: T,
        options: PdfExportOptions,
        snapshot: PrintCompositionSnapshot,
    },
    Failed,
}

fn map_panel_response<T>(
    response: PanelResponse,
    print_info: Option<T>,
    include_title_page: bool,
    snapshot: PrintCompositionSnapshot,
) -> PanelMapping<T> {
    match response {
        PanelResponse::Cancelled => PanelMapping::Cancelled,
        PanelResponse::Failed => PanelMapping::Failed,
        PanelResponse::Accepted => match print_info {
            Some(print_info) => PanelMapping::Accepted {
                print_info,
                options: PdfExportOptions {
                    include_title_page,
                },
                snapshot,
            },
            None => PanelMapping::Failed,
        },
    }
}

fn include_title_page_checkbox(main_thread: MainThreadMarker) -> Retained<NSButton> {
    let label = NSString::from_str("Include Title Page");
    // SAFETY: nil target/action leaves a standard self-tracking checkbox without
    // a callback crossing this adapter boundary; construction is main-thread-only.
    #[allow(unsafe_code)]
    let checkbox = unsafe { NSButton::checkboxWithTitle_target_action(&label, None, None, main_thread) };
    checkbox.setState(if fresh_title_page_default() {
        NSControlStateValueOn
    } else {
        NSControlStateValueOff
    });
    checkbox.setFrame(objc2_foundation::NSRect::new(
        NSPoint::new(0.0, 2.0),
        NSSize::new(ACCESSORY_WIDTH, 24.0),
    ));
    checkbox
}

fn fresh_title_page_default() -> bool {
    PdfExportOptions::default().include_title_page
}

#[derive(Debug)]
struct AccessoryIvars {
    checkbox: Retained<NSButton>,
}

// SAFETY: This subclass is main-thread-confined; all ivars contain retained
// AppKit objects and it has no custom Drop behavior.
#[allow(unsafe_code)]
define_class!(
    #[unsafe(super = NSViewController)]
    #[thread_kind = MainThreadOnly]
    #[ivars = AccessoryIvars]
    struct TitlePageAccessoryController;

    // SAFETY: NSObjectProtocol has no additional implementation requirements.
    unsafe impl NSObjectProtocol for TitlePageAccessoryController {}

    // SAFETY: Both methods return the AppKit-required Foundation collection
    // shapes; the preview dependency names the controller's KVC option value.
    unsafe impl NSPrintPanelAccessorizing for TitlePageAccessoryController {
        #[unsafe(method_id(localizedSummaryItems))]
        #[unsafe(method_family = none)]
        #[allow(unsafe_code)]
        fn localizedSummaryItems(
            &self,
        ) -> Retained<NSArray<NSDictionary<NSPrintPanelAccessorySummaryKey, NSString>>> {
            let name = NSString::from_str("Include Title Page");
            let description = NSString::from_str(if self.include_title_page() {
                "Included"
            } else {
                "Excluded"
            });
            // SAFETY: These are immutable framework constants with static
            // storage. The dictionary copies its keys and retains the values.
            let (name_key, description_key) = unsafe {
                (
                    NSPrintPanelAccessorySummaryItemNameKey,
                    NSPrintPanelAccessorySummaryItemDescriptionKey,
                )
            };
            let summary = NSDictionary::from_slices(
                &[name_key, description_key],
                &[&*name, &*description],
            );
            NSArray::from_retained_slice(&[summary])
        }

        #[unsafe(method_id(keyPathsForValuesAffectingPreview))]
        #[unsafe(method_family = none)]
        fn keyPathsForValuesAffectingPreview(&self) -> Retained<NSSet<NSString>> {
            NSSet::from_retained_slice(&[NSString::from_str("includeTitlePage")])
        }
    }

    impl TitlePageAccessoryController {
        #[unsafe(method(includeTitlePage))]
        fn include_title_page_for_key_value_coding(&self) -> bool {
            self.include_title_page()
        }
    }
);

impl TitlePageAccessoryController {
    fn new(checkbox: Retained<NSButton>, main_thread: MainThreadMarker) -> Retained<Self> {
        let view = NSView::new(main_thread);
        view.setFrame(objc2_foundation::NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(ACCESSORY_WIDTH, ACCESSORY_HEIGHT),
        ));
        view.addSubview(&checkbox);

        let this = Self::alloc(main_thread).set_ivars(AccessoryIvars { checkbox });
        // SAFETY: NSViewController's designated NSObject initializer is invoked
        // on its valid allocated subclass and the superclass has no extra init
        // preconditions. The retained custom view and ivars stay on main thread.
        #[allow(unsafe_code)]
        let controller: Retained<Self> = unsafe { msg_send![super(this), init] };
        controller.setView(&view);
        controller.setPreferredContentSize(NSSize::new(ACCESSORY_WIDTH, ACCESSORY_HEIGHT));
        controller
    }

    fn include_title_page(&self) -> bool {
        self.ivars().checkbox.state() == NSControlStateValueOn
    }
}

#[cfg(test)]
mod tests {
    use super::{
        fresh_title_page_default, map_panel_response, PanelMapping, PanelResponse,
    };
    use crate::print_export::PrintCompositionSnapshot;
    use scriptcraft_commands::PdfExportOptions;
    use scriptcraft_core::{LayoutInputs, Project};

    fn snapshot() -> PrintCompositionSnapshot {
        let project = Project::new().unwrap_or_else(|_| unreachable!("new project is valid"));
        PrintCompositionSnapshot::capture(&project, &LayoutInputs::default(), None)
    }

    #[test]
    fn accepted_result_maps_settings_and_checkbox_choice() {
        assert!(fresh_title_page_default());
        let accepted = map_panel_response(PanelResponse::Accepted, Some(()), false, snapshot());
        let PanelMapping::Accepted {
            options,
            ..
        } = accepted
        else {
            unreachable!("complete accepted test result has settings")
        };
        assert!(!options.include_title_page);
        assert!(matches!(
            map_panel_response(PanelResponse::Accepted, Some(()), true, snapshot()),
            PanelMapping::Accepted { options: PdfExportOptions { include_title_page: true }, .. }
        ));
    }

    #[test]
    fn cancellation_and_incomplete_or_failed_acceptance_return_no_settings() {
        assert!(matches!(
            map_panel_response(PanelResponse::Cancelled, None::<()>, true, snapshot()),
            PanelMapping::Cancelled
        ));
        assert!(matches!(
            map_panel_response(PanelResponse::Accepted, None::<()>, true, snapshot()),
            PanelMapping::Failed
        ));
        assert!(matches!(
            map_panel_response(PanelResponse::Failed, None::<()>, false, snapshot()),
            PanelMapping::Failed
        ));
    }


}
