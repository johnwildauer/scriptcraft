//! Development-only, value-free application responsiveness observations.
//!
//! Update FPS is not display-present FPS. Edit-to-app-paint stops after the
//! application paints the resulting revision and excludes compositor/display
//! latency. Histories and pending state have fixed bounds.

use scriptcraft_commands::CommandId;
use std::time::{Duration, Instant};

const HISTORY_CAPACITY: usize = 120;
const MIN_UPDATE_SAMPLES: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DurationSummary {
    average_nanos: u64,
    p95_nanos: u64,
    maximum_nanos: u64,
}

struct DurationHistory {
    values_nanos: [u64; HISTORY_CAPACITY],
    next: usize,
    len: usize,
}

impl Default for DurationHistory {
    fn default() -> Self {
        Self {
            values_nanos: [0; HISTORY_CAPACITY],
            next: 0,
            len: 0,
        }
    }
}

impl DurationHistory {
    fn push(&mut self, duration: Duration) {
        let nanos = duration.as_nanos().min(u64::MAX as u128) as u64;
        self.values_nanos[self.next] = nanos;
        self.next = (self.next + 1) % HISTORY_CAPACITY;
        self.len = self.len.saturating_add(1).min(HISTORY_CAPACITY);
    }

    fn summary(&self) -> Option<DurationSummary> {
        if self.len == 0 {
            return None;
        }
        let first = (self.next + HISTORY_CAPACITY - self.len) % HISTORY_CAPACITY;
        let mut sorted = [0_u64; HISTORY_CAPACITY];
        let mut total = 0_u128;
        let mut maximum = 0_u64;
        for (offset, value) in sorted.iter_mut().take(self.len).enumerate() {
            let nanos = self.values_nanos[(first + offset) % HISTORY_CAPACITY];
            *value = nanos;
            total = total.saturating_add(u128::from(nanos));
            maximum = maximum.max(nanos);
        }
        sorted[..self.len].sort_unstable();
        let p95_rank = self.len.saturating_mul(95).saturating_add(99) / 100;
        let p95_index = p95_rank.saturating_sub(1).min(self.len - 1);
        Some(DurationSummary {
            average_nanos: (total / self.len as u128).min(u64::MAX as u128) as u64,
            p95_nanos: sorted[p95_index],
            maximum_nanos: maximum,
        })
    }
}

#[derive(Clone, Copy)]
struct UpdateObservation {
    started: Instant,
    ended: Instant,
}

struct UpdateObservationWindow {
    observations: [Option<UpdateObservation>; HISTORY_CAPACITY],
    next: usize,
    len: usize,
}

impl Default for UpdateObservationWindow {
    fn default() -> Self {
        Self {
            observations: [None; HISTORY_CAPACITY],
            next: 0,
            len: 0,
        }
    }
}

impl UpdateObservationWindow {
    fn push(&mut self, started: Instant, ended: Instant) {
        self.observations[self.next] = Some(UpdateObservation { started, ended });
        self.next = (self.next + 1) % HISTORY_CAPACITY;
        self.len = self.len.saturating_add(1).min(HISTORY_CAPACITY);
    }

    fn updates_per_second(&self) -> Option<f64> {
        if self.len < MIN_UPDATE_SAMPLES {
            return None;
        }
        let oldest_index = (self.next + HISTORY_CAPACITY - self.len) % HISTORY_CAPACITY;
        let newest_index = (self.next + HISTORY_CAPACITY - 1) % HISTORY_CAPACITY;
        let oldest = self.observations[oldest_index]?;
        let newest = self.observations[newest_index]?;
        let elapsed = newest.ended.saturating_duration_since(oldest.started);
        let elapsed_seconds = elapsed.as_secs_f64();
        if elapsed_seconds <= 0.0 {
            None
        } else {
            Some(self.len as f64 / elapsed_seconds)
        }
    }
}

