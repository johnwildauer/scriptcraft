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

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_font() -> ElementFontSpec {
        ElementFontSpec {
            family: "Courier Final Draft".to_owned(),
            size: 12,
            bold: false,
            italic: false,
            underline: false,
            all_caps: false,
            hidden_text: false,
            color: None,
            adornment_style: 0,
            revision_id: 0,
        }
    }

    fn valid_paragraph() -> ElementParagraphSpec {
        ElementParagraphSpec {
            alignment: ParagraphAlignment::Left,
            first_indent: FixedPoint::from_raw(-10),
            left_indent: FixedPoint::from_points(1),
            right_indent: FixedPoint::from_points(7),
            leading: "Regular".to_owned(),
            space_before: FixedPoint::from_points(0),
            spacing: 1,
            starts_new_page: false,
        }
    }

    fn valid_behavior() -> ElementBehavior {
        ElementBehavior {
            paginate_as: None,
            return_key: None,
            shortcut: Some(String::new()),
            tab_key: None,
            win_shortcut: None,
        }
    }

    fn valid_definition(key: ElementTypeKey) -> ElementDefinition {
        ElementDefinition {
            key,
            font: valid_font(),
            paragraph: valid_paragraph(),
            behavior: valid_behavior(),
            outline: ElementOutline {
                can_hide: false,
                level: 1,
            },
        }
    }

    #[test]
    fn owned_values_clone_and_allow_unresolved_behavior_targets() {
        let Ok(key) = ElementTypeKey::new("Lyrics") else {
            assert!(false, "arbitrary key construction should succeed");
            return;
        };
        let Ok(target) = ElementTypeKey::new("Later Type") else {
            assert!(false, "arbitrary target construction should succeed");
            return;
        };
        let mut definition = valid_definition(key);
        definition.behavior.return_key = Some(target);
        let Ok(action) = ElementTypeKey::new("Action") else {
            assert!(false, "base key construction should succeed");
            return;
        };
        definition.behavior.tab_key = Some(action);

        assert!(definition.validate().is_ok());
        assert_eq!(definition.clone(), definition);
        assert_eq!(definition.behavior.shortcut.as_deref(), Some(""));
        assert_eq!(
            definition
                .behavior
                .return_key
                .as_ref()
                .map(ElementTypeKey::as_str),
            Some("Later Type")
        );
        assert_eq!(definition.key.as_str(), "Lyrics");
    }

    #[test]
    fn paragraph_keeps_negative_first_indent() {
        let paragraph = valid_paragraph();
        assert_eq!(paragraph.first_indent.raw(), -10);
        assert!(paragraph.validate().is_ok());
    }

    #[test]
    fn validates_required_and_bounded_values() {
        let mut font = valid_font();
        font.family.clear();
        assert_eq!(font.validate(), Err(ModelError::EmptyElementFontFamily));

        let mut font = valid_font();
        font.size = 0;
        assert_eq!(font.validate(), Err(ModelError::ZeroElementFontSize));

        let mut font = valid_font();
        font.color = Some("x".repeat(MAX_ELEMENT_DEFINITION_STRING_BYTES + 1));
        assert_eq!(
            font.validate(),
            Err(ModelError::ElementDefinitionValueTooLong)
        );

        let mut font = valid_font();
        font.family = "f".repeat(MAX_ELEMENT_DEFINITION_STRING_BYTES + 1);
        assert_eq!(
            font.validate(),
            Err(ModelError::ElementDefinitionValueTooLong)
        );

        let mut paragraph = valid_paragraph();
        paragraph.leading = "l".repeat(MAX_ELEMENT_DEFINITION_STRING_BYTES + 1);
        assert_eq!(
            paragraph.validate(),
            Err(ModelError::ElementDefinitionValueTooLong)
        );

        let mut behavior = valid_behavior();
        behavior.paginate_as = Some("p".repeat(MAX_ELEMENT_DEFINITION_STRING_BYTES + 1));
        assert_eq!(
            behavior.validate(),
            Err(ModelError::ElementDefinitionValueTooLong)
        );

        let mut behavior = valid_behavior();
        behavior.shortcut = Some("s".repeat(MAX_ELEMENT_DEFINITION_STRING_BYTES + 1));
        assert_eq!(
            behavior.validate(),
            Err(ModelError::ElementDefinitionValueTooLong)
        );

        let mut behavior = valid_behavior();
        behavior.win_shortcut = Some("w".repeat(MAX_ELEMENT_DEFINITION_STRING_BYTES + 1));
        assert_eq!(
            behavior.validate(),
            Err(ModelError::ElementDefinitionValueTooLong)
        );
    }

    #[test]
    fn definition_constructor_validates_nested_values() {
        let mut font = valid_font();
        font.size = 0;
        let Ok(action) = ElementTypeKey::new("Action") else {
            assert!(false, "base key construction should succeed");
            return;
        };
        assert_eq!(
            ElementDefinition::new(
                action,
                font,
                valid_paragraph(),
                valid_behavior(),
                ElementOutline {
                    can_hide: false,
                    level: 1
                },
            ),
            Err(ModelError::ZeroElementFontSize)
        );
    }

    #[test]
    fn capabilities_map_only_exact_base_labels_to_single_flags() {
        let cases = [
            (
                "Scene Heading",
                SemanticCapabilities {
                    scene_boundary: true,
                    ..SemanticCapabilities::default()
                },
            ),
            (
                "Character",
                SemanticCapabilities {
                    character_cue: true,
                    ..SemanticCapabilities::default()
                },
            ),
            (
                "Dialogue",
                SemanticCapabilities {
                    dialogue: true,
                    ..SemanticCapabilities::default()
                },
            ),
            (
                "Parenthetical",
                SemanticCapabilities {
                    parenthetical: true,
                    ..SemanticCapabilities::default()
                },
            ),
            (
                "Transition",
                SemanticCapabilities {
                    transition: true,
                    ..SemanticCapabilities::default()
                },
            ),
            (
                "Cast List",
                SemanticCapabilities {
                    cast_list: true,
                    ..SemanticCapabilities::default()
                },
            ),
        ];

        for (label, expected) in cases {
            let key = ElementTypeKey::new(label);
            assert_eq!(
                key.as_ref().map(SemanticCapabilities::for_key),
                Ok(expected)
            );
            let definition = key.map(valid_definition);
            assert_eq!(
                definition.map(|definition| definition.capabilities()),
                Ok(expected)
            );
        }
    }

    #[test]
    fn non_base_case_variants_and_pagination_labels_gain_no_capabilities() {
        let non_structural = [
            "General",
            "Action",
            "Note",
            "Shot",
            "New Act",
            "End of Act",
            "Sequence",
            "Outline 1",
            "Outline 2",
            "Outline 3",
            "Summary",
            "Lyrics",
            "Scene Citation",
        ];

        for label in non_structural {
            let key = ElementTypeKey::new(label);
            assert_eq!(
                key.map(|key| SemanticCapabilities::for_key(&key)),
                Ok(SemanticCapabilities::default())
            );
        }

        let case_variants = [
            "Scene heading",
            "CHARACTER",
            "dialogue",
            "parenthetical",
            "transition",
            "cast list",
        ];
        for label in case_variants {
            let key = ElementTypeKey::new(label);
            assert_eq!(
                key.map(|key| SemanticCapabilities::for_key(&key)),
                Ok(SemanticCapabilities::default())
            );
        }

        let shot = ElementTypeKey::new("Shot").map(valid_definition);
        let end_of_act = ElementTypeKey::new("End of Act").map(valid_definition);
        let shot = shot.map(|mut definition| {
            definition.behavior.paginate_as = Some("Scene Heading".to_owned());
            definition
        });
        let end_of_act = end_of_act.map(|mut definition| {
            definition.behavior.paginate_as = Some("Character".to_owned());
            definition
        });

        assert_eq!(
            shot.map(|definition| definition.capabilities()),
            Ok(SemanticCapabilities::default())
        );
        assert_eq!(
            end_of_act.map(|definition| definition.capabilities()),
            Ok(SemanticCapabilities::default())
        );
    }
}
