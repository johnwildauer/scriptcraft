use crate::{
    dual_dialogue::DualDialogueError, element_type::ElementTypeKey, preservation::PreservationError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelError {
    EmptyId,
    InvalidId,
    DuplicateId,
    TooManyElements,
    TextTooLong,
    StringTooLong,
    TooManyRuns,
    TooManyRunAttributes,
    InvalidRunProjection,
    InvalidParagraphOverride,
    InvalidUtf8Boundary,
    InvalidNumber,
    EmptyElementTypeKey,
    ElementTypeKeyTooLong,
    EmptyElementFontFamily,
    ZeroElementFontSize,
    ElementDefinitionValueTooLong,
    TooManyElementDefinitions,
    DuplicateElementDefinitionKey,
    MissingElementDefinition(ElementTypeKey),
    InvalidPreservationState(PreservationError),
    InvalidDualDialogue(DualDialogueError),
}
