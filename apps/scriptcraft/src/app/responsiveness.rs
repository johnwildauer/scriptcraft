//! Development-only, value-free application responsiveness observations.
//!
//! Update FPS is not display-present FPS. Edit-to-app-paint stops after the
//! application paints the resulting revision and excludes compositor/display
//! latency. Histories and pending state have fixed bounds.

use scriptcraft_commands::{CommandId, EditorSurface};
use scriptcraft_layout::LayoutBuildDiagnostics;
use scriptcraft_session::editing::EditMutationDiagnostics;
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
            totals.structural_retries = totals
                .structural_retries
                .saturating_add(counts.structural_retries);
            totals.scoped_attempts = totals
                .scoped_attempts
                .saturating_add(counts.scoped_attempts);
            totals.scoped_accepts = totals.scoped_accepts.saturating_add(counts.scoped_accepts);
            totals.full_fallbacks = totals.full_fallbacks.saturating_add(counts.full_fallbacks);
            totals.full_validations = totals
                .full_validations
                .saturating_add(counts.full_validations);
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
        self.core_durations
            .push_nanos(u128::from(observation.total_nanos));
        self.candidate_setup_durations
            .push_nanos(u128::from(observation.candidate_setup_nanos));
        self.edit_operation_durations
            .push_nanos(u128::from(observation.edit_operation_nanos));
        self.preservation_durations
            .push_nanos(u128::from(observation.preservation_nanos));
        push_optional_nanos(
            &mut self.structural_retry_durations,
            observation.structural_retry_nanos,
        );
        push_optional_nanos(
            &mut self.scoped_validation_durations,
            observation.scoped_validation_nanos,
        );
        push_optional_nanos(
            &mut self.full_validation_durations,
            observation.full_validation_nanos,
        );
        push_optional_nanos(
            &mut self.no_op_equality_durations,
            observation.no_op_equality_nanos,
        );
        push_optional_nanos(
            &mut self.derived_classification_durations,
            observation.derived_classification_nanos,
        );
        push_optional_nanos(
            &mut self.dirty_comparison_durations,
            observation.dirty_comparison_nanos,
        );
        push_optional_nanos(
            &mut self.history_publication_durations,
            observation.history_publication_nanos,
        );
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
pub(crate) enum RootOtherBucket {
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

#[derive(Debug, Default, Clone, Copy)]
struct AltOutcomeCounts {
    availability_rejected: u64,
    changed: u64,
    no_op: u64,
    unavailable: u64,
    rejected: u64,
    failed: u64,
    preservation_failures: u64,
    formatted_model_failures: u64,
    other_failures: u64,
}

#[derive(Debug, Default, Clone, Copy)]
struct AltPointerCounts {
    ordinary: u64,
    double: u64,
    triple: u64,
    submitted: u64,
}

#[derive(Default)]
struct FontPreparationDiagnostics {
    signature_durations: DurationHistory,
    miss_preparation_durations: DurationHistory,
    hits: u64,
    misses: u64,
    cache_clears: u64,
}

impl FontPreparationDiagnostics {
    fn record(
        &mut self,
        signature_duration: Duration,
        cache_hit: bool,
        miss_preparation_duration: Option<Duration>,
        cache_clear_count: u64,
    ) {
        self.signature_durations.push(signature_duration);
        if cache_hit {
            self.hits = self.hits.saturating_add(1);
        } else {
            self.misses = self.misses.saturating_add(1);
            if let Some(duration) = miss_preparation_duration {
                self.miss_preparation_durations.push(duration);
            }
        }
        self.cache_clears = self
            .cache_clears
            .saturating_add(cache_clear_count);
    }

