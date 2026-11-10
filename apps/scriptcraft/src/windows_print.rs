//! Native Windows print-session adapter for the shared FilePrint command.
//!
//! The FilePrint source follows Microsoft's Win2D `CanvasPrintDocument` pattern:
//! one immutable Print-layout package is selected for preview and final pages,
//! preview is drawn to the supplied DXGI target, and final pages use Direct2D
//! print control.

#[path = "windows_print_document.rs"]
mod document;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::mem::ManuallyDrop;
use std::sync::{Arc, Mutex};
use std::thread::ThreadId;
use std::time::{Duration, Instant};

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::core as windows_core;
use windows::core::{
    implement, Error as WindowsError, HSTRING, HRESULT, IUnknownImpl, Interface, Ref,
    Result as WinResult,
};
use windows::Foundation::TypedEventHandler;
use windows::Graphics::Printing::{
    IPrintDocumentSource, IPrintDocumentSource_Impl, IPrintTaskOptionsCore, PrintManager,
    PrintPageDescription, PrintTask, PrintTaskCompletion, PrintTaskCompletedEventArgs,
    PrintTaskOptions, PrintTaskRequestedEventArgs, PrintTaskSourceRequestedHandler,
    StandardPrintTaskOptions,
};
use windows::Graphics::Printing::OptionDetails::{
    PrintTaskOptionChangedEventArgs, PrintTaskOptionDetails,
};
use windows::Foundation::{IPropertyValue, PropertyValue};
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_IGNORE, D2D1_COLOR_F, D2D1_PIXEL_FORMAT, D2D_RECT_F, D2D_SIZE_F,
    D2D_SIZE_U,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1_BITMAP_OPTIONS_TARGET, D2D1_BITMAP_PROPERTIES1,
    D2D1_DEVICE_CONTEXT_OPTIONS_NONE, D2D1_DRAW_TEXT_OPTIONS_NONE, D2D1CreateDevice,
    ID2D1Device, ID2D1DeviceContext,
};
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_11_0};
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION, ID3D11Device,
};
use windows::Win32::Graphics::DirectWrite::{
    DWriteCreateFactory, IDWriteFactory, DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_NORMAL,
    DWRITE_FONT_STYLE_ITALIC, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_BOLD,
    DWRITE_FONT_WEIGHT_NORMAL, DWRITE_MEASURING_MODE_NATURAL, DWRITE_WORD_WRAPPING_NO_WRAP,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Imaging::{CLSID_WICImagingFactory, IWICImagingFactory};
use windows::Win32::Graphics::Printing::{
    FinalPageCount, IPrintPreviewDxgiPackageTarget,
};
use windows::Win32::Storage::Xps::Printing::IPrintDocumentPackageTarget;
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
use windows::Win32::System::WinRT::Printing::{
    IPrintDocumentPageSource, IPrintDocumentPageSource_Impl, IPrintManagerInterop,
    IPrintPreviewPageCollection, IPrintPreviewPageCollection_Impl,
};
use windows::Win32::System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_SINGLETHREADED};
use windows_future::{AsyncOperationCompletedHandler, IAsyncOperation};

const MAX_FINAL_RUNS_PER_CALLBACK: usize = 65_536;
const MAX_FINAL_TEXT_BYTES_PER_CALLBACK: usize = 8 * 1024 * 1024;
const CALLBACK_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_PREVIEW_EDGE_DIP: f32 = 16_384.0;
const MAX_PREVIEW_PIXELS: usize = 16 * 1024 * 1024;
const TITLE_PAGE_OPTION_ID: &str = "com.scriptcraft.dev.print.title-page.v1";
const TITLE_PAGE_INCLUDE_VALUE: &str = "include";
const TITLE_PAGE_OMIT_VALUE: &str = "omit";
const MAX_RUNS_PER_CALLBACK_PAGE: usize = 8_192;
const MAX_TEXT_BYTES_PER_CALLBACK_PAGE: usize = 1 * 1024 * 1024;
const MAX_FINAL_PAGES_PER_CALLBACK: usize = 512;
static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

fn next_request_id() -> Option<u64> {
    let mut current = NEXT_REQUEST_ID.load(Ordering::Relaxed);
    loop {
        let next = current.checked_add(1)?;
        match NEXT_REQUEST_ID.compare_exchange_weak(
            current,
            next,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => return Some(current),
            Err(observed) => current = observed,
        }
    }
}

#[derive(Clone)]
pub(super) struct PreparedPackages {
    request_id: u64,
    request_revision: u64,
    job_title: String,
    include: Arc<document::WindowsPrintPackage>,
    omit: Arc<document::WindowsPrintPackage>,
}

impl PreparedPackages {
    fn package(&self, include_title_page: bool) -> &Arc<document::WindowsPrintPackage> {
        if include_title_page { &self.include } else { &self.omit }
    }

