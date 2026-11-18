use crate::{dual_dialogue::DualDialogueError, preservation::PreservationError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelError {
    EmptyId,
    InvalidId,
    DuplicateId,
    InvalidSchemaVersion,
    TooManyElements,
    TextTooLong,
    StringTooLong,
    TooManyRuns,
    TooManyRunAttributes,
    InvalidRunProjection,
    InvalidParagraphOverride,
    InvalidUtf8Boundary,
    InvalidNumber,
    InvalidJson,
    UnsupportedJsonState,
    MissingField(&'static str),
    WrongType(&'static str),
    InvalidPreservationState(PreservationError),
    InvalidDualDialogue(DualDialogueError),
}
