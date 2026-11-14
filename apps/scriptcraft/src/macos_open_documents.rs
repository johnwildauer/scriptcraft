//! Narrow Launch Services Open Documents ingress for macOS.
//!
//! The adapter registers only the `kCoreEventClass` / `kAEOpenDocuments` pair,
//! and its retained custom NSObject is an Apple-event target, not an
//! `NSApplicationDelegate`. It accepts either one bare `typeFileURL` direct
//! object or a `typeAEList` containing exactly one file URL. Lists are handled
//! atomically: zero, multiple, or malformed items produce one bounded error
//! event and never accept a valid prefix.
//!
//! Installation happens on the main thread before eframe starts. The shared
//! inbox is available immediately, while repaint remains cold until the app
//! creation callback attaches its egui context. A retained will-finish-launching
//! observer re-registers the same target after AppKit setup and before Finder's
//! launch event; no application
//! delegate or NSDocumentController route is involved. The controller retains
//! the manager, target, observer, and inbox, then removes only its observer and
//! Open Documents event registration on orderly drop, leaving Quit untouched.

use crate::macos_document_open::{
    MacOsDocumentOpenError, MacOsDocumentOpenEvent, MacOsDocumentOpenInbox,
    MacOsDocumentOpenRequest,
};
use objc2_app_kit::NSApplicationWillFinishLaunchingNotification;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_core_services::{
    kAEOpenDocuments, keyDirectObject, kCoreEventClass, typeAEList, typeFileURL,
};
use objc2_foundation::{
    NSAppleEventDescriptor, NSAppleEventManager, NSNotification, NSNotificationCenter, NSObject,
    NSObjectProtocol, NSURL,
};
use std::ffi::{CStr, OsStr};
use std::marker::PhantomData;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::rc::Rc;

const INSTALLATION_ERROR: &str =
    "native macOS Open Documents handler installation failed; startup aborted";

/// Direct-object shape accepted by the parser after the event's descriptor type
/// and (for lists) item count have been inspected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DescriptorRoute {
    BareFileUrl,
    SingleItemList,
}

fn descriptor_route(
    descriptor_type: u32,
    list_item_count: Option<isize>,
) -> Result<DescriptorRoute, MacOsDocumentOpenError> {
    if descriptor_type == typeFileURL {
        return Ok(DescriptorRoute::BareFileUrl);
    }
    if descriptor_type != typeAEList {
        return Err(MacOsDocumentOpenError::UnsupportedUrlOrDescriptor);
    }

    match list_item_count {
        Some(1) => Ok(DescriptorRoute::SingleItemList),
        Some(_) => Err(MacOsDocumentOpenError::UnsupportedDocumentCount),
        None => Err(MacOsDocumentOpenError::UnsupportedUrlOrDescriptor),
    }
}

fn file_url_request(
    descriptor: &NSAppleEventDescriptor,
) -> Result<MacOsDocumentOpenRequest, MacOsDocumentOpenError> {
    file_url_request_with(
        || descriptor.fileURLValue(),
        |url| request_from_url(url),
    )
}

fn file_url_request_with<T>(
    file_url_value: impl FnOnce() -> Option<T>,
    validate_url: impl FnOnce(&T) -> Result<MacOsDocumentOpenRequest, MacOsDocumentOpenError>,
) -> Result<MacOsDocumentOpenRequest, MacOsDocumentOpenError> {
    let url = file_url_value().ok_or(MacOsDocumentOpenError::UnsupportedUrlOrDescriptor)?;
    validate_url(&url)
}

