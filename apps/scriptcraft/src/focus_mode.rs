use std::time::Duration;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(4);
const SETTLE_INTERVAL: Duration = Duration::from_millis(600);
const POLL_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FocusRequest {
    pub(super) generation: u64,
    pub(super) target: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PendingRequest {
    request: FocusRequest,
    started_at: Duration,
    target_reported_at: Option<Duration>,
}

/// Current backend observation and a truthful, accessible projection for the
/// status action. A reported fullscreen value is not proof that the native
/// animation completed; the UI deliberately calls it a report, not success.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FocusModeView {
    observed: Option<bool>,
    pending: Option<bool>,
    uncertain: bool,
    external_settling: bool,
}

impl FocusModeView {
    pub(super) fn enabled(self) -> bool {
        self.observed.is_some()
            && self.pending.is_none()
            && !self.uncertain
            && !self.external_settling
    }

    pub(super) fn accessible_name(self) -> String {
        match (self.pending, self.observed, self.uncertain) {
            (_, None, true) => {
                "Focus Mode unavailable: the previous fullscreen request is unresolved and fullscreen state is not reported".to_owned()
            }
            (Some(true), None, false) => {
                "Focus Mode unavailable: fullscreen state is not reported while entering fullscreen".to_owned()
            }
            (Some(false), None, false) => {
                "Focus Mode unavailable: fullscreen state is not reported while exiting fullscreen".to_owned()
            }
            (Some(true), _, _) => {
                "Focus Mode transition pending: entering fullscreen; native completion is not confirmed".to_owned()
            }
            (Some(false), _, _) => {
                "Focus Mode transition pending: exiting fullscreen; native completion is not confirmed".to_owned()
            }
            (_, None, false) => {
                "Focus Mode unavailable: fullscreen state is not reported by this window".to_owned()
            }
            (_, _, true) => {
                "Focus Mode unavailable: the previous fullscreen request is unresolved; waiting for a stable window report".to_owned()
            }
            (_, Some(true), false) if self.external_settling => {
                "Focus Mode temporarily unavailable: fullscreen state changed outside the app; window reports fullscreen while waiting for a stable report".to_owned()
            }
            (_, Some(false), false) if self.external_settling => {
                "Focus Mode temporarily unavailable: fullscreen state changed outside the app; window reports windowed while waiting for a stable report".to_owned()
            }
            (_, Some(true), false) => {
                "Exit Focus Mode; the window reports fullscreen".to_owned()
            }
            (_, Some(false), false) => {
                "Enter Focus Mode; the window reports windowed".to_owned()
            }
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct FocusModeController {
    observed: Option<bool>,
    observation_since: Option<Duration>,
    pending: Option<PendingRequest>,
    uncertain: bool,
    recovery_since: Option<Duration>,
    external_settling_since: Option<Duration>,
    generation: u64,
}

impl FocusModeController {
    pub(super) fn view(&self) -> FocusModeView {
        FocusModeView {
            observed: self.observed,
            pending: self.pending.map(|pending| pending.request.target),
            uncertain: self.uncertain,
            external_settling: self.external_settling_since.is_some(),
        }
    }

    /// Reconcile one per-window backend observation. The settle interval is
    /// solely a bounded duplicate-suppression heuristic: egui/winit exposes no
    /// native did-enter/did-exit completion callback through this app boundary.
    pub(super) fn observe(&mut self, observed: Option<bool>, now: Duration) {
        let previous = self.observed;
        let app_request_in_flight = self.pending.is_some();
        let was_uncertain = self.uncertain;
        if observed != self.observed {
            self.observed = observed;
            self.observation_since = observed.map(|_| now);
            if self.uncertain {
                self.recovery_since = observed.map(|_| now);
            }
            if !app_request_in_flight && !was_uncertain {
                if matches!((previous, observed), (Some(before), Some(after)) if before != after) {
                    self.external_settling_since = Some(now);
                } else if observed.is_none() {
                    self.external_settling_since = None;
                }
            }
        } else if observed.is_some() && self.observation_since.is_none() {
            self.observation_since = Some(now);
            if self.uncertain && self.recovery_since.is_none() {
                self.recovery_since = Some(now);
            }
        } else if observed.is_none() {
            self.observation_since = None;
            if self.uncertain {
                self.recovery_since = None;
            }
        }

        if let Some(mut pending) = self.pending {
            if now.saturating_sub(pending.started_at) >= REQUEST_TIMEOUT {
                self.pending = None;
                self.uncertain = true;
                self.recovery_since = observed.map(|_| now);
                self.external_settling_since = None;
                return;
            }

            if observed == Some(pending.request.target) {
                let reported_at = *pending.target_reported_at.get_or_insert(now);
                if now.saturating_sub(reported_at) >= SETTLE_INTERVAL {
                    // This only means the reported target stayed stable long
                    // enough to permit a new explicit action; it is not a
                    // native animation-completion acknowledgement.
                    self.pending = None;
                    self.uncertain = false;
                    self.recovery_since = None;
                } else {
                    self.pending = Some(pending);
                }
            } else if observed.is_none() {
                pending.target_reported_at = None;
                self.pending = Some(pending);
            } else if pending.target_reported_at.is_some() {
                // The backend reversed after reporting the requested target.
                // Do not replay the stale intent or infer which OS action won.
                self.pending = None;
                self.uncertain = true;
                self.recovery_since = Some(now);
                self.external_settling_since = None;
            }
        } else if self.uncertain {
            if observed.is_none() {
                self.recovery_since = None;
            } else if let Some(recovery_since) = self.recovery_since {
                if now.saturating_sub(recovery_since) >= SETTLE_INTERVAL {
                    self.uncertain = false;
                    self.recovery_since = None;
                }
            }
        } else if let Some(settling_since) = self.external_settling_since {
            if observed.is_none() {
                self.external_settling_since = None;
            } else if now.saturating_sub(settling_since) >= SETTLE_INTERVAL {
                // This guard suppresses reentry during OS-driven transitions;
                // stability is not a native completion acknowledgement.
                self.external_settling_since = None;
            }
        }
    }

    /// Commit a pending request before the caller emits the viewport command.
    /// Reentrant/repeated activation therefore cannot enqueue an inverse.
    pub(super) fn request(&mut self, now: Duration) -> Option<FocusRequest> {
        let observed = self.observed?;
        if self.pending.is_some() || self.uncertain || self.external_settling_since.is_some() {
            return None;
        }
        self.generation = self.generation.saturating_add(1);
        let request = FocusRequest {
            generation: self.generation,
            target: !observed,
        };
        self.pending = Some(PendingRequest {
            request,
            started_at: now,
            target_reported_at: None,
        });
        Some(request)
    }

    /// Schedule bounded checks only while a request/recovery gate is unresolved.
    pub(super) fn next_repaint_delay(&self, now: Duration) -> Option<Duration> {
        let remaining = if let Some(pending) = self.pending {
            REQUEST_TIMEOUT.saturating_sub(now.saturating_sub(pending.started_at))
        } else if self.uncertain {
            SETTLE_INTERVAL.saturating_sub(now.saturating_sub(self.recovery_since.unwrap_or(now)))
        } else {
            let settling_since = self.external_settling_since?;
            SETTLE_INTERVAL.saturating_sub(now.saturating_sub(settling_since))
        };
        Some(remaining.min(POLL_INTERVAL).max(Duration::from_millis(1)))
    }
}
