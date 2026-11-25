//! Narrow AppKit adapter for the native Print panel and Include Title Page option.
//!
//! The shared coordinator supplies the actual root owner and owns each panel and
//! delivery lifetime. This adapter deliberately omits the panel preview option
//! until a Print-composed document preview can be supplied.

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{define_class, msg_send, sel, AnyThread, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
    NSButton, NSControlStateValueOff, NSControlStateValueOn, NSPrintInfo, NSPrintOperation,
    NSPrintPanel, NSPrintPanelAccessorizing, NSPrintPanelAccessorySummaryItemDescriptionKey,
    NSPrintPanelAccessorySummaryItemNameKey, NSPrintPanelAccessorySummaryKey, NSPrintPanelOptions,
    NSPrintPanelResult, NSView, NSViewController, NSWindow,
};
use objc2_foundation::{
    NSArray, NSData, NSDictionary, NSObjectProtocol, NSPoint, NSSet, NSSize, NSString,
};
use objc2_pdf_kit::{PDFDocument, PDFPrintScalingMode};
use scriptcraft_commands::PdfExportOptions;

use crate::documents::print::PrintCompositionSnapshot;
use crate::platform::macos::macos_native_dialogs::{
    NativeOperationCompletionSink, NativeOperationOutcome, NativePanelResponse,
};
use std::ffi::c_void;

const ACCESSORY_WIDTH: f64 = 300.0;
const ACCESSORY_HEIGHT: f64 = 28.0;
const PRINT_OPERATION_SHOWS_PANEL: bool = false;
const PDF_PRINT_AUTO_ROTATE: bool = true;
const PDF_PRINT_SCALING_MODE: PDFPrintScalingMode = PDFPrintScalingMode::PageScaleNone;

/// The one owned accepted-panel result. The snapshot is taken before display;
/// accepted AppKit settings and the title-page choice travel with it.
pub(crate) struct AcceptedPrintSettings {
    print_info: Retained<NSPrintInfo>,
    options: PdfExportOptions,
    snapshot: PrintCompositionSnapshot,
    _accessory_controller: Retained<TitlePageAccessoryController>,
}

pub(crate) enum PrintPanelCompletion {
    Cancelled,
    Accepted(AcceptedPrintSettings),
    Failed,
}