/// Copies NSURL's native filesystem representation and applies the shared
/// nonempty UTF-8 / DocumentPath validation without reading or normalizing it.
#[allow(unsafe_code)]
fn request_from_url(url: &NSURL) -> Result<MacOsDocumentOpenRequest, MacOsDocumentOpenError> {
    if !url.isFileURL() {
        return Err(MacOsDocumentOpenError::UnsupportedUrlOrDescriptor);
    }

    let representation = url.fileSystemRepresentation();
    // SAFETY: NSURL returns a NUL-terminated filesystem representation. The
    // caller retains the NSURL for this copy, and CStr reads only through that
    // terminator; the resulting bytes are copied into an owned PathBuf below.
    let bytes = unsafe { CStr::from_ptr(representation.as_ptr()).to_bytes() };
    if bytes.is_empty() {
        return Err(MacOsDocumentOpenError::UnsupportedUrlOrDescriptor);
    }

    let path = PathBuf::from(OsStr::from_bytes(bytes));
    MacOsDocumentOpenRequest::from_path(path)
}

fn event_from_apple_event(event: &NSAppleEventDescriptor) -> MacOsDocumentOpenEvent {
    let request = (|| {
        let direct_object = event
            .paramDescriptorForKeyword(keyDirectObject)
            .ok_or(MacOsDocumentOpenError::EmptyOrMalformedEvent)?;
        let list_item_count = (direct_object.descriptorType() == typeAEList)
            .then(|| direct_object.numberOfItems());
        match descriptor_route(direct_object.descriptorType(), list_item_count)? {
            DescriptorRoute::BareFileUrl => {
                file_url_request(&direct_object)
            }
            DescriptorRoute::SingleItemList => {
                let item = direct_object
                    .descriptorAtIndex(1)
                    .ok_or(MacOsDocumentOpenError::UnsupportedUrlOrDescriptor)?;
                file_url_request(&item)
            }
        }
    })();

    match request {
        Ok(request) => MacOsDocumentOpenEvent::Request(request),
        Err(error) => MacOsDocumentOpenEvent::Error(error),
    }
}

struct NativeOpenDocumentsHandlerIvars {
    inbox: MacOsDocumentOpenInbox,
}

// SAFETY: This Objective-C subclass is created and retained on the main thread.
// Its only ivar is the thread-safe, bounded inbox handle.
#[allow(unsafe_code)]
define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = NativeOpenDocumentsHandlerIvars]
    struct NativeOpenDocumentsHandler;

    // SAFETY: NSObjectProtocol adds no implementation requirements.
    unsafe impl NSObjectProtocol for NativeOpenDocumentsHandler {}

    impl NativeOpenDocumentsHandler {
        #[unsafe(method(handleOpenDocuments:withReplyEvent:))]
        fn handle_open_documents(
            &self,
            event: &NSAppleEventDescriptor,
            _reply_event: &NSAppleEventDescriptor,
        ) {
            // The callback never touches app/session state, displays errors, or
            // acknowledges success. One native event maps to exactly one enqueue.
            let result = event_from_apple_event(event);
            let _ = self.ivars().inbox.enqueue(result);
        }
    }
);

impl NativeOpenDocumentsHandler {
    fn new(
        inbox: MacOsDocumentOpenInbox,
        main_thread: MainThreadMarker,
    ) -> Retained<Self> {
        let this = Self::alloc(main_thread).set_ivars(NativeOpenDocumentsHandlerIvars { inbox });
        // SAFETY: NSObject's initializer is invoked on a valid allocated
        // subclass instance during main-thread-only adapter installation.
        #[allow(unsafe_code)]
        let handler: Retained<Self> = unsafe { msg_send![super(this), init] };
        handler
    }
}

struct OpenDocumentsLaunchObserverIvars {
    event_manager: Retained<NSAppleEventManager>,
    handler: Retained<NativeOpenDocumentsHandler>,
}

// SAFETY: The observer is created and retained on the main thread, and its
// will-finish-launching callback only re-registers the retained main-thread
// Apple-event target after AppKit setup and before Finder's launch event.
#[allow(unsafe_code)]
define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = OpenDocumentsLaunchObserverIvars]
    struct OpenDocumentsLaunchObserver;

    // SAFETY: NSObjectProtocol adds no implementation requirements.
    unsafe impl NSObjectProtocol for OpenDocumentsLaunchObserver {}

    impl OpenDocumentsLaunchObserver {
        #[unsafe(method(applicationWillFinishLaunching:))]
        fn application_will_finish_launching(&self, _notification: &NSNotification) {
            register_open_documents_handler(
                &self.ivars().event_manager,
                &self.ivars().handler,
            );
        }
    }
);