    fn summary(&self) -> String {
        let signature = format_optional_duration_summary(&self.signature_durations);
        let preparation = format_optional_duration_summary(&self.miss_preparation_durations);
        format!(
            "Font prep ms avg/p95/max | Signature {signature} | Miss-only {preparation} | Hits/misses/clears {}/{}/{}",
            self.hits, self.misses, self.cache_clears
        )
    }
}

pub(crate) struct ResponsivenessDiagnostics {
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
    alt_outcomes: AltOutcomeCounts,
    alt_pointer: AltPointerCounts,
    font_preparation: FontPreparationDiagnostics,
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
            alt_outcomes: AltOutcomeCounts::default(),
            alt_pointer: AltPointerCounts::default(),
            font_preparation: FontPreparationDiagnostics::default(),
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
    let surface_nanos = update
        .surfaces
        .iter()
        .flatten()
        .fold(0_u128, |total, surface| {
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
    pub(crate) fn begin_update(&mut self) {
        self.current_update = Some(PendingAlignedUpdate::default());
        self.active_surface = None;
        self.active_other_bucket = None;
        self.editor_command_depth = 0;
    }

    pub(crate) fn begin_other_bucket(&mut self, bucket: RootOtherBucket) {
        self.active_other_bucket = Some(bucket);
    }

    pub(crate) fn end_other_bucket(&mut self, bucket: RootOtherBucket, duration: Duration) {
        if self.active_other_bucket == Some(bucket) {
            if let Some(update) = self.current_update.as_mut() {
                let index = bucket.index();
                update.other_buckets.elapsed_nanos[index] =
                    update.other_buckets.elapsed_nanos[index].saturating_add(duration.as_nanos());
            }
        }
        self.active_other_bucket = None;
    }

    pub(crate) fn record_diagnostics_calculation(&mut self, duration: Duration) {
        if self.active_other_bucket != Some(RootOtherBucket::Chrome) {
            return;
        }
        if let Some(update) = self.current_update.as_mut() {
            update.diagnostics_calculation_nanos = update
                .diagnostics_calculation_nanos
                .saturating_add(duration.as_nanos());
        }
    }

    pub(crate) fn begin_editor_command(&mut self) {
        self.editor_command_depth = self.editor_command_depth.saturating_add(1);
    }

    pub(crate) fn end_editor_command(&mut self, duration: Duration) {
        let outermost = self.editor_command_depth == 1;
        self.editor_command_depth = self.editor_command_depth.saturating_sub(1);
        self.record_editor_command_inner(duration, outermost);
    }

    pub(crate) fn record_editor_command(&mut self, duration: Duration) {
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
                update.command_inside_surface_nanos =
                    update.command_inside_surface_nanos.saturating_add(nanos);
            } else {
                update.command_outside_surface_nanos =
                    update.command_outside_surface_nanos.saturating_add(nanos);
                // Nested internal commands remain in the raw aggregate but are
                // not subtracted again from the root Update or its active bucket.
                if outermost {
                    update.command_outside_nonoverlap_nanos = update
                        .command_outside_nonoverlap_nanos
                        .saturating_add(nanos);
                    if let Some(bucket) = active_other_bucket {
                        let index = bucket.index();
                        update.other_buckets.command_nanos[index] =
                            update.other_buckets.command_nanos[index].saturating_add(nanos);
                    }
                }
            }
        }
    }

    pub(crate) fn begin_document_surface(&mut self, surface: EditorSurface) {
        self.active_surface = Some(surface);
        let index = aligned_surface_index(surface);
        if let Some(update) = self.current_update.as_mut() {
            update.surfaces[index].get_or_insert_with(AlignedSurfaceObservation::default);
        }
    }

