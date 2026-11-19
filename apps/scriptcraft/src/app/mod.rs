//! Application-level composition.

pub(super) mod diagnostics;
pub(super) mod render;
pub(super) mod responsiveness;
pub(super) mod startup;
pub(super) mod state;
pub(super) mod surface;
pub(super) mod update;
pub(super) mod window;

pub(super) use state::{ScriptCraftApp, TitlePageViewState};

pub(super) use diagnostics::{
    add_performance_counter, add_performance_nanos, measure_root_other_bucket,
    NativeInputDiagnostics, PerformanceDiagnostics,
};