impl OpenDocumentsLaunchObserver {
    fn new(
        event_manager: Retained<NSAppleEventManager>,
        handler: Retained<NativeOpenDocumentsHandler>,
        main_thread: MainThreadMarker,
    ) -> Retained<Self> {
        let this = Self::alloc(main_thread).set_ivars(OpenDocumentsLaunchObserverIvars {
            event_manager,
            handler,
        });
        // SAFETY: NSObject's initializer is invoked on a valid allocated
        // main-thread observer instance.
        #[allow(unsafe_code)]
        let observer: Retained<Self> = unsafe { msg_send![super(this), init] };
        observer
    }
}

fn register_open_documents_handler(
    event_manager: &NSAppleEventManager,
    handler: &NativeOpenDocumentsHandler,
) {
    // SAFETY: The retained main-thread target implements this exact typed
    // selector, and only the core-class Open Documents event is registered.
    #[allow(unsafe_code)]
        unsafe {
            event_manager.setEventHandler_andSelector_forEventClass_andEventID(
                handler,
                sel!(handleOpenDocuments:withReplyEvent:),
                kCoreEventClass,
                kAEOpenDocuments,
            );
        }
}

/// Retains the shared manager, native target, launch observer, and bounded inbox
/// for app lifetime. The Rc marker confines removal and destruction to main.
pub(super) struct NativeOpenDocumentsController {
    event_manager: Retained<NSAppleEventManager>,
    _handler: Retained<NativeOpenDocumentsHandler>,
    notification_center: Retained<NSNotificationCenter>,
    _launch_observer: Retained<OpenDocumentsLaunchObserver>,
    // Retained until startup slot allocation and FIFO running-event draining.
    inbox: MacOsDocumentOpenInbox,
    _main_thread_only: PhantomData<Rc<()>>,
}

impl NativeOpenDocumentsController {
    /// Installs the sole Open Documents registration without touching winit's
    /// application delegate or the separate core Quit event registration.
    pub(super) fn install(
        inbox: MacOsDocumentOpenInbox,
    ) -> Result<Self, &'static str> {
        let Some(main_thread) = MainThreadMarker::new() else {
            return Err(INSTALLATION_ERROR);
        };
        let handler = NativeOpenDocumentsHandler::new(inbox.clone(), main_thread);
        let event_manager = NSAppleEventManager::sharedAppleEventManager();
        let notification_center = NSNotificationCenter::defaultCenter();
        let launch_observer = OpenDocumentsLaunchObserver::new(
            event_manager.clone(),
            handler.clone(),
            main_thread,
        );

        let controller = Self {
            event_manager: event_manager.clone(),
            _handler: handler,
            notification_center,
            _launch_observer: launch_observer,
            inbox,
            _main_thread_only: PhantomData,
        };
        controller.register_handler();
        // SAFETY: This retained, main-thread observer implements the exact
        // notification selector; the name and unfiltered object match AppKit's
        // application-will-finish-launching notification, before Finder's
        // launch document event.
        #[allow(unsafe_code)]
        unsafe {
            controller.notification_center.addObserver_selector_name_object(
                controller._launch_observer.as_ref() as &AnyObject,
                sel!(applicationWillFinishLaunching:),
                Some(NSApplicationWillFinishLaunchingNotification),
                None,
            );
        }
        Ok(controller)
    }

    fn register_handler(&self) {
        register_open_documents_handler(&self.event_manager, &self._handler);
    }

    /// Attaches the app's repaint wakeup after eframe exposes its context.
    pub(super) fn attach_repaint_context(&self, context: eframe::egui::Context) {
        self.inbox.attach_repaint_context(context);
    }

    /// Removes the first cold request only when no command-line path owns startup.
    /// The selected path is ordinary independent startup work, never a handoff.
    pub(super) fn take_cold_startup_path(
        &self,
        startup_argument_exists: bool,
    ) -> Option<PathBuf> {
        self.inbox
            .take_cold_startup_request(startup_argument_exists)
            .map(|request| request.into_path())
    }

    /// Takes every accepted event and the bounded overflow count in FIFO order.
    pub(super) fn drain(&self) -> crate::macos_document_open::MacOsDocumentOpenDrain {
        self.inbox.drain()
    }
}