#[derive(Clone, Copy)]
struct PendingEditPaint {
    accepted_at: Instant,
    target_revision: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PageGauges {
    visible: usize,
    total: usize,
}

pub(super) struct ResponsivenessDiagnostics {
    update_observations: UpdateObservationWindow,
    update_durations: DurationHistory,
    edit_paint_durations: DurationHistory,
    pending_edit: Option<PendingEditPaint>,
    pages: Option<PageGauges>,
}

impl Default for ResponsivenessDiagnostics {
    fn default() -> Self {
        Self {
            update_observations: UpdateObservationWindow::default(),
            update_durations: DurationHistory::default(),
            edit_paint_durations: DurationHistory::default(),
            pending_edit: None,
            pages: None,
        }
    }
}

impl ResponsivenessDiagnostics {
    pub(super) fn record_update(&mut self, started: Instant, ended: Instant) {
        self.update_observations.push(started, ended);
        self.update_durations
            .push(ended.saturating_duration_since(started));
    }

    pub(super) fn note_accepted_edit(&mut self, revision: u64, accepted_at: Instant) {
        match self.pending_edit.as_mut() {
            Some(pending) => {
                // Coalesce a burst into one worst-case sample: retain the first
                // accepted timestamp and wait for the latest resulting revision.
                pending.target_revision = revision;
            }
            None => {
                self.pending_edit = Some(PendingEditPaint {
                    accepted_at,
                    target_revision: revision,
                });
            }
        }
    }

    pub(super) fn note_revision_painted(&mut self, revision: u64, painted_at: Instant) {
        let Some(pending) = self.pending_edit else {
            return;
        };
        if revision < pending.target_revision {
            return;
        }
        self.pending_edit = None;
        self.edit_paint_durations
            .push(painted_at.saturating_duration_since(pending.accepted_at));
    }

    pub(super) fn clear_pending_edit(&mut self) {
        self.pending_edit = None;
    }

    pub(super) fn note_pages(&mut self, visible: usize, total: usize) {
        self.pages = Some(PageGauges { visible, total });
    }

    fn update_fps(&self) -> Option<f64> {
        self.update_observations.updates_per_second()
    }

    fn update_summary(&self) -> Option<DurationSummary> {
        (self.update_durations.len >= MIN_UPDATE_SAMPLES)
            .then(|| self.update_durations.summary())
            .flatten()
    }

    fn edit_paint_summary(&self) -> Option<DurationSummary> {
        self.edit_paint_durations.summary()
    }