    fn cache_key(&self, include_title_page: bool) -> PackageCacheKey {
        PackageCacheKey {
            request_id: self.request_id,
            include_title_page,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PackageCacheKey {
    request_id: u64,
    include_title_page: bool,
}

fn title_page_choice(value: &str) -> Option<bool> {
    match value {
        TITLE_PAGE_INCLUDE_VALUE => Some(true),
        TITLE_PAGE_OMIT_VALUE => Some(false),
        _ => None,
    }
}

fn changed_selection(current: (bool, u64), include: bool) -> Option<(bool, u64)> {
    if current.0 == include {
        Some(current)
    } else {
        Some((include, current.1.checked_add(1)?))
    }
}

fn page_cache_is_current(paginated_generation: u64, selected_generation: u64) -> bool {
    paginated_generation == selected_generation
}

/// Builds both immutable title-page variants off the UI/COM callback threads.
/// Each result is scoped to one captured request; later jobs always recapture.
pub(super) struct PrintPreparation {
    result: Receiver<Result<PreparedPackages, StartFailure>>,
    cancelled: Arc<AtomicBool>,
}

impl PrintPreparation {
    pub(super) fn start(
        project: scriptcraft_core::Project,
        inputs: scriptcraft_core::LayoutInputs,
        request_revision: u64,
        job_title: String,
    ) -> Result<Self, StartFailure> {
        let request_id = next_request_id().ok_or(StartFailure::PreparationFailed)?;
        let (sender, result) = mpsc::channel();
        let cancelled = Arc::new(AtomicBool::new(false));
        let worker_cancelled = Arc::clone(&cancelled);
        std::thread::Builder::new()
            .name("scriptcraft-print-package".to_owned())
            .spawn(move || {
                let include = document::build_package(&project, &inputs, true)
                    .map_err(|_| StartFailure::PreparationFailed);
                let prepared = match include {
                    Ok(include) if !worker_cancelled.load(Ordering::Acquire) => {
                        document::build_package(&project, &inputs, false)
                            .map(|omit| PreparedPackages {
                                request_id,
                                request_revision,
                                job_title,
                                include: Arc::new(include),
                                omit: Arc::new(omit),
                            })
                            .map_err(|_| StartFailure::PreparationFailed)
                    }
                    Ok(_) => return,
                    Err(error) => Err(error),
                };
                let _ = sender.send(prepared);
            })
            .map_err(|_| StartFailure::PreparationFailed)?;
        Ok(Self { result, cancelled })
    }

    pub(super) fn take_result(&self) -> Option<Result<PreparedPackages, StartFailure>> {
        match self.result.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Err(StartFailure::PreparationFailed)),
        }
    }
}

impl Drop for PrintPreparation {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PrintResult {
    Submitted,
    Completed,
    Cancelled,
    Failed,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StartFailure {
    UnsupportedWindow,
    ApartmentUnavailable,
    ManagerUnavailable,
    RegistrationFailed,
    UiUnavailable,
    PreparationFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SessionEvent {
    Submitted,
    TaskCompleted(PrintResult),
    UiCompleted(bool),
    Failed,
}

/// Owns the one event registration, async print UI, result channel, and WinRT
/// apartment reference. The owner window and this value are used only on the
/// eframe UI thread. Async callbacks only send bounded, copyable state events.
pub(super) struct PrintSession {
    manager: PrintManager,
    requested_token: Option<i64>,
    events: Receiver<SessionEvent>,
    apartment: Option<WinRtApartment>,
    owner_thread: ThreadId,
    completion_started_at: Option<Instant>,
    ui_shown: Option<bool>,
    task_result: Option<PrintResult>,
    submitted: bool,
    submitted_reported: bool,
    terminal_error: bool,
    terminal_reported: bool,
}

impl PrintSession {
    /// Start the modern system print UI for eframe's borrowed owner HWND. This
    /// never waits synchronously: all UI/task completion arrives through events.
    #[allow(unsafe_code)]
    pub(super) fn start(
        frame: &eframe::Frame,
        packages: PreparedPackages,
    ) -> Result<Self, StartFailure> {
        let handle = frame
            .window_handle()
            .map_err(|_| StartFailure::UnsupportedWindow)?;
        let RawWindowHandle::Win32(owner) = handle.as_raw() else {
            return Err(StartFailure::UnsupportedWindow);
        };
        let hwnd = HWND(owner.hwnd.get() as *mut _);

        let apartment = WinRtApartment::initialize()?;
        // SAFETY: PrintManagerInterop is obtained from the Windows Runtime
        // activation factory on the current UI thread. `hwnd` is copied from
        // eframe's live borrowed Win32 handle and is used before leaving this
        // callback; the session is polled and dropped on that same UI thread.
        let interop: IPrintManagerInterop = unsafe {
            windows::Win32::System::WinRT::RoGetActivationFactory(
                &HSTRING::from("Windows.Graphics.Printing.PrintManager"),
            )
        }
        .map_err(|_| StartFailure::ManagerUnavailable)?;
        // SAFETY: The HWND is the actual active eframe viewport owner, not an
        // unrelated foreground window. WinRT calls remain on its UI thread.
        let manager: PrintManager = unsafe { interop.GetForWindow(hwnd) }
            .map_err(|_| StartFailure::ManagerUnavailable)?;

        let (sender, events) = mpsc::channel();
        let task_sender = sender.clone();
        let task_packages = packages.clone();
        let requested = TypedEventHandler::<PrintManager, PrintTaskRequestedEventArgs>::new(
            move |_, args| {
                let Some(args) = args.as_ref() else {
                    let _ = task_sender.send(SessionEvent::Failed);
                    return Ok(());
                };
                match create_screenplay_print_task(args, task_sender.clone(), task_packages.clone()) {
                    Ok(()) => Ok(()),
                    Err(_) => {
                        let _ = task_sender.send(SessionEvent::Failed);
                        Err(callback_error())
                    }
                }
            },
        );
        let requested_token = manager
            .PrintTaskRequested(&requested)
            .map_err(|_| StartFailure::RegistrationFailed)?;

        // SAFETY: The owner HWND remains owned by eframe while this session is
        // active. ShowPrintUIForWindowAsync is observed by a callback; the app
        // event loop is not blocked and no synchronous modal wait is used.
        let operation = match unsafe {
            interop.ShowPrintUIForWindowAsync::<IAsyncOperation<bool>>(hwnd)
        } {
            Ok(operation) => operation,
            Err(_) => {
                let _ = manager.RemovePrintTaskRequested(requested_token);
                return Err(StartFailure::UiUnavailable);
            }
        };
        let completion_sender = sender.clone();
        let completed = AsyncOperationCompletedHandler::<bool>::new(move |operation, _status| {
            let shown = operation
                .as_ref()
                .ok_or_else(callback_error)
                .and_then(|operation| operation.GetResults())
                .unwrap_or(false);
            let _ = completion_sender.send(SessionEvent::UiCompleted(shown));
            Ok(())
        });
        if operation.SetCompleted(&completed).is_err() {
            let _ = manager.RemovePrintTaskRequested(requested_token);
            return Err(StartFailure::UiUnavailable);
        }

        Ok(Self {
            manager,
            requested_token: Some(requested_token),
            events,
            apartment: Some(apartment),
            owner_thread: std::thread::current().id(),
            completion_started_at: None,
            ui_shown: None,
            task_result: None,
            submitted: false,
            submitted_reported: false,
            terminal_error: false,
            terminal_reported: false,
        })
    }

    pub(super) fn take_submitted(&mut self) -> bool {
        if self.submitted && !self.submitted_reported {
            self.submitted_reported = true;
            true
        } else {
            false
        }
    }

    /// Drain only queued events, never block. Returns exactly one terminal
    /// result; a stalled callback/UI session is bounded by CALLBACK_TIMEOUT.
    pub(super) fn poll(&mut self) -> Option<PrintResult> {
        debug_assert_eq!(self.owner_thread, std::thread::current().id());
        if self.terminal_reported {
            return None;
        }
        loop {
            match self.events.try_recv() {
                Ok(SessionEvent::Submitted) => {
                    self.submitted = true;
                    self.completion_started_at.get_or_insert_with(Instant::now);
                }
                Ok(SessionEvent::TaskCompleted(result)) => {
                    self.task_result = Some(result);
                    self.completion_started_at.get_or_insert_with(Instant::now);
                }
                Ok(SessionEvent::UiCompleted(shown)) => {
                    self.ui_shown = Some(shown);
                    self.completion_started_at.get_or_insert_with(Instant::now);
                }
                Ok(SessionEvent::Failed) => self.terminal_error = true,
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.terminal_error = true;
                    break;
                }
            }
        }

        if self
            .completion_started_at
            .is_some_and(callback_deadline_expired)
            && (self.task_result.is_none() || self.ui_shown.is_none())
        {
            self.terminal_error = true;
        }
        if self.terminal_error {
            self.unregister();
            return take_terminal_result(&mut self.terminal_reported, Some(PrintResult::Failed));
        }
        let result = terminal_result(self.ui_shown, self.task_result);
        if let Some(result) = result {
            self.unregister();
            return take_terminal_result(&mut self.terminal_reported, Some(result));
        }
        None
    }

    fn unregister(&mut self) {
        if let Some(token) = take_registration_token(&mut self.requested_token) {
            let _ = self.manager.RemovePrintTaskRequested(token);
        }
    }
}

impl Drop for PrintSession {
    fn drop(&mut self) {
        debug_assert_eq!(self.owner_thread, std::thread::current().id());
        self.unregister();
    }
}

/// RoInitialize returns success for both S_OK and S_FALSE; each successful
/// increment is paired exactly once with RoUninitialize on the same UI thread.
struct WinRtApartment {
    owner_thread: ThreadId,
}

impl WinRtApartment {
    #[allow(unsafe_code)]
    fn initialize() -> Result<Self, StartFailure> {
        // SAFETY: Called on the eframe UI thread. We accept only a successful
        // single-threaded WinRT initialization; changed-mode and other failures
        // are mapped to a bounded unsupported result and never reconfigured.
        unsafe { RoInitialize(RO_INIT_SINGLETHREADED) }
            .map_err(|_| StartFailure::ApartmentUnavailable)?;
        Ok(Self {
            owner_thread: std::thread::current().id(),
        })
    }
}

#[allow(unsafe_code)]
impl Drop for WinRtApartment {
    fn drop(&mut self) {
        debug_assert_eq!(self.owner_thread, std::thread::current().id());
        // SAFETY: This guard is constructed only after a successful RoInitialize
        // on this thread and is dropped exactly once after session unregistration.
        unsafe { RoUninitialize() };
    }
}

fn create_screenplay_print_task(
    args: &PrintTaskRequestedEventArgs,
    sender: Sender<SessionEvent>,
    packages: PreparedPackages,
) -> WinResult<()> {
    let request = args.Request()?;
    let job_title = packages.job_title.clone();
    let source_state = Arc::new(SourceState::new(packages));
    let option_state = Arc::clone(&source_state);
    let source_requested = PrintTaskSourceRequestedHandler::new(move |args| {
        let args = args.as_ref().ok_or_else(callback_error)?;
        let source_interface: IPrintDocumentSource = PrintSource::new(Arc::clone(&source_state)).into();
        args.SetSource(&source_interface)
    });
    let task = request.CreatePrintTask(&HSTRING::from(job_title.as_str()), &source_requested)?;

    let task_options = task.Options()?;
    let option_details = PrintTaskOptionDetails::GetFromPrintTaskOptions(&task_options)?;
    let title_option = option_details.CreateItemListOption(
        &HSTRING::from(TITLE_PAGE_OPTION_ID),
        &HSTRING::from("Title page"),
    )?;
    title_option.AddItem(
        &HSTRING::from(TITLE_PAGE_INCLUDE_VALUE),
        &HSTRING::from("Include"),
    )?;
    title_option.AddItem(
        &HSTRING::from(TITLE_PAGE_OMIT_VALUE),
        &HSTRING::from("Omit"),
    )?;
    let include_value = PropertyValue::CreateString(&HSTRING::from(TITLE_PAGE_INCLUDE_VALUE))?;
    if !title_option.TrySetValue(&include_value)? {
        return Err(callback_error());
    }
    // Display only built-in printer settings plus the one screenplay-specific
    // choice. No second screenplay option is introduced.
    let displayed = option_details.DisplayedOptions()?;
    for option in [
        StandardPrintTaskOptions::Copies()?,
        StandardPrintTaskOptions::Collation()?,
        StandardPrintTaskOptions::ColorMode()?,
        StandardPrintTaskOptions::Duplex()?,
        StandardPrintTaskOptions::MediaSize()?,
        StandardPrintTaskOptions::Orientation()?,
        StandardPrintTaskOptions::PrintQuality()?,
        StandardPrintTaskOptions::CustomPageRanges()?,
    ] {
        displayed.Append(&option)?;
    }
    displayed.Append(&HSTRING::from(TITLE_PAGE_OPTION_ID))?;

    let changed_state = Arc::clone(&option_state);
    let changed_option = title_option.clone();
    let option_changed = TypedEventHandler::<PrintTaskOptionDetails, PrintTaskOptionChangedEventArgs>::new(move |_, args| {
        let args = args.as_ref().ok_or_else(callback_error)?;
        let changed_id = args.OptionId()?.cast::<IPropertyValue>()?.GetString()?;
        if changed_id == HSTRING::from(TITLE_PAGE_OPTION_ID) {
            let selected = changed_option.Value()?.cast::<IPropertyValue>()?.GetString()?;
            let selected_text = selected.to_string_lossy();
            let include = title_page_choice(&selected_text).ok_or_else(callback_error)?;
            changed_state.change_title_page_option(include)?;
        }
        Ok(())
    });
    let option_token = option_details.OptionChanged(&option_changed)?;

    let submitted_sender = sender.clone();
    let submitting = TypedEventHandler::new(move |_, _| {
        let _ = submitted_sender.send(SessionEvent::Submitted);
        Ok(())
    });
    if task.Submitting(&submitting).is_err() {
        let _ = option_details.RemoveOptionChanged(option_token);
        return Err(callback_error());
    }
    let completed_details = option_details.clone();
    let completed = TypedEventHandler::<PrintTask, PrintTaskCompletedEventArgs>::new(
        move |_, args| {
            let _ = completed_details.RemoveOptionChanged(option_token);
            let result = args
                .ok()
                .and_then(|args| args.Completion())
                .map(map_task_completion)
                .unwrap_or(PrintResult::Failed);
            let _ = sender.send(SessionEvent::TaskCompleted(result));
            Ok(())
        },
    );
    if task.Completed(&completed).is_err() {
        let _ = option_details.RemoveOptionChanged(option_token);
        return Err(callback_error());
    }
    Ok(())
}

fn map_task_completion(completion: PrintTaskCompletion) -> PrintResult {
    if completion == PrintTaskCompletion::Submitted {
        PrintResult::Completed
    } else if completion == PrintTaskCompletion::Canceled {
        PrintResult::Cancelled
    } else {
        PrintResult::Failed
    }
}

fn callback_error() -> WindowsError {
    WindowsError::from_hresult(HRESULT(0x8000_4005_u32 as i32))
}

fn callback_deadline_expired(started: Instant) -> bool {
    started.elapsed() >= CALLBACK_TIMEOUT
}

fn take_terminal_result(
    reported: &mut bool,
    result: Option<PrintResult>,
) -> Option<PrintResult> {
    if *reported {
        return None;
    }
    let result = result?;
    *reported = true;
    Some(result)
}

fn take_registration_token(token: &mut Option<i64>) -> Option<i64> {
    token.take()
}

fn terminal_result(
    ui_shown: Option<bool>,
    task_result: Option<PrintResult>,
) -> Option<PrintResult> {
    match (ui_shown, task_result) {
        (Some(false), Some(result)) => Some(result),
        (Some(false), None) => Some(PrintResult::Cancelled),
        (Some(true), Some(result)) => Some(result),
        _ => None,
    }
}

struct SourceState {
    packages: PreparedPackages,
    selection: Mutex<(bool, u64)>,
    paginated_generation: AtomicU64,
}

impl SourceState {
    fn new(packages: PreparedPackages) -> Self {
        Self {
            packages,
            selection: Mutex::new((true, 0)),
            paginated_generation: AtomicU64::new(u64::MAX),
        }
    }

