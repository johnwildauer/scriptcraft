//! Semantic title-page paragraphs and the reference-backed New-document template.

use crate::{
    FixedPoint, ModelError, Role, RunAttribute, ScreenplayElement, SourceProvenance, StableId,
    StableIdAllocator, TextRun, MAX_PROVENANCE_STRING_BYTES, MAX_RUNS_PER_ELEMENT,
    MAX_RUN_ATTRIBUTES, MAX_RUN_ATTRIBUTE_BYTES, MAX_TEXT_BYTES,
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
    pub leading_regular: Option<bool>,
    pub space_before: Option<FixedPoint>,
    pub spacing_single: Option<bool>,
    pub starts_new_page: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TitlePageParagraph {
    pub id: StableId,
    /// Recognized source Type for legacy typed title items; reference title
    /// paragraphs are Type-less and therefore have no screenplay role.
    pub role: Option<Role>,
    /// Canonical UTF-8 paragraph text, projected from the ordered runs.
    pub text: String,
    pub runs: Vec<TextRun>,
    pub source_provenance: Option<SourceProvenance>,
    pub formatting: TitlePageParagraphFormatting,
}

impl TitlePageParagraph {
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
            role: None,
            text,
            runs: Vec::new(),
            source_provenance: None,
            formatting,
        })
    }

    pub fn with_runs(
        id: impl Into<String>,
        role: Option<Role>,
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
            role,
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

    pub(crate) fn screenplay_projection(&self) -> Result<ScreenplayElement, ModelError> {
        let element = ScreenplayElement {
            id: self.id.clone(),
            // This adapter is available only for legacy typed title paragraphs.
            // Type-less content must wait for the dedicated FDX writer shape.
            role: self.role.ok_or(ModelError::UnsupportedJsonState)?,
            text: self.text.clone(),
            runs: self.runs.clone(),
            source_provenance: self.source_provenance.clone(),
        };
        crate::validate_element(&element)?;
        Ok(element)
    }

    /// Convert only legacy-compatible title content at the old JSON boundary.
    pub(crate) fn legacy_projection(&self) -> Result<ScreenplayElement, ModelError> {
        if !self.runs.is_empty()
            || self.source_provenance.is_some()
            || self.formatting != TitlePageParagraphFormatting::default()
        {
            return Err(ModelError::UnsupportedJsonState);
        }
        let role = self.role.ok_or(ModelError::UnsupportedJsonState)?;
        ScreenplayElement::new(self.id.as_str(), role, self.text.clone())
    }

    pub(crate) fn from_legacy_element(
        element: ScreenplayElement,
    ) -> Result<Self, ModelError> {
        let paragraph = Self {
            id: element.id,
            role: Some(element.role),
            text: element.text,
            runs: element.runs,
            source_provenance: element.source_provenance,
            formatting: TitlePageParagraphFormatting::default(),
        };
        paragraph.validate()?;
        Ok(paragraph)
    }
}

pub(crate) fn reference_boilerplate(
    ids: &mut StableIdAllocator,
) -> Result<Vec<TitlePageParagraph>, ModelError> {
    let mut paragraphs = Vec::with_capacity(54);
    for index in 0..54 {
        let alignment = match index {
            0..=16 | 42..=43 => TitlePageAlignment::Left,
            17..=41 => TitlePageAlignment::Center,
            _ => TitlePageAlignment::Justified,
        };
        let text = match index {
            17 => Some(("Script Title", "Underline+AllCaps")),
            21 => Some(("Written by", "")),
            24 => Some(("Name of First Writer", "")),
            29 => Some(("Based on, If Any", "")),
            52 => Some(("Address", "")),
            53 => Some(("Phone Number", "")),
            _ => None,
        };
        let formatting = reference_formatting(alignment);
        let runs = text
            .map(|(value, style)| {
                vec![TextRun {
                    text: value.to_owned(),
                    attributes: vec![
                        RunAttribute { name: "AdornmentStyle".into(), value: "0".into() },
                        RunAttribute { name: "Font".into(), value: "Courier Final Draft".into() },
                        RunAttribute { name: "RevisionID".into(), value: "0".into() },
                        RunAttribute { name: "Size".into(), value: "12".into() },
                        RunAttribute { name: "Style".into(), value: style.into() },
                    ],
                }]
            })
            .unwrap_or_default();
        paragraphs.push(TitlePageParagraph::with_runs(
            ids.allocate("title")?.as_str().to_owned(),
            None,
            runs,
            None,
            formatting,
        )?);
    }
    Ok(paragraphs)
}

fn reference_formatting(alignment: TitlePageAlignment) -> TitlePageParagraphFormatting {
    TitlePageParagraphFormatting {
        alignment: Some(alignment),
        first_indent: Some(FixedPoint::from_points(0)),
        left_indent: Some(FixedPoint::from_points(72)),
        right_indent: Some(FixedPoint::from_raw(54_000)),
        leading_regular: Some(true),
        space_before: Some(FixedPoint::from_points(0)),
        spacing_single: Some(true),
        starts_new_page: Some(false),
    }
}