    pub(crate) fn record_layout(&mut self, duration: Duration, cache_hit: bool) {
        if cache_hit {
            self.layout_hit_durations.push(duration);
        } else {
            self.layout_build_durations.push(duration);
        }
        let nanos = duration.as_nanos();
        if let (Some(update), Some(surface)) = (self.current_update.as_mut(), self.active_surface) {
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

    pub(crate) fn record_layout_build(&mut self, observation: &LayoutBuildDiagnostics) {
        self.layout_build_stages
            .push(LayoutBuildStageSample::from(observation));
    }

    pub(crate) fn record_command_detail(
        &mut self,
        executor_duration: Duration,
        observation: EditMutationDiagnostics,
    ) {
        self.command_detail.record(executor_duration, observation);
    }

    pub(crate) fn record_alt_availability_rejection(&mut self) {
        self.alt_outcomes.availability_rejected =
            self.alt_outcomes.availability_rejected.saturating_add(1);
    }

    pub(crate) fn record_alt_unavailable(&mut self) {
        self.alt_outcomes.unavailable = self.alt_outcomes.unavailable.saturating_add(1);
    }

    pub(crate) fn record_alt_outcome(
        &mut self,
        outcome: &scriptcraft_commands::CommandOutcome,
    ) {
        use scriptcraft_commands::CommandOutcome;
        match outcome {
            CommandOutcome::Changed => {
                self.alt_outcomes.changed = self.alt_outcomes.changed.saturating_add(1)
            }
            CommandOutcome::NoOp => {
                self.alt_outcomes.no_op = self.alt_outcomes.no_op.saturating_add(1)
            }
            CommandOutcome::Unavailable(_) => self.record_alt_unavailable(),
            CommandOutcome::Rejected(_) => {
                self.alt_outcomes.rejected = self.alt_outcomes.rejected.saturating_add(1)
            }
            CommandOutcome::Failed(error) => {
                self.alt_outcomes.failed = self.alt_outcomes.failed.saturating_add(1);
                use scriptcraft_session::editing::EditError;
                if matches!(
                    error,
                    EditError::Preservation(_)
                        | EditError::Model(
                            scriptcraft_model::error::ModelError::InvalidPreservationState(_)
                        )
                        | EditError::FormattedText(
                            scriptcraft_model::formatted_text::FormattedTextError::Preservation(_)
                        )
                        | EditError::FormattedText(
                            scriptcraft_model::formatted_text::FormattedTextError::Model(
                                scriptcraft_model::error::ModelError::InvalidPreservationState(_)
                            )
                        )
                ) {
                    self.alt_outcomes.preservation_failures =
                        self.alt_outcomes.preservation_failures.saturating_add(1);
                } else if matches!(error, EditError::FormattedText(_) | EditError::Model(_)) {
                    self.alt_outcomes.formatted_model_failures =
                        self.alt_outcomes.formatted_model_failures.saturating_add(1);
                } else {
                    self.alt_outcomes.other_failures =
                        self.alt_outcomes.other_failures.saturating_add(1);
                }
            }
            CommandOutcome::ApplyTemplateFailed(_) => {}
        }
    }

    pub(crate) fn record_alt_pointer_classification(&mut self, classification: AltPointerClass) {
        let count = match classification {
            AltPointerClass::Ordinary => &mut self.alt_pointer.ordinary,
            AltPointerClass::Double => &mut self.alt_pointer.double,
            AltPointerClass::Triple => &mut self.alt_pointer.triple,
        };
        *count = count.saturating_add(1);
    }

    pub(crate) fn record_alt_pointer_submitted(&mut self) {
        self.alt_pointer.submitted = self.alt_pointer.submitted.saturating_add(1);
    }

    pub(crate) fn record_font_preparation(
        &mut self,
        signature_duration: Duration,
        cache_hit: bool,
        miss_preparation_duration: Option<Duration>,
        cache_clear_count: u64,
    ) {
        self.font_preparation.record(
            signature_duration,
            cache_hit,
            miss_preparation_duration,
            cache_clear_count,
        );
    }

    pub(crate) fn alt_outcome_summary(&self) -> String {
        let counts = self.alt_outcomes;
        format!(
            "Alt outcomes | Changed/no-op/unavailable/rejected/failed {}/{}/{}/{}/{} | Availability rejected {} | Failure preserve/formatted-model/other {}/{}/{}",
            counts.changed,
            counts.no_op,
            counts.unavailable,
            counts.rejected,
            counts.failed,
            counts.availability_rejected,
            counts.preservation_failures,
            counts.formatted_model_failures,
            counts.other_failures,
        )
    }

    pub(crate) fn alt_pointer_summary(&self) -> String {
        let counts = self.alt_pointer;
        format!(
            "Alt pointer | Ordinary/double/triple/submitted {}/{}/{}/{}",
            counts.ordinary, counts.double, counts.triple, counts.submitted
        )
    }

    pub(crate) fn font_preparation_summary(&self) -> String {
        self.font_preparation.summary()
    }

    pub(crate) fn record_document_surface(
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
            update.other_buckets.surface_nanos[index] =
                update.other_buckets.surface_nanos[index].saturating_add(duration.as_nanos());
        }
        if let (Some(update), Some(surface)) = (self.current_update.as_mut(), self.active_surface) {
            let observation = update.surfaces[aligned_surface_index(surface)]
                .get_or_insert_with(AlignedSurfaceObservation::default);
            observation.duration_nanos = observation
                .duration_nanos
                .saturating_add(duration.as_nanos());
            observation.page_paint_nanos = observation.page_paint_nanos.saturating_add(paint_nanos);
        }
        self.active_surface = None;
    }

    pub(crate) fn record_update(&mut self, started: Instant, ended: Instant) {
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

    pub(crate) fn note_accepted_edit(&mut self, revision: u64, accepted_at: Instant) {
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

    pub(crate) fn note_revision_painted(&mut self, revision: u64, painted_at: Instant) {
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

    pub(crate) fn clear_pending_edit(&mut self) {
        self.pending_edit = None;
    }

    pub(crate) fn note_pages(&mut self, visible: usize, total: usize) {
        self.pages = Some(PageGauges { visible, total });
    }

    pub(crate) fn aligned_edit_update_summaries(&self) -> [String; 2] {
        let samples = &self.aligned_edit_updates;
        let update = format_optional_summary(samples.summary(|sample| Some(sample.update_nanos)));
        let command = format_optional_summary(samples.summary(|sample| Some(sample.command_nanos)));
        let surface = format_optional_summary(samples.summary(|sample| Some(sample.surface_nanos)));
        let residual =
            format_optional_summary(samples.summary(|sample| Some(sample.residual_nanos)));
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

    pub(crate) fn aligned_edit_update_other_detail_summary(&self) -> String {
        let samples = &self.aligned_edit_updates;
        let input =
            format_optional_summary(samples.summary(|sample| Some(sample.input_dispatch_nanos)));
        let derived = format_optional_summary(samples.summary(|sample| Some(sample.derived_nanos)));
        let chrome = format_optional_summary(samples.summary(|sample| Some(sample.chrome_nanos)));
        let diagnostics = format_optional_summary(
            samples.summary(|sample| Some(sample.diagnostics_calculation_nanos)),
        );
        let host = format_optional_summary(samples.summary(|sample| Some(sample.host_nanos)));
        let remainder =
            format_optional_summary(samples.summary(|sample| Some(sample.other_remainder_nanos)));
        format!(
            "Edit update Other detail ms avg/p95/max | Input/dispatch {input} | Derived {derived} | Chrome {chrome} | Diagnostics nested {diagnostics} | Host {host} | Remainder {remainder}"
        )
    }

    pub(crate) fn layout_build_detail_summary(&self) -> String {
        let samples = &self.layout_build_stages;
        let lookup =
            format_optional_summary(samples.summary(|sample| Some(sample.prepared_lookup_nanos)));
        let miss = format_optional_summary(samples.summary(|sample| sample.prepared_miss_nanos));
        let placement =
            format_optional_summary(samples.summary(|sample| Some(sample.placement_nanos)));
        let rebase = format_optional_summary(samples.summary(|sample| Some(sample.rebase_nanos)));
        let indexes =
            format_optional_summary(samples.summary(|sample| Some(sample.interaction_index_nanos)));
        let sort =
            format_optional_summary(samples.summary(|sample| Some(sample.body_y_sort_nanos)));
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

    pub(crate) fn phase_summary(&self) -> String {
        let command = format_optional_duration_summary(&self.editor_command_durations);
        let layout_hit = format_optional_duration_summary(&self.layout_hit_durations);
        let layout_build = format_optional_duration_summary(&self.layout_build_durations);
        let surface = format_optional_duration_summary(&self.document_surface_durations);
        let page_paint = format_optional_duration_summary(&self.page_paint_durations);
        format!(
            "Phases ms avg/p95/max | Command {command} | Layout hit {layout_hit} | Layout build {layout_build} | Surface {surface} | Page paint {page_paint}"
        )
    }

    pub(crate) fn command_detail_summaries(&self) -> [String; 3] {
        self.command_detail.summaries()
    }

    pub(crate) fn safe_summary(&self) -> String {
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
    history
        .summary()
        .map_or_else(|| "collecting".to_owned(), format_duration_summary)
}

fn format_duration_summary(summary: DurationSummary) -> String {
    format!(
        "{:.2}/{:.2}/{:.2}",
        summary.average_nanos as f64 / 1_000_000.0,
        summary.p95_nanos as f64 / 1_000_000.0,
        summary.maximum_nanos as f64 / 1_000_000.0
    )
}

pub(crate) fn is_text_editing_command(command: CommandId) -> bool {
    matches!(
        command,
        CommandId::EditorTextInsert
            | CommandId::EditorTextPaste
            | CommandId::EditorDeleteBackward
            | CommandId::EditDelete
            | CommandId::EditorParagraphEnter
            | CommandId::EditorDialogueAltAdd
            | CommandId::EditorDialogueAltDelete
            | CommandId::EditorDialogueAltNavigate
    )
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum AltPointerClass {
    Ordinary,
    Double,
    Triple,
}