    fn selected(&self) -> WinResult<(Arc<document::WindowsPrintPackage>, PackageCacheKey, u64)> {
        let (include, generation) = *self.selection.lock().map_err(|_| callback_error())?;
        Ok((
            Arc::clone(self.packages.package(include)),
            self.packages.cache_key(include),
            generation,
        ))
    }

    fn change_title_page_option(&self, include: bool) -> WinResult<()> {
        let mut selection = self.selection.lock().map_err(|_| callback_error())?;
        let next = changed_selection(*selection, include).ok_or_else(callback_error)?;
        if next != *selection {
            *selection = next;
            self.paginated_generation.store(u64::MAX, Ordering::Release);
        }
        Ok(())
    }
}

// Isolate generated COM ABI glue and native handle operations to this module.
#[allow(unsafe_code)]
mod custom_source {
    use super::*;

    #[implement(IPrintDocumentSource, IPrintDocumentPageSource, IPrintPreviewPageCollection)]
    pub(super) struct PrintSource {
        state: Arc<SourceState>,
        preview_target: Mutex<Option<IPrintPreviewDxgiPackageTarget>>,
    }

    impl PrintSource {
        pub(super) fn new(state: Arc<SourceState>) -> Self {
            Self {
                state,
                preview_target: Mutex::new(None),
            }
        }
    }

