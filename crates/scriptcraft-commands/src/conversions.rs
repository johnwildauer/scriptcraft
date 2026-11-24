use std::{fmt, path::Path};

use scriptcraft_fdx::BuiltinTemplateId;
use scriptcraft_model::{Position as CorePosition, Project, Selection as CoreSelection};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use unicode_segmentation::UnicodeSegmentation;

use super::{BuiltinTemplateValue, EditorSurface, PointerPhase, BeatCardIdValue, BeatSelectionValue,
    BeatTextFieldValue, BeatTextPositionValue, BeatTextEditValue, BeatNavigationValue};

/// Checked conversion from command selection payload to the session's canonical Beat selection.
pub fn beat_selection_to_core(value: &BeatSelectionValue) -> Result<scriptcraft_session::editing::BeatSelection, PositionError> {
    use scriptcraft_session::editing::{BeatSelection, BeatTextField};
    Ok(match value {
        BeatSelectionValue::None => BeatSelection::None,
        BeatSelectionValue::Card { card } => BeatSelection::Card { card: card.as_core().clone() },
        BeatSelectionValue::Text { card, field, anchor, focus } => BeatSelection::Text {
            card: card.as_core().clone(),
            field: match field { BeatTextFieldValue::Title => BeatTextField::Title, BeatTextFieldValue::Body => BeatTextField::Body },
            anchor: anchor.to_core()?, focus: focus.to_core()?,
        },
    })
}

/// Checked conversion from the session's Beat selection to its wire-safe value.
pub fn beat_selection_from_core(value: &scriptcraft_session::editing::BeatSelection) -> Result<BeatSelectionValue, PositionError> {
    use scriptcraft_session::editing::{BeatSelection, BeatTextField};
    Ok(match value {
        BeatSelection::None => BeatSelectionValue::None,
        BeatSelection::Card { card } => BeatSelectionValue::Card { card: BeatCardIdValue::from_core(card.clone()) },
        BeatSelection::Text { card, field, anchor, focus } => BeatSelectionValue::Text {
            card: BeatCardIdValue::from_core(card.clone()),
            field: match field { BeatTextField::Title => BeatTextFieldValue::Title, BeatTextField::Body => BeatTextFieldValue::Body },
            anchor: BeatTextPositionValue::from_core(*anchor)?, focus: BeatTextPositionValue::from_core(*focus)?,
        },
    })
}

pub fn beat_text_edit_to_core(value: &BeatTextEditValue) -> scriptcraft_session::editing::BeatTextEditOperation {
    use scriptcraft_session::editing::BeatTextEditOperation;
    match value {
        BeatTextEditValue::Insert { text } => BeatTextEditOperation::Insert { text: text.clone() },
        BeatTextEditValue::Paste { text } => BeatTextEditOperation::Paste { text: text.clone() },
        BeatTextEditValue::DeleteBackward => BeatTextEditOperation::DeleteBackward,
        BeatTextEditValue::DeleteForward => BeatTextEditOperation::DeleteForward,
        BeatTextEditValue::Enter => BeatTextEditOperation::Enter,
    }
}

pub fn beat_navigation_to_core(value: BeatNavigationValue) -> (scriptcraft_session::editing::BeatHorizontalDirection, Option<scriptcraft_session::editing::BeatLineEdge>) {
    use scriptcraft_session::editing::{BeatHorizontalDirection, BeatLineEdge};
    match value {
        BeatNavigationValue::Horizontal { direction: super::HorizontalDirection::Left } => (BeatHorizontalDirection::Left, None),
        BeatNavigationValue::Horizontal { direction: super::HorizontalDirection::Right } => (BeatHorizontalDirection::Right, None),
        BeatNavigationValue::LineEdge { edge: super::LineEdge::Start } => (BeatHorizontalDirection::Left, Some(BeatLineEdge::Start)),
        BeatNavigationValue::LineEdge { edge: super::LineEdge::End } => (BeatHorizontalDirection::Right, Some(BeatLineEdge::End)),
    }
}

/// Converts the closed command-catalog value to its exact Phase 7 catalog ID.
pub const fn builtin_template_id(value: BuiltinTemplateValue) -> BuiltinTemplateId {
    match value {
        BuiltinTemplateValue::Screenplay => BuiltinTemplateId::Screenplay,
        BuiltinTemplateValue::Stageplay => BuiltinTemplateId::Stageplay,
        BuiltinTemplateValue::DramatistsGuildMusical => BuiltinTemplateId::DramatistsGuildMusical,
    }
}

