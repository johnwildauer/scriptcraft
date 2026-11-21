//! Complete owned element-definition values.
//!
//! These canonical values own every field and are distinct from
//! [`SourceFontSpec`](crate::formatting::SourceFontSpec) and
//! [`SourceParagraphSpec`](crate::formatting::SourceParagraphSpec), which are
//! partial optional projections of recognized source settings. Behavior
//! targets intentionally remain representable even when a later document
//! registry does not contain them; resolution policy belongs to later phases.

use crate::{
    element_type::{ElementTypeKey, MAX_ELEMENT_TYPE_KEY_BYTES},
    formatting::{FixedPoint, ParagraphAlignment},
    ModelError,
};

/// Maximum UTF-8 byte length for owned definition strings other than a key.
pub const MAX_ELEMENT_DEFINITION_STRING_BYTES: usize = MAX_ELEMENT_TYPE_KEY_BYTES;

/// Complete, owned font settings for one element definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementFontSpec {
    pub family: String,
    pub size: u8,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub all_caps: bool,
    pub hidden_text: bool,
    pub color: Option<String>,
    pub adornment_style: u8,
    pub revision_id: u8,
}

impl ElementFontSpec {
    pub fn validate(&self) -> Result<(), ModelError> {
        if self.family.is_empty() {
            return Err(ModelError::EmptyElementFontFamily);
        }
        validate_definition_string(&self.family)?;
        if self.size == 0 {
            return Err(ModelError::ZeroElementFontSize);
        }
        validate_optional_string(self.color.as_deref())
    }
}

/// Complete, owned paragraph settings for one element definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementParagraphSpec {
    pub alignment: ParagraphAlignment,
    pub first_indent: FixedPoint,
    pub left_indent: FixedPoint,
    pub right_indent: FixedPoint,
    pub leading: String,
    pub space_before: FixedPoint,
    pub spacing: u8,
    pub starts_new_page: bool,
}

impl ElementParagraphSpec {
    pub fn validate(&self) -> Result<(), ModelError> {
        validate_definition_string(&self.leading)
    }
}

/// Owned document Behavior values. Empty shortcut strings are valid values;
/// missing and unresolved Return/Tab targets are likewise representable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementBehavior {
    pub paginate_as: Option<String>,
    pub return_key: Option<ElementTypeKey>,
    pub shortcut: Option<String>,
    pub tab_key: Option<ElementTypeKey>,
    pub win_shortcut: Option<String>,
}

impl ElementBehavior {
    pub fn validate(&self) -> Result<(), ModelError> {
        validate_optional_string(self.paginate_as.as_deref())?;
        validate_optional_string(self.shortcut.as_deref())?;
        validate_optional_string(self.win_shortcut.as_deref())
    }
}

/// Owned outline settings for one element definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElementOutline {
    pub can_hide: bool,
    pub level: u8,
}

/// Structural semantics granted by an exact built-in element label.
///
/// These flags are derived from the definition key only. In particular,
/// arbitrary labels and `PaginateAs` values do not imply screenplay structure.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SemanticCapabilities {
    pub scene_boundary: bool,
    pub character_cue: bool,
    pub dialogue: bool,
    pub parenthetical: bool,
    pub transition: bool,
    pub cast_list: bool,
}

impl SemanticCapabilities {
    /// Derives structural capabilities from the exact, case-sensitive key.
    pub fn for_key(key: &ElementTypeKey) -> Self {
        match key.as_str() {
            "Scene Heading" => Self {
                scene_boundary: true,
                ..Self::default()
            },
            "Character" => Self {
                character_cue: true,
                ..Self::default()
            },
            "Dialogue" => Self {
                dialogue: true,
                ..Self::default()
            },
            "Parenthetical" => Self {
                parenthetical: true,
                ..Self::default()
            },
            "Transition" => Self {
                transition: true,
                ..Self::default()
            },
            "Cast List" => Self {
                cast_list: true,
                ..Self::default()
            },
            _ => Self::default(),
        }
    }
}

/// A document-owned type key and its complete font, paragraph, Behavior, and
/// Outline values. Equality therefore compares every owned field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementDefinition {
    pub key: ElementTypeKey,
    pub font: ElementFontSpec,
    pub paragraph: ElementParagraphSpec,
    pub behavior: ElementBehavior,
    pub outline: ElementOutline,
}

impl ElementDefinition {
    pub fn new(
        key: ElementTypeKey,
        font: ElementFontSpec,
        paragraph: ElementParagraphSpec,
        behavior: ElementBehavior,
        outline: ElementOutline,
    ) -> Result<Self, ModelError> {
        let definition = Self {
            key,
            font,
            paragraph,
            behavior,
            outline,
        };
        definition.validate()?;
        Ok(definition)
    }

    pub fn validate(&self) -> Result<(), ModelError> {
        self.font.validate()?;
        self.paragraph.validate()?;
        self.behavior.validate()
    }

    /// Returns capabilities derived solely from this definition's exact key.
    pub fn capabilities(&self) -> SemanticCapabilities {
        SemanticCapabilities::for_key(&self.key)
    }
}

fn validate_optional_string(value: Option<&str>) -> Result<(), ModelError> {
    if let Some(value) = value {
        validate_definition_string(value)?;
    }
    Ok(())
}

fn validate_definition_string(value: &str) -> Result<(), ModelError> {
    if value.len() > MAX_ELEMENT_DEFINITION_STRING_BYTES {
        return Err(ModelError::ElementDefinitionValueTooLong);
    }
    Ok(())
}

