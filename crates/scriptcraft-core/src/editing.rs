//! Temporary Phase 3 compatibility facade for the session editing API; remove in Phase 4.

pub use scriptcraft_session::editing::{
    enter_transition, tab_transition, transition, DerivedWorkSummary, EditError,
    EditMutationDiagnostics, EditPolicy, EditingSession, ParagraphKeyTarget,
    ParagraphKeyTargets, Position, Selection,
};