pub(crate) struct PrintPanelResources {
    panel: Retained<NSPrintPanel>,
    print_info: Retained<NSPrintInfo>,
    checkbox: Retained<NSButton>,
    accessory_controller: Retained<TitlePageAccessoryController>,
    snapshot: PrintCompositionSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PrintDeliveryResult {
    OperationReportedSuccess,
    OperationCancelledOrFailed,
    PdfBuildFailed,
    PdfDocumentInvalid,
    MainThreadUnavailable,
    PrintOperationUnavailable,
}

pub(crate) struct PrintDeliveryOutcome {
    pub(crate) result: PrintDeliveryResult,
    pub(crate) fallback_summary: Option<String>,
}

pub(crate) struct PrintDeliveryResources {
    _settings: AcceptedPrintSettings,
    request: crate::documents::print::PrintRequest,
    _data: Retained<NSData>,
    _document: Retained<PDFDocument>,
    operation: Retained<NSPrintOperation>,
    _completion_delegate: Option<Retained<PrintOperationCompletionDelegate>>,
}

pub(crate) fn prepare_print_panel(
    snapshot: PrintCompositionSnapshot,
    main_thread: MainThreadMarker,
) -> PrintPanelResources {
    let checkbox = include_title_page_checkbox(main_thread);
    let accessory_controller = TitlePageAccessoryController::new(checkbox.clone(), main_thread);
    let print_info = NSPrintInfo::new();
    let (paper_width, paper_height) = snapshot.paper_size_points();
    print_info.setPaperSize(NSSize::new(paper_width as _, paper_height as _));
    print_info.setLeftMargin(0.0);
    print_info.setRightMargin(0.0);
    print_info.setTopMargin(0.0);
    print_info.setBottomMargin(0.0);
    print_info.setScalingFactor(1.0);
    print_info.setHorizontallyCentered(false);
    print_info.setVerticallyCentered(false);
    let panel = NSPrintPanel::printPanel(main_thread);
    let options = NSPrintPanelOptions::ShowsCopies
        | NSPrintPanelOptions::ShowsPageRange
        | NSPrintPanelOptions::ShowsPaperSize
        | NSPrintPanelOptions::ShowsOrientation
        | NSPrintPanelOptions::ShowsScaling;
    panel.setOptions(options);

    // SAFETY: `accessory_controller` is a retained main-thread NSViewController
    // subclass implementing NSPrintPanelAccessorizing and remains retained by
    // PrintPanelResources until its asynchronous sheet completion is routed.
    #[allow(unsafe_code)]
    unsafe {
        panel.addAccessoryController(&accessory_controller);
    }

    PrintPanelResources {
        panel,
        print_info,
        checkbox,
        accessory_controller,
        snapshot,
    }
}

pub(crate) fn begin_print_panel(
    resources: &PrintPanelResources,
    owner: &NSWindow,
    completion_sink: NativeOperationCompletionSink,
) {
    let completion = block2::RcBlock::new(move |response: NSPrintPanelResult| {
        let outcome = if response == NSPrintPanelResult::Printed {
            NativeOperationOutcome::Panel(NativePanelResponse::Accepted)
        } else if response == NSPrintPanelResult::Cancelled {
            NativeOperationOutcome::Panel(NativePanelResponse::Cancelled)
        } else {
            NativeOperationOutcome::Panel(NativePanelResponse::Failed)
        };
        let _ = completion_sink.enqueue(outcome);
    });
    resources.panel.beginSheetUsingPrintInfo_onWindow_completionHandler(
        &resources.print_info,
        owner,
        Some(&*completion),
    );
}

pub(crate) fn complete_print_panel(
    resources: PrintPanelResources,
    response: NativePanelResponse,
) -> PrintPanelCompletion {
    match response {
        NativePanelResponse::Cancelled => PrintPanelCompletion::Cancelled,
        NativePanelResponse::Accepted => PrintPanelCompletion::Accepted(AcceptedPrintSettings {
            print_info: resources.panel.printInfo(),
            options: PdfExportOptions {
                include_title_page: resources.checkbox.state() == NSControlStateValueOn,
            },
            snapshot: resources.snapshot,
            _accessory_controller: resources.accessory_controller,
        }),
        NativePanelResponse::Unavailable | NativePanelResponse::Failed => {
            PrintPanelCompletion::Failed
        }
    }
}

pub(crate) fn prepare_print_delivery(
    settings: AcceptedPrintSettings,
    main_thread: MainThreadMarker,
) -> Result<PrintDeliveryResources, PrintDeliveryResult> {
    let request = settings
        .snapshot
        .build_request(settings.options)
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
            Some(&settings.print_info),
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

    Ok(PrintDeliveryResources {
        _settings: settings,
        request,
        _data: data,
        _document: document,
        operation,
        _completion_delegate: None,
    })
}

pub(crate) fn begin_print_delivery(
    resources: &mut PrintDeliveryResources,
    owner: &NSWindow,
    completion_sink: NativeOperationCompletionSink,
    main_thread: MainThreadMarker,
) {
    let delegate = PrintOperationCompletionDelegate::new(completion_sink, main_thread);
    resources._completion_delegate = Some(delegate.clone());
    // SAFETY: The pinned objc2-app-kit binding documents this API as an
    // asynchronous window-modal operation. The retained delegate implements
    // the exact `printOperationDidRun:success:contextInfo:` selector and remains
    // owned by PrintDeliveryResources;
    // the documented Boolean argument maps to Rust bool, and null is a valid
    // context pointer because operation identity is captured by the sink.
    #[allow(unsafe_code)]
    unsafe {
        resources
            .operation
            .runOperationModalForWindow_delegate_didRunSelector_contextInfo(
                owner,
                Some(&delegate as &AnyObject),
                Some(sel!(printOperationDidRun:success:contextInfo:)),
                std::ptr::null_mut(),
            );
    }
}

pub(crate) fn complete_print_delivery(
    resources: PrintDeliveryResources,
    reported_success: bool,
) -> PrintDeliveryOutcome {
    let fallback_summary = Some(resources.request.fallback_summary());
    let result = if reported_success {
        PrintDeliveryResult::OperationReportedSuccess
    } else {
        PrintDeliveryResult::OperationCancelledOrFailed
    };
    drop(resources);
    PrintDeliveryOutcome {
        result,
        fallback_summary,
    }
}

impl PrintDeliveryResources {
    pub(crate) fn fallback_summary(&self) -> String {
        self.request.fallback_summary()
    }

    pub(crate) fn failure_outcome(error: PrintDeliveryResult) -> PrintDeliveryOutcome {
        PrintDeliveryOutcome {
            result: error,
            fallback_summary: None,
        }
    }
}

fn include_title_page_checkbox(main_thread: MainThreadMarker) -> Retained<NSButton> {
    let label = NSString::from_str("Include Title Page");
    // SAFETY: nil target/action leaves a standard self-tracking checkbox without
    // a callback crossing this adapter boundary; construction is main-thread-only.
    #[allow(unsafe_code)]
    let checkbox =
        unsafe { NSButton::checkboxWithTitle_target_action(&label, None, None, main_thread) };
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

struct PrintOperationCompletionIvars {
    completion_sink: NativeOperationCompletionSink,
}

// SAFETY: The delegate is created and retained on the AppKit main thread. Its
// callback only forwards the primitive success value to the bounded sink.
#[allow(unsafe_code)]
define_class!(
    #[unsafe(super = objc2_foundation::NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = PrintOperationCompletionIvars]
    struct PrintOperationCompletionDelegate;

    // SAFETY: NSObjectProtocol adds no implementation requirements.
    unsafe impl NSObjectProtocol for PrintOperationCompletionDelegate {}

    impl PrintOperationCompletionDelegate {
        #[unsafe(method(printOperationDidRun:success:contextInfo:))]
        fn print_operation_did_run(
            &self,
            _operation: &NSPrintOperation,
            reported_success: bool,
            _context_info: *mut c_void,
        ) {
            let _ = self.ivars().completion_sink.enqueue(
                NativeOperationOutcome::PrintDelivered(reported_success),
            );
        }
    }
);

impl PrintOperationCompletionDelegate {
    fn new(
        completion_sink: NativeOperationCompletionSink,
        main_thread: MainThreadMarker,
    ) -> Retained<Self> {
        let this = Self::alloc(main_thread).set_ivars(PrintOperationCompletionIvars {
            completion_sink,
        });
        // SAFETY: NSObject's initializer is invoked on a valid allocated
        // main-thread-only delegate instance.
        #[allow(unsafe_code)]
        let delegate: Retained<Self> = unsafe { msg_send![super(this), init] };
        delegate
    }
}
