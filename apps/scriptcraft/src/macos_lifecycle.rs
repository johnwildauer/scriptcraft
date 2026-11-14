//! Main-thread macOS lifecycle ingress for native application Quit events.
//!
//! The Apple-event callback only sets a coalescing atomic flag and requests an
//! egui repaint. The UI thread turns that flag into the ordinary typed Quit
//! command, so all close authorization remains in the root viewport lifecycle.

use objc2::rc::Retained;
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_foundation::{NSAppleEventDescriptor, NSAppleEventManager, NSObject, NSObjectProtocol};
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const CORE_EVENT_CLASS: u32 = u32::from_be_bytes(*b"aevt");
const QUIT_APPLICATION_EVENT_ID: u32 = u32::from_be_bytes(*b"quit");
const INSTALLATION_ERROR: &str = "native macOS Quit handler installation failed";

/// A one-slot inbox: repeated native requests before a UI drain become one Quit.
#[derive(Debug, Default)]
struct QuitRequestInbox {
    queued: AtomicBool,
}

impl QuitRequestInbox {
    fn enqueue(&self) {
        self.queued.store(true, Ordering::Release);
    }

    fn take_pending(&self) -> bool {
        self.queued.swap(false, Ordering::AcqRel)
    }
}

#[derive(Debug)]
struct NativeQuitHandlerIvars {
    inbox: Arc<QuitRequestInbox>,
    repaint_context: eframe::egui::Context,
}

// SAFETY: The Objective-C subclass is created and retained on the main thread;
// its ivars are thread-safe request state and an egui repaint handle.
#[allow(unsafe_code)]
define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = NativeQuitHandlerIvars]
    struct NativeQuitEventHandler;

    // SAFETY: NSObjectProtocol adds no implementation requirements.
    unsafe impl NSObjectProtocol for NativeQuitEventHandler {}

    impl NativeQuitEventHandler {
        #[unsafe(method(handleQuit:withReplyEvent:))]
        fn handle_quit(
            &self,
            _event: &NSAppleEventDescriptor,
            _reply_event: &NSAppleEventDescriptor,
        ) {
            let ivars = self.ivars();
            ivars.inbox.enqueue();
            ivars.repaint_context.request_repaint();
        }
    }
);

impl NativeQuitEventHandler {
    fn new(
        inbox: Arc<QuitRequestInbox>,
        repaint_context: eframe::egui::Context,
        main_thread: MainThreadMarker,
    ) -> Retained<Self> {
        let this = Self::alloc(main_thread).set_ivars(NativeQuitHandlerIvars {
            inbox,
            repaint_context,
        });
        // SAFETY: This invokes NSObject's initializer on a valid allocated
        // subclass instance and is confined to the main-thread installation.
        #[allow(unsafe_code)]
        let handler: Retained<Self> = unsafe { msg_send![super(this), init] };
        handler
    }
}

/// Retains the Apple-event manager and its installed handler for app lifetime.
/// The marker makes the AppKit ownership boundary explicitly main-thread-only.
pub(super) struct NativeLifecycleController {
    _event_manager: Retained<NSAppleEventManager>,
    _handler: Retained<NativeQuitEventHandler>,
    inbox: Arc<QuitRequestInbox>,
    _main_thread_only: PhantomData<Rc<()>>,
}

impl NativeLifecycleController {
    /// Installs the core-class Quit handler without replacing winit's delegate.
    pub(super) fn install(
        repaint_context: eframe::egui::Context,
    ) -> Result<Self, &'static str> {
        let Some(main_thread) = MainThreadMarker::new() else {
            return Err(INSTALLATION_ERROR);
        };

        let inbox = Arc::new(QuitRequestInbox::default());
        let handler = NativeQuitEventHandler::new(
            Arc::clone(&inbox),
            repaint_context,
            main_thread,
        );
        let event_manager = NSAppleEventManager::sharedAppleEventManager();

        // SAFETY: The retained main-thread target implements this exact
        // two-argument selector. The four-character codes identify only the
        // core Quit Apple event; no Open Documents event is registered.
        #[allow(unsafe_code)]
        unsafe {
            event_manager.setEventHandler_andSelector_forEventClass_andEventID(
                &handler,
                sel!(handleQuit:withReplyEvent:),
                CORE_EVENT_CLASS,
                QUIT_APPLICATION_EVENT_ID,
            );
        }

        Ok(Self {
            _event_manager: event_manager,
            _handler: handler,
            inbox,
            _main_thread_only: PhantomData,
        })
    }

    /// Transfers at most one native request to the UI-thread caller.
    pub(super) fn take_quit_request(&self) -> bool {
        self.inbox.take_pending()
    }
}

#[cfg(test)]
mod tests {
    use super::QuitRequestInbox;

    #[test]
    fn repeated_native_quit_requests_coalesce_until_the_ui_drain() {
        let inbox = QuitRequestInbox::default();

        inbox.enqueue();
        inbox.enqueue();
        inbox.enqueue();

        assert!(inbox.take_pending());
        assert!(!inbox.take_pending());
    }

    #[test]
    fn a_request_after_a_drain_is_available_for_the_next_ui_turn() {
        let inbox = QuitRequestInbox::default();

        inbox.enqueue();
        assert!(inbox.take_pending());
        inbox.enqueue();

        assert!(inbox.take_pending());
        assert!(!inbox.take_pending());
    }
}