    impl IPrintDocumentSource_Impl for PrintSource_Impl {}

    impl IPrintDocumentPageSource_Impl for PrintSource_Impl {
        fn GetPreviewPageCollection(
            &self,
            target: Ref<IPrintDocumentPackageTarget>,
        ) -> WinResult<IPrintPreviewPageCollection> {
            let target = target.as_ref().ok_or_else(callback_error)?;
            // SAFETY: The target is the live package target supplied to this source
            // callback. The IID comes from the SDK projection for the exact DXGI
            // preview package-target interface used by Microsoft's Win2D sample.
            let preview = unsafe {
                target.GetPackageTarget::<IPrintPreviewDxgiPackageTarget>(
                    &IPrintPreviewDxgiPackageTarget::IID,
                )
            }?;
            *self.preview_target
                .lock()
                .map_err(|_| callback_error())? = Some(preview);
            Ok(self.to_interface())
        }

        fn MakeDocument(
            &self,
            options: Ref<windows::core::IInspectable>,
            target: Ref<IPrintDocumentPackageTarget>,
        ) -> WinResult<()> {
            let options_inspectable = options
                .as_ref()
                .ok_or_else(callback_error)?;
            let options_core = options_inspectable.cast::<IPrintTaskOptionsCore>()?;
            let options_ranges = options_inspectable.cast::<PrintTaskOptions>()?;
            let target = target.as_ref().ok_or_else(callback_error)?;
            let (package, _cache_key, generation) = self.state.selected()?;
            let selected_pages = selected_final_pages(&package, &options_ranges)?;
            write_final_pages(&package, &selected_pages, &options_core, target)?;
            if self.state.selected()?.2 != generation {
                return Err(callback_error());
            }
            Ok(())
        }
    }

    impl IPrintPreviewPageCollection_Impl for PrintSource_Impl {
        fn Paginate(
            &self,
            _current_job_page: u32,
            options: Ref<windows::core::IInspectable>,
        ) -> WinResult<()> {
            let target = self
                .preview_target
                .lock()
                .map_err(|_| callback_error())?
                .clone()
                .ok_or_else(callback_error)?;
            let options = options.as_ref().ok_or_else(callback_error)?;
            let _ = options.cast::<IPrintTaskOptionsCore>()?;
            let (package, _cache_key, generation) = self.state.selected()?;
            let page_count = u32::try_from(package.pages().len()).map_err(|_| callback_error())?;
            if page_count == 0 || page_count as usize != package.pages().len() {
                return Err(callback_error());
            }
            // Package preparation completed before UI launch, so this request
            // reports a final count rather than publishing a speculative count.
            self.state
                .paginated_generation
                .store(generation, Ordering::Release);
            if let Err(error) = unsafe { target.SetJobPageCount(FinalPageCount, page_count) } {
                self.state
                    .paginated_generation
                    .store(u64::MAX, Ordering::Release);
                return Err(error);
            }
            Ok(())
        }

