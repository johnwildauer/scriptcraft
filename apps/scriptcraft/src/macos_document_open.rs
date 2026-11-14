//! Bounded, platform-neutral model for macOS operating-system document-open requests.
//!
//! The inbox retains at most [`MAX_PENDING_EVENTS`] accepted events in FIFO order.
//! Each input event is accepted all-or-nothing as either one validated path request
//! or one fixed error category; a full inbox rejects the new event, preserves every
//! older event, and records a bounded overflow count for the application to surface.
//! Repaint attachment is independent of enqueue so native requests arriving before
//! eframe has created its context remain available for first-frame draining.
//!
//! The first valid cold request owns the initial-instance slot unless a startup
//! document argument already owns it; errors never claim the slot. This module
//! also routes post-startup events to independent children; it never replaces
//! receiver documents, makes dirty/close decisions, or dispatches in-app Open.

use eframe::egui;
use scriptcraft_commands::DocumentPath;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

/// Maximum number of pending accepted events retained by one inbox.
pub(super) const MAX_PENDING_EVENTS: usize = 16;

/// A filesystem path which has passed the shared UTF-8 and `DocumentPath` checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct MacOsDocumentOpenRequest {
    path: PathBuf,
}

impl MacOsDocumentOpenRequest {
    /// Validates path representation only; does not inspect or normalize the filesystem.
    pub(super) fn from_path(path: PathBuf) -> Result<Self, MacOsDocumentOpenError> {
        let Some(utf8_path) = path.to_str() else {
            return Err(MacOsDocumentOpenError::InvalidUtf8Path);
        };
        DocumentPath::new(utf8_path).map_err(|_| MacOsDocumentOpenError::EmptyOrMalformedEvent)?;
        Ok(Self { path })
    }

    pub(super) fn path(&self) -> &PathBuf {
        &self.path
    }

    pub(super) fn into_path(self) -> PathBuf {
        self.path
    }
}

/// Fixed, content-free reasons emitted by the native adapter or bounded inbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MacOsDocumentOpenError {
    EmptyOrMalformedEvent,
    UnsupportedDocumentCount,
    UnsupportedUrlOrDescriptor,
    InvalidUtf8Path,
    InboxOverflow,
}

/// One all-or-nothing event accepted into the pending FIFO.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum MacOsDocumentOpenEvent {
    Request(MacOsDocumentOpenRequest),
    Error(MacOsDocumentOpenError),
}

#[derive(Debug, Default)]
struct InboxState {
    events: VecDeque<MacOsDocumentOpenEvent>,
    overflow_count: usize,
}

/// Shared enqueue/drain handle used by the native callback and application.
///
/// Both locks recover poisoned guards through `into_inner`; the queue is never
/// discarded as a side effect of poison recovery. `overflow_count` is a saturating
/// machine-sized count, so arbitrarily many rejected callbacks cannot allocate
/// additional strings or unbounded storage.
#[derive(Clone, Default)]
pub(super) struct MacOsDocumentOpenInbox {
    state: Arc<Mutex<InboxState>>,
    repaint_context: Arc<Mutex<Option<egui::Context>>>,
}

impl MacOsDocumentOpenInbox {
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// Attaches repaint separately from queue construction and request delivery.
    pub(super) fn attach_repaint_context(&self, context: egui::Context) {
        *lock_recover(&self.repaint_context) = Some(context);
    }

    /// Enqueues one typed event, or rejects it without evicting older FIFO entries.
    /// Both accepted events and rejections wake egui when a context is attached.
    pub(super) fn enqueue(
        &self,
        event: MacOsDocumentOpenEvent,
    ) -> Result<(), MacOsDocumentOpenError> {
        let result = {
            let mut state = lock_recover(&self.state);
            if state.events.len() < MAX_PENDING_EVENTS {
                state.events.push_back(event);
                Ok(())
            } else {
                state.overflow_count = state.overflow_count.saturating_add(1);
                Err(MacOsDocumentOpenError::InboxOverflow)
            }
        };
        self.request_repaint_if_attached();
        result
    }

