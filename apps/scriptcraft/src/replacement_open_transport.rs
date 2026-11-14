//! Parent-side process and pipe transport for acknowledged replacement opens.
//!
//! The UI owns [`ReplacementOpenTransport`] (including its retained child) and
//! the receiver for the bounded application event queue. A single detached
//! reader thread owns the child's piped stdout, reads at most one bounded
//! protocol line, and submits exactly one content-free event with `try_send`.
//! It never owns the child, an editing session, or UI state. A cloned egui
//! context is used only to request repaint after successful queue submission.
//!
//! Dropping this transport merely drops the parent's child handle; it does not
//! kill or wait for the process. Pipe, reader-thread, and queue failures are
//! reported without blocking the UI and never trigger a second unacknowledged
//! spawn. Independent New and ordinary document launches continue to use the
//! existing unacknowledged launcher.

use crate::menu::MenuPlatformChoice;
use crate::replacement_open::{
    decode_handoff_result, HandoffCodecError, HandoffId, ReplacementOpenChannelFailure,
    ReplacementOpenEvent,
};
use eframe::egui;
use std::ffi::OsString;
use std::io::{self, Read};
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};
use std::thread;

/// The application queue has a fixed bound so an unconsumed UI receiver cannot
/// cause the pipe reader to accumulate an unbounded number of events.
pub(crate) const REPLACEMENT_OPEN_EVENT_QUEUE_CAPACITY: usize = 8;

pub(crate) type ReplacementOpenEventSender = SyncSender<ReplacementOpenEvent>;
pub(crate) type ReplacementOpenEventReceiver = Receiver<ReplacementOpenEvent>;

/// Creates the bounded app-level channel. The UI retains the receiver; each
/// spawned replacement reader receives a clone of the sender.
pub(crate) fn event_channel() -> (ReplacementOpenEventSender, ReplacementOpenEventReceiver) {
    mpsc::sync_channel(REPLACEMENT_OPEN_EVENT_QUEUE_CAPACITY)
}

/// Retained parent-side identity and child process for one acknowledged spawn.
/// WO-002.5 can poll `child` with `try_wait`; dropping the object does not kill
/// or synchronously wait for the child.
pub(crate) struct ReplacementOpenTransport {
    pub(crate) handoff_id: HandoffId,
    pub(crate) child: Child,
}

impl ReplacementOpenTransport {
    /// Polls the retained child without blocking. Dropping this transport never
    /// kills or waits for the child process.
    pub(crate) fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        self.child.try_wait()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReplacementOpenTransportError {
    InvalidDocumentPath,
    SpawnFailed,
    MissingStdoutPipe,
    ReaderThreadFailed,
}

/// Projects the existing startup options plus exactly one document path and
/// the matching WO-002.1 handoff identity. The base projection preserves the
/// development and menu-platform options and rejects non-UTF-8 paths.
pub(crate) fn acknowledged_child_arguments(
    development: bool,
    menu_platform: MenuPlatformChoice,
    document_path: &Path,
    handoff_id: HandoffId,
) -> Result<Vec<OsString>, ReplacementOpenTransportError> {
    let mut arguments = super::child_arguments(development, menu_platform, Some(document_path))
        .map_err(|_| ReplacementOpenTransportError::InvalidDocumentPath)?;
    arguments.push(OsString::from(format!("--open-handoff={}", handoff_id.get())));
    Ok(arguments)
}

/// Starts a replacement child with stdout piped for the one-way acknowledgement
/// protocol. The pipe is taken and its reader thread started before success is
/// returned; any setup failure is reported without killing the already spawned
/// child or retrying with an unacknowledged launch.
pub(crate) fn spawn_acknowledged_replacement(
    executable: &Path,
    development: bool,
    menu_platform: MenuPlatformChoice,
    document_path: &Path,
    handoff_id: HandoffId,
    events: &ReplacementOpenEventSender,
    repaint_context: &egui::Context,
) -> Result<ReplacementOpenTransport, ReplacementOpenTransportError> {
    let arguments = acknowledged_child_arguments(
        development,
        menu_platform,
        document_path,
        handoff_id,
    )?;
    let mut child = Command::new(executable)
        .args(arguments)
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|_| ReplacementOpenTransportError::SpawnFailed)?;

    let stdout = child
        .stdout
        .take()
        .ok_or(ReplacementOpenTransportError::MissingStdoutPipe)?;
    let events = events.clone();
    let repaint_context = repaint_context.clone();
    thread::Builder::new()
        .name("scriptcraft-replacement-open-reader".to_owned())
        .spawn(move || {
            let event = read_terminal_event(stdout, handoff_id);
            forward_event(&events, event, || repaint_context.request_repaint());
        })
        .map_err(|_| ReplacementOpenTransportError::ReaderThreadFailed)?;

    Ok(ReplacementOpenTransport { handoff_id, child })
}