        fn MakePage(&self, page: u32, width: f32, height: f32) -> WinResult<()> {
            if page == 0
                || !width.is_finite()
                || !height.is_finite()
                || width <= 0.0
                || height <= 0.0
                || width > MAX_PREVIEW_EDGE_DIP
                || height > MAX_PREVIEW_EDGE_DIP
            {
                return Err(callback_error());
            }
            let (package, _cache_key, generation) = self.state.selected()?;
            if !page_cache_is_current(
                self.state.paginated_generation.load(Ordering::Acquire),
                generation,
            ) {
                return Err(callback_error());
            }
            let page_model = package
                .pages()
                .get(usize::try_from(page - 1).map_err(|_| callback_error())?)
                .ok_or_else(callback_error)?;
            let target = self
                .preview_target
                .lock()
                .map_err(|_| callback_error())?
                .clone()
                .ok_or_else(callback_error)?;
            draw_preview_page(
                page,
                page_model,
                &target,
                &self.state,
                generation,
                width,
                height,
            )
        }
    }
}

use custom_source::PrintSource;

#[allow(unsafe_code)]
fn selected_final_pages(
    package: &document::WindowsPrintPackage,
    options: &PrintTaskOptions,
) -> WinResult<Vec<usize>> {
    let ranges = options.CustomPageRanges()?;
    let mut page_ranges = Vec::new();
    for range in ranges {
        page_ranges.push((range.FirstPageNumber()?, range.LastPageNumber()?));
    }
    normalize_page_ranges(package.pages().len(), &page_ranges).ok_or_else(callback_error)
}

fn normalize_page_ranges(page_count: usize, ranges: &[(i32, i32)]) -> Option<Vec<usize>> {
    if page_count == 0 || page_count > MAX_FINAL_PAGES_PER_CALLBACK {
        return None;
    }
    if ranges.is_empty() {
        return Some((0..page_count).collect());
    }
    if ranges.len() > MAX_FINAL_PAGES_PER_CALLBACK {
        return None;
    }
    let page_count = i32::try_from(page_count).ok()?;
    let mut selected = Vec::new();
    for &(first, last) in ranges {
        if first < 1 || last < first || last > page_count {
            return None;
        }
        let range_len = usize::try_from(last - first + 1).ok()?;
        if selected.len().checked_add(range_len)? > MAX_FINAL_PAGES_PER_CALLBACK {
            return None;
        }
        selected.extend((first..=last).map(|page| (page - 1) as usize));
    }
    (!selected.is_empty()).then_some(selected)
}

#[allow(unsafe_code)]
fn write_final_pages(
    package: &document::WindowsPrintPackage,
    selected_pages: &[usize],
    options: &IPrintTaskOptionsCore,
    target: &IPrintDocumentPackageTarget,
) -> WinResult<()> {
    let started = Instant::now();
    let mut total_runs = 0usize;
    let mut total_text_bytes = 0usize;
    for &page_index in selected_pages {
        if callback_deadline_expired(started) {
            return Err(callback_error());
        }
        let page = package.pages().get(page_index).ok_or_else(callback_error)?;
        total_runs = total_runs.checked_add(page.runs().len()).ok_or_else(callback_error)?;
        total_text_bytes = page
            .runs()
            .iter()
            .try_fold(total_text_bytes, |total, run| total.checked_add(run.text().len()))
            .ok_or_else(callback_error)?;
        if total_runs > MAX_FINAL_RUNS_PER_CALLBACK
            || total_text_bytes > MAX_FINAL_TEXT_BYTES_PER_CALLBACK
        {
            return Err(callback_error());
        }
    }
    let device = create_d2d_device()?;
    let context = unsafe { device.CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE) }?;
    let wic: IWICImagingFactory = unsafe {
        CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)
    }?;
    // SAFETY: The print control is created with this callback's live package
    // target and its owned D2D device. The WIC factory and command list outlive
    // page addition and Close; no package stream or printer is opened directly.
    let print_control = unsafe { device.CreatePrintControl(&wic, target, None) }?;
    for (job_index, &page_index) in selected_pages.iter().enumerate() {
        if job_index >= MAX_FINAL_PAGES_PER_CALLBACK || callback_deadline_expired(started) {
            return Err(callback_error());
        }
        let page = package.pages().get(page_index).ok_or_else(callback_error)?;
        let job_page_number = u32::try_from(job_index + 1).map_err(|_| callback_error())?;
        let description: PrintPageDescription = unsafe { options.GetPageDescription(job_page_number) }?;
        let page_size = D2D_SIZE_F {
            width: description.PageSize.Width,
            height: description.PageSize.Height,
        };
        let imageable = description.ImageableRect;
        let transform = page_transform(
            page,
            page_size.width,
            page_size.height,
            Some((imageable.X, imageable.Y, imageable.Width, imageable.Height)),
        )
        .ok_or_else(callback_error)?;
        let command_list = unsafe { context.CreateCommandList() }?;
        unsafe { context.SetTarget(&command_list) };
        unsafe { context.BeginDraw() };
        unsafe { context.Clear(Some(&D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 1.0 })) };
        draw_vector_page(&context, page, transform)?;
        if callback_deadline_expired(started) {
            return Err(callback_error());
        }
        unsafe { context.EndDraw(None, None) }?;
        unsafe { command_list.Close() }?;
        // Each selected source page is added once, in selected range order, at
        // the current system job page description. Direct2D consumes the vector
        // command list and page-size description; no second layout/pagination.
        unsafe { print_control.AddPage(&command_list, page_size, None, None, None)? };
    }
    unsafe { print_control.Close() }?;
    if callback_deadline_expired(started) {
        return Err(callback_error());
    }
    Ok(())
}

#[allow(unsafe_code)]
fn draw_preview_page(
    page_number: u32,
    page: &document::VectorPage,
    target: &IPrintPreviewDxgiPackageTarget,
    state: &SourceState,
    generation: u64,
    width: f32,
    height: f32,
) -> WinResult<()> {
    let started = Instant::now();
    let (pixel_width, pixel_height) = preview_bitmap_dimensions(width, height)
        .ok_or_else(callback_error)?;
    let device = create_d2d_device()?;
    let context = unsafe { device.CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE) }?;
    let properties = D2D1_BITMAP_PROPERTIES1 {
        pixelFormat: D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: D2D1_ALPHA_MODE_IGNORE,
        },
        dpiX: 96.0,
        dpiY: 96.0,
        bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET,
        colorContext: ManuallyDrop::new(None),
    };
    let bitmap = unsafe {
        context.CreateBitmap(
            D2D_SIZE_U {
                width: pixel_width,
                height: pixel_height,
            },
            None,
            0,
            &properties,
        )
    }?;
    if callback_deadline_expired(started) {
        return Err(callback_error());
    }
    unsafe { context.SetTarget(&bitmap) };
    unsafe { context.BeginDraw() };
    unsafe { context.Clear(Some(&D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 1.0 })) };
    let transform = page_transform(page, width, height, None).ok_or_else(callback_error)?;
    draw_vector_page(&context, page, transform)?;
    unsafe { context.EndDraw(None, None) }?;
    if callback_deadline_expired(started) || state.selected()?.2 != generation {
        return Err(callback_error());
    }
    // SAFETY: The bitmap is a D2D target bitmap backed by a DXGI surface and is
    // retained through DrawPage. The preview target belongs to the supplied
    // package-target bridge and consumes this page's single rendered surface.
    let surface = unsafe { bitmap.GetSurface() }?;
    unsafe { target.DrawPage(page_number, &surface, 96.0, 96.0) }
}

#[allow(unsafe_code)]
fn create_d2d_device() -> WinResult<ID2D1Device> {
    let mut d3d_device: Option<ID3D11Device> = None;
    let mut feature_level = D3D_FEATURE_LEVEL_11_0;
    // SAFETY: D3D11 receives output slots initialized to None, requests only a
    // BGRA-capable hardware device for Direct2D, and retains the returned device.
    unsafe {
        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            windows::Win32::Foundation::HMODULE::default(),
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            Some(&[D3D_FEATURE_LEVEL_11_0]),
            D3D11_SDK_VERSION,
            Some(&mut d3d_device),
            Some(&mut feature_level),
            None,
        )
    }?;
    let dxgi_device = d3d_device
        .ok_or_else(callback_error)?
        .cast::<windows::Win32::Graphics::Dxgi::IDXGIDevice>()?;
    // SAFETY: DXGI device is a retained D3D11 device queried for the exact
    // IDXGIDevice interface expected by Direct2D; no raw pointer escapes.
    unsafe { D2D1CreateDevice(&dxgi_device, None) }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PageTransform {
    scale: f32,
    offset_x: f32,
    offset_y: f32,
}

/// Fit the canonical page into the supplied page/imageable rectangle once.
/// Source dimensions and coordinates are converted from hundredths-of-a-point
/// to DIPs here; final AddPage receives the printer's native page size without
/// a second application-side scale.
fn page_transform(
    page: &document::VectorPage,
    target_width: f32,
    target_height: f32,
    imageable: Option<(f32, f32, f32, f32)>,
) -> Option<PageTransform> {
    let (page_width, page_height) = page.size();
    let source_width = page_width.to_points_f32() * (96.0 / 72.0);
    let source_height = page_height.to_points_f32() * (96.0 / 72.0);
    fit_page_transform(
        source_width,
        source_height,
        target_width,
        target_height,
        imageable,
    )
}

