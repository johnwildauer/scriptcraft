//! Bounded, content-free acknowledgement protocol for in-app replacement opens.
//!
//! The wire grammar is exactly `SCRIPTCRAFT-OPEN/1 <decimal-id> <result>\n`,
//! where `<result>` is `opened`, `failed`, or `cancelled`. The complete line is
//! ASCII and at most [`MAX_HANDOFF_LINE_LENGTH`] bytes, including its newline.
//! It carries no document paths, screenplay content, or other private data.
//! Handoff IDs are nonzero process-local unsigned integers. The internal
//! `--open-handoff=<id>` startup option associates an ID with a document path;
//! it is not an OS-ingress option and ordinary document startup has no ID.
//!
//! The parent coordinator keeps one active identity and a monotonic 30-second
//! deadline. It drains queued events before polling the retained child's
//! nonblocking exit status. The first matching terminal event wins; stale and
//! late identities are inert. Only matching `Opened` authorizes the one-shot
//! root close continuation. Every other terminal condition clears the entire
//! pending record and resumes the existing document. Timeout never kills or
//! waits for the child.

use std::io::Write;
use std::num::NonZeroU64;
use std::time::{Duration, Instant};

/// Applies the WO-001 lifecycle decision to a selected-path Open. A clean
/// session continues without a prompt; a dirty session continues only for
/// Don't Save or a clean `Saved` result.
pub(crate) fn replacement_open_gate_allows(
    is_dirty: bool,
    choice: Option<crate::document_lifecycle::UnsavedChangesChoice>,
    save_outcome: Option<crate::document_lifecycle::SaveOutcome>,
) -> bool {
    if !is_dirty {
        return true;
    }
    choice.is_some_and(|choice| {
        crate::document_lifecycle::confirmed_close_for_choice(choice, save_outcome)
    })
}

/// Maximum time to wait for a replacement child to acknowledge activation.
pub(crate) const PENDING_REPLACEMENT_TIMEOUT: Duration = Duration::from_secs(30);

/// Upper bound between parent-side child-exit polls while an acknowledgement
/// is pending. Reader submissions independently request an immediate repaint.
pub(crate) const PENDING_REPLACEMENT_POLL_INTERVAL: Duration = Duration::from_millis(50);

/// The terminal or deferred result of opening a staged native document.
/// `AwaitingMigration` is not an activation: the pending source remains owned
/// by this child until the user continues or cancels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StagedOpenOutcome {
    Activated,
    AwaitingMigration,
    Cancelled,
    Failed,
}

/// Projects a staged startup result onto the child acknowledgement protocol.
/// Awaiting migration is deliberately deferred, never acknowledged as success.
fn terminal_result(outcome: StagedOpenOutcome) -> Option<ReplacementOpenResult> {
    match outcome {
        StagedOpenOutcome::Activated => Some(ReplacementOpenResult::Opened),
        StagedOpenOutcome::Failed => Some(ReplacementOpenResult::Failed),
        StagedOpenOutcome::Cancelled => Some(ReplacementOpenResult::Cancelled),
        StagedOpenOutcome::AwaitingMigration => None,
    }
}

/// Consumes and writes one terminal handoff result, ignoring ordinary I/O
/// failures. The encoded protocol line is fixed-format and bounded; callers
/// retain the identity while migration is pending and pass it here only when
/// startup reaches a terminal staged outcome.
pub(crate) fn write_terminal_handoff_result<W: Write>(
    handoff_id: &mut Option<HandoffId>,
    outcome: StagedOpenOutcome,
    writer: &mut W,
) -> bool {
    let Some(result) = terminal_result(outcome) else {
        return false;
    };
    let Some(handoff_id) = handoff_id.take() else {
        return false;
    };

    if let Ok(line) = encode_handoff_result(handoff_id, result) {
        // A closed/short parent pipe is not allowed to undo activation or
        // terminate the child. Attempt flush even when write_all reports error.
        let _ = writer.write_all(line.as_bytes());
        let _ = writer.flush();
    }
    true
}

/// The fixed maximum size of a complete acknowledgement line, including `\n`.
pub(crate) const MAX_HANDOFF_LINE_LENGTH: usize = 64;

const HANDOFF_MARKER: &str = "SCRIPTCRAFT-OPEN/1";

/// A nonzero process-local identity for one replacement-open handoff.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct HandoffId(NonZeroU64);

