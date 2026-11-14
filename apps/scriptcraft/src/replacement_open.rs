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

#[cfg(test)]
mod tests {
    use super::{
        decode_handoff_result, encode_handoff_result, write_terminal_handoff_result,
        HandoffCodecError, HandoffId, HandoffIdParseError, HandoffIdSequence,
        HandoffIdSequenceExhausted, ReplacementOpenMessage, ReplacementOpenResult,
        StagedOpenOutcome, MAX_HANDOFF_LINE_LENGTH,
    };
    use crate::document_lifecycle::{SaveOutcome, UnsavedChangesChoice};
    use std::io::{self, Write};

    #[derive(Default)]
    struct FallibleWriter {
        bytes: Vec<u8>,
        fail_write: bool,
        fail_after_write: bool,
        max_write: Option<usize>,
        write_calls: usize,
        fail_flush: bool,
        flush_called: bool,
    }

    impl Write for FallibleWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if self.fail_write || (self.fail_after_write && self.write_calls > 0) {
                return Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed parent"));
            }
            let written = self.max_write.unwrap_or(bytes.len()).min(bytes.len());
            self.bytes.extend_from_slice(&bytes[..written]);
            self.write_calls += 1;
            Ok(written)
        }

        fn flush(&mut self) -> io::Result<()> {
            self.flush_called = true;
            if self.fail_flush {
                Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed parent"))
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn identity_parser_accepts_only_nonzero_unsigned_ascii_decimal() {
        assert_eq!(HandoffId::parse_decimal("1").map(HandoffId::get), Ok(1));
        assert_eq!(
            HandoffId::parse_decimal("18446744073709551615").map(HandoffId::get),
            Ok(u64::MAX)
        );
        for value in [
            "",
            "0",
            "00",
            "01",
            "+1",
            "-1",
            " 1",
            "1 ",
            "١",
            "18446744073709551616",
        ] {
            assert!(HandoffId::parse_decimal(value).is_err(), "accepted {value:?}");
        }
        assert_eq!(HandoffId::parse_decimal("0"), Err(HandoffIdParseError::Zero));
        assert_eq!(
            HandoffId::parse_decimal("18446744073709551616"),
            Err(HandoffIdParseError::OutOfRange)
        );
    }

    #[test]
    fn process_local_sequence_is_monotonic_and_exhaustion_does_not_wrap() {
        let mut sequence = HandoffIdSequence::default();
        assert_eq!(sequence.next_id().map(HandoffId::get), Ok(1));
        assert_eq!(sequence.next_id().map(HandoffId::get), Ok(2));

        let mut exhausted = HandoffIdSequence {
            next: std::num::NonZeroU64::new(u64::MAX),
        };
        assert_eq!(exhausted.next_id().map(HandoffId::get), Ok(u64::MAX));
        assert_eq!(
            exhausted.next_id(),
            Err(HandoffIdSequenceExhausted)
        );
    }

    #[test]
    fn codec_round_trips_each_wire_outcome_with_only_bounded_ascii_content() {
        let id = HandoffId::parse_decimal("42");
        assert!(id.is_ok());
        if let Ok(id) = id {
            for result in [
                ReplacementOpenResult::Opened,
                ReplacementOpenResult::Failed,
                ReplacementOpenResult::Cancelled,
            ] {
                let encoded = encode_handoff_result(id, result);
                assert!(encoded.is_ok());
                if let Ok(encoded) = encoded {
                    assert!(encoded.is_ascii());
                    assert!(encoded.len() <= MAX_HANDOFF_LINE_LENGTH);
                    assert_eq!(
                        decode_handoff_result(encoded.as_bytes()),
                        Ok(ReplacementOpenMessage {
                            handoff_id: id,
                            result,
                        })
                    );
                }
            }
        }
    }

    #[test]
    fn codec_rejects_wrong_markers_codes_fields_payloads_and_unbounded_lines() {
        for line in [
            b"OTHER-APP/1 1 opened\n".as_slice(),
            b"SCRIPTCRAFT-OPEN/1 1 unknown\n",
            b"SCRIPTCRAFT-OPEN/1 1 opened extra\n",
            b"SCRIPTCRAFT-OPEN/1 1\n",
            b"SCRIPTCRAFT-OPEN/1 1 opened /draft.fdx\n",
            b"SCRIPTCRAFT-OPEN/1 1 opened\nextra",
            b"SCRIPTCRAFT-OPEN/1 1 opened",
            b"SCRIPTCRAFT-OPEN/1  1 opened\n",
            b"SCRIPTCRAFT-OPEN/1 1 opened\r\n",
        ] {
            assert!(decode_handoff_result(line).is_err(), "accepted {line:?}");
        }
        assert_eq!(
            decode_handoff_result(b"SCRIPTCRAFT-OPEN/1 0 opened\n"),
            Err(HandoffCodecError::InvalidIdentity)
        );
        assert_eq!(
            decode_handoff_result(&vec![b'a'; MAX_HANDOFF_LINE_LENGTH + 1]),
            Err(HandoffCodecError::LineTooLong)
        );
        assert_eq!(
            decode_handoff_result(b"SCRIPTCRAFT-OPEN/1 1 op\xffned\n"),
            Err(HandoffCodecError::InvalidLine)
        );
    }

    #[test]
    fn selected_path_gate_skips_clean_prompt_and_requires_dirty_confirmation() {
        assert!(super::replacement_open_gate_allows(false, None, None));
        assert!(!super::replacement_open_gate_allows(
            true,
            Some(UnsavedChangesChoice::Cancel),
            None,
        ));
        assert!(!super::replacement_open_gate_allows(
            true,
            Some(UnsavedChangesChoice::Save),
            Some(SaveOutcome::Cancelled),
        ));
        assert!(!super::replacement_open_gate_allows(
            true,
            Some(UnsavedChangesChoice::Save),
            Some(SaveOutcome::Failed),
        ));
        assert!(super::replacement_open_gate_allows(
            true,
            Some(UnsavedChangesChoice::Save),
            Some(SaveOutcome::Saved),
        ));
        assert!(super::replacement_open_gate_allows(
            true,
            Some(UnsavedChangesChoice::DontSave),
            None,
        ));
        assert!(!super::replacement_open_gate_allows(true, None, None));
    }

    #[test]
    fn staged_outcomes_project_to_one_terminal_line_and_pending_retains_identity() {
        let Ok(id) = HandoffId::parse_decimal("42") else {
            return;
        };
        for (outcome, expected) in [
            (StagedOpenOutcome::Activated, "SCRIPTCRAFT-OPEN/1 42 opened\n"),
            (StagedOpenOutcome::Failed, "SCRIPTCRAFT-OPEN/1 42 failed\n"),
            (StagedOpenOutcome::Cancelled, "SCRIPTCRAFT-OPEN/1 42 cancelled\n"),
        ] {
            let mut identity = Some(id);
            let mut writer = FallibleWriter::default();
            assert!(!write_terminal_handoff_result(
                &mut identity,
                StagedOpenOutcome::AwaitingMigration,
                &mut writer,
            ));
            assert_eq!(identity, Some(id));
            assert!(writer.bytes.is_empty());

            assert!(write_terminal_handoff_result(&mut identity, outcome, &mut writer));
            assert_eq!(identity, None);
            assert_eq!(writer.bytes.as_slice(), expected.as_bytes());
            assert!(writer.flush_called);

            assert!(!write_terminal_handoff_result(&mut identity, outcome, &mut writer));
            assert_eq!(writer.bytes.as_slice(), expected.as_bytes());
        }
    }

    #[test]
    fn absent_identity_and_io_failures_are_non_panicking_one_shot_attempts() {
        let Ok(id) = HandoffId::parse_decimal("7") else {
            return;
        };

        let mut ordinary_identity = None;
        let mut ordinary_writer = FallibleWriter::default();
        assert!(!write_terminal_handoff_result(
            &mut ordinary_identity,
            StagedOpenOutcome::Activated,
            &mut ordinary_writer,
        ));
        assert!(ordinary_writer.bytes.is_empty());
        assert!(!ordinary_writer.flush_called);

        let mut write_failure_identity = Some(id);
        let mut write_failure = FallibleWriter {
            fail_write: true,
            ..FallibleWriter::default()
        };
        assert!(write_terminal_handoff_result(
            &mut write_failure_identity,
            StagedOpenOutcome::Activated,
            &mut write_failure,
        ));
        assert_eq!(write_failure_identity, None);
        assert!(write_failure.flush_called);
        assert!(!write_terminal_handoff_result(
            &mut write_failure_identity,
            StagedOpenOutcome::Activated,
            &mut write_failure,
        ));

        let mut short_write_identity = Some(id);
        let mut short_write = FallibleWriter {
            fail_after_write: true,
            max_write: Some(3),
            ..FallibleWriter::default()
        };
        assert!(write_terminal_handoff_result(
            &mut short_write_identity,
            StagedOpenOutcome::Failed,
            &mut short_write,
        ));
        assert_eq!(short_write_identity, None);
        assert_eq!(short_write.bytes.as_slice(), b"SCR");
        assert!(short_write.flush_called);

        let mut flush_failure_identity = Some(id);
        let mut flush_failure = FallibleWriter {
            fail_flush: true,
            ..FallibleWriter::default()
        };
        assert!(write_terminal_handoff_result(
            &mut flush_failure_identity,
            StagedOpenOutcome::Cancelled,
            &mut flush_failure,
        ));
        assert_eq!(flush_failure_identity, None);
        assert!(flush_failure.flush_called);
        assert_eq!(
            flush_failure.bytes.as_slice(),
            b"SCRIPTCRAFT-OPEN/1 7 cancelled\n"
        );
        assert!(!write_terminal_handoff_result(
            &mut flush_failure_identity,
            StagedOpenOutcome::Cancelled,
            &mut flush_failure,
        ));
    }
}

#[cfg(test)]
mod pending_replacement_tests {
    use super::{
        decide_pending_replacement, HandoffId, PendingReplacementDecision,
        PendingReplacementMetadata, ReplacementOpenChannelFailure,
        ReplacementOpenChildObservation, ReplacementOpenEvent, ReplacementOpenMessage,
        ReplacementOpenResumeReason, ReplacementOpenResult, PENDING_REPLACEMENT_POLL_INTERVAL,
    };
    use std::time::{Duration, Instant};

    fn id(value: &str) -> HandoffId {
        match HandoffId::parse_decimal(value) {
            Ok(id) => id,
            Err(_) => unreachable!("test identity is valid"),
        }
    }

    fn result(handoff_id: HandoffId, result: ReplacementOpenResult) -> ReplacementOpenEvent {
        ReplacementOpenEvent::Result(ReplacementOpenMessage { handoff_id, result })
    }

    fn metadata(handoff_id: HandoffId, now: Instant) -> PendingReplacementMetadata {
        PendingReplacementMetadata::new(handoff_id, now)
    }

    #[test]
    fn matching_opened_wins_before_exit_and_only_authorizes_close() {
        let now = Instant::now();
        let active = metadata(id("10"), now);
        assert_eq!(
            decide_pending_replacement(
                Some(&active),
                now,
                [
                    result(active.handoff_id, ReplacementOpenResult::Opened),
                    result(active.handoff_id, ReplacementOpenResult::Failed),
                ],
                ReplacementOpenChildObservation::Exited,
            ),
            PendingReplacementDecision::AuthorizeClose
        );
    }

    #[test]
    fn any_exit_including_zero_status_resumes_without_opened() {
        let now = Instant::now();
        let active = metadata(id("11"), now);
        // `Exited` intentionally carries no success bit: try_wait reporting
        // `Some(ExitStatus::from_raw(0))` is still not an acknowledgement.
        assert_eq!(
            decide_pending_replacement(
                Some(&active),
                now,
                [],
                ReplacementOpenChildObservation::Exited,
            ),
            PendingReplacementDecision::Resume(ReplacementOpenResumeReason::ChildExited)
        );
    }

    #[test]
    fn failed_cancelled_and_channel_terminal_events_resume_with_specific_reasons() {
        let now = Instant::now();
        let active = metadata(id("12"), now);
        for (event, reason) in [
            (
                result(active.handoff_id, ReplacementOpenResult::Failed),
                ReplacementOpenResumeReason::ChildFailed,
            ),
            (
                result(active.handoff_id, ReplacementOpenResult::Cancelled),
                ReplacementOpenResumeReason::ChildCancelled,
            ),
            (
                ReplacementOpenEvent::ChannelFailed {
                    handoff_id: active.handoff_id,
                    reason: ReplacementOpenChannelFailure::Malformed,
                },
                ReplacementOpenResumeReason::ChannelFailed(
                    ReplacementOpenChannelFailure::Malformed,
                ),
            ),
            (
                ReplacementOpenEvent::ChannelFailed {
                    handoff_id: active.handoff_id,
                    reason: ReplacementOpenChannelFailure::EndOfStream,
                },
                ReplacementOpenResumeReason::ChannelFailed(
                    ReplacementOpenChannelFailure::EndOfStream,
                ),
            ),
        ] {
            assert_eq!(
                decide_pending_replacement(
                    Some(&active),
                    now,
                    [event],
                    ReplacementOpenChildObservation::Running,
                ),
                PendingReplacementDecision::Resume(reason)
            );
        }
    }

    #[test]
    fn exit_poll_failure_is_a_distinct_safe_resume() {
        let now = Instant::now();
        let active = metadata(id("13"), now);
        assert_eq!(
            decide_pending_replacement(
                Some(&active),
                now,
                [],
                ReplacementOpenChildObservation::PollFailed,
            ),
            PendingReplacementDecision::Resume(ReplacementOpenResumeReason::ExitPollFailed)
        );
    }

    #[test]
    fn exact_deadline_times_out_and_earlier_poll_schedules_bounded_repaint() {
        let start = Instant::now();
        let active = metadata(id("14"), start);
        assert_eq!(
            decide_pending_replacement(
                Some(&active),
                active.deadline,
                [],
                ReplacementOpenChildObservation::Running,
            ),
            PendingReplacementDecision::Resume(ReplacementOpenResumeReason::Timeout)
        );
        assert_eq!(
            decide_pending_replacement(
                Some(&active),
                start,
                [],
                ReplacementOpenChildObservation::Running,
            ),
            PendingReplacementDecision::Pending {
                repaint_after: PENDING_REPLACEMENT_POLL_INTERVAL.min(Duration::from_secs(30)),
            }
        );
    }

    #[test]
    fn stale_and_duplicate_events_are_inert_after_first_matching_terminal() {
        let now = Instant::now();
        let active = metadata(id("15"), now);
        let stale = result(id("16"), ReplacementOpenResult::Opened);
        assert!(matches!(
            decide_pending_replacement(
                Some(&active),
                now,
                [stale],
                ReplacementOpenChildObservation::Running,
            ),
            PendingReplacementDecision::Pending { .. }
        ));
        assert_eq!(
            decide_pending_replacement(
                Some(&active),
                now,
                [
                    result(active.handoff_id, ReplacementOpenResult::Failed),
                    result(active.handoff_id, ReplacementOpenResult::Opened),
                ],
                ReplacementOpenChildObservation::Running,
            ),
            PendingReplacementDecision::Resume(ReplacementOpenResumeReason::ChildFailed)
        );
    }

    #[test]
    fn late_opened_after_timeout_is_ignored_without_an_active_identity() {
        let start = Instant::now();
        let active = metadata(id("17"), start);
        assert_eq!(
            decide_pending_replacement(
                Some(&active),
                active.deadline,
                [],
                ReplacementOpenChildObservation::Running,
            ),
            PendingReplacementDecision::Resume(ReplacementOpenResumeReason::Timeout)
        );
        assert_eq!(
            decide_pending_replacement(
                None,
                active.deadline + Duration::from_secs(1),
                [result(active.handoff_id, ReplacementOpenResult::Opened)],
                ReplacementOpenChildObservation::Exited,
            ),
            PendingReplacementDecision::Inactive
        );
    }
}