    pub(super) fn safe_summary(&self) -> String {
        let update_fps = self
            .update_fps()
            .map_or_else(|| "collecting".to_owned(), |fps| format!("{fps:.1}"));
        let update_ms = self.update_summary().map_or_else(
            || "collecting".to_owned(),
            |summary| format_duration_summary(summary),
        );
        let edit_paint_ms = self.edit_paint_summary().map_or_else(
            || "collecting".to_owned(),
            |summary| format_duration_summary(summary),
        );
        let pages = self.pages.map_or_else(
            || "collecting".to_owned(),
            |pages| format!("{}/{}", pages.visible, pages.total),
        );
        format!(
            "Update FPS {update_fps} | Update ms avg/p95/max {update_ms} | Edit to app paint ms avg/p95/max {edit_paint_ms} | Pages visible/total {pages}"
        )
    }
}

fn format_duration_summary(summary: DurationSummary) -> String {
    format!(
        "{:.2}/{:.2}/{:.2}",
        summary.average_nanos as f64 / 1_000_000.0,
        summary.p95_nanos as f64 / 1_000_000.0,
        summary.maximum_nanos as f64 / 1_000_000.0
    )
}

pub(super) fn is_text_editing_command(command: CommandId) -> bool {
    matches!(
        command,
        CommandId::EditorTextInsert
            | CommandId::EditorTextPaste
            | CommandId::EditorDeleteBackward
            | CommandId::EditDelete
            | CommandId::EditorParagraphEnter
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rolling_duration_history_is_bounded_and_computes_average_p95_and_max() {
        let mut history = DurationHistory::default();
        for milliseconds in 1..=HISTORY_CAPACITY as u64 + 10 {
            history.push(Duration::from_millis(milliseconds));
        }
        assert_eq!(history.len, HISTORY_CAPACITY);
        assert_eq!(
            history.summary(),
            Some(DurationSummary {
                average_nanos: 70_500_000,
                p95_nanos: 124_000_000,
                maximum_nanos: 130_000_000,
            })
        );
    }

    #[test]
    fn update_fps_uses_actual_elapsed_time_and_a_bounded_rolling_window() {
        let mut diagnostics = ResponsivenessDiagnostics::default();
        let start = Instant::now();
        assert_eq!(diagnostics.update_fps(), None);
        diagnostics.record_update(start, start + Duration::from_millis(10));
        assert_eq!(diagnostics.update_fps(), None);
        diagnostics.record_update(
            start + Duration::from_millis(100),
            start + Duration::from_millis(110),
        );
        let fps = diagnostics.update_fps().expect("two updates provide FPS");
        assert!((fps - 2.0 / 0.110).abs() < 0.000_001);

        for index in 2..=HISTORY_CAPACITY as u64 + 1 {
            let started = start + Duration::from_millis(index * 100);
            diagnostics.record_update(started, started + Duration::from_millis(10));
        }
        assert_eq!(diagnostics.update_observations.len, HISTORY_CAPACITY);
        let elapsed_ms = (HISTORY_CAPACITY as u64 + 1) * 100 + 10 - 100;
        let expected = HISTORY_CAPACITY as f64 / (elapsed_ms as f64 / 1_000.0);
        let rolled_fps = diagnostics.update_fps().expect("rolled window provides FPS");
        assert!((rolled_fps - expected).abs() < 0.000_001);
    }

    #[test]
    fn pending_edit_coalesces_to_earliest_input_and_latest_revision_without_a_queue() {
        let mut diagnostics = ResponsivenessDiagnostics::default();
        let accepted = Instant::now();
        diagnostics.note_accepted_edit(4, accepted);
        diagnostics.note_accepted_edit(5, accepted + Duration::from_millis(2));
        assert!(diagnostics.pending_edit.is_some());
        assert_eq!(diagnostics.edit_paint_durations.len, 0);

        diagnostics.note_revision_painted(4, accepted + Duration::from_millis(5));
        assert_eq!(diagnostics.edit_paint_durations.len, 0);
        diagnostics.note_revision_painted(5, accepted + Duration::from_millis(20));
        assert!(diagnostics.pending_edit.is_none());
        assert_eq!(
            diagnostics.edit_paint_summary(),
            Some(DurationSummary {
                average_nanos: 20_000_000,
                p95_nanos: 20_000_000,
                maximum_nanos: 20_000_000,
            })
        );

        diagnostics.note_revision_painted(5, accepted + Duration::from_millis(30));
        assert_eq!(diagnostics.edit_paint_durations.len, 1);
        diagnostics.note_accepted_edit(6, accepted + Duration::from_millis(40));
        diagnostics.clear_pending_edit();
        diagnostics.note_revision_painted(6, accepted + Duration::from_millis(50));
        assert_eq!(diagnostics.edit_paint_durations.len, 1);
    }

    #[test]
    fn pages_are_current_gauges_and_summary_order_is_stable() {
        let mut diagnostics = ResponsivenessDiagnostics::default();
        assert!(diagnostics.safe_summary().contains("Pages visible/total collecting"));
        diagnostics.note_pages(2, 90);
        diagnostics.note_pages(3, 91);
        assert_eq!(diagnostics.pages, Some(PageGauges { visible: 3, total: 91 }));
        let summary = diagnostics.safe_summary();
        let update_fps = summary.find("Update FPS").expect("FPS label");
        let update_ms = summary.find("Update ms").expect("update duration label");
        let edit_paint = summary
            .find("Edit to app paint ms")
            .expect("edit-paint label");
        let pages = summary.find("Pages visible/total 3/91").expect("page gauges");
        assert!(update_fps < update_ms && update_ms < edit_paint && edit_paint < pages);
        assert!(!summary.contains("focus="));
        assert!(!summary.contains("received-events="));
    }

    #[test]
    fn typing_ime_paste_and_deletion_commands_are_measured_without_selection_commands() {
        for command in [
            CommandId::EditorTextInsert,
            CommandId::EditorTextPaste,
            CommandId::EditorDeleteBackward,
            CommandId::EditDelete,
            CommandId::EditorParagraphEnter,
        ] {
            assert!(is_text_editing_command(command));
        }
        assert!(!is_text_editing_command(CommandId::EditorSelectionSet));
        assert!(!is_text_editing_command(CommandId::EditorNavigationHorizontal));
    }
}