/// Reads a single newline-terminated response while retaining no more than the
/// protocol's fixed maximum plus one detection byte. Only the reader thread may
/// call this function; its potentially blocking pipe read is never UI work.
fn read_terminal_event(
    mut reader: impl Read,
    expected_handoff_id: HandoffId,
) -> ReplacementOpenEvent {
    let mut line = Vec::with_capacity(crate::replacement_open::MAX_HANDOFF_LINE_LENGTH);
    loop {
        let mut byte = [0_u8; 1];
        match reader.read(&mut byte) {
            Ok(0) => {
                return ReplacementOpenEvent::ChannelFailed {
                    handoff_id: expected_handoff_id,
                    reason: ReplacementOpenChannelFailure::EndOfStream,
                };
            }
            Ok(_) => {
                line.push(byte[0]);
                if line.len() > crate::replacement_open::MAX_HANDOFF_LINE_LENGTH {
                    return ReplacementOpenEvent::ChannelFailed {
                        handoff_id: expected_handoff_id,
                        reason: ReplacementOpenChannelFailure::Oversized,
                    };
                }
                if byte[0] == b'\n' {
                    return match decode_handoff_result(&line) {
                        Ok(message) => ReplacementOpenEvent::Result(message),
                        Err(HandoffCodecError::LineTooLong) => {
                            ReplacementOpenEvent::ChannelFailed {
                                handoff_id: expected_handoff_id,
                                reason: ReplacementOpenChannelFailure::Oversized,
                            }
                        }
                        Err(_) => ReplacementOpenEvent::ChannelFailed {
                            handoff_id: expected_handoff_id,
                            reason: ReplacementOpenChannelFailure::Malformed,
                        },
                    };
                }
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => {
                return ReplacementOpenEvent::ChannelFailed {
                    handoff_id: expected_handoff_id,
                    reason: ReplacementOpenChannelFailure::ReadFailed,
                };
            }
        }
    }
}

/// Nonblocking event submission. Repaint is requested only after the event was
/// accepted by the bounded UI queue; full/disconnected queues simply end this
/// one-shot reader without blocking it.
fn forward_event(
    events: &ReplacementOpenEventSender,
    event: ReplacementOpenEvent,
    request_repaint: impl FnOnce(),
) -> bool {
    match events.try_send(event) {
        Ok(()) => {
            request_repaint();
            true
        }
        Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        acknowledged_child_arguments, event_channel, forward_event, read_terminal_event,
        ReplacementOpenTransportError,
    };
    use crate::menu::MenuPlatformChoice;
    use crate::replacement_open::{
        HandoffId, ReplacementOpenChannelFailure, ReplacementOpenEvent, ReplacementOpenMessage,
        ReplacementOpenResult,
    };
    use std::ffi::OsString;
    use std::io::{self, Cursor, Read};
    use std::path::Path;

    #[allow(clippy::panic)]
    fn id(value: &str) -> HandoffId {
        match HandoffId::parse_decimal(value) {
            Ok(id) => id,
            Err(_) => unreachable!("test identity is valid"),
        }
    }

    #[test]
    fn acknowledged_projection_preserves_options_and_adds_one_path_and_identity() {
        assert_eq!(
            acknowledged_child_arguments(
                true,
                MenuPlatformChoice::Windows,
                Path::new("/drafts/Été first draft.fdx"),
                id("27"),
            ),
            Ok(vec![
                OsString::from("--menu-platform=windows"),
                OsString::from("--dev"),
                OsString::from("--document-path=/drafts/Été first draft.fdx"),
                OsString::from("--open-handoff=27"),
            ])
        );
    }

    #[cfg(unix)]
    #[test]
    fn acknowledged_projection_rejects_non_utf8_path() {
        use std::os::unix::ffi::OsStrExt;
        let path = Path::new(std::ffi::OsStr::from_bytes(b"/drafts/\xff.fdx"));
        assert_eq!(
            acknowledged_child_arguments(false, MenuPlatformChoice::Auto, path, id("1")),
            Err(ReplacementOpenTransportError::InvalidDocumentPath)
        );
    }

    #[test]
    fn reader_decodes_one_result_and_preserves_a_stale_wire_identity() {
        let event = read_terminal_event(
            Cursor::new(b"SCRIPTCRAFT-OPEN/1 8 opened\n"),
            id("9"),
        );
        assert_eq!(
            event,
            ReplacementOpenEvent::Result(ReplacementOpenMessage {
                handoff_id: id("8"),
                result: ReplacementOpenResult::Opened,
            })
        );
    }

    #[test]
    fn reader_classifies_malformed_oversized_eof_and_read_failure() {
        assert_eq!(
            read_terminal_event(Cursor::new(b"not a protocol line\n"), id("4")),
            ReplacementOpenEvent::ChannelFailed {
                handoff_id: id("4"),
                reason: ReplacementOpenChannelFailure::Malformed,
            }
        );
        assert_eq!(
            read_terminal_event(Cursor::new(vec![b'x'; 65]), id("4")),
            ReplacementOpenEvent::ChannelFailed {
                handoff_id: id("4"),
                reason: ReplacementOpenChannelFailure::Oversized,
            }
        );
        assert_eq!(
            read_terminal_event(Cursor::new(b""), id("4")),
            ReplacementOpenEvent::ChannelFailed {
                handoff_id: id("4"),
                reason: ReplacementOpenChannelFailure::EndOfStream,
            }
        );
        struct FailedReader;
        impl Read for FailedReader {
            fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::new(io::ErrorKind::Other, "injected"))
            }
        }
        assert_eq!(
            read_terminal_event(FailedReader, id("4")),
            ReplacementOpenEvent::ChannelFailed {
                handoff_id: id("4"),
                reason: ReplacementOpenChannelFailure::ReadFailed,
            }
        );
    }

    #[test]
    fn forwarding_is_nonblocking_and_repaints_only_after_successful_submission() {
        let (sender, receiver) = event_channel();
        let event = ReplacementOpenEvent::Result(ReplacementOpenMessage {
            handoff_id: id("3"),
            result: ReplacementOpenResult::Cancelled,
        });
        let mut repainted = false;
        assert!(forward_event(&sender, event, || repainted = true));
        assert!(repainted);
        assert_eq!(receiver.try_recv(), Ok(event));

        let (full_sender, _full_receiver) = std::sync::mpsc::sync_channel(0);
        let mut repainted = false;
        assert!(!forward_event(&full_sender, event, || repainted = true));
        assert!(!repainted);

        let (disconnected_sender, disconnected_receiver) = event_channel();
        drop(disconnected_receiver);
        assert!(!forward_event(&disconnected_sender, event, || {}));
    }
}