/// Converts the Phase 7 catalog ID to its exact closed command value.
pub const fn builtin_template_value(id: BuiltinTemplateId) -> BuiltinTemplateValue {
    match id {
        BuiltinTemplateId::Screenplay => BuiltinTemplateValue::Screenplay,
        BuiltinTemplateId::Stageplay => BuiltinTemplateValue::Stageplay,
        BuiltinTemplateId::DramatistsGuildMusical => BuiltinTemplateValue::DramatistsGuildMusical,
    }
}

/// UTF-8 byte position in the semantic screenplay model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Position {
    pub element: u32,
    pub offset: u32,
}

impl Position {
    pub fn from_core(position: CorePosition) -> Result<Self, PositionError> {
        Ok(Self {
            element: u32::try_from(position.element).map_err(|_| PositionError::IndexOutOfRange)?,
            offset: u32::try_from(position.offset).map_err(|_| PositionError::IndexOutOfRange)?,
        })
    }

    pub fn to_core(self) -> Result<CorePosition, PositionError> {
        Ok(CorePosition {
            element: usize::try_from(self.element).map_err(|_| PositionError::IndexOutOfRange)?,
            offset: usize::try_from(self.offset).map_err(|_| PositionError::IndexOutOfRange)?,
        })
    }
}

/// Selection expressed as element indexes and UTF-8 byte offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub anchor: Position,
    pub focus: Position,
}

impl Selection {
    pub fn from_core(selection: CoreSelection) -> Result<Self, PositionError> {
        Ok(Self {
            anchor: Position::from_core(selection.anchor)?,
            focus: Position::from_core(selection.focus)?,
        })
    }

    pub fn to_core(self) -> Result<CoreSelection, PositionError> {
        Ok(CoreSelection {
            anchor: self.anchor.to_core()?,
            focus: self.focus.to_core()?,
        })
    }

    /// Validates element indexes and UTF-8 byte boundaries against the active
    /// semantic project before a host passes this selection to an executor.
    pub fn validate_for(self, project: &Project) -> Result<(), SelectionError> {
        self.validate_for_surface(project, EditorSurface::Body)
    }

    pub fn validate_for_surface(
        self,
        project: &Project,
        surface: EditorSurface,
    ) -> Result<(), SelectionError> {
        validate_position(self.anchor, project, surface)?;
        validate_position(self.focus, project, surface)
    }
}

fn validate_position(
    position: Position,
    project: &Project,
    surface: EditorSurface,
) -> Result<(), SelectionError> {
    let index = usize::try_from(position.element).map_err(|_| SelectionError::ElementOutOfRange)?;
    let text = match surface {
        EditorSurface::Body => project
            .elements
            .get(index)
            .map(|element| element.text.as_str()),
        EditorSurface::TitlePage => project
            .title_page
            .get(index)
            .map(|paragraph| paragraph.text.as_str()),
    }
    .ok_or(SelectionError::ElementOutOfRange)?;
    let offset = usize::try_from(position.offset).map_err(|_| SelectionError::OffsetOutOfRange)?;
    if offset > text.len() {
        return Err(SelectionError::OffsetOutOfRange);
    }
    if !text.is_char_boundary(offset) {
        return Err(SelectionError::InvalidUtf8Boundary);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionError {
    ElementOutOfRange,
    OffsetOutOfRange,
    InvalidUtf8Boundary,
    CrossElementRange,
}

impl fmt::Display for SelectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ElementOutOfRange => formatter.write_str("selection element is out of range"),
            Self::OffsetOutOfRange => formatter.write_str("selection byte offset is out of range"),
            Self::InvalidUtf8Boundary => {
                formatter.write_str("selection is not on a UTF-8 boundary")
            }
            Self::CrossElementRange => {
                formatter.write_str("element-type change must select one paragraph")
            }
        }
    }
}

impl std::error::Error for SelectionError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PositionError {
    IndexOutOfRange,
}

impl fmt::Display for PositionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IndexOutOfRange => formatter.write_str("selection index is out of range"),
        }
    }
}

impl std::error::Error for PositionError {}

/// A portable, nonempty UTF-8 path payload; native handles never cross the wire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentPath(String);