    /// Drains at most the fixed inbox capacity and resets the accumulated overflow
    /// count. The returned batch is bounded independently of producer activity.
    pub(super) fn drain(&self) -> MacOsDocumentOpenDrain {
        let mut state = lock_recover(&self.state);
        let event_count = state.events.len().min(MAX_PENDING_EVENTS);
        let events = state.events.drain(..event_count).collect();
        let overflow_count = std::mem::take(&mut state.overflow_count);
        MacOsDocumentOpenDrain {
            events,
            overflow_count,
        }
    }

    /// Allocates at most the first valid cold request to the new initial instance.
    /// Intake errors stay queued for UI presentation, and later requests remain in
    /// their original FIFO order for the subsequent running-instance router.
    pub(super) fn take_cold_startup_request(
        &self,
        startup_argument_exists: bool,
    ) -> Option<MacOsDocumentOpenRequest> {
        if startup_argument_exists {
            return None;
        }

        let mut state = lock_recover(&self.state);
        let request_index = state
            .events
            .iter()
            .position(|event| matches!(event, MacOsDocumentOpenEvent::Request(_)))?;
        match state.events.remove(request_index) {
            Some(MacOsDocumentOpenEvent::Request(request)) => Some(request),
            Some(MacOsDocumentOpenEvent::Error(_)) | None => None,
        }
    }

    fn request_repaint_if_attached(&self) {
        let context = lock_recover(&self.repaint_context).clone();
        if let Some(context) = context {
            context.request_repaint();
        }
    }
}

fn lock_recover<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// One bounded drain result. Overflow is aggregated numerically rather than
/// materialized as one allocated error string per rejected callback.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct MacOsDocumentOpenDrain {
    pub(super) events: VecDeque<MacOsDocumentOpenEvent>,
    pub(super) overflow_count: usize,
}

/// Native feedback emitted by the running-instance router. Launch failures are
/// deliberately content-free; their child remains independent of the receiver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MacOsDocumentOpenFeedback {
    IntakeError(MacOsDocumentOpenError),
    IndependentLaunchFailed,
    InboxOverflow,
}

/// Routes one bounded drain in FIFO order through an injected independent
/// launcher. The router has no receiver/session reference and never treats
/// successful process creation as document activation.
pub(super) fn route_independent_requests(
    drain: MacOsDocumentOpenDrain,
    mut launch: impl FnMut(&std::path::Path) -> Result<(), ()>,
    mut present_feedback: impl FnMut(MacOsDocumentOpenFeedback),
) {
    for event in drain.events {
        match event {
            MacOsDocumentOpenEvent::Request(request) => {
                if launch(request.path()).is_err() {
                    present_feedback(MacOsDocumentOpenFeedback::IndependentLaunchFailed);
                }
            }
            MacOsDocumentOpenEvent::Error(error) => {
                present_feedback(MacOsDocumentOpenFeedback::IntakeError(error));
            }
        }
    }
    if drain.overflow_count > 0 {
        present_feedback(MacOsDocumentOpenFeedback::InboxOverflow);
    }
}

/// Deterministic disposition of a valid OS request relative to the initial slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MacOsDocumentOpenDisposition {
    /// The startup argument owns the initial instance, so OS requests need children.
    RunningChild,
    /// The first valid cold OS request becomes the initial instance's document.
    InitialInstance,
    /// A later cold OS request is independent work and must not replace the first.
    IndependentChild,
}

/// Stateful cursor for the pure, deterministic initial-slot allocation rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct MacOsDocumentOpenInitialSlot {
    startup_argument_exists: bool,
    initial_slot_claimed: bool,
}

impl MacOsDocumentOpenInitialSlot {
    pub(super) const fn new(startup_argument_exists: bool) -> Self {
        Self {
            startup_argument_exists,
            initial_slot_claimed: startup_argument_exists,
        }
    }

    /// Errors do not consume the slot; each valid request advances this rule once.
    pub(super) fn allocate(
        &mut self,
        event: &MacOsDocumentOpenEvent,
    ) -> Option<MacOsDocumentOpenDisposition> {
        let (disposition, initial_slot_claimed) = allocate_initial_slot(
            self.startup_argument_exists,
            self.initial_slot_claimed,
            event,
        );
        self.initial_slot_claimed = initial_slot_claimed;
        disposition
    }
}