impl Drop for NativeOpenDocumentsController {
    fn drop(&mut self) {
        // The controller's Rc marker keeps both removals on the main thread.
        // Remove only the owned observer and Open Documents event; Quit remains
        // independently registered by the WO-001 lifecycle adapter.
        // SAFETY: The retained observer is the exact observer previously added
        // with this notification name and no object filter.
        #[allow(unsafe_code)]
        unsafe {
            self.notification_center.removeObserver_name_object(
                self._launch_observer.as_ref() as &AnyObject,
                Some(NSApplicationWillFinishLaunchingNotification),
                None,
            );
        }
        self.event_manager
            .removeEventHandlerForEventClass_andEventID(kCoreEventClass, kAEOpenDocuments);
    }
}

#[cfg(test)]
mod tests {
    use super::{descriptor_route, file_url_request_with, DescriptorRoute};
    use crate::macos_document_open::MacOsDocumentOpenError;
    use objc2_core_services::{
        typeAEList, typeAlias, typeBookmarkData, typeFileURL, typeFSRef,
    };
    use std::cell::Cell;

    #[test]
    fn accepts_bare_file_url_and_exactly_one_item_list() {
        assert_eq!(
            descriptor_route(typeFileURL, None),
            Ok(DescriptorRoute::BareFileUrl)
        );
        assert_eq!(
            descriptor_route(typeAEList, Some(1)),
            Ok(DescriptorRoute::SingleItemList)
        );
    }

    #[test]
    fn rejects_unexpected_direct_object_descriptor_types() {
        for descriptor_type in [typeAlias, typeBookmarkData, typeFSRef] {
            assert_eq!(
                descriptor_route(descriptor_type, None),
                Err(MacOsDocumentOpenError::UnsupportedUrlOrDescriptor)
            );
        }
    }

    #[test]
    fn rejects_empty_or_multi_item_lists_atomically() {
        assert_eq!(
            descriptor_route(typeAEList, Some(0)),
            Err(MacOsDocumentOpenError::UnsupportedDocumentCount)
        );
        assert_eq!(
            descriptor_route(typeAEList, Some(2)),
            Err(MacOsDocumentOpenError::UnsupportedDocumentCount)
        );
    }

    #[test]
    fn requests_file_url_value_before_url_validation() {
        let order = Cell::new(0);
        let result = file_url_request_with(
            || {
                order.set(1);
                Some(())
            },
            |_| {
                assert_eq!(order.get(), 1);
                order.set(2);
                Err(MacOsDocumentOpenError::UnsupportedUrlOrDescriptor)
            },
        );

        assert_eq!(
            result,
            Err(MacOsDocumentOpenError::UnsupportedUrlOrDescriptor)
        );
        assert_eq!(order.get(), 2);
    }

    #[test]
    fn rejects_missing_file_url_value_before_url_validation() {
        let validation_called = Cell::new(false);
        let result = file_url_request_with::<()>(
            || None,
            |_| {
                validation_called.set(true);
                Err(MacOsDocumentOpenError::UnsupportedUrlOrDescriptor)
            },
        );

        assert_eq!(
            result,
            Err(MacOsDocumentOpenError::UnsupportedUrlOrDescriptor)
        );
        assert!(!validation_called.get());
    }

    #[test]
    fn rejects_unexpected_descriptor_types_and_missing_list_count() {
        assert_eq!(
            descriptor_route(u32::from_be_bytes(*b"TEXT"), None),
            Err(MacOsDocumentOpenError::UnsupportedUrlOrDescriptor)
        );
        assert_eq!(
            descriptor_route(typeAEList, None),
            Err(MacOsDocumentOpenError::UnsupportedUrlOrDescriptor)
        );
    }
}