impl DocumentPath {
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidDocumentPath> {
        let value = value.into();
        if value.trim().is_empty() || value.contains('\0') {
            return Err(InvalidDocumentPath);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn as_path(&self) -> &Path {
        Path::new(&self.0)
    }
}

impl Serialize for DocumentPath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for DocumentPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(String::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidDocumentPath;

impl fmt::Display for InvalidDocumentPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("document path must be nonempty UTF-8 without NUL")
    }
}

impl std::error::Error for InvalidDocumentPath {}

/// Maximum UTF-8 byte length for a single diagnostic status line.
pub const MAX_DIAGNOSTIC_MESSAGE_BYTES: usize = 512;

/// A bounded, single-line diagnostic message suitable for the UI status channel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticMessage(String);

impl DiagnosticMessage {
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidDiagnosticMessage> {
        let value = value.into();
        if value.trim().is_empty()
            || value.len() > MAX_DIAGNOSTIC_MESSAGE_BYTES
            || value.chars().any(|character| {
                character.is_control() || matches!(character, '\u{2028}' | '\u{2029}')
            })
        {
            return Err(InvalidDiagnosticMessage);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Serialize for DiagnosticMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for DiagnosticMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(String::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidDiagnosticMessage;

impl fmt::Display for InvalidDiagnosticMessage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "diagnostic message must be nonempty, single-line, and no longer than {MAX_DIAGNOSTIC_MESSAGE_BYTES} UTF-8 bytes"
        )
    }
}

impl std::error::Error for InvalidDiagnosticMessage {}

/// Whole-number zoom percentage constrained to the current desktop range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct ZoomPercent(u16);

impl ZoomPercent {
    pub const MIN: u16 = 5;
    pub const MAX: u16 = 300;

    pub fn new(value: u16) -> Result<Self, InvalidZoomPercent> {
        (Self::MIN..=Self::MAX)
            .contains(&value)
            .then_some(Self(value))
            .ok_or(InvalidZoomPercent)
    }

    pub const fn get(self) -> u16 {
        self.0
    }
}

impl<'de> Deserialize<'de> for ZoomPercent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(u16::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidZoomPercent;

impl fmt::Display for InvalidZoomPercent {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "zoom percentage must be in {}..={}",
            ZoomPercent::MIN,
            ZoomPercent::MAX
        )
    }
}

impl std::error::Error for InvalidZoomPercent {}

/// Positive finite multiplicative zoom input.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(transparent)]
pub struct ZoomFactor(f32);

impl ZoomFactor {
    pub fn new(value: f32) -> Result<Self, InvalidZoomFactor> {
        (value.is_finite() && value > 0.0)
            .then_some(Self(value))
            .ok_or(InvalidZoomFactor)
    }

    pub const fn get(self) -> f32 {
        self.0
    }
}

impl<'de> Deserialize<'de> for ZoomFactor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(f32::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidZoomFactor;

impl fmt::Display for InvalidZoomFactor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("zoom factor must be finite and greater than zero")
    }
}

impl std::error::Error for InvalidZoomFactor {}

/// Resolve a pointer phase to a semantic selection for the active screenplay
/// surface. Multi-click selection uses canonical UTF-8 byte ranges and Unicode
/// word boundaries; it does not depend on rendered coordinates or font metrics.
pub fn pointer_selection(
    project: &Project,
    current: CoreSelection,
    position: CorePosition,
    surface: EditorSurface,
    phase: PointerPhase,
) -> Option<CoreSelection> {
    let text = match surface {
        EditorSurface::Body => project
            .elements
            .get(position.element)
            .map(|element| element.text.as_str()),
        EditorSurface::TitlePage => project
            .title_page
            .get(position.element)
            .map(|paragraph| paragraph.text.as_str()),
    }?;
    if position.offset > text.len() || !text.is_char_boundary(position.offset) {
        return None;
    }

    Some(match phase {
        PointerPhase::Press | PointerPhase::Release => CoreSelection::caret(position),
        PointerPhase::Drag => CoreSelection {
            anchor: current.anchor,
            focus: position,
        },
        PointerPhase::DoubleClick => match word_range_at(text, position.offset) {
            Some((start, end)) => CoreSelection {
                anchor: CorePosition {
                    element: position.element,
                    offset: start,
                },
                focus: CorePosition {
                    element: position.element,
                    offset: end,
                },
            },
            None => CoreSelection::caret(position),
        },
        PointerPhase::TripleClick => CoreSelection {
            anchor: CorePosition {
                element: position.element,
                offset: 0,
            },
            focus: CorePosition {
                element: position.element,
                offset: text.len(),
            },
        },
    })
}

fn word_range_at(text: &str, offset: usize) -> Option<(usize, usize)> {
    let probe = if offset == text.len() {
        text.char_indices().next_back().map(|(start, _)| start)?
    } else {
        offset
    };

    text.unicode_word_indices()
        .find(|(start, word)| probe >= *start && probe < start.saturating_add(word.len()))
        .map(|(start, word)| (start, start.saturating_add(word.len())))
}