/// Pure transition for one queued event and the current initial-slot ownership.
fn allocate_initial_slot(
    startup_argument_exists: bool,
    initial_slot_claimed: bool,
    event: &MacOsDocumentOpenEvent,
) -> (Option<MacOsDocumentOpenDisposition>, bool) {
    match event {
        MacOsDocumentOpenEvent::Error(_) => (None, initial_slot_claimed),
        MacOsDocumentOpenEvent::Request(_) if startup_argument_exists => {
            (Some(MacOsDocumentOpenDisposition::RunningChild), true)
        }
        MacOsDocumentOpenEvent::Request(_) if !initial_slot_claimed => {
            (Some(MacOsDocumentOpenDisposition::InitialInstance), true)
        }
        MacOsDocumentOpenEvent::Request(_) => {
            (Some(MacOsDocumentOpenDisposition::IndependentChild), true)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        MacOsDocumentOpenDisposition as Disposition, MacOsDocumentOpenError as Error,
        MacOsDocumentOpenEvent as Event, MacOsDocumentOpenInbox as Inbox,
        MacOsDocumentOpenInitialSlot as InitialSlot, MacOsDocumentOpenRequest as Request,
        route_independent_requests, MacOsDocumentOpenFeedback as Feedback,
        MacOsDocumentOpenDrain, MAX_PENDING_EVENTS,
    };
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    fn request(path: &str) -> Event {
        match Request::from_path(PathBuf::from(path)) {
            Ok(request) => Event::Request(request),
            Err(_) => Event::Error(Error::EmptyOrMalformedEvent),
        }
    }

    #[test]
    fn empty_drain_has_no_events_or_overflow() {
        let drain = Inbox::new().drain();
        assert!(drain.events.is_empty());
        assert_eq!(drain.overflow_count, 0);
    }

    #[test]
    fn request_validation_preserves_spaces_and_unicode_and_rejects_invalid_paths() {
        let valid = Request::from_path(PathBuf::from("/drafts/Été first draft.fdx"));
        assert_eq!(
            valid.map(|request| request.path().clone()),
            Ok(PathBuf::from("/drafts/Été first draft.fdx"))
        );
        assert_eq!(
            Request::from_path(PathBuf::new()),
            Err(Error::EmptyOrMalformedEvent)
        );
        assert_eq!(
            Request::from_path(PathBuf::from("   ")),
            Err(Error::EmptyOrMalformedEvent)
        );
        assert_eq!(
            Request::from_path(PathBuf::from("/drafts/invalid\0name.fdx")),
            Err(Error::EmptyOrMalformedEvent)
        );
    }

    #[cfg(unix)]
    #[test]
    fn request_validation_rejects_non_utf8_native_path_bytes() {
        use std::os::unix::ffi::OsStringExt;

        let path = PathBuf::from(std::ffi::OsString::from_vec(vec![b'/', 0xff]));
        assert_eq!(Request::from_path(path), Err(Error::InvalidUtf8Path));
    }

    #[test]
    fn inbox_accepts_exact_capacity_then_rejects_without_evicting_fifo_entries() {
        let inbox = Inbox::new();
        for index in 0..MAX_PENDING_EVENTS {
            assert_eq!(
                inbox.enqueue(request(&format!("/drafts/{index}.fdx"))),
                Ok(())
            );
        }
        assert_eq!(
            inbox.enqueue(request("/drafts/rejected.fdx")),
            Err(Error::InboxOverflow)
        );

        let drain = inbox.drain();
        assert_eq!(drain.events.len(), MAX_PENDING_EVENTS);
        assert_eq!(drain.overflow_count, 1);
        for (index, event) in drain.events.into_iter().enumerate() {
            assert_eq!(event, request(&format!("/drafts/{index}.fdx")));
        }
        assert_eq!(inbox.drain().overflow_count, 0);
    }

    #[test]
    fn inbox_preserves_fifo_for_requests_and_typed_errors() {
        let inbox = Inbox::new();
        let first = request("/drafts/first.fdx");
        let rejected = Event::Error(Error::UnsupportedUrlOrDescriptor);
        let last = request("/drafts/last.fdx");
        assert_eq!(inbox.enqueue(first.clone()), Ok(()));
        assert_eq!(inbox.enqueue(rejected.clone()), Ok(()));
        assert_eq!(inbox.enqueue(last.clone()), Ok(()));

        assert_eq!(
            inbox.drain().events.into_iter().collect::<Vec<_>>(),
            vec![first, rejected, last]
        );
    }

    #[test]
    fn cold_allocation_skips_but_retains_errors_and_later_requests() {
        let inbox = Inbox::new();
        let error = Event::Error(Error::UnsupportedUrlOrDescriptor);
        let first = request("/drafts/first.fdx");
        let later = request("/drafts/later.fdx");
        assert_eq!(inbox.enqueue(error.clone()), Ok(()));
        assert_eq!(inbox.enqueue(first.clone()), Ok(()));
        assert_eq!(inbox.enqueue(later.clone()), Ok(()));

        assert_eq!(
            inbox.take_cold_startup_request(false),
            match first.clone() {
                Event::Request(request) => Some(request),
                Event::Error(_) => None,
            }
        );
        let remaining = inbox.drain();
        assert_eq!(
            remaining.events,
            vec![error.clone(), later]
                .into_iter()
                .collect::<std::collections::VecDeque<_>>()
        );
        let mut launched = Vec::new();
        let mut feedback = Vec::new();
        route_independent_requests(
            remaining,
            |path| {
                launched.push(path.to_path_buf());
                Ok(())
            },
            |message| feedback.push(message),
        );
        assert_eq!(launched, vec![PathBuf::from("/drafts/later.fdx")]);
        assert_eq!(
            feedback,
            vec![Feedback::IntakeError(Error::UnsupportedUrlOrDescriptor)]
        );
        assert_eq!(error, Event::Error(Error::UnsupportedUrlOrDescriptor));
    }

    #[test]
    fn startup_argument_leaves_every_native_request_for_child_routing() {
        let inbox = Inbox::new();
        let first = request("/drafts/from-finder.fdx");
        let second = request("/drafts/from-dock.fdx");
        assert_eq!(inbox.enqueue(first.clone()), Ok(()));
        assert_eq!(inbox.enqueue(second.clone()), Ok(()));
        assert_eq!(inbox.take_cold_startup_request(true), None);

        let mut slot = InitialSlot::new(true);
        assert_eq!(slot.allocate(&first), Some(Disposition::RunningChild));
        assert_eq!(slot.allocate(&second), Some(Disposition::RunningChild));

        let mut launched = Vec::new();
        route_independent_requests(
            inbox.drain(),
            |path| {
                launched.push(path.to_path_buf());
                Ok(())
            },
            |_| {},
        );
        assert_eq!(
            launched,
            vec![
                PathBuf::from("/drafts/from-finder.fdx"),
                PathBuf::from("/drafts/from-dock.fdx")
            ]
        );
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct ReceiverSnapshot {
        session: &'static str,
        path: &'static str,
        baseline: &'static str,
        title: &'static str,
        dirty: bool,
        modal: &'static str,
        native_output_active: bool,
        title_page_focused: bool,
        replacement_pending: bool,
    }

    #[derive(Debug, PartialEq, Eq)]
    enum InjectedEffect {
        IndependentLaunch(PathBuf),
        BoundedNativeFeedback(Feedback),
    }

    #[test]
    fn injected_independent_router_preserves_clean_dirty_and_modal_receivers() {
        for receiver in [
            ReceiverSnapshot {
                session: "clean-session",
                path: "/drafts/clean.fdx",
                baseline: "clean-baseline",
                title: "Clean title",
                dirty: false,
                modal: "none",
                native_output_active: false,
                title_page_focused: false,
                replacement_pending: false,
            },
            ReceiverSnapshot {
                session: "dirty-session",
                path: "/drafts/dirty.fdx",
                baseline: "dirty-baseline",
                title: "Unsaved title",
                dirty: true,
                modal: "none",
                native_output_active: false,
                title_page_focused: true,
                replacement_pending: false,
            },
            ReceiverSnapshot {
                session: "modal-session",
                path: "/drafts/modal.fdx",
                baseline: "modal-baseline",
                title: "Migration title",
                dirty: true,
                modal: "migration-native-output-and-replacement-lock",
                native_output_active: true,
                title_page_focused: true,
                replacement_pending: true,
            },
        ] {
            let before = receiver.clone();
            let mut observed_effects = Vec::new();
            route_independent_requests(
                MacOsDocumentOpenDrain {
                    events: vec![
                        request("/incoming/first.fdx"),
                        Event::Error(Error::UnsupportedDocumentCount),
                        request("/incoming/second.fdx"),
                    ]
                    .into(),
                    overflow_count: 0,
                },
                |path| {
                    observed_effects.push(InjectedEffect::IndependentLaunch(path.to_path_buf()));
                    Ok(())
                },
                |message| {
                    observed_effects.push(InjectedEffect::BoundedNativeFeedback(message));
                },
            );

            assert_eq!(receiver, before);
            assert_eq!(
                observed_effects,
                vec![
                    InjectedEffect::IndependentLaunch(PathBuf::from("/incoming/first.fdx")),
                    InjectedEffect::BoundedNativeFeedback(Feedback::IntakeError(
                        Error::UnsupportedDocumentCount
                    )),
                    InjectedEffect::IndependentLaunch(PathBuf::from("/incoming/second.fdx")),
                ]
            );
        }
    }

    #[test]
    fn independent_router_keeps_separate_single_events_fifo_and_rejects_multi_event() {
        let mut launched = Vec::new();
        let mut feedback = Vec::new();
        route_independent_requests(
            MacOsDocumentOpenDrain {
                events: vec![
                    request("/incoming/one.fdx"),
                    request("/incoming/two.fdx"),
                    Event::Error(Error::UnsupportedDocumentCount),
                    request("/incoming/three.fdx"),
                ]
                .into(),
                overflow_count: 0,
            },
            |path| {
                launched.push(path.to_path_buf());
                Ok(())
            },
            |message| feedback.push(message),
        );
        assert_eq!(
            launched,
            vec![
                PathBuf::from("/incoming/one.fdx"),
                PathBuf::from("/incoming/two.fdx"),
                PathBuf::from("/incoming/three.fdx")
            ]
        );
        assert_eq!(
            feedback,
            vec![Feedback::IntakeError(Error::UnsupportedDocumentCount)]
        );
    }

    #[test]
    fn multi_document_event_is_rejected_without_any_child_launch() {
        let mut launched = Vec::new();
        let mut feedback = Vec::new();
        route_independent_requests(
            MacOsDocumentOpenDrain {
                events: vec![Event::Error(Error::UnsupportedDocumentCount)].into(),
                overflow_count: 0,
            },
            |path| {
                launched.push(path.to_path_buf());
                Ok(())
            },
            |message| feedback.push(message),
        );
        assert!(launched.is_empty());
        assert_eq!(
            feedback,
            vec![Feedback::IntakeError(Error::UnsupportedDocumentCount)]
        );
    }

    #[test]
    fn independent_router_reports_launch_failures_and_continues_to_later_events() {
        let mut attempts = Vec::new();
        let mut feedback = Vec::new();
        route_independent_requests(
            MacOsDocumentOpenDrain {
                events: vec![request("/incoming/fail.fdx"), request("/incoming/succeed.fdx")]
                    .into(),
                overflow_count: 2,
            },
            |path| {
                attempts.push(path.to_path_buf());
                if path.ends_with("fail.fdx") {
                    Err(())
                } else {
                    Ok(())
                }
            },
            |message| feedback.push(message),
        );
        assert_eq!(
            attempts,
            vec![
                PathBuf::from("/incoming/fail.fdx"),
                PathBuf::from("/incoming/succeed.fdx")
            ]
        );
        assert_eq!(
            feedback,
            vec![Feedback::IndependentLaunchFailed, Feedback::InboxOverflow]
        );
    }

    #[test]
    fn startup_argument_prevents_cold_allocation_without_consuming_requests() {
        let inbox = Inbox::new();
        let queued = request("/drafts/queued.fdx");
        assert_eq!(inbox.enqueue(queued.clone()), Ok(()));

        assert_eq!(inbox.take_cold_startup_request(true), None);
        assert_eq!(inbox.drain().events, vec![queued].into());
    }

    #[test]
    fn poisoned_inbox_lock_recovers_queued_events() {
        let inbox = Inbox::new();
        let retained = request("/drafts/retained.fdx");
        assert_eq!(inbox.enqueue(retained.clone()), Ok(()));

        let state = Arc::clone(&inbox.state);
        let poisoner = std::thread::spawn(move || {
            let _guard = match state.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            std::panic::resume_unwind(Box::new("poison inbox lock for model test"));
        });
        let _ = poisoner.join();

        assert_eq!(inbox.drain().events.into_iter().collect::<Vec<_>>(), vec![retained]);
    }

    #[test]
    fn repaint_context_is_attached_and_woken_for_acceptance_and_overflow() {
        let inbox = Inbox::new();
        let repaint_count = Arc::new(AtomicUsize::new(0));
        let callback_count = Arc::clone(&repaint_count);
        let context = eframe::egui::Context::default();
        context.set_request_repaint_callback(move |_| {
            callback_count.fetch_add(1, Ordering::SeqCst);
        });
        inbox.attach_repaint_context(context);

        assert_eq!(inbox.enqueue(request("/drafts/accepted.fdx")), Ok(()));
        for index in 1..MAX_PENDING_EVENTS {
            assert_eq!(inbox.enqueue(request(&format!("/drafts/{index}.fdx"))), Ok(()));
        }
        assert_eq!(
            inbox.enqueue(request("/drafts/overflow.fdx")),
            Err(Error::InboxOverflow)
        );
        assert_eq!(repaint_count.load(Ordering::SeqCst), MAX_PENDING_EVENTS + 1);
    }

    #[test]
    fn cold_initial_slot_ignores_errors_and_allocates_each_valid_request_once() {
        let mut slot = InitialSlot::new(false);
        let error = Event::Error(Error::UnsupportedDocumentCount);
        assert_eq!(slot.allocate(&error), None);
        assert_eq!(slot.allocate(&request("/drafts/first.fdx")), Some(Disposition::InitialInstance));
        assert_eq!(slot.allocate(&request("/drafts/second.fdx")), Some(Disposition::IndependentChild));
        assert_eq!(slot.allocate(&request("/drafts/third.fdx")), Some(Disposition::IndependentChild));
    }

    #[test]
    fn multiple_cold_single_requests_never_replace_the_first_slot_owner() {
        let mut slot = InitialSlot::new(false);
        let first = request("/drafts/one.fdx");
        assert_eq!(slot.allocate(&first), Some(Disposition::InitialInstance));
        for path in ["/drafts/two.fdx", "/drafts/three.fdx", "/drafts/four.fdx"] {
            assert_eq!(slot.allocate(&request(path)), Some(Disposition::IndependentChild));
        }
        assert_eq!(slot.allocate(&first), Some(Disposition::IndependentChild));
    }

    #[test]
    fn existing_startup_argument_owns_initial_for_every_valid_os_request() {
        let mut slot = InitialSlot::new(true);
        assert_eq!(slot.allocate(&request("/drafts/one.fdx")), Some(Disposition::RunningChild));
        assert_eq!(slot.allocate(&request("/drafts/two.fdx")), Some(Disposition::RunningChild));
    }

    #[test]
    fn startup_slot_stays_claimed_after_failure_or_migration_decision() {
        use crate::replacement_open::StagedOpenOutcome;

        let mut slot = InitialSlot::new(false);
        assert_eq!(
            slot.allocate(&request("/drafts/initial.fdx")),
            Some(Disposition::InitialInstance)
        );
        for outcome in [
            StagedOpenOutcome::Failed,
            StagedOpenOutcome::AwaitingMigration,
            StagedOpenOutcome::Cancelled,
            StagedOpenOutcome::Activated,
        ] {
            let _terminal_or_pending_outcome = outcome;
            assert_eq!(
                slot.allocate(&request("/drafts/later.fdx")),
                Some(Disposition::IndependentChild)
            );
        }
    }
}