fn fit_page_transform(
    source_width: f32,
    source_height: f32,
    target_width: f32,
    target_height: f32,
    imageable: Option<(f32, f32, f32, f32)>,
) -> Option<PageTransform> {
    let (x, y, width, height) = imageable.unwrap_or((0.0, 0.0, target_width, target_height));
    if ![x, y, width, height, source_width, source_height]
        .into_iter()
        .all(f32::is_finite)
        || target_width <= 0.0
        || target_height <= 0.0
        || target_width > MAX_PREVIEW_EDGE_DIP
        || target_height > MAX_PREVIEW_EDGE_DIP
        || width <= 0.0
        || height <= 0.0
        || source_width <= 0.0
        || source_height <= 0.0
    {
        return None;
    }
    let scale = (width / source_width).min(height / source_height);
    let transform = PageTransform {
        scale,
        offset_x: x + (width - source_width * scale) / 2.0,
        offset_y: y + (height - source_height * scale) / 2.0,
    };
    [transform.scale, transform.offset_x, transform.offset_y]
        .into_iter()
        .all(f32::is_finite)
        .then_some(transform)
}

fn preview_bitmap_dimensions(width: f32, height: f32) -> Option<(u32, u32)> {
    if !width.is_finite()
        || !height.is_finite()
        || width <= 0.0
        || height <= 0.0
        || width > MAX_PREVIEW_EDGE_DIP
        || height > MAX_PREVIEW_EDGE_DIP
    {
        return None;
    }
    let pixel_width = width.ceil().max(1.0) as u32;
    let pixel_height = height.ceil().max(1.0) as u32;
    let pixels = usize::try_from(pixel_width)
        .ok()?
        .checked_mul(usize::try_from(pixel_height).ok()?)?;
    (pixels <= MAX_PREVIEW_PIXELS).then_some((pixel_width, pixel_height))
}

#[allow(unsafe_code)]
fn draw_vector_page(
    context: &ID2D1DeviceContext,
    page: &document::VectorPage,
    transform: PageTransform,
) -> WinResult<()> {
    let started = Instant::now();
    let mut text_bytes = 0usize;
    if page.runs().len() > MAX_RUNS_PER_CALLBACK_PAGE {
        return Err(callback_error());
    }
    let mut total_runs = 0usize;
    for run in page.runs() {
        text_bytes = text_bytes
            .checked_add(run.text().len())
            .ok_or_else(callback_error)?;
        if text_bytes > MAX_TEXT_BYTES_PER_CALLBACK_PAGE {
            return Err(callback_error());
        }
        total_runs += 1;
    }

    let factory: IDWriteFactory = unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED) }?;
    let ink = D2D1_COLOR_F {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    let brush = unsafe { context.CreateSolidColorBrush(&ink, None) }?;
    let locale = HSTRING::from("en-us");

    for (index, run) in page.runs().iter().enumerate() {
        if index.is_multiple_of(64) && callback_deadline_expired(started) {
            return Err(callback_error());
        }
        if run.text().is_empty() {
            continue;
        }
        let geometry = run.geometry();
        let rect = scaled_run_rect(geometry, transform).ok_or_else(callback_error)?;
        if ![rect.left, rect.top, rect.right, rect.bottom]
            .into_iter()
            .all(f32::is_finite)
            || rect.right <= rect.left
            || rect.bottom <= rect.top
        {
            return Err(callback_error());
        }
        let bold = matches!(
            run.face(),
            document::PrintFontFace::CourierNewBold
                | document::PrintFontFace::CourierNewBoldItalic
        );
        let italic = matches!(
            run.face(),
            document::PrintFontFace::CourierNewItalic
                | document::PrintFontFace::CourierNewBoldItalic
        );
        let face = HSTRING::from(run.face().family_name());
        let format = unsafe {
            factory.CreateTextFormat(
                windows::core::PCWSTR(face.as_ptr()),
                None,
                if bold { DWRITE_FONT_WEIGHT_BOLD } else { DWRITE_FONT_WEIGHT_NORMAL },
                if italic { DWRITE_FONT_STYLE_ITALIC } else { DWRITE_FONT_STYLE_NORMAL },
                DWRITE_FONT_STRETCH_NORMAL,
                f32::from(run.font_size_pt()) * (96.0 / 72.0) * transform.scale,
                windows::core::PCWSTR(locale.as_ptr()),
            )
        }?;
        // SAFETY: The format is a live DirectWrite object created on this
        // callback thread and the enum is a valid SDK word-wrapping value.
        unsafe { format.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP) }?;
        let text = run.text().encode_utf16().collect::<Vec<_>>();
        unsafe {
            context.DrawText(
                &text,
                &format,
                &rect,
                &brush,
                D2D1_DRAW_TEXT_OPTIONS_NONE,
                DWRITE_MEASURING_MODE_NATURAL,
            )
        };

        let (underline, strikethrough) = decoration_rects(rect, transform.scale);
        if run.underline() {
            let line = underline;
            unsafe { context.FillRectangle(&line, &brush) };
        }
        if run.strikethrough() {
            let line = strikethrough;
            unsafe { context.FillRectangle(&line, &brush) };
        }
    }
    if total_runs > MAX_RUNS_PER_CALLBACK_PAGE || text_bytes > MAX_TEXT_BYTES_PER_CALLBACK_PAGE {
        return Err(callback_error());
    }
    Ok(())
}

fn scaled_run_rect(geometry: scriptcraft_core::layout::FixedRect, transform: PageTransform) -> Option<D2D_RECT_F> {
    let point_to_dip = 96.0 / 72.0;
    let rect = D2D_RECT_F {
        left: transform.offset_x + geometry.x.to_points_f32() * point_to_dip * transform.scale,
        top: transform.offset_y + geometry.y.to_points_f32() * point_to_dip * transform.scale,
        right: transform.offset_x
            + (geometry.x.to_points_f32() + geometry.width.to_points_f32())
                * point_to_dip
                * transform.scale,
        bottom: transform.offset_y
            + (geometry.y.to_points_f32() + geometry.height.to_points_f32())
                * point_to_dip
                * transform.scale,
    };
    ([rect.left, rect.top, rect.right, rect.bottom]
        .into_iter()
        .all(f32::is_finite)
        && rect.right > rect.left
        && rect.bottom > rect.top)
        .then_some(rect)
}