impl HandoffId {
    /// Parses a strict, unsigned ASCII decimal identity.
    pub(crate) fn parse_decimal(value: &str) -> Result<Self, HandoffIdParseError> {
        if value.is_empty()
            || (value.len() > 1 && value.starts_with('0'))
            || !value.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(HandoffIdParseError::InvalidDecimal);
        }

        let value = value
            .parse::<u64>()
            .map_err(|_| HandoffIdParseError::OutOfRange)?;
        NonZeroU64::new(value)
            .map(Self)
            .ok_or(HandoffIdParseError::Zero)
    }

    pub(crate) const fn get(self) -> u64 {
        self.0.get()
    }
}

/// Reasons a startup or wire identity is rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HandoffIdParseError {
    InvalidDecimal,
    Zero,
    OutOfRange,
}

/// A monotonically increasing process-local ID source. It never wraps or
/// reuses an ID; callers must obtain an ID before attempting to spawn a child.
#[derive(Debug, Clone)]
pub(crate) struct HandoffIdSequence {
    next: Option<NonZeroU64>,
}

impl Default for HandoffIdSequence {
    fn default() -> Self {
        Self {
            next: NonZeroU64::new(1),
        }
    }
}

impl HandoffIdSequence {
    /// Reserves the next unique ID, or reports exhaustion without wrapping.
    pub(crate) fn next_id(&mut self) -> Result<HandoffId, HandoffIdSequenceExhausted> {
        let current = self.next.ok_or(HandoffIdSequenceExhausted)?;
        self.next = match current.get().checked_add(1) {
            Some(next) => NonZeroU64::new(next),
            None => None,
        };
        Ok(HandoffId(current))
    }
}

/// The sequence can no longer allocate a unique identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HandoffIdSequenceExhausted;

/// The only child outcomes that may appear on the acknowledgement wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReplacementOpenResult {
    Opened,
    Failed,
    Cancelled,
}

/// One decoded acknowledgement, scoped to a single handoff identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ReplacementOpenMessage {
    pub(crate) handoff_id: HandoffId,
    pub(crate) result: ReplacementOpenResult,
}

/// A terminal protocol result or a failure of the child acknowledgement pipe.
/// Result identities are kept exactly as decoded so the parent coordinator can
/// reject stale events without accidentally attributing them to a newer handoff.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReplacementOpenEvent {
    Result(ReplacementOpenMessage),
    ChannelFailed {
        handoff_id: HandoffId,
        reason: ReplacementOpenChannelFailure,
    },
}

/// Content-free reason why the acknowledgement stream could not yield a result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReplacementOpenChannelFailure {
    Malformed,
    Oversized,
    EndOfStream,
    ReadFailed,
}

/// Identity and monotonic deadline for the single parent-side pending handoff.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PendingReplacementMetadata {
    pub(crate) handoff_id: HandoffId,
    pub(crate) deadline: Instant,
}

impl PendingReplacementMetadata {
    pub(crate) fn new(handoff_id: HandoffId, now: Instant) -> Self {
        Self {
            handoff_id,
            // An unrepresentable deadline safely becomes immediately due.
            deadline: now
                .checked_add(PENDING_REPLACEMENT_TIMEOUT)
                .unwrap_or(now),
        }
    }
}

/// Parent-side causes that safely resume the currently open document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReplacementOpenResumeReason {
    ChildFailed,
    ChildCancelled,
    ChannelFailed(ReplacementOpenChannelFailure),
    ChildExited,
    ExitPollFailed,
    Timeout,
}

/// Injectable observation of the retained child's nonblocking `try_wait`.
/// Exit status is deliberately collapsed: even status zero is not activation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReplacementOpenChildObservation {
    Running,
    Exited,
    PollFailed,
}

/// Pure parent-side coordinator result. `Pending` carries the next repaint
/// interval; terminal decisions require clearing the full pending record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PendingReplacementDecision {
    Inactive,
    Pending { repaint_after: Duration },
    AuthorizeClose,
    Resume(ReplacementOpenResumeReason),
}

