//! Development-only, value-free application responsiveness observations.
//!
//! Update FPS is not display-present FPS. Edit-to-app-paint stops after the
//! application paints the resulting revision and excludes compositor/display
//! latency. Histories and pending state have fixed bounds.

use scriptcraft_commands::{CommandId, EditorSurface};
use scriptcraft_core::editing::EditMutationDiagnostics;
use scriptcraft_core::layout::LayoutBuildDiagnostics;
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
        self.push_nanos(duration.as_nanos());
    }

    fn push_nanos(&mut self, nanos: u128) {
        let nanos = nanos.min(u64::MAX as u128) as u64;
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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct CommandDetailCounts {
    structural_retries: u64,
    scoped_attempts: u64,
    scoped_accepts: u64,
    full_fallbacks: u64,
    full_validations: u64,
    no_ops: u64,
}

struct CommandDetailCountWindow {
    values: [CommandDetailCounts; HISTORY_CAPACITY],
    next: usize,
    len: usize,
}

impl Default for CommandDetailCountWindow {
    fn default() -> Self {
        Self {
            values: [CommandDetailCounts::default(); HISTORY_CAPACITY],
            next: 0,
            len: 0,
        }
    }
}

impl CommandDetailCountWindow {
    fn push(&mut self, observation: &EditMutationDiagnostics) {
        self.values[self.next] = CommandDetailCounts {
            structural_retries: observation.structural_retries,
            scoped_attempts: observation.scoped_candidate_attempts,
            scoped_accepts: observation.scoped_candidate_accepts,
            full_fallbacks: observation.full_validation_fallbacks,
            full_validations: observation.full_candidate_validations,
            no_ops: u64::from(observation.no_op),
        };
        self.next = (self.next + 1) % HISTORY_CAPACITY;
        self.len = self.len.saturating_add(1).min(HISTORY_CAPACITY);
    }

    fn totals(&self) -> CommandDetailCounts {
        let first = (self.next + HISTORY_CAPACITY - self.len) % HISTORY_CAPACITY;
        let mut totals = CommandDetailCounts::default();
        for offset in 0..self.len {
            let counts = self.values[(first + offset) % HISTORY_CAPACITY];
            totals.structural_retries = totals.structural_retries.saturating_add(counts.structural_retries);
            totals.scoped_attempts = totals.scoped_attempts.saturating_add(counts.scoped_attempts);
            totals.scoped_accepts = totals.scoped_accepts.saturating_add(counts.scoped_accepts);
            totals.full_fallbacks = totals.full_fallbacks.saturating_add(counts.full_fallbacks);
            totals.full_validations = totals.full_validations.saturating_add(counts.full_validations);
            totals.no_ops = totals.no_ops.saturating_add(counts.no_ops);
        }
        totals
    }
}

#[derive(Default)]
struct CommandDetailDiagnostics {
    executor_durations: DurationHistory,
    core_durations: DurationHistory,
    candidate_setup_durations: DurationHistory,
    edit_operation_durations: DurationHistory,
    structural_retry_durations: DurationHistory,
    preservation_durations: DurationHistory,
    scoped_validation_durations: DurationHistory,
    full_validation_durations: DurationHistory,
    no_op_equality_durations: DurationHistory,
    derived_classification_durations: DurationHistory,
    dirty_comparison_durations: DurationHistory,
    history_publication_durations: DurationHistory,
    counts: CommandDetailCountWindow,
}

impl CommandDetailDiagnostics {
    fn record(&mut self, executor_duration: Duration, observation: EditMutationDiagnostics) {
        self.executor_durations.push(executor_duration);
        self.core_durations.push_nanos(u128::from(observation.total_nanos));
        self.candidate_setup_durations.push_nanos(u128::from(observation.candidate_setup_nanos));
        self.edit_operation_durations.push_nanos(u128::from(observation.edit_operation_nanos));
        self.preservation_durations.push_nanos(u128::from(observation.preservation_nanos));
        push_optional_nanos(&mut self.structural_retry_durations, observation.structural_retry_nanos);
        push_optional_nanos(&mut self.scoped_validation_durations, observation.scoped_validation_nanos);
        push_optional_nanos(&mut self.full_validation_durations, observation.full_validation_nanos);
        push_optional_nanos(&mut self.no_op_equality_durations, observation.no_op_equality_nanos);
        push_optional_nanos(&mut self.derived_classification_durations, observation.derived_classification_nanos);
        push_optional_nanos(&mut self.dirty_comparison_durations, observation.dirty_comparison_nanos);
        push_optional_nanos(&mut self.history_publication_durations, observation.history_publication_nanos);
        self.counts.push(&observation);
    }

    fn summaries(&self) -> [String; 3] {
        let counts = self.counts.totals();
        let executor = format_optional_duration_summary(&self.executor_durations);
        let core = format_optional_duration_summary(&self.core_durations);
        let setup = format_optional_duration_summary(&self.candidate_setup_durations);
        let edit = format_optional_duration_summary(&self.edit_operation_durations);
        let retry = format_optional_duration_summary(&self.structural_retry_durations);
        let preservation = format_optional_duration_summary(&self.preservation_durations);
        let scoped = format_optional_duration_summary(&self.scoped_validation_durations);
        let full = format_optional_duration_summary(&self.full_validation_durations);
        let no_op = format_optional_duration_summary(&self.no_op_equality_durations);
        let derived = format_optional_duration_summary(&self.derived_classification_durations);
        let dirty = format_optional_duration_summary(&self.dirty_comparison_durations);
        let publication = format_optional_duration_summary(&self.history_publication_durations);
        [
            format!(
                "Cmd core ms avg/p95/max | Executor {executor} | Mutation {core} | Setup {setup} | Edit {edit} | Retry {retry} r{}",
                counts.structural_retries
            ),
            format!(
                "Cmd checks ms avg/p95/max | Preserve {preservation} | Scoped {scoped} sa/sA {}/{} | Full {full} fb/fv {}/{} | NoEq {no_op} n{}",
                counts.scoped_attempts,
                counts.scoped_accepts,
                counts.full_fallbacks,
                counts.full_validations,
                counts.no_ops
            ),
            format!(
                "Cmd publish ms avg/p95/max | Dirty {dirty} | Derived {derived} | Hist/pub {publication}"
            ),
        ]
    }
}

fn push_optional_nanos(history: &mut DurationHistory, nanos: Option<u64>) {
    if let Some(nanos) = nanos {
        history.push_nanos(u128::from(nanos));
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
pub(super) enum RootOtherBucket {
    InputDispatch,
    Derived,
    Chrome,
    Host,
}

impl RootOtherBucket {
    const COUNT: usize = 4;

    const fn index(self) -> usize {
        match self {
            Self::InputDispatch => 0,
            Self::Derived => 1,
            Self::Chrome => 2,
            Self::Host => 3,
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct AlignedOtherBucketLedger {
    elapsed_nanos: [u128; RootOtherBucket::COUNT],
    command_nanos: [u128; RootOtherBucket::COUNT],
    surface_nanos: [u128; RootOtherBucket::COUNT],
}

impl AlignedOtherBucketLedger {
    fn net_nanos(&self, bucket: RootOtherBucket) -> u128 {
        self.elapsed_nanos[bucket.index()]
            .saturating_sub(self.command_nanos[bucket.index()])
            .saturating_sub(self.surface_nanos[bucket.index()])
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct AlignedSurfaceObservation {
    duration_nanos: u128,
    page_paint_nanos: u128,
    layout_nanos: u128,
    layout_hits: u64,
    layout_builds: u64,
    painted_accepted_edit: bool,
}

#[derive(Debug, Default, Clone, Copy)]
struct PendingAlignedUpdate {
    command_nanos: u128,
    command_inside_surface_nanos: u128,
    command_outside_surface_nanos: u128,
    command_outside_nonoverlap_nanos: u128,
    other_buckets: AlignedOtherBucketLedger,
    diagnostics_calculation_nanos: u128,
    surfaces: [Option<AlignedSurfaceObservation>; 2],
    painted_accepted_revision: Option<u64>,
}

#[derive(Debug, Clone, Copy)]
struct AlignedBuildSurfaceSample {
    surface_nanos: u128,
    layout_nanos: u128,
    page_paint_nanos: u128,
}

/// All fields are finalized together from one root update. Layout and Page
/// paint remain nested in Surface; inside-surface Commands remain nested too.
#[derive(Debug, Clone, Copy)]
struct AlignedEditUpdateSample {
    update_nanos: u128,
    command_nanos: u128,
    command_inside_surface_nanos: u128,
    command_outside_surface_nanos: u128,
    surface_nanos: u128,
    build_surface: Option<AlignedBuildSurfaceSample>,
    other_surface_hits: u64,
    other_surface_builds: u64,
    residual_nanos: u128,
    input_dispatch_nanos: u128,
    derived_nanos: u128,
    chrome_nanos: u128,
    diagnostics_calculation_nanos: u128,
    host_nanos: u128,
    other_remainder_nanos: u128,
}

/// One actual build's aggregate stages. Rebase is nested in Placement and Sort
/// is nested in Indexes; neither is subtracted when computing Other.
#[derive(Debug, Clone, Copy)]
struct LayoutBuildStageSample {
    prepared_lookup_nanos: u128,
    prepared_miss_nanos: Option<u128>,
    placement_nanos: u128,
    rebase_nanos: u128,
    interaction_index_nanos: u128,
    body_y_sort_nanos: u128,
    other_nanos: u128,
    prepared_hits: u64,
    prepared_misses: u64,
}

impl From<&LayoutBuildDiagnostics> for LayoutBuildStageSample {
    fn from(observation: &LayoutBuildDiagnostics) -> Self {
        let nonoverlap = observation
            .prepared_lookup_nanos
            .saturating_add(observation.prepared_miss_nanos)
            .saturating_add(observation.placement_nanos)
            .saturating_add(observation.interaction_index_nanos);
        Self {
            prepared_lookup_nanos: observation.prepared_lookup_nanos,
            prepared_miss_nanos: (observation.prepared_misses > 0)
                .then_some(observation.prepared_miss_nanos),
            placement_nanos: observation.placement_nanos,
            rebase_nanos: observation.rebase_nanos,
            interaction_index_nanos: observation.interaction_index_nanos,
            body_y_sort_nanos: observation.body_y_sort_nanos,
            other_nanos: observation.whole_build_nanos.saturating_sub(nonoverlap),
            prepared_hits: observation.prepared_hits,
            prepared_misses: observation.prepared_misses,
        }
    }
}

struct BoundedSampleWindow<S: Copy> {
    values: [Option<S>; HISTORY_CAPACITY],
    next: usize,
    len: usize,
}

impl<S: Copy> Default for BoundedSampleWindow<S> {
    fn default() -> Self {
        Self {
            values: [None; HISTORY_CAPACITY],
            next: 0,
            len: 0,
        }
    }
}

impl<S: Copy> BoundedSampleWindow<S> {
    fn push(&mut self, sample: S) {
        self.values[self.next] = Some(sample);
        self.next = (self.next + 1) % HISTORY_CAPACITY;
        self.len = self.len.saturating_add(1).min(HISTORY_CAPACITY);
    }

    fn summary(&self, select: impl Fn(&S) -> Option<u128>) -> Option<DurationSummary> {
        let first = (self.next + HISTORY_CAPACITY - self.len) % HISTORY_CAPACITY;
        let mut sorted = [0_u64; HISTORY_CAPACITY];
        let mut count = 0_usize;
        let mut total = 0_u128;
        let mut maximum = 0_u64;
        for offset in 0..self.len {
            let Some(sample) = self.values[(first + offset) % HISTORY_CAPACITY].as_ref() else {
                continue;
            };
            let Some(nanos) = select(sample) else {
                continue;
            };
            let nanos = nanos.min(u64::MAX as u128) as u64;
            sorted[count] = nanos;
            count = count.saturating_add(1);
            total = total.saturating_add(u128::from(nanos));
            maximum = maximum.max(nanos);
        }
        if count == 0 {
            return None;
        }
        sorted[..count].sort_unstable();
        let p95_rank = count.saturating_mul(95).saturating_add(99) / 100;
        let p95_index = p95_rank.saturating_sub(1).min(count - 1);
        Some(DurationSummary {
            average_nanos: (total / count as u128).min(u64::MAX as u128) as u64,
            p95_nanos: sorted[p95_index],
            maximum_nanos: maximum,
        })
    }

    fn total(&self, select: impl Fn(&S) -> u64) -> u64 {
        self.values
            .iter()
            .flatten()
            .fold(0_u64, |total, sample| total.saturating_add(select(sample)))
    }
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
    editor_command_durations: DurationHistory,
    layout_hit_durations: DurationHistory,
    layout_build_durations: DurationHistory,
    command_detail: CommandDetailDiagnostics,
    document_surface_durations: DurationHistory,
    page_paint_durations: DurationHistory,
    aligned_edit_updates: BoundedSampleWindow<AlignedEditUpdateSample>,
    layout_build_stages: BoundedSampleWindow<LayoutBuildStageSample>,
    pending_edit: Option<PendingEditPaint>,
    current_update: Option<PendingAlignedUpdate>,
    active_surface: Option<EditorSurface>,
    active_other_bucket: Option<RootOtherBucket>,
    editor_command_depth: usize,
    pages: Option<PageGauges>,
}

impl Default for ResponsivenessDiagnostics {
    fn default() -> Self {
        Self {
            update_observations: UpdateObservationWindow::default(),
            update_durations: DurationHistory::default(),
            edit_paint_durations: DurationHistory::default(),
            editor_command_durations: DurationHistory::default(),
            layout_hit_durations: DurationHistory::default(),
            layout_build_durations: DurationHistory::default(),
            command_detail: CommandDetailDiagnostics::default(),
            document_surface_durations: DurationHistory::default(),
            page_paint_durations: DurationHistory::default(),
            aligned_edit_updates: BoundedSampleWindow::default(),
            layout_build_stages: BoundedSampleWindow::default(),
            pending_edit: None,
            current_update: None,
            active_surface: None,
            active_other_bucket: None,
            editor_command_depth: 0,
            pages: None,
        }
    }
}

fn aligned_surface_index(surface: EditorSurface) -> usize {
    match surface {
        EditorSurface::Body => 0,
        EditorSurface::TitlePage => 1,
    }
}

fn finalize_aligned_edit_update(
    update: PendingAlignedUpdate,
    update_nanos: u128,
) -> AlignedEditUpdateSample {
    // The painting surface owns the relevant build only when that same surface
    // actually missed the complete-layout cache. A cached final paint does not
    // borrow another surface's build or invent a zero-valued build sample.
    let build_surface_index = update.surfaces.iter().position(|surface| {
        surface.is_some_and(|surface| surface.painted_accepted_edit && surface.layout_builds > 0)
    });
    let build_surface = build_surface_index.and_then(|index| {
        update.surfaces[index].map(|surface| AlignedBuildSurfaceSample {
            surface_nanos: surface.duration_nanos,
            layout_nanos: surface.layout_nanos,
            page_paint_nanos: surface.page_paint_nanos,
        })
    });
    let mut other_surface_hits = 0_u64;
    let mut other_surface_builds = 0_u64;
    for (index, surface) in update.surfaces.iter().enumerate() {
        if Some(index) == build_surface_index {
            continue;
        }
        if let Some(surface) = surface {
            other_surface_hits = other_surface_hits.saturating_add(surface.layout_hits);
            other_surface_builds = other_surface_builds.saturating_add(surface.layout_builds);
        }
    }
    let surface_nanos = update.surfaces.iter().flatten().fold(0_u128, |total, surface| {
        total.saturating_add(surface.duration_nanos)
    });
    // Exact residual ownership: document surfaces are sequential root-update
    // work, while only outermost Commands outside every surface are additional
    // nonoverlapping work. Nested Layout/Page/inside-Commands are never subtracted.
    let residual_nanos = update_nanos
        .saturating_sub(surface_nanos)
        .saturating_sub(update.command_outside_nonoverlap_nanos);
    let input_dispatch_nanos = update
        .other_buckets
        .net_nanos(RootOtherBucket::InputDispatch);
    let derived_nanos = update.other_buckets.net_nanos(RootOtherBucket::Derived);
    let chrome_nanos = update.other_buckets.net_nanos(RootOtherBucket::Chrome);
    let host_nanos = update.other_buckets.net_nanos(RootOtherBucket::Host);
    let other_remainder_nanos = residual_nanos
        .saturating_sub(input_dispatch_nanos)
        .saturating_sub(derived_nanos)
        .saturating_sub(chrome_nanos)
        .saturating_sub(host_nanos);
    AlignedEditUpdateSample {
        update_nanos,
        command_nanos: update.command_nanos,
        command_inside_surface_nanos: update.command_inside_surface_nanos,
        command_outside_surface_nanos: update.command_outside_surface_nanos,
        surface_nanos,
        build_surface,
        other_surface_hits,
        other_surface_builds,
        residual_nanos,
        input_dispatch_nanos,
        derived_nanos,
        chrome_nanos,
        diagnostics_calculation_nanos: update.diagnostics_calculation_nanos,
        host_nanos,
        other_remainder_nanos,
    }
}

impl ResponsivenessDiagnostics {
    pub(super) fn begin_update(&mut self) {
        self.current_update = Some(PendingAlignedUpdate::default());
        self.active_surface = None;
        self.active_other_bucket = None;
        self.editor_command_depth = 0;
    }

    pub(super) fn begin_other_bucket(&mut self, bucket: RootOtherBucket) {
        self.active_other_bucket = Some(bucket);
    }

    pub(super) fn end_other_bucket(&mut self, bucket: RootOtherBucket, duration: Duration) {
        if self.active_other_bucket == Some(bucket) {
            if let Some(update) = self.current_update.as_mut() {
                let index = bucket.index();
                update.other_buckets.elapsed_nanos[index] = update.other_buckets.elapsed_nanos[index]
                    .saturating_add(duration.as_nanos());
            }
        }
        self.active_other_bucket = None;
    }

    pub(super) fn record_diagnostics_calculation(&mut self, duration: Duration) {
        if self.active_other_bucket != Some(RootOtherBucket::Chrome) {
            return;
        }
        if let Some(update) = self.current_update.as_mut() {
            update.diagnostics_calculation_nanos = update
                .diagnostics_calculation_nanos
                .saturating_add(duration.as_nanos());
        }
    }

    pub(super) fn begin_editor_command(&mut self) {
        self.editor_command_depth = self.editor_command_depth.saturating_add(1);
    }

    pub(super) fn end_editor_command(&mut self, duration: Duration) {
        let outermost = self.editor_command_depth == 1;
        self.editor_command_depth = self.editor_command_depth.saturating_sub(1);
        self.record_editor_command_inner(duration, outermost);
    }

    pub(super) fn record_editor_command(&mut self, duration: Duration) {
        self.record_editor_command_inner(duration, true);
    }

    fn record_editor_command_inner(&mut self, duration: Duration, outermost: bool) {
        self.editor_command_durations.push(duration);
        let nanos = duration.as_nanos();
        let inside_surface = self.active_surface.is_some();
        let active_other_bucket = self.active_other_bucket;
        if let Some(update) = self.current_update.as_mut() {
            update.command_nanos = update.command_nanos.saturating_add(nanos);
            if inside_surface {
                update.command_inside_surface_nanos = update
                    .command_inside_surface_nanos
                    .saturating_add(nanos);
            } else {
                update.command_outside_surface_nanos = update
                    .command_outside_surface_nanos
                    .saturating_add(nanos);
                // Nested internal commands remain in the raw aggregate but are
                // not subtracted again from the root Update or its active bucket.
                if outermost {
                    update.command_outside_nonoverlap_nanos = update
                        .command_outside_nonoverlap_nanos
                        .saturating_add(nanos);
                    if let Some(bucket) = active_other_bucket {
                        let index = bucket.index();
                        update.other_buckets.command_nanos[index] = update.other_buckets
                            .command_nanos[index]
                            .saturating_add(nanos);
                    }
                }
            }
        }
    }

    pub(super) fn begin_document_surface(&mut self, surface: EditorSurface) {
        self.active_surface = Some(surface);
        let index = aligned_surface_index(surface);
        if let Some(update) = self.current_update.as_mut() {
            update.surfaces[index].get_or_insert_with(AlignedSurfaceObservation::default);
        }
    }

    pub(super) fn record_layout(&mut self, duration: Duration, cache_hit: bool) {
        if cache_hit {
            self.layout_hit_durations.push(duration);
        } else {
            self.layout_build_durations.push(duration);
        }
        let nanos = duration.as_nanos();
        if let (Some(update), Some(surface)) =
            (self.current_update.as_mut(), self.active_surface)
        {
            let observation = update.surfaces[aligned_surface_index(surface)]
                .get_or_insert_with(AlignedSurfaceObservation::default);
            observation.layout_nanos = observation.layout_nanos.saturating_add(nanos);
            if cache_hit {
                observation.layout_hits = observation.layout_hits.saturating_add(1);
            } else {
                observation.layout_builds = observation.layout_builds.saturating_add(1);
            }
        }
    }

    pub(super) fn record_layout_build(&mut self, observation: &LayoutBuildDiagnostics) {
        self.layout_build_stages
            .push(LayoutBuildStageSample::from(observation));
    }

    pub(super) fn record_command_detail(
        &mut self,
        executor_duration: Duration,
        observation: EditMutationDiagnostics,
    ) {
        self.command_detail.record(executor_duration, observation);
    }

    pub(super) fn record_document_surface(
        &mut self,
        duration: Duration,
        paint_nanos_before: u128,
        paint_nanos_after: u128,
    ) {
        self.document_surface_durations.push(duration);
        let paint_nanos = paint_nanos_after.saturating_sub(paint_nanos_before);
        self.page_paint_durations.push_nanos(paint_nanos);
        if let (Some(update), Some(bucket)) =
            (self.current_update.as_mut(), self.active_other_bucket)
        {
            let index = bucket.index();
            update.other_buckets.surface_nanos[index] = update.other_buckets.surface_nanos[index]
                .saturating_add(duration.as_nanos());
        }
        if let (Some(update), Some(surface)) =
            (self.current_update.as_mut(), self.active_surface)
        {
            let observation = update.surfaces[aligned_surface_index(surface)]
                .get_or_insert_with(AlignedSurfaceObservation::default);
            observation.duration_nanos = observation
                .duration_nanos
                .saturating_add(duration.as_nanos());
            observation.page_paint_nanos = observation
                .page_paint_nanos
                .saturating_add(paint_nanos);
        }
        self.active_surface = None;
    }

    pub(super) fn record_update(&mut self, started: Instant, ended: Instant) {
        self.update_observations.push(started, ended);
        let update_nanos = ended.saturating_duration_since(started).as_nanos();
        self.update_durations.push_nanos(update_nanos);
        self.active_surface = None;
        self.active_other_bucket = None;
        self.editor_command_depth = 0;
        if let Some(update) = self.current_update.take() {
            if update.painted_accepted_revision.is_some() {
                self.aligned_edit_updates
                    .push(finalize_aligned_edit_update(update, update_nanos));
            }
        }
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
        if let Some(update) = self.current_update.as_mut() {
            update.painted_accepted_revision = Some(revision);
            if let Some(surface) = self.active_surface {
                let observation = update.surfaces[aligned_surface_index(surface)]
                    .get_or_insert_with(AlignedSurfaceObservation::default);
                observation.painted_accepted_edit = true;
            }
        }
    }

    pub(super) fn clear_pending_edit(&mut self) {
        self.pending_edit = None;
    }

    pub(super) fn note_pages(&mut self, visible: usize, total: usize) {
        self.pages = Some(PageGauges { visible, total });
    }

    pub(super) fn aligned_edit_update_summaries(&self) -> [String; 2] {
        let samples = &self.aligned_edit_updates;
        let update = format_optional_summary(samples.summary(|sample| Some(sample.update_nanos)));
        let command = format_optional_summary(samples.summary(|sample| Some(sample.command_nanos)));
        let surface = format_optional_summary(samples.summary(|sample| Some(sample.surface_nanos)));
        let residual = format_optional_summary(samples.summary(|sample| Some(sample.residual_nanos)));
        let build_surface = format_optional_summary(
            samples.summary(|sample| sample.build_surface.map(|surface| surface.surface_nanos)),
        );
        let layout = format_optional_summary(
            samples.summary(|sample| sample.build_surface.map(|surface| surface.layout_nanos)),
        );
        let page_paint = format_optional_summary(
            samples.summary(|sample| sample.build_surface.map(|surface| surface.page_paint_nanos)),
        );
        let command_inside = format_optional_summary(
            samples.summary(|sample| Some(sample.command_inside_surface_nanos)),
        );
        let command_outside = format_optional_summary(
            samples.summary(|sample| Some(sample.command_outside_surface_nanos)),
        );
        let other_hits = samples.total(|sample| sample.other_surface_hits);
        let other_builds = samples.total(|sample| sample.other_surface_builds);
        [
            format!(
                "Edit update ms avg/p95/max | Update {update} | Command {command} | Surface {surface} | Other {residual}"
            ),
            format!(
                "Edit update nested ms avg/p95/max | Build Surface {build_surface} | Layout {layout} | Page paint {page_paint} | Cmd in/out {command_inside}/{command_outside} | Other surfaces h/b {other_hits}/{other_builds}"
            ),
        ]
    }

    pub(super) fn aligned_edit_update_other_detail_summary(&self) -> String {
        let samples = &self.aligned_edit_updates;
        let input = format_optional_summary(
            samples.summary(|sample| Some(sample.input_dispatch_nanos)),
        );
        let derived = format_optional_summary(samples.summary(|sample| Some(sample.derived_nanos)));
        let chrome = format_optional_summary(samples.summary(|sample| Some(sample.chrome_nanos)));
        let diagnostics = format_optional_summary(
            samples.summary(|sample| Some(sample.diagnostics_calculation_nanos)),
        );
        let host = format_optional_summary(samples.summary(|sample| Some(sample.host_nanos)));
        let remainder = format_optional_summary(
            samples.summary(|sample| Some(sample.other_remainder_nanos)),
        );
        format!(
            "Edit update Other detail ms avg/p95/max | Input/dispatch {input} | Derived {derived} | Chrome {chrome} | Diagnostics nested {diagnostics} | Host {host} | Remainder {remainder}"
        )
    }

    pub(super) fn layout_build_detail_summary(&self) -> String {
        let samples = &self.layout_build_stages;
        let lookup = format_optional_summary(
            samples.summary(|sample| Some(sample.prepared_lookup_nanos)),
        );
        let miss = format_optional_summary(samples.summary(|sample| sample.prepared_miss_nanos));
        let placement = format_optional_summary(samples.summary(|sample| Some(sample.placement_nanos)));
        let rebase = format_optional_summary(samples.summary(|sample| Some(sample.rebase_nanos)));
        let indexes = format_optional_summary(
            samples.summary(|sample| Some(sample.interaction_index_nanos)),
        );
        let sort = format_optional_summary(samples.summary(|sample| Some(sample.body_y_sort_nanos)));
        let other = format_optional_summary(samples.summary(|sample| Some(sample.other_nanos)));
        let hits = samples.total(|sample| sample.prepared_hits);
        let misses = samples.total(|sample| sample.prepared_misses);
        format!(
            "Layout build detail ms avg/p95/max | Prep/lookup {lookup} h/m {hits}/{misses} | Miss prep {miss} | Placement {placement} | Rebase nested {rebase} | Indexes {indexes} | Sort nested {sort} | Other {other}"
        )
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

    pub(super) fn phase_summary(&self) -> String {
        let command = format_optional_duration_summary(&self.editor_command_durations);
        let layout_hit = format_optional_duration_summary(&self.layout_hit_durations);
        let layout_build = format_optional_duration_summary(&self.layout_build_durations);
        let surface = format_optional_duration_summary(&self.document_surface_durations);
        let page_paint = format_optional_duration_summary(&self.page_paint_durations);
        format!(
            "Phases ms avg/p95/max | Command {command} | Layout hit {layout_hit} | Layout build {layout_build} | Surface {surface} | Page paint {page_paint}"
        )
    }

    pub(super) fn command_detail_summaries(&self) -> [String; 3] {
        self.command_detail.summaries()
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

fn format_optional_summary(summary: Option<DurationSummary>) -> String {
    summary.map_or_else(|| "collecting".to_owned(), format_duration_summary)
}

fn format_optional_duration_summary(history: &DurationHistory) -> String {
    history.summary().map_or_else(
        || "collecting".to_owned(),
        format_duration_summary,
    )
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
    fn phase_summary_separates_layout_hits_from_builds_and_aggregates_surface_page_paint() {
        let mut diagnostics = ResponsivenessDiagnostics::default();
        assert_eq!(
            diagnostics.phase_summary(),
            "Phases ms avg/p95/max | Command collecting | Layout hit collecting | Layout build collecting | Surface collecting | Page paint collecting"
        );

        diagnostics.record_editor_command(Duration::from_millis(1));
        diagnostics.record_editor_command(Duration::from_millis(3));
        diagnostics.record_layout(Duration::from_millis(4), true);
        diagnostics.record_layout(Duration::from_millis(8), true);
        diagnostics.record_layout(Duration::from_millis(40), false);
        diagnostics.record_document_surface(
            Duration::from_millis(20),
            2_000_000,
            8_000_000,
        );

        assert_eq!(diagnostics.layout_hit_durations.len, 2);
        assert_eq!(diagnostics.layout_build_durations.len, 1);
        assert_eq!(diagnostics.document_surface_durations.len, 1);
        assert_eq!(diagnostics.page_paint_durations.len, 1);
        assert_eq!(
            diagnostics.phase_summary(),
            "Phases ms avg/p95/max | Command 2.00/3.00/3.00 | Layout hit 6.00/8.00/8.00 | Layout build 40.00/40.00/40.00 | Surface 20.00/20.00/20.00 | Page paint 6.00/6.00/6.00"
        );
        assert!(!diagnostics.safe_summary().contains("Phases ms"));
    }

    #[test]
    fn layout_hit_rollover_never_evicts_or_averages_the_build_population() {
        let mut diagnostics = ResponsivenessDiagnostics::default();
        diagnostics.record_layout(Duration::from_millis(40), false);
        for _ in 0..HISTORY_CAPACITY + 20 {
            diagnostics.record_layout(Duration::from_millis(2), true);
        }

        assert_eq!(diagnostics.layout_hit_durations.len, HISTORY_CAPACITY);
        assert_eq!(diagnostics.layout_build_durations.len, 1);
        assert!(diagnostics.phase_summary().contains(
            "Layout hit 2.00/2.00/2.00 | Layout build 40.00/40.00/40.00"
        ));
    }

    #[test]
    fn command_detail_collecting_states_do_not_fabricate_zero_durations() {
        let diagnostics = ResponsivenessDiagnostics::default();
        assert_eq!(
            diagnostics.command_detail_summaries(),
            [
                "Cmd core ms avg/p95/max | Executor collecting | Mutation collecting | Setup collecting | Edit collecting | Retry collecting r0",
                "Cmd checks ms avg/p95/max | Preserve collecting | Scoped collecting sa/sA 0/0 | Full collecting fb/fv 0/0 | NoEq collecting n0",
                "Cmd publish ms avg/p95/max | Dirty collecting | Derived collecting | Hist/pub collecting",
            ]
        );
    }

    #[test]
    fn command_detail_records_durations_and_rolls_counts_with_actual_observations() {
        let mut diagnostics = ResponsivenessDiagnostics::default();
        diagnostics.record_command_detail(
            Duration::from_millis(20),
            EditMutationDiagnostics {
                total_nanos: 10_000_000,
                candidate_setup_nanos: 1_000_000,
                edit_operation_nanos: 2_000_000,
                structural_retry_nanos: Some(2_000_000),
                preservation_nanos: 3_000_000,
                scoped_validation_nanos: Some(4_000_000),
                no_op_equality_nanos: Some(1_000_000),
                derived_classification_nanos: Some(6_000_000),
                dirty_comparison_nanos: Some(1_000_000),
                history_publication_nanos: Some(7_000_000),
                structural_retries: 1,
                scoped_candidate_attempts: 1,
                scoped_candidate_accepts: 1,
                no_op: true,
                ..EditMutationDiagnostics::default()
            },
        );
        diagnostics.record_command_detail(
            Duration::from_millis(40),
            EditMutationDiagnostics {
                total_nanos: 30_000_000,
                candidate_setup_nanos: 3_000_000,
                edit_operation_nanos: 4_000_000,
                preservation_nanos: 5_000_000,
                full_validation_nanos: Some(20_000_000),
                no_op_equality_nanos: Some(1_000_000),
                full_candidate_validations: 1,
                full_validation_fallbacks: 1,
                ..EditMutationDiagnostics::default()
            },
        );

        assert_eq!(diagnostics.command_detail.core_durations.len, 2);
        assert_eq!(diagnostics.command_detail.scoped_validation_durations.len, 1);
        assert_eq!(diagnostics.command_detail.full_validation_durations.len, 1);
        assert_eq!(diagnostics.command_detail.structural_retry_durations.len, 1);
        assert_eq!(
            diagnostics.command_detail_summaries(),
            [
                "Cmd core ms avg/p95/max | Executor 30.00/40.00/40.00 | Mutation 20.00/30.00/30.00 | Setup 2.00/3.00/3.00 | Edit 3.00/4.00/4.00 | Retry 2.00/2.00/2.00 r1",
                "Cmd checks ms avg/p95/max | Preserve 4.00/5.00/5.00 | Scoped 4.00/4.00/4.00 sa/sA 1/1 | Full 20.00/20.00/20.00 fb/fv 1/1 | NoEq 1.00/1.00/1.00 n1",
                "Cmd publish ms avg/p95/max | Dirty 1.00/1.00/1.00 | Derived 6.00/6.00/6.00 | Hist/pub 7.00/7.00/7.00",
            ]
        );

        for _ in 0..HISTORY_CAPACITY + 10 {
            diagnostics.record_command_detail(
                Duration::ZERO,
                EditMutationDiagnostics {
                    no_op: true,
                    ..EditMutationDiagnostics::default()
                },
            );
        }
        assert_eq!(diagnostics.command_detail.counts.len, HISTORY_CAPACITY);
        assert_eq!(diagnostics.command_detail.counts.totals().no_ops, HISTORY_CAPACITY as u64);
    }

    fn paint_aligned_edit(
        diagnostics: &mut ResponsivenessDiagnostics,
        started: Instant,
        revision: u64,
        update_milliseconds: u64,
    ) {
        diagnostics.begin_update();
        diagnostics.note_accepted_edit(revision, started);
        diagnostics.begin_other_bucket(RootOtherBucket::InputDispatch);
        diagnostics.end_other_bucket(RootOtherBucket::InputDispatch, Duration::from_millis(1));
        diagnostics.begin_other_bucket(RootOtherBucket::Derived);
        diagnostics.end_other_bucket(RootOtherBucket::Derived, Duration::from_millis(2));
        diagnostics.begin_other_bucket(RootOtherBucket::Chrome);
        diagnostics.record_diagnostics_calculation(Duration::from_millis(3));
        diagnostics.begin_document_surface(EditorSurface::Body);
        diagnostics.record_layout(Duration::from_millis(1), false);
        diagnostics.note_revision_painted(revision, started + Duration::from_millis(1));
        diagnostics.record_document_surface(Duration::from_millis(1), 0, 0);
        diagnostics.end_other_bucket(RootOtherBucket::Chrome, Duration::from_millis(5));
        diagnostics.begin_other_bucket(RootOtherBucket::Host);
        diagnostics.end_other_bucket(RootOtherBucket::Host, Duration::from_millis(4));
        diagnostics.record_update(
            started,
            started + Duration::from_millis(update_milliseconds),
        );
    }

    #[test]
    fn aligned_edit_update_owns_final_paint_and_never_subtracts_nested_command_twice() {
        let mut diagnostics = ResponsivenessDiagnostics::default();
        let started = Instant::now();
        diagnostics.begin_update();
        diagnostics.note_accepted_edit(8, started);

        diagnostics.begin_editor_command();
        diagnostics.begin_editor_command();
        diagnostics.end_editor_command(Duration::from_millis(3));
        diagnostics.end_editor_command(Duration::from_millis(5));

        diagnostics.begin_document_surface(EditorSurface::Body);
        diagnostics.begin_editor_command();
        diagnostics.end_editor_command(Duration::from_millis(7));
        diagnostics.record_layout(Duration::from_millis(40), false);
        diagnostics.note_revision_painted(8, started + Duration::from_millis(45));
        diagnostics.record_document_surface(Duration::from_millis(50), 2_000_000, 12_000_000);

        diagnostics.begin_document_surface(EditorSurface::TitlePage);
        diagnostics.record_layout(Duration::from_millis(2), true);
        diagnostics.record_document_surface(Duration::from_millis(20), 0, 0);
        diagnostics.record_update(started, started + Duration::from_millis(100));

        assert_eq!(diagnostics.aligned_edit_updates.len, 1);
        let sample = diagnostics.aligned_edit_updates.values.iter().flatten().next().expect("aligned sample");
        assert_eq!(sample.update_nanos, Duration::from_millis(100).as_nanos());
        assert_eq!(sample.command_nanos, Duration::from_millis(15).as_nanos());
        assert_eq!(sample.command_inside_surface_nanos, Duration::from_millis(7).as_nanos());
        assert_eq!(sample.command_outside_surface_nanos, Duration::from_millis(8).as_nanos());
        assert_eq!(sample.surface_nanos, Duration::from_millis(70).as_nanos());
        assert_eq!(sample.residual_nanos, Duration::from_millis(25).as_nanos());
        assert_eq!(sample.other_surface_hits, 1);
        assert_eq!(sample.other_surface_builds, 0);
        let build = sample.build_surface.expect("painting Body owns its actual build");
        assert_eq!(build.surface_nanos, Duration::from_millis(50).as_nanos());
        assert_eq!(build.layout_nanos, Duration::from_millis(40).as_nanos());
        assert_eq!(build.page_paint_nanos, Duration::from_millis(10).as_nanos());
        assert_eq!(diagnostics.edit_paint_summary().expect("existing edit metric").maximum_nanos, 45_000_000);
    }

    #[test]
    fn aligned_other_buckets_share_one_sample_and_exclude_owned_commands_and_surface() {
        let mut diagnostics = ResponsivenessDiagnostics::default();
        assert_eq!(
            diagnostics.aligned_edit_update_other_detail_summary(),
            "Edit update Other detail ms avg/p95/max | Input/dispatch collecting | Derived collecting | Chrome collecting | Diagnostics nested collecting | Host collecting | Remainder collecting"
        );
        let started = Instant::now();
        diagnostics.begin_update();
        diagnostics.note_accepted_edit(8, started);

        diagnostics.begin_other_bucket(RootOtherBucket::InputDispatch);
        diagnostics.begin_editor_command();
        diagnostics.begin_editor_command();
        diagnostics.end_editor_command(Duration::from_millis(1));
        diagnostics.end_editor_command(Duration::from_millis(5));
        diagnostics.end_other_bucket(RootOtherBucket::InputDispatch, Duration::from_millis(20));

        diagnostics.begin_other_bucket(RootOtherBucket::Derived);
        diagnostics.record_editor_command(Duration::from_millis(6));
        diagnostics.end_other_bucket(RootOtherBucket::Derived, Duration::from_millis(22));

        diagnostics.begin_other_bucket(RootOtherBucket::Chrome);
        diagnostics.record_editor_command(Duration::from_millis(7));
        diagnostics.record_diagnostics_calculation(Duration::from_millis(4));
        diagnostics.begin_document_surface(EditorSurface::Body);
        diagnostics.record_editor_command(Duration::from_millis(3));
        diagnostics.note_revision_painted(8, started + Duration::from_millis(40));
        diagnostics.record_document_surface(Duration::from_millis(10), 0, 0);
        diagnostics.end_other_bucket(RootOtherBucket::Chrome, Duration::from_millis(40));

        diagnostics.begin_other_bucket(RootOtherBucket::Host);
        diagnostics.record_editor_command(Duration::from_millis(8));
        diagnostics.end_other_bucket(RootOtherBucket::Host, Duration::from_millis(12));
        diagnostics.begin_other_bucket(RootOtherBucket::Host);
        diagnostics.end_other_bucket(RootOtherBucket::Host, Duration::from_millis(18));
        diagnostics.record_update(started, started + Duration::from_millis(150));

        assert_eq!(diagnostics.aligned_edit_updates.len, 1);
        let sample = diagnostics
            .aligned_edit_updates
            .values
            .iter()
            .flatten()
            .next()
            .expect("aligned Other sample");
        assert_eq!(sample.command_nanos, Duration::from_millis(30).as_nanos());
        assert_eq!(sample.command_outside_surface_nanos, Duration::from_millis(27).as_nanos());
        assert_eq!(sample.command_inside_surface_nanos, Duration::from_millis(3).as_nanos());
        assert_eq!(sample.surface_nanos, Duration::from_millis(10).as_nanos());
        assert_eq!(sample.residual_nanos, Duration::from_millis(114).as_nanos());
        assert_eq!(sample.input_dispatch_nanos, Duration::from_millis(15).as_nanos());
        assert_eq!(sample.derived_nanos, Duration::from_millis(16).as_nanos());
        assert_eq!(sample.chrome_nanos, Duration::from_millis(23).as_nanos());
        assert_eq!(sample.diagnostics_calculation_nanos, Duration::from_millis(4).as_nanos());
        assert_eq!(sample.host_nanos, Duration::from_millis(22).as_nanos());
        assert_eq!(sample.other_remainder_nanos, Duration::from_millis(38).as_nanos());
        let detail = diagnostics.aligned_edit_update_other_detail_summary();
        assert!(detail.contains("Input/dispatch 15.00/15.00/15.00"));
        assert!(detail.contains("Derived 16.00/16.00/16.00"));
        assert!(detail.contains("Chrome 23.00/23.00/23.00"));
        assert!(detail.contains("Diagnostics nested 4.00/4.00/4.00"));
        assert!(detail.contains("Host 22.00/22.00/22.00"));
        assert!(detail.contains("Remainder 38.00/38.00/38.00"));
        assert!(!diagnostics.aligned_edit_update_summaries()[0].contains("Input/dispatch"));
    }

    #[test]
    fn aligned_other_bucket_accumulates_after_early_return_and_remainder_saturates() {
        let mut diagnostics = ResponsivenessDiagnostics::default();
        let started = Instant::now();
        diagnostics.begin_update();
        diagnostics.note_accepted_edit(9, started);
        diagnostics.begin_other_bucket(RootOtherBucket::InputDispatch);
        diagnostics.record_editor_command(Duration::from_millis(5));
        diagnostics.end_other_bucket(RootOtherBucket::InputDispatch, Duration::from_millis(2));

        diagnostics.begin_other_bucket(RootOtherBucket::Derived);
        let mut early_return = |should_return: bool| -> Result<(), ()> {
            diagnostics.record_diagnostics_calculation(Duration::from_millis(50));
            if should_return {
                return Err(());
            }
            Ok(())
        };
        assert_eq!(early_return(true), Err(()));
        diagnostics.end_other_bucket(RootOtherBucket::Derived, Duration::from_millis(60));
        diagnostics.begin_document_surface(EditorSurface::Body);
        diagnostics.note_revision_painted(9, started + Duration::from_millis(1));
        diagnostics.record_document_surface(Duration::from_millis(1), 0, 0);
        diagnostics.record_update(started, started + Duration::from_millis(10));

        let sample = diagnostics
            .aligned_edit_updates
            .values
            .iter()
            .flatten()
            .next()
            .expect("saturating aligned Other sample");
        assert_eq!(sample.input_dispatch_nanos, 0);
        assert_eq!(sample.derived_nanos, Duration::from_millis(60).as_nanos());
        assert_eq!(sample.diagnostics_calculation_nanos, 0);
        assert_eq!(sample.residual_nanos, Duration::from_millis(4).as_nanos());
        assert_eq!(sample.other_remainder_nanos, 0);
    }

    #[test]
    fn aligned_edit_update_excludes_idle_scroll_and_unrelated_command_only_updates() {
        let mut diagnostics = ResponsivenessDiagnostics::default();
        let started = Instant::now();
        diagnostics.begin_update();
        diagnostics.begin_other_bucket(RootOtherBucket::Host);
        diagnostics.end_other_bucket(RootOtherBucket::Host, Duration::from_millis(8));
        diagnostics.record_update(started, started + Duration::from_millis(10));

        diagnostics.begin_update();
        diagnostics.record_editor_command(Duration::from_millis(3));
        diagnostics.record_update(started, started + Duration::from_millis(20));

        diagnostics.begin_update();
        diagnostics.begin_other_bucket(RootOtherBucket::Chrome);
        diagnostics.record_diagnostics_calculation(Duration::from_millis(2));
        diagnostics.begin_document_surface(EditorSurface::Body);
        diagnostics.record_layout(Duration::from_millis(1), true);
        diagnostics.note_revision_painted(9, started + Duration::from_millis(2));
        diagnostics.record_document_surface(Duration::from_millis(5), 0, 1_000_000);
        diagnostics.end_other_bucket(RootOtherBucket::Chrome, Duration::from_millis(20));
        diagnostics.record_update(started, started + Duration::from_millis(30));
        assert_eq!(diagnostics.aligned_edit_updates.len, 0);
        assert!(diagnostics.aligned_edit_update_summaries()[0].contains("Update collecting"));
        assert!(diagnostics
            .aligned_edit_update_other_detail_summary()
            .contains("Input/dispatch collecting"));
    }

    #[test]
    fn aligned_burst_sample_describes_only_the_final_painting_root_update() {
        let mut diagnostics = ResponsivenessDiagnostics::default();
        let accepted = Instant::now();
        diagnostics.begin_update();
        diagnostics.note_accepted_edit(4, accepted);
        diagnostics.note_accepted_edit(5, accepted + Duration::from_millis(1));
        diagnostics.note_revision_painted(4, accepted + Duration::from_millis(5));
        diagnostics.record_update(accepted, accepted + Duration::from_millis(80));
        assert_eq!(diagnostics.aligned_edit_updates.len, 0);

        diagnostics.begin_update();
        diagnostics.begin_document_surface(EditorSurface::Body);
        diagnostics.record_layout(Duration::from_millis(12), false);
        diagnostics.note_revision_painted(5, accepted + Duration::from_millis(20));
        diagnostics.record_document_surface(Duration::from_millis(20), 0, 3_000_000);
        diagnostics.record_update(accepted + Duration::from_millis(80), accepted + Duration::from_millis(110));

        assert_eq!(diagnostics.edit_paint_durations.len, 1);
        assert_eq!(diagnostics.edit_paint_summary().expect("burst edit metric").maximum_nanos, 20_000_000);
        let sample = diagnostics.aligned_edit_updates.values.iter().flatten().next().expect("final update sample");
        assert_eq!(sample.update_nanos, Duration::from_millis(30).as_nanos());
        assert_eq!(sample.surface_nanos, Duration::from_millis(20).as_nanos());
        assert_eq!(sample.command_nanos, 0);
    }

    #[test]
    fn layout_stage_populations_labels_and_remainder_are_explicit() {
        let mut diagnostics = ResponsivenessDiagnostics::default();
        assert_eq!(
            diagnostics.layout_build_detail_summary(),
            "Layout build detail ms avg/p95/max | Prep/lookup collecting h/m 0/0 | Miss prep collecting | Placement collecting | Rebase nested collecting | Indexes collecting | Sort nested collecting | Other collecting"
        );
        assert!(diagnostics.aligned_edit_update_summaries()[1].contains("Build Surface collecting"));
        diagnostics.record_layout(Duration::from_millis(2), true);
        assert_eq!(diagnostics.layout_build_stages.len, 0);

        diagnostics.record_layout_build(&LayoutBuildDiagnostics {
            whole_build_nanos: 100_000_000,
            prepared_lookup_nanos: 10_000_000,
            prepared_hits: 0,
            prepared_misses: 2,
            prepared_miss_nanos: 20_000_000,
            placement_nanos: 50_000_000,
            rebase_nanos: 15_000_000,
            interaction_index_nanos: 15_000_000,
            body_y_sort_nanos: 5_000_000,
            ..LayoutBuildDiagnostics::default()
        });
        diagnostics.record_layout_build(&LayoutBuildDiagnostics {
            whole_build_nanos: 80_000_000,
            prepared_lookup_nanos: 6_000_000,
            prepared_hits: 2,
            prepared_misses: 0,
            prepared_miss_nanos: 0,
            placement_nanos: 50_000_000,
            rebase_nanos: 20_000_000,
            interaction_index_nanos: 10_000_000,
            body_y_sort_nanos: 4_000_000,
            ..LayoutBuildDiagnostics::default()
        });

        assert_eq!(diagnostics.layout_build_stages.len, 2);
        let detail = diagnostics.layout_build_detail_summary();
        assert!(detail.contains("Prep/lookup 8.00/10.00/10.00 h/m 2/2"));
        assert!(detail.contains("Miss prep 20.00/20.00/20.00"));
        assert!(detail.contains("Placement 50.00/50.00/50.00"));
        assert!(detail.contains("Rebase nested 17.50/20.00/20.00"));
        assert!(detail.contains("Indexes 12.50/15.00/15.00"));
        assert!(detail.contains("Sort nested 4.50/5.00/5.00"));
        assert!(detail.contains("Other 9.50/14.00/14.00"));
        assert!(!diagnostics.phase_summary().contains("Layout build detail"));
    }

    #[test]
    fn aligned_samples_and_layout_stage_samples_have_bounded_associated_histories() {
        let mut diagnostics = ResponsivenessDiagnostics::default();
        let started = Instant::now();
        for index in 1..=HISTORY_CAPACITY as u64 + 10 {
            let update_started = started + Duration::from_secs(index);
            paint_aligned_edit(&mut diagnostics, update_started, index, index);
            diagnostics.record_layout_build(&LayoutBuildDiagnostics::default());
        }
        assert_eq!(diagnostics.aligned_edit_updates.len, HISTORY_CAPACITY);
        assert_eq!(diagnostics.layout_build_stages.len, HISTORY_CAPACITY);
        let other_detail = diagnostics.aligned_edit_update_other_detail_summary();
        assert!(other_detail.contains("Input/dispatch 1.00/1.00/1.00"));
        assert!(other_detail.contains("Derived 2.00/2.00/2.00"));
        assert!(other_detail.contains("Chrome 4.00/4.00/4.00"));
        assert!(other_detail.contains("Diagnostics nested 3.00/3.00/3.00"));
        assert!(other_detail.contains("Host 4.00/4.00/4.00"));
        assert_eq!(
            diagnostics.aligned_edit_updates.summary(|sample| Some(sample.update_nanos)),
            Some(DurationSummary {
                average_nanos: 70_500_000,
                p95_nanos: 124_000_000,
                maximum_nanos: 130_000_000,
            })
        );
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
