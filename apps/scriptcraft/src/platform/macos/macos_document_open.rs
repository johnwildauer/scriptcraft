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
pub(crate) enum MacOsDocumentOpenError {
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
pub(crate) struct MacOsDocumentOpenInbox {
    state: Arc<Mutex<InboxState>>,
    repaint_context: Arc<Mutex<Option<egui::Context>>>,
}

impl MacOsDocumentOpenInbox {
    pub(crate) fn new() -> Self {
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
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// One bounded drain result. Overflow is aggregated numerically rather than
/// materialized as one allocated error string per rejected callback.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct MacOsDocumentOpenDrain {
    pub(super) events: VecDeque<MacOsDocumentOpenEvent>,
    pub(super) overflow_count: usize,
}

/// Native feedback emitted by the running-instance router. Launch failures are
/// deliberately content-free; their child remains independent of the receiver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MacOsDocumentOpenFeedback {
    IntakeError(MacOsDocumentOpenError),
    IndependentLaunchFailed,
    InboxOverflow,
}

/// Routes one bounded drain in FIFO order through an injected independent
/// launcher. The router has no receiver/session reference and never treats
/// successful process creation as document activation.
pub(crate) fn route_independent_requests(
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