/// Applies queued events in order before considering process exit or timeout.
/// The iterator is fully consumed even after the first matching terminal event
/// so callers can drain the bounded queue before any exit classification.
pub(crate) fn decide_pending_replacement(
    pending: Option<&PendingReplacementMetadata>,
    now: Instant,
    events: impl IntoIterator<Item = ReplacementOpenEvent>,
    child: ReplacementOpenChildObservation,
) -> PendingReplacementDecision {
    let Some(pending) = pending else {
        return PendingReplacementDecision::Inactive;
    };

    let mut first_terminal = None;
    for event in events {
        if first_terminal.is_some() {
            continue;
        }
        let (handoff_id, terminal) = match event {
            ReplacementOpenEvent::Result(message) => {
                let decision = match message.result {
                    ReplacementOpenResult::Opened => PendingReplacementDecision::AuthorizeClose,
                    ReplacementOpenResult::Failed => PendingReplacementDecision::Resume(
                        ReplacementOpenResumeReason::ChildFailed,
                    ),
                    ReplacementOpenResult::Cancelled => PendingReplacementDecision::Resume(
                        ReplacementOpenResumeReason::ChildCancelled,
                    ),
                };
                (message.handoff_id, decision)
            }
            ReplacementOpenEvent::ChannelFailed { handoff_id, reason } => (
                handoff_id,
                PendingReplacementDecision::Resume(
                    ReplacementOpenResumeReason::ChannelFailed(reason),
                ),
            ),
        };
        if handoff_id == pending.handoff_id {
            first_terminal = Some(terminal);
        }
    }
    if let Some(terminal) = first_terminal {
        return terminal;
    }

    match child {
        ReplacementOpenChildObservation::Exited => {
            return PendingReplacementDecision::Resume(ReplacementOpenResumeReason::ChildExited);
        }
        ReplacementOpenChildObservation::PollFailed => {
            return PendingReplacementDecision::Resume(
                ReplacementOpenResumeReason::ExitPollFailed,
            );
        }
        ReplacementOpenChildObservation::Running => {}
    }

    if now >= pending.deadline {
        return PendingReplacementDecision::Resume(ReplacementOpenResumeReason::Timeout);
    }

    let until_deadline = pending.deadline.saturating_duration_since(now);
    PendingReplacementDecision::Pending {
        repaint_after: until_deadline.min(PENDING_REPLACEMENT_POLL_INTERVAL),
    }
}

/// Errors are intentionally content-free so malformed input is never echoed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HandoffCodecError {
    InvalidLine,
    InvalidIdentity,
    LineTooLong,
}

/// Encodes one exact, bounded ASCII acknowledgement line.
pub(crate) fn encode_handoff_result(
    handoff_id: HandoffId,
    result: ReplacementOpenResult,
) -> Result<String, HandoffCodecError> {
    let result_code = match result {
        ReplacementOpenResult::Opened => "opened",
        ReplacementOpenResult::Failed => "failed",
        ReplacementOpenResult::Cancelled => "cancelled",
    };
    let line = format!("{HANDOFF_MARKER} {} {result_code}\n", handoff_id.get());
    if line.len() > MAX_HANDOFF_LINE_LENGTH {
        return Err(HandoffCodecError::LineTooLong);
    }
    Ok(line)
}

/// Decodes bytes without lossy UTF-8 conversion, accepting exactly one line.
pub(crate) fn decode_handoff_result(
    bytes: &[u8],
) -> Result<ReplacementOpenMessage, HandoffCodecError> {
    if bytes.len() > MAX_HANDOFF_LINE_LENGTH {
        return Err(HandoffCodecError::LineTooLong);
    }
    if bytes.is_empty()
        || bytes.last() != Some(&b'\n')
        || bytes[..bytes.len() - 1].contains(&b'\n')
        || !bytes.is_ascii()
    {
        return Err(HandoffCodecError::InvalidLine);
    }

    let line = std::str::from_utf8(&bytes[..bytes.len() - 1])
        .map_err(|_| HandoffCodecError::InvalidLine)?;
    let mut fields = line.split(' ');
    let marker = fields.next().ok_or(HandoffCodecError::InvalidLine)?;
    let identity = fields.next().ok_or(HandoffCodecError::InvalidLine)?;
    let result = fields.next().ok_or(HandoffCodecError::InvalidLine)?;
    if marker != HANDOFF_MARKER || fields.next().is_some() {
        return Err(HandoffCodecError::InvalidLine);
    }
    let handoff_id = HandoffId::parse_decimal(identity)
        .map_err(|_| HandoffCodecError::InvalidIdentity)?;
    let result = match result {
        "opened" => ReplacementOpenResult::Opened,
        "failed" => ReplacementOpenResult::Failed,
        "cancelled" => ReplacementOpenResult::Cancelled,
        _ => return Err(HandoffCodecError::InvalidLine),
    };

    Ok(ReplacementOpenMessage {
        handoff_id,
        result,
    })
}