fn decoration_rects(rect: D2D_RECT_F, scale: f32) -> (D2D_RECT_F, D2D_RECT_F) {
    let height = (0.7 * scale).max(0.25);
    let width = rect.right - rect.left;
    let underline_top = rect.bottom - height;
    let strike_top = rect.top + (rect.bottom - rect.top) * 0.52;
    (
        D2D_RECT_F {
            left: rect.left,
            top: underline_top,
            right: rect.left + width,
            bottom: underline_top + height,
        },
        D2D_RECT_F {
            left: rect.left,
            top: strike_top,
            right: rect.left + width,
            bottom: strike_top + height,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::{
        changed_selection, decoration_rects, fit_page_transform, map_task_completion,
        normalize_page_ranges, page_cache_is_current, scaled_run_rect, take_registration_token,
        callback_deadline_expired, take_terminal_result, terminal_result, title_page_choice,
        preview_bitmap_dimensions, PackageCacheKey, PageTransform, PrintResult,
        TITLE_PAGE_INCLUDE_VALUE, TITLE_PAGE_OMIT_VALUE, TITLE_PAGE_OPTION_ID, CALLBACK_TIMEOUT,
    };
    use scriptcraft_core::formatting::FixedPoint;
    use scriptcraft_core::layout::FixedRect;
    use scriptcraft_core::{LayoutInputs, Project, Role, ScreenplayElement};
    use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;
    use windows::Graphics::Printing::PrintTaskCompletion;

    fn valid<T, E>(value: Result<T, E>) -> T {
        value.unwrap_or_else(|_| unreachable!("native print fixtures use valid bounded input"))
    }

    fn project_with_action(text: &str) -> Project {
        let mut project = valid(Project::new());
        project.elements = vec![valid(ScreenplayElement::new("print-action", Role::Action, text))];
        project
    }

    #[test]
    fn task_completion_maps_each_native_terminal_state_explicitly() {
        assert_eq!(
            map_task_completion(PrintTaskCompletion::Submitted),
            PrintResult::Completed
        );
        assert_eq!(
            map_task_completion(PrintTaskCompletion::Canceled),
            PrintResult::Cancelled
        );
        assert_eq!(
            map_task_completion(PrintTaskCompletion::Failed),
            PrintResult::Failed
        );
        assert_eq!(
            map_task_completion(PrintTaskCompletion::Abandoned),
            PrintResult::Failed
        );
    }

    #[test]
    fn stalled_native_callbacks_have_a_finite_poll_deadline() {
        assert_eq!(CALLBACK_TIMEOUT.as_secs(), 30);
        assert!(!callback_deadline_expired(std::time::Instant::now()));
        assert!(callback_deadline_expired(
            std::time::Instant::now() - CALLBACK_TIMEOUT
        ));
    }

    #[test]
    fn every_fresh_request_defaults_to_include_with_stable_two_value_option() {
        assert_eq!(TITLE_PAGE_OPTION_ID, "com.scriptcraft.dev.print.title-page.v1");
        assert_eq!(title_page_choice(TITLE_PAGE_INCLUDE_VALUE), Some(true));
        assert_eq!(title_page_choice(TITLE_PAGE_OMIT_VALUE), Some(false));
        assert_eq!(title_page_choice("other"), None);
        assert_eq!(changed_selection((true, 0), true), Some((true, 0)));
        assert_eq!(changed_selection((true, 0), false), Some((false, 1)));
        assert_eq!(changed_selection((false, 1), true), Some((true, 2)));
    }

    #[test]
    fn each_new_source_resets_title_page_to_include_without_reusing_active_option_state() {
        let project = project_with_action("FRESH_OPTION_SOURCE");
        let inputs = LayoutInputs::default();
        let packages = |request_id| super::PreparedPackages {
            request_id,
            request_revision: request_id,
            job_title: format!("request-{request_id}"),
            include: std::sync::Arc::new(valid(super::document::build_package(
                &project, &inputs, true,
            ))),
            omit: std::sync::Arc::new(valid(super::document::build_package(
                &project, &inputs, false,
            ))),
        };
        let first = super::SourceState::new(packages(61));
        valid(first.change_title_page_option(false));
        assert!(!valid(first.selected()).0.includes_title_page());

        let repeated = super::SourceState::new(packages(62));
        let (package, key, generation) = valid(repeated.selected());
        assert!(package.includes_title_page());
        assert!(key.include_title_page);
        assert_eq!(key.request_id, 62);
        assert_eq!(generation, 0);
        assert_ne!(first.packages.request_id, repeated.packages.request_id);
    }

    #[test]
    fn title_option_invalidates_preview_generation_and_requires_repagination() {
        let initial = (true, 17);
        let omitted = changed_selection(initial, false).unwrap_or(initial);
        assert_eq!(omitted, (false, 18));
        assert!(!page_cache_is_current(17, omitted.1));
        assert!(page_cache_is_current(18, omitted.1));
        assert_eq!(changed_selection(omitted, false), Some(omitted));
    }

    #[test]
    fn source_state_switches_only_between_its_captured_packages_and_repagination_keys() {
        let project = project_with_action("CAPTURED_REVISION_A");
        let inputs = LayoutInputs::default();
        let packages = super::PreparedPackages {
            request_id: 500,
            request_revision: 12,
            job_title: "capture-a".to_owned(),
            include: std::sync::Arc::new(valid(super::document::build_package(&project, &inputs, true))),
            omit: std::sync::Arc::new(valid(super::document::build_package(&project, &inputs, false))),
        };
        let state = super::SourceState::new(packages);
        assert_eq!(state.packages.request_revision, 12);
        assert_eq!(state.packages.job_title, "capture-a");
        let (included, include_key, generation) = valid(state.selected());
        assert!(included.includes_title_page());
        assert!(include_key.include_title_page);
        assert_eq!(generation, 0);

        valid(state.change_title_page_option(false));
        let (omitted, omit_key, generation) = valid(state.selected());
        assert!(!omitted.includes_title_page());
        assert!(!omit_key.include_title_page);
        assert_eq!(include_key.request_id, omit_key.request_id);
        assert_eq!(generation, 1);
        assert_eq!(state.paginated_generation.load(std::sync::atomic::Ordering::Acquire), u64::MAX);
        assert!(!page_cache_is_current(u64::MAX, generation));
    }

    #[test]
    fn edited_document_uses_a_fresh_request_and_cannot_mutate_accepted_packages() {
        let original_project = project_with_action("CAPTURED_REVISION_A");
        let inputs = LayoutInputs::default();
        let original = valid(super::document::build_package(&original_project, &inputs, true));
        let mut edited_project = original_project.clone();
        edited_project.elements[0] = valid(ScreenplayElement::new(
            "print-action",
            Role::Action,
            "EDITED_REVISION_B",
        ));
        let edited = valid(super::document::build_package(&edited_project, &inputs, true));
        let contains_text = |package: &super::document::WindowsPrintPackage, expected: &str| {
            package.pages().iter().flat_map(|page| page.runs()).any(|run| run.text().contains(expected))
        };
        assert!(contains_text(&original, "CAPTURED_REVISION_A"));
        assert!(!contains_text(&original, "EDITED_REVISION_B"));
        assert!(contains_text(&edited, "EDITED_REVISION_B"));
        assert!(!contains_text(&edited, "CAPTURED_REVISION_A"));
        assert_ne!(
            PackageCacheKey { request_id: 500, include_title_page: true },
            PackageCacheKey { request_id: 501, include_title_page: true },
        );
    }

    #[test]
    fn package_cache_key_is_bound_to_request_revision_and_title_option() {
        let include_request_a = PackageCacheKey { request_id: 41, include_title_page: true };
        let omit_request_a = PackageCacheKey { request_id: 41, include_title_page: false };
        let include_request_b = PackageCacheKey { request_id: 42, include_title_page: true };
        assert_ne!(include_request_a, omit_request_a);
        assert_ne!(include_request_a, include_request_b);
        assert_eq!(include_request_a.request_id, omit_request_a.request_id);
    }

    #[test]
    fn preview_and_final_page_selection_share_the_same_immutable_option_key() {
        let project = project_with_action("PREVIEW_FINAL_SENTINEL");
        let inputs = LayoutInputs::default();
        let state = super::SourceState::new(super::PreparedPackages {
            request_id: 7,
            request_revision: 1,
            job_title: "preview-final".to_owned(),
            include: std::sync::Arc::new(valid(super::document::build_package(&project, &inputs, true))),
            omit: std::sync::Arc::new(valid(super::document::build_package(&project, &inputs, false))),
        });
        valid(state.change_title_page_option(false));
        let (preview_package, preview_package_key, preview_generation) = valid(state.selected());
        let (final_package, final_package_key, final_generation) = valid(state.selected());
        assert!(std::sync::Arc::ptr_eq(&preview_package, &final_package));
        assert_eq!(preview_generation, final_generation);
        assert_eq!(preview_generation, 1);
        assert_eq!(preview_package_key, final_package_key);
        assert!(!preview_package_key.include_title_page);
    }

    #[test]
    fn pagination_range_selection_honors_all_single_and_multiple_ranges() {
        assert_eq!(normalize_page_ranges(5, &[]), Some(vec![0, 1, 2, 3, 4]));
        assert_eq!(normalize_page_ranges(5, &[(3, 3)]), Some(vec![2]));
        assert_eq!(normalize_page_ranges(6, &[(1, 2), (5, 6)]), Some(vec![0, 1, 4, 5]));
        assert_eq!(normalize_page_ranges(3, &[(0, 2)]), None);
        assert_eq!(normalize_page_ranges(3, &[(2, 4)]), None);
        assert_eq!(normalize_page_ranges(3, &[(3, 2)]), None);
        assert_eq!(normalize_page_ranges(0, &[]), None);
    }

    #[test]
    fn printer_imageable_rect_is_used_for_single_fit_and_centering() {
        let transform = fit_page_transform(600.0, 800.0, 700.0, 900.0, Some((20.0, 30.0, 500.0, 700.0)))
            .unwrap_or(PageTransform { scale: 0.0, offset_x: 0.0, offset_y: 0.0 });
        assert_eq!(transform.scale, 500.0 / 600.0);
        assert_eq!(transform.offset_x, 20.0);
        assert!((transform.offset_y - 46.666_668).abs() < 0.001);
        assert!(fit_page_transform(600.0, 800.0, 0.0, 900.0, None).is_none());
        assert!(fit_page_transform(f32::NAN, 800.0, 700.0, 900.0, None).is_none());
        assert!(fit_page_transform(600.0, 800.0, 20_000.0, 900.0, None).is_none());
    }

    #[test]
    fn preview_bitmap_allocation_has_finite_edge_and_pixel_bounds() {
        assert_eq!(preview_bitmap_dimensions(100.2, 200.1), Some((101, 201)));
        assert_eq!(preview_bitmap_dimensions(16_384.0, 16_384.0), None);
        assert_eq!(preview_bitmap_dimensions(f32::INFINITY, 10.0), None);
    }

    #[test]
    fn fixed_run_geometry_and_decorations_follow_the_same_page_scale() {
        let fixed = FixedRect {
            x: FixedPoint::from_points(72),
            y: FixedPoint::from_points(144),
            width: FixedPoint::from_points(36),
            height: FixedPoint::from_points(12),
        };
        let transform = PageTransform { scale: 0.5, offset_x: 10.0, offset_y: 20.0 };
        let rect = scaled_run_rect(fixed, transform).unwrap_or(D2D_RECT_F::default());
        for (actual, expected) in [
            (rect.left, 58.0),
            (rect.top, 116.0),
            (rect.right, 82.0),
            (rect.bottom, 124.0),
        ] {
            assert!((actual - expected).abs() < 0.001);
        }
        let (underline, strike) = decoration_rects(rect, transform.scale);
        assert_eq!(underline.left, rect.left);
        assert_eq!(underline.right, rect.right);
        assert!(((underline.bottom - underline.top) - 0.35).abs() < 0.001);
        assert!(strike.top > rect.top && strike.top < rect.bottom);
    }

    #[test]
    fn preparation_cancellation_and_repeated_jobs_have_independent_ownership() {
        let (sender, result) = std::sync::mpsc::channel::<
            Result<super::PreparedPackages, super::StartFailure>,
        >();
        drop(sender);
        let cancelled = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let preparation = super::PrintPreparation {
            result,
            cancelled: std::sync::Arc::clone(&cancelled),
        };
        assert!(!cancelled.load(std::sync::atomic::Ordering::Acquire));
        drop(preparation);
        assert!(cancelled.load(std::sync::atomic::Ordering::Acquire));
        let first = PackageCacheKey { request_id: 100, include_title_page: true };
        let second = PackageCacheKey { request_id: 101, include_title_page: true };
        assert_ne!(first, second);
    }

    #[test]
    fn requested_event_token_has_single_owner_and_single_removal() {
        let mut token = Some(0x51);
        assert_eq!(take_registration_token(&mut token), Some(0x51));
        assert_eq!(take_registration_token(&mut token), None);
        assert_eq!(token, None);
    }

    #[test]
    fn terminal_result_waits_for_ui_close_and_task_completion() {
        assert_eq!(terminal_result(None, Some(PrintResult::Completed)), None);
        assert_eq!(terminal_result(Some(true), None), None);
        assert_eq!(
            terminal_result(Some(true), Some(PrintResult::Completed)),
            Some(PrintResult::Completed)
        );
        assert_eq!(terminal_result(Some(false), None), Some(PrintResult::Cancelled));
        assert_eq!(
            terminal_result(Some(false), Some(PrintResult::Failed)),
            Some(PrintResult::Failed)
        );
    }

    #[test]
    fn failed_and_cancelled_sessions_remain_distinct_terminal_results() {
        assert_eq!(terminal_result(Some(true), Some(PrintResult::Cancelled)), Some(PrintResult::Cancelled));
        assert_eq!(terminal_result(Some(true), Some(PrintResult::Failed)), Some(PrintResult::Failed));
    }

    #[test]
    fn terminal_completion_is_delivered_once_even_if_polled_repeatedly() {
        let mut reported = false;
        assert_eq!(
            take_terminal_result(&mut reported, Some(PrintResult::Completed)),
            Some(PrintResult::Completed)
        );
        assert_eq!(take_terminal_result(&mut reported, Some(PrintResult::Failed)), None);
        assert!(reported);
    }

    #[test]
    fn native_option_and_registration_callbacks_have_explicit_bounded_identifiers() {
        assert!(!TITLE_PAGE_OPTION_ID.is_empty());
        assert!(CALLBACK_TIMEOUT <= std::time::Duration::from_secs(30));
        let mut token = Some(73);
        assert_eq!(take_registration_token(&mut token), Some(73));
        assert_eq!(take_registration_token(&mut token), None);
    }
}
