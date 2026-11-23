//! Semantic title-page paragraphs and their canonical formatting values.

use crate::{
    element::{
        RunAttribute, SourceProvenance, TextRun, MAX_PROVENANCE_STRING_BYTES, MAX_RUNS_PER_ELEMENT,
        MAX_RUN_ATTRIBUTES, MAX_RUN_ATTRIBUTE_BYTES,
    },
    element_type::ElementTypeKey,
    formatting::{FixedPoint, ParagraphLeading, ParagraphSpacing},
    identity::StableId,
    ModelError, MAX_TEXT_BYTES,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TitlePageAlignment {
    Left,
    Right,
    Center,
    Justified,
}

/// Supported direct formatting carried by one title paragraph. Unset fields
/// remain source-owned/defaulted; a populated value is an explicit override.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TitlePageParagraphFormatting {
    pub alignment: Option<TitlePageAlignment>,
    pub first_indent: Option<FixedPoint>,
    pub left_indent: Option<FixedPoint>,
    pub right_indent: Option<FixedPoint>,
    pub leading: Option<ParagraphLeading>,
    pub space_before: Option<FixedPoint>,
    pub spacing: Option<ParagraphSpacing>,
    pub starts_new_page: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TitlePageParagraph {
    pub id: StableId,
    /// Canonical exact element type; `None` preserves type-less title paragraphs.
    pub element_type: Option<ElementTypeKey>,
    /// Canonical UTF-8 paragraph text, projected from the ordered runs.
    pub text: String,
    pub runs: Vec<TextRun>,
    pub source_provenance: Option<SourceProvenance>,
    pub formatting: TitlePageParagraphFormatting,
}

impl TitlePageParagraph {
    /// Returns the canonical exact element-type key, if this title paragraph is typed.
    pub fn element_type(&self) -> Option<ElementTypeKey> {
        self.element_type.clone()
    }

    /// Sets the optional canonical key without changing any other title value.
    pub fn set_element_type(&mut self, element_type: Option<ElementTypeKey>) {
        self.element_type = element_type;
    }

    pub fn new(
        id: impl Into<String>,
        text: impl Into<String>,
        formatting: TitlePageParagraphFormatting,
    ) -> Result<Self, ModelError> {
        let text = text.into();
        if text.len() > MAX_TEXT_BYTES {
            return Err(ModelError::TextTooLong);
        }
        Ok(Self {
            id: StableId::new(id)?,
            element_type: None,
            text,
            runs: Vec::new(),
            source_provenance: None,
            formatting,
        })
    }

    pub fn with_runs(
        id: impl Into<String>,
        element_type: Option<ElementTypeKey>,
        runs: Vec<TextRun>,
        source_provenance: Option<SourceProvenance>,
        formatting: TitlePageParagraphFormatting,
    ) -> Result<Self, ModelError> {
        if runs.len() > MAX_RUNS_PER_ELEMENT {
            return Err(ModelError::TooManyRuns);
        }
        let attribute_count = runs.iter().fold(0usize, |count, run| {
            count.saturating_add(run.attributes.len())
        });
        if attribute_count > MAX_RUN_ATTRIBUTES {
            return Err(ModelError::TooManyRunAttributes);
        }
        let mut text = String::new();
        for run in &runs {
            if text.len().saturating_add(run.text.len()) > MAX_TEXT_BYTES {
                return Err(ModelError::TextTooLong);
            }
            text.push_str(&run.text);
        }
        let paragraph = Self {
            id: StableId::new(id)?,
            element_type,
            text,
            runs,
            source_provenance,
            formatting,
        };
        paragraph.validate()?;
        Ok(paragraph)
    }

    pub fn is_boundary(&self, byte_offset: usize) -> bool {
        self.text.is_char_boundary(byte_offset)
    }

    pub fn validate_boundary(&self, byte_offset: usize) -> Result<(), ModelError> {
        if self.is_boundary(byte_offset) {
            Ok(())
        } else {
            Err(ModelError::InvalidUtf8Boundary)
        }
    }

    pub fn validate(&self) -> Result<(), ModelError> {
        StableId::new(self.id.as_str())?;
        if self.text.len() > MAX_TEXT_BYTES {
            return Err(ModelError::TextTooLong);
        }
        if self.runs.len() > MAX_RUNS_PER_ELEMENT {
            return Err(ModelError::TooManyRuns);
        }
        let mut text = String::new();
        let mut attributes = 0usize;
        for run in &self.runs {
            if text.len().saturating_add(run.text.len()) > MAX_TEXT_BYTES {
                return Err(ModelError::TextTooLong);
            }
            text.push_str(&run.text);
            attributes = attributes.saturating_add(run.attributes.len());
            if run.attributes.iter().any(|attribute| {
                attribute.name.is_empty()
                    || attribute.name.len() > MAX_RUN_ATTRIBUTE_BYTES
                    || attribute.value.len() > MAX_RUN_ATTRIBUTE_BYTES
            }) {
                return Err(ModelError::StringTooLong);
            }
        }
        if attributes > MAX_RUN_ATTRIBUTES {
            return Err(ModelError::TooManyRunAttributes);
        }
        if !self.runs.is_empty() && text != self.text {
            return Err(ModelError::InvalidRunProjection);
        }
        if self.source_provenance.as_ref().is_some_and(|source| {
            source
                .foreign_id
                .as_ref()
                .is_some_and(|value| value.len() > MAX_PROVENANCE_STRING_BYTES)
                || source
                    .foreign_role
                    .as_ref()
                    .is_some_and(|value| value.len() > MAX_PROVENANCE_STRING_BYTES)
        }) {
            return Err(ModelError::StringTooLong);
        }
        Ok(())
    }
}
