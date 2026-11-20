use super::{
    geometry::{LayoutError, LayoutErrorKind},
    FixedRect,
};
use scriptcraft_model::{
    formatting::{
        EffectiveTextStyle, FixedPoint, FontFallback, ParagraphAlignment, SourceLayoutSettings,
    },
    layout_inputs::PageLayoutInputs,
    typography::{DocumentFontEnvironment, EffectiveTextStyle as ResolvedTextStyle, FaceStyle},
    ElementDefinition, ElementTypeKey, LayoutInputs, ModelError, Project, StableId, TextRun,
    TitlePageParagraph,
};
use std::{
    hash::{Hash, Hasher},
    sync::Arc,
};

/// Exact pagination meaning derived from the current definition's PaginateAs
/// value. This policy never grants semantic capabilities to an element key.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum PaginationPolicy {
    #[default]
    Ordinary,
    KeepWithNext,
}

impl PaginationPolicy {
    fn from_paginate_as(value: Option<&str>) -> Self {
        match value {
            Some("Scene Heading" | "Character" | "Parenthetical") => Self::KeepWithNext,
            _ => Self::Ordinary,
        }
    }
}

/// Type-less title paragraphs use an explicit neutral Courier Prime baseline;
/// this is an environment choice and never a guessed element definition.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NeutralTitleStyle {
    pub size: u16,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub all_caps: bool,
    pub hidden_text: bool,
    pub source_color: Option<String>,
}

impl Default for NeutralTitleStyle {
    fn default() -> Self {
        Self {
            size: 12,
            bold: false,
            italic: false,
            underline: false,
            all_caps: false,
            hidden_text: false,
            source_color: None,
        }
    }
}

/// All transient inputs needed to resolve effective document layout values.
/// Host font discovery remains outside this crate; the supplied font
/// environment is the same exact identity/metrics snapshot used by desktop.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LayoutEnvironment {
    pub page: PageLayoutInputs,
    pub fonts: Arc<DocumentFontEnvironment>,
    pub neutral_title_style: NeutralTitleStyle,
}

impl LayoutEnvironment {
    pub fn new(page: PageLayoutInputs, fonts: impl Into<Arc<DocumentFontEnvironment>>) -> Self {
        Self {
            page,
            fonts: fonts.into(),
            neutral_title_style: NeutralTitleStyle::default(),
        }
    }

    pub fn with_neutral_title_style(mut self, style: NeutralTitleStyle) -> Self {
        self.neutral_title_style = style;
        self
    }

    /// Cheap bounded cache candidate. Cache entries still compare this entire
    /// environment for equality after fingerprint lookup.
    pub(crate) fn cache_fingerprint(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.page.hash(&mut hasher);
        self.fonts.content_fingerprint().hash(&mut hasher);
        self.neutral_title_style.hash(&mut hasher);
        hasher.finish()
    }

    pub fn neutral_title_text_style(&self) -> Result<ResolvedTextStyle, ModelError> {
        let id = self
            .fonts
            .neutral_request()
            .face_for_flags(
                self.neutral_title_style.bold,
                self.neutral_title_style.italic,
            )
            .clone();
        ResolvedTextStyle::new(
            scriptcraft_model::typography::NEUTRAL_COURIER_PRIME_FAMILY,
            self.neutral_title_style.source_color.clone(),
            id,
            self.neutral_title_style.size,
            self.neutral_title_style.bold,
            self.neutral_title_style.italic,
            self.neutral_title_style.underline,
            self.neutral_title_style.all_caps,
            self.neutral_title_style.hidden_text,
        )
    }
}

/// Selects which document surface is laid out. Print's compatibility builders
/// include the title page by default; callers can explicitly disable it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LayoutComposition {
    BodyEditor,
    TitlePageEditor,
    Print { include_title_page: bool },
}

/// Transient presentation choice for one Body-editor layout request. This is
/// deliberately separate from `Project` and is not accepted by Print builders.
#[derive(Debug, Clone, Default)]
pub struct BodyEditorPresentation {
    pub active_group_id: Option<StableId>,
    project_identity: Option<Arc<()>>,
}

impl BodyEditorPresentation {
    /// Binds a transient active-group choice to the current document instance.
    /// Reusing it after document replacement safely resolves to inactive mode.
    pub fn for_project(project: &Project, active_group_id: Option<StableId>) -> Self {
        Self {
            active_group_id,
            project_identity: Some(project.layout_identity().clone()),
        }
    }

    pub fn active_group_id(&self) -> Option<&StableId> {
        self.active_group_id.as_ref()
    }

    pub(super) fn is_for_project(&self, project: &Project) -> bool {
        self.project_identity
            .as_ref()
            .is_none_or(|identity| Arc::ptr_eq(identity, project.layout_identity()))
    }
}

impl PartialEq for BodyEditorPresentation {
    fn eq(&self, other: &Self) -> bool {
        self.active_group_id == other.active_group_id
            && match (&self.project_identity, &other.project_identity) {
                (Some(left), Some(right)) => Arc::ptr_eq(left, right),
                (None, None) => true,
                _ => false,
            }
    }
}

impl Eq for BodyEditorPresentation {}

impl Hash for BodyEditorPresentation {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.active_group_id.hash(state);
        self.project_identity
            .as_ref()
            .map(|identity| Arc::as_ptr(identity) as usize)
            .hash(state);
    }
}

pub(super) fn validate_profile(inputs: &LayoutInputs) -> Result<(), LayoutError> {
    if inputs.margin_left.saturating_add(inputs.margin_right) >= inputs.paper_width
        || inputs.margin_top.saturating_add(inputs.margin_bottom) >= inputs.paper_height
        || inputs.lines_per_page == 0
        || inputs.chars_per_line == 0
        || inputs.line_advance == 0
    {
        return Err(LayoutError {
            kind: LayoutErrorKind::InvalidProfile,
        });
    }
    Ok(())
}

pub(super) fn validate_page_profile(inputs: &LayoutInputs) -> Result<(), LayoutError> {
    if inputs.name.len() > 262_144
        || inputs.format_version == 0
        || inputs.paper_width == 0
        || inputs.paper_height == 0
        || inputs.margin_left.saturating_add(inputs.margin_right) >= inputs.paper_width
        || inputs.margin_top.saturating_add(inputs.margin_bottom) >= inputs.paper_height
        || inputs.boundary_gap > 1_000
    {
        return Err(LayoutError {
            kind: LayoutErrorKind::InvalidProfile,
        });
    }
    Ok(())
}

pub(super) fn effective_paragraph(
    inputs: &LayoutInputs,
    source: &SourceLayoutSettings,
    definition: &ElementDefinition,
    element_id: &StableId,
    text_style: &EffectiveTextStyle,
) -> EffectiveParagraph {
    let fallback_left = FixedPoint::from_points(i64::from(inputs.margin_left));
    let fallback_right = FixedPoint::from_points(i64::from(
        inputs.paper_width.saturating_sub(inputs.margin_right),
    ));
    let mut left = definition.paragraph.left_indent;
    let mut right = definition.paragraph.right_indent;
    let direct = source.direct_paragraph_spec(element_id);
    let page_right = FixedPoint::from_points(i64::from(inputs.paper_width));
    if left.raw() < 0 || right <= left || right > page_right {
        left = fallback_left;
        right = fallback_right;
    }
    let direct_left = direct.paragraph.left_indent.unwrap_or(left);
    let direct_right = direct.paragraph.right_indent.unwrap_or(right);
    if direct_left.raw() >= 0 && direct_right > direct_left && direct_right <= page_right {
        left = direct_left;
        right = direct_right;
    }
    let default_indent = definition.paragraph.first_indent;
    let char_width = i64::from(inputs.char_width.max(1)).saturating_mul(FixedPoint::SCALE);
    let usable = right.raw().saturating_sub(left.raw());
    let indent_fits = |indent: FixedPoint| {
        left.raw().saturating_add(indent.raw()) >= 0
            && usable.saturating_sub(indent.raw()) >= char_width
    };
    let first_indent = direct
        .paragraph
        .first_indent
        .filter(|indent| indent_fits(*indent))
        .unwrap_or_else(|| {
            if indent_fits(default_indent) {
                default_indent
            } else {
                FixedPoint::default()
            }
        });
    let alignment = direct
        .paragraph
        .alignment
        .unwrap_or(definition.paragraph.alignment);
    let space_before = direct
        .paragraph
        .space_before
        .unwrap_or(definition.paragraph.space_before);
    let scaled_metric = |base: u32| {
        let numerator = i64::from(base)
            .saturating_mul(i64::from(text_style.size))
            .saturating_mul(FixedPoint::SCALE);
        FixedPoint::from_raw(numerator.saturating_add(6).div_euclid(12).max(1))
    };
    EffectiveParagraph {
        left,
        right,
        first_indent,
        alignment,
        space_before,
        line_advance: scaled_metric(inputs.line_advance),
        char_width: scaled_metric(inputs.char_width),
        starts_new_page: direct
            .starts_new_page
            .unwrap_or(definition.paragraph.starts_new_page),
        pagination: PaginationPolicy::Ordinary,
    }
}

pub(super) fn compatibility_text_style(definition: &ElementDefinition) -> EffectiveTextStyle {
    EffectiveTextStyle {
        declared_family: definition.font.family.clone(),
        family: FontFallback::CourierPrime,
        size: definition.font.size,
        bold: definition.font.bold,
        italic: definition.font.italic,
        underline: definition.font.underline,
        all_caps: definition.font.all_caps,
        hidden_text: definition.font.hidden_text,
        source_color: definition.font.color.clone(),
    }
}

/// Editor-only page geometry for the active group's upper and lower boundary.
/// The zero-height rectangles are metadata for desktop painting, not lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorGroupBoundary {
    pub group_id: StableId,
    pub upper_page: usize,
    pub upper: FixedRect,
    pub lower_page: usize,
    pub lower: FixedRect,
}

impl LayoutComposition {
    pub(super) const fn purpose(self) -> LayoutPurpose {
        match self {
            Self::Print { .. } => LayoutPurpose::Print,
            Self::BodyEditor | Self::TitlePageEditor => LayoutPurpose::Editor,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LayoutPurpose {
    Editor,
    Print,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum EffectiveGeometryMode {
    Sequential,
    DualColumn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EffectiveParagraph {
    pub left: FixedPoint,
    pub right: FixedPoint,
    pub first_indent: FixedPoint,
    pub alignment: ParagraphAlignment,
    pub space_before: FixedPoint,
    pub line_advance: FixedPoint,
    pub char_width: FixedPoint,
    pub starts_new_page: bool,
    pub pagination: PaginationPolicy,
}

/// A fully resolved paragraph value. Definition values are the base; matching
/// stable-ID direct values override them; page bounds are the only neutral
/// paragraph fallback. No Role/static formatting row participates.
pub fn resolve_effective_paragraph(
    project: &Project,
    element_key: &ElementTypeKey,
    element_id: &StableId,
    runs: &[TextRun],
    environment: &LayoutEnvironment,
) -> Result<EffectiveParagraph, LayoutError> {
    let definition = project
        .element_definitions()
        .get(element_key)
        .ok_or(LayoutError {
            kind: LayoutErrorKind::NoGeometry,
        })?;
    let direct = project
        .direct_paragraph_overrides
        .get(element_id)
        .copied()
        .unwrap_or_default();
    resolve_paragraph_values(definition, direct, runs, environment)
}

/// Resolves the outer dual wrapper from the current exact `General` definition
/// and only the direct override keyed to that wrapper identity.
pub fn resolve_effective_dual_wrapper_paragraph(
    project: &Project,
    wrapper_id: &StableId,
    environment: &LayoutEnvironment,
) -> Result<EffectiveParagraph, LayoutError> {
    let general = ElementTypeKey::new("General")?;
    let definition = project
        .element_definitions()
        .get(&general)
        .ok_or(LayoutError {
            kind: LayoutErrorKind::NoGeometry,
        })?;
    let mut direct = project
        .dual_dialogue_paragraph_overrides
        .get(wrapper_id)
        .copied()
        .unwrap_or_default();
    // The FDX wrapper's RightIndent is an inset from the paper's right edge,
    // whereas the resolved dual constraint stores the right edge as a page-x
    // coordinate. Convert only this wrapper-specific value at the boundary.
    if let Some(right_inset) = direct.paragraph.right_indent {
        direct.paragraph.right_indent = Some(FixedPoint::from_raw(
            FixedPoint::from_points(i64::from(environment.page.paper_width))
                .raw()
                .saturating_sub(right_inset.raw()),
        ));
    }
    resolve_paragraph_values(definition, direct, &[], environment)
}

/// Resolves the base style from an exact current definition, then applies the
/// supplied source-run overlays through the same prepared face environment.
pub fn resolve_effective_text_style(
    project: &Project,
    element_key: &ElementTypeKey,
    environment: &LayoutEnvironment,
) -> Result<ResolvedTextStyle, LayoutError> {
    let definition = project
        .element_definitions()
        .get(element_key)
        .ok_or(LayoutError {
            kind: LayoutErrorKind::NoGeometry,
        })?;
    effective_definition_style(definition, &environment.fonts)
}

/// Applies one source run's direct Font/Size/Style/Color overlays to the exact
/// definition style. Every run resolves independently; formatting never leaks
/// from a preceding run.
pub fn resolve_effective_run_text_style(
    project: &Project,
    element_key: &ElementTypeKey,
    run: &TextRun,
    environment: &LayoutEnvironment,
) -> Result<ResolvedTextStyle, LayoutError> {
    let base = resolve_effective_text_style(project, element_key, environment)?;
    resolve_one_run_style(base, run, &environment.fonts)
}

/// Resolve a typed title from its exact current definition. Type-less titles
/// use the explicit neutral style and page-bounds paragraph baseline.
pub fn resolve_effective_title(
    project: &Project,
    title: &TitlePageParagraph,
    environment: &LayoutEnvironment,
) -> Result<(EffectiveParagraph, ResolvedTextStyle), LayoutError> {
    let (definition, base_style) = match title.element_type.as_ref() {
        Some(key) => {
            let definition = project.element_definitions().get(key).ok_or(LayoutError {
                kind: LayoutErrorKind::NoGeometry,
            })?;
            (
                Some(definition),
                effective_definition_style(definition, &environment.fonts)?,
            )
        }
        None => (None, environment.neutral_title_text_style()?),
    };
    let style = base_style;
    let paragraph = if let Some(definition) = definition {
        let overrides = title.formatting;
        let direct = scriptcraft_model::formatting::SourceParagraphOverrides {
            paragraph: scriptcraft_model::formatting::SourceParagraphSpec {
                alignment: overrides.alignment.map(|alignment| match alignment {
                    scriptcraft_model::title_page::TitlePageAlignment::Left => {
                        ParagraphAlignment::Left
                    }
                    scriptcraft_model::title_page::TitlePageAlignment::Right => {
                        ParagraphAlignment::Right
                    }
                    scriptcraft_model::title_page::TitlePageAlignment::Center => {
                        ParagraphAlignment::Center
                    }
                    scriptcraft_model::title_page::TitlePageAlignment::Justified => {
                        ParagraphAlignment::Justified
                    }
                }),
                first_indent: overrides.first_indent,
                left_indent: overrides.left_indent,
                right_indent: overrides.right_indent,
                leading_regular: overrides.leading_regular,
                space_before: overrides.space_before,
                spacing_single: overrides.spacing_single,
            },
            starts_new_page: overrides.starts_new_page,
        };
        resolve_paragraph_values(definition, direct, &title.runs, environment)?
    } else {
        resolve_page_baseline_paragraph(title.formatting, &title.runs, &style, environment)?
    };
    Ok((paragraph, style))
}

fn effective_definition_style(
    definition: &ElementDefinition,
    fonts: &DocumentFontEnvironment,
) -> Result<ResolvedTextStyle, LayoutError> {
    let font = &definition.font;
    let requested_style = FaceStyle::from_flags(font.bold, font.italic);
    let id = fonts
        .resolve_face_id(&font.family, requested_style, None)?
        .clone();
    Ok(ResolvedTextStyle::new(
        font.family.clone(),
        font.color.clone(),
        id,
        u16::from(font.size),
        font.bold,
        font.italic,
        font.underline,
        font.all_caps,
        font.hidden_text,
    )?)
}

fn resolve_one_run_style(
    mut base: ResolvedTextStyle,
    run: &TextRun,
    fonts: &DocumentFontEnvironment,
) -> Result<ResolvedTextStyle, LayoutError> {
    let element_style = FaceStyle::from_flags(base.bold, base.italic);
    let mut family = base.declared_family.clone();
    let mut size = base.size;
    let mut bold = base.bold;
    let mut italic = base.italic;
    let mut underline = base.underline;
    let mut all_caps = base.all_caps;
    let mut hidden = base.hidden_text;
    let mut color = base.source_color.clone();
    for attribute in &run.attributes {
        match attribute.name.as_str() {
            "Font" if !attribute.value.is_empty() => family = attribute.value.clone(),
            "Size" => {
                if let Ok(parsed) = attribute.value.parse::<u16>() {
                    if (1..=scriptcraft_model::typography::MAX_SUPPORTED_FONT_SIZE)
                        .contains(&parsed)
                    {
                        size = parsed;
                    }
                }
            }
            "Color" => color = Some(attribute.value.clone()),
            "Style" => {
                for token in attribute.value.split('+') {
                    match token {
                        "Bold" => bold = true,
                        "Italic" => italic = true,
                        "Underline" => underline = true,
                        "AllCaps" => all_caps = true,
                        "HiddenText" => hidden = true,
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    let requested_style = FaceStyle::from_flags(bold, italic);
    let preferred_style = (requested_style != element_style).then_some(element_style);
    let id = fonts
        .resolve_face_id(&family, requested_style, preferred_style)?
        .clone();
    base = ResolvedTextStyle::new(
        family, color, id, size, bold, italic, underline, all_caps, hidden,
    )?;
    Ok(base)
}

fn resolve_paragraph_values(
    definition: &ElementDefinition,
    direct: scriptcraft_model::formatting::SourceParagraphOverrides,
    runs: &[TextRun],
    environment: &LayoutEnvironment,
) -> Result<EffectiveParagraph, LayoutError> {
    let page_right = FixedPoint::from_points(i64::from(environment.page.paper_width));
    let page_left = FixedPoint::from_points(i64::from(environment.page.margin_left));
    let page_right_margin = FixedPoint::from_points(i64::from(
        environment
            .page
            .paper_width
            .saturating_sub(environment.page.margin_right),
    ));
    let paragraph = &definition.paragraph;
    let valid_pair = |left: FixedPoint, right: FixedPoint| {
        left.raw() >= 0 && right > left && right <= page_right
    };
    let (definition_left, definition_right) =
        if valid_pair(paragraph.left_indent, paragraph.right_indent) {
            (paragraph.left_indent, paragraph.right_indent)
        } else {
            (page_left, page_right_margin)
        };
    let left = direct.paragraph.left_indent.unwrap_or(definition_left);
    let right = direct.paragraph.right_indent.unwrap_or(definition_right);
    let (left, right) = if valid_pair(left, right) {
        (left, right)
    } else {
        (definition_left, definition_right)
    };
    let usable = right.raw().saturating_sub(left.raw());
    let indent_fits = |indent: FixedPoint| {
        left.raw().saturating_add(indent.raw()) >= 0 && usable.saturating_sub(indent.raw()) > 0
    };
    let first_indent = direct
        .paragraph
        .first_indent
        .filter(|indent| indent_fits(*indent))
        .or_else(|| indent_fits(paragraph.first_indent).then_some(paragraph.first_indent))
        .unwrap_or_default();
    let style = effective_definition_style(definition, &environment.fonts)?;
    let line_advance = paragraph_line_advance(definition, &style, runs, environment)?;
    let regular = match paragraph.leading.as_str() {
        "Regular" => true,
        "Single" => false,
        _ => {
            return Err(LayoutError {
                kind: LayoutErrorKind::Model(ModelError::InvalidNumber),
            })
        }
    };
    let line_advance = if direct.paragraph.leading_regular.unwrap_or(regular) {
        line_advance
    } else {
        FixedPoint::from_points(i64::from(maximum_effective_size(
            &style,
            runs,
            &environment.fonts,
        )?))
    };
    let spacing_single = direct
        .paragraph
        .spacing_single
        .unwrap_or(paragraph.spacing == 1);
    let line_advance = if spacing_single {
        line_advance
    } else {
        return Err(LayoutError {
            kind: LayoutErrorKind::Model(ModelError::InvalidNumber),
        });
    };
    let pagination = PaginationPolicy::from_paginate_as(definition.behavior.paginate_as.as_deref());
    Ok(EffectiveParagraph {
        left,
        right,
        first_indent,
        alignment: direct.paragraph.alignment.unwrap_or(paragraph.alignment),
        space_before: direct
            .paragraph
            .space_before
            .unwrap_or(paragraph.space_before),
        line_advance,
        // Fixed-cell wrap is replaced by WO-005.5; this field is an inert
        // compatibility projection and never selects a definition/default.
        char_width: FixedPoint::default(),
        starts_new_page: direct.starts_new_page.unwrap_or(paragraph.starts_new_page),
        pagination,
    })
}

fn resolve_page_baseline_paragraph(
    formatting: scriptcraft_model::title_page::TitlePageParagraphFormatting,
    runs: &[TextRun],
    style: &ResolvedTextStyle,
    environment: &LayoutEnvironment,
) -> Result<EffectiveParagraph, LayoutError> {
    let page_left = FixedPoint::from_points(i64::from(environment.page.margin_left));
    let page_right = FixedPoint::from_points(i64::from(
        environment
            .page
            .paper_width
            .saturating_sub(environment.page.margin_right),
    ));
    let page_edge = FixedPoint::from_points(i64::from(environment.page.paper_width));
    let valid_pair =
        |left: FixedPoint, right: FixedPoint| left.raw() >= 0 && right > left && right <= page_edge;
    let requested_left = formatting.left_indent.unwrap_or(page_left);
    let requested_right = formatting.right_indent.unwrap_or(page_right);
    let (left, right) = if valid_pair(requested_left, requested_right) {
        (requested_left, requested_right)
    } else {
        (page_left, page_right)
    };
    let usable = right.raw().saturating_sub(left.raw());
    let indent_fits = |indent: FixedPoint| {
        left.raw().saturating_add(indent.raw()) >= 0 && usable.saturating_sub(indent.raw()) > 0
    };
    let first_indent = formatting
        .first_indent
        .filter(|indent| indent_fits(*indent))
        .unwrap_or_default();
    let line_advance = paragraph_line_advance_for_style(style, runs, environment)?;
    let line_advance = if formatting.leading_regular == Some(false) {
        FixedPoint::from_points(i64::from(maximum_effective_size(
            style,
            runs,
            &environment.fonts,
        )?))
    } else {
        line_advance
    };
    if formatting.spacing_single == Some(false) {
        return Err(LayoutError {
            kind: LayoutErrorKind::Model(ModelError::InvalidNumber),
        });
    }
    Ok(EffectiveParagraph {
        left,
        right,
        first_indent,
        alignment: formatting
            .alignment
            .map(|alignment| match alignment {
                scriptcraft_model::title_page::TitlePageAlignment::Left => ParagraphAlignment::Left,
                scriptcraft_model::title_page::TitlePageAlignment::Right => {
                    ParagraphAlignment::Right
                }
                scriptcraft_model::title_page::TitlePageAlignment::Center => {
                    ParagraphAlignment::Center
                }
                scriptcraft_model::title_page::TitlePageAlignment::Justified => {
                    ParagraphAlignment::Justified
                }
            })
            .unwrap_or(ParagraphAlignment::Left),
        space_before: formatting.space_before.unwrap_or_default(),
        line_advance,
        char_width: FixedPoint::default(),
        starts_new_page: formatting.starts_new_page.unwrap_or(false),
        pagination: PaginationPolicy::Ordinary,
    })
}

fn paragraph_line_advance(
    definition: &ElementDefinition,
    _style: &ResolvedTextStyle,
    runs: &[TextRun],
    environment: &LayoutEnvironment,
) -> Result<FixedPoint, LayoutError> {
    let base = effective_definition_style(definition, &environment.fonts)?;
    maximum_line_advance(&base, runs, environment)
}

fn paragraph_line_advance_for_style(
    style: &ResolvedTextStyle,
    runs: &[TextRun],
    environment: &LayoutEnvironment,
) -> Result<FixedPoint, LayoutError> {
    maximum_line_advance(style, runs, environment)
}

fn maximum_line_advance(
    base: &ResolvedTextStyle,
    runs: &[TextRun],
    environment: &LayoutEnvironment,
) -> Result<FixedPoint, LayoutError> {
    let metric = |style: &ResolvedTextStyle| -> Result<FixedPoint, LayoutError> {
        let face = environment.fonts.require_face(&style.resolved_font_id)?;
        Ok(face.metrics.scale_regular_line_advance(style.size)?)
    };
    let mut maximum = metric(base)?;
    for run in runs {
        let style = resolve_one_run_style(base.clone(), run, &environment.fonts)?;
        maximum = maximum.max(metric(&style)?);
    }
    Ok(maximum)
}

fn maximum_effective_size(
    base: &ResolvedTextStyle,
    runs: &[TextRun],
    fonts: &DocumentFontEnvironment,
) -> Result<u16, LayoutError> {
    let mut maximum = base.size;
    for run in runs {
        maximum = maximum.max(resolve_one_run_style(base.clone(), run, fonts)?.size);
    }
    Ok(maximum)
}

#[cfg(test)]
mod dynamic_resolution_tests {
    use super::*;
    use scriptcraft_model::formatting::SourceParagraphOverrides;
    use scriptcraft_model::typography::{
        DocumentFontEnvironment, FontResolutionKind, RequestedFontResolution, ResolvedFaceMetrics,
        ResolvedFontFace, ResolvedFontId, ResolvedFontSourceKind,
    };
    use std::collections::BTreeMap;

    fn environment() -> Result<LayoutEnvironment, ModelError> {
        let mut faces = BTreeMap::new();
        let mut ids = Vec::new();
        for (index, style) in FaceStyle::ALL.into_iter().enumerate() {
            let id = ResolvedFontId::new(format!("test-face-{index}"))?;
            let face = ResolvedFontFace::new(
                id.clone(),
                "Synthetic",
                format!("Synthetic {style:?}"),
                style,
                ResolvedFontSourceKind::Synthetic,
                100,
                index as u64 + 1,
                ResolvedFaceMetrics::new(1_000, 800, -200, 0, BTreeMap::new())?,
            )?;
            ids.push(id.clone());
            faces.insert(id, face);
        }
        let resolution = RequestedFontResolution {
            kind: FontResolutionKind::CourierAlias,
            regular: ids[0].clone(),
            bold: ids[1].clone(),
            italic: ids[2].clone(),
            bold_italic: ids[3].clone(),
        };
        let fonts = DocumentFontEnvironment::new(
            BTreeMap::from([
                ("Courier Prime".to_owned(), resolution.clone()),
                ("Courier Final Draft".to_owned(), resolution.clone()),
                ("Arial".to_owned(), resolution),
            ]),
            faces,
        )?;
        Ok(LayoutEnvironment::new(PageLayoutInputs::default(), fonts))
    }

    fn environment_with_partial_futura() -> Result<LayoutEnvironment, ModelError> {
        let mut faces = BTreeMap::new();
        let mut ids = Vec::new();
        for style in FaceStyle::ALL {
            let id = ResolvedFontId::new(format!("futura-{style:?}"))?;
            ids.push(id.clone());
            if style == FaceStyle::BoldItalic {
                continue;
            }
            faces.insert(
                id.clone(),
                ResolvedFontFace::new(
                    id,
                    "Futura",
                    format!("Futura {style:?}"),
                    style,
                    ResolvedFontSourceKind::Host,
                    100,
                    style as u64 + 1,
                    ResolvedFaceMetrics::new(1_000, 800, -200, 0, BTreeMap::new())?,
                )?,
            );
        }
        let courier_ids = FaceStyle::ALL
            .into_iter()
            .enumerate()
            .map(|(index, style)| {
                let id = ResolvedFontId::new(format!("courier-{style:?}"))?;
                faces.insert(
                    id.clone(),
                    ResolvedFontFace::new(
                        id.clone(),
                        "Courier Prime",
                        format!("Courier Prime {style:?}"),
                        style,
                        ResolvedFontSourceKind::Bundled,
                        100,
                        index as u64 + 20,
                        ResolvedFaceMetrics::new(1_000, 800, -200, 0, BTreeMap::new())?,
                    )?,
                );
                Ok::<_, ModelError>(id)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let courier = RequestedFontResolution {
            kind: FontResolutionKind::CourierAlias,
            regular: courier_ids[0].clone(),
            bold: courier_ids[1].clone(),
            italic: courier_ids[2].clone(),
            bold_italic: courier_ids[3].clone(),
        };
        let futura = RequestedFontResolution {
            kind: FontResolutionKind::ExactHost,
            regular: ids[0].clone(),
            bold: ids[1].clone(),
            italic: ids[2].clone(),
            // The document resolver uses Bold Italic → Bold → Italic → Regular.
            bold_italic: ids[1].clone(),
        };
        let fonts = DocumentFontEnvironment::new(
            BTreeMap::from([
                ("Courier Prime".to_owned(), courier),
                ("Futura".to_owned(), futura),
            ]),
            faces,
        )?;
        Ok(LayoutEnvironment::new(PageLayoutInputs::default(), fonts))
    }

    #[test]
    fn resolves_exact_definition_and_matching_direct_overrides_only() -> Result<(), ModelError> {
        let mut project = scriptcraft_fdx::instantiate_builtin_template(
            scriptcraft_fdx::DEFAULT_BUILTIN_TEMPLATE_ID,
        )
        .map(|document| document.project)
        .unwrap_or_else(|_| unreachable!("packaged Screenplay fixture is valid"));
        let action = ElementTypeKey::new("Action")?;
        let id = StableId::new("action-owner")?;
        let mut overrides = SourceParagraphOverrides::default();
        overrides.paragraph.alignment = Some(ParagraphAlignment::Center);
        overrides.paragraph.first_indent = Some(FixedPoint::from_points(-3));
        overrides.paragraph.space_before = Some(FixedPoint::from_points(7));
        overrides.starts_new_page = Some(true);
        project
            .direct_paragraph_overrides
            .insert(id.clone(), overrides);
        let env = environment()?;
        let resolved = resolve_effective_paragraph(&project, &action, &id, &[], &env)
            .map_err(|_| ModelError::InvalidNumber)?;
        assert_eq!(resolved.alignment, ParagraphAlignment::Center);
        assert_eq!(resolved.first_indent, FixedPoint::from_points(-3));
        assert_eq!(resolved.space_before, FixedPoint::from_points(7));
        assert!(resolved.starts_new_page);

        let different_id = StableId::new("different-owner")?;
        let unmatched_override =
            resolve_effective_paragraph(&project, &action, &different_id, &[], &env)
                .map_err(|_| ModelError::InvalidNumber)?;
        assert_ne!(unmatched_override.alignment, ParagraphAlignment::Center);
        assert!(!unmatched_override.starts_new_page);

        let mut invalid_pair = SourceParagraphOverrides::default();
        invalid_pair.paragraph.left_indent = Some(FixedPoint::from_points(500));
        invalid_pair.paragraph.right_indent = Some(FixedPoint::from_points(400));
        invalid_pair.paragraph.first_indent = Some(FixedPoint::from_points(1_000));
        project
            .direct_paragraph_overrides
            .insert(different_id.clone(), invalid_pair);
        let invalid_pair_result =
            resolve_effective_paragraph(&project, &action, &different_id, &[], &env)
                .map_err(|_| ModelError::InvalidNumber)?;
        assert_eq!(
            invalid_pair_result.left,
            FixedPoint::parse_inches("1.50").ok_or(ModelError::InvalidNumber)?
        );
        assert_eq!(
            invalid_pair_result.right,
            FixedPoint::parse_inches("7.50").ok_or(ModelError::InvalidNumber)?
        );
        assert_eq!(invalid_pair_result.first_indent, FixedPoint::default());
        Ok(())
    }

    #[test]
    fn dual_wrapper_uses_current_general_definition_and_its_own_direct_values(
    ) -> Result<(), ModelError> {
        let mut project = scriptcraft_fdx::instantiate_builtin_template(
            scriptcraft_fdx::DEFAULT_BUILTIN_TEMPLATE_ID,
        )
        .map(|document| document.project)
        .unwrap_or_else(|_| unreachable!("packaged Screenplay fixture is valid"));
        let wrapper = StableId::new("dual-wrapper")?;
        let mut direct = SourceParagraphOverrides::default();
        direct.paragraph.left_indent = Some(FixedPoint::from_points(140));
        direct.paragraph.right_indent = Some(FixedPoint::from_points(112));
        direct.starts_new_page = Some(true);
        project
            .dual_dialogue_paragraph_overrides
            .insert(wrapper.clone(), direct);
        let resolved =
            resolve_effective_dual_wrapper_paragraph(&project, &wrapper, &environment()?)
                .map_err(|_| ModelError::InvalidNumber)?;
        assert_eq!(resolved.left, FixedPoint::from_points(140));
        assert_eq!(resolved.right, FixedPoint::from_points(500));
        assert!(resolved.starts_new_page);
        Ok(())
    }

    #[test]
    fn paginate_as_is_exact_pagination_policy_not_structural_identity() -> Result<(), ModelError> {
        for value in ["Scene Heading", "Character", "Parenthetical"] {
            assert_eq!(
                PaginationPolicy::from_paginate_as(Some(value)),
                PaginationPolicy::KeepWithNext
            );
        }
        for value in ["Action", "scene heading", "Shot", "End of Act", "Other"] {
            assert_eq!(
                PaginationPolicy::from_paginate_as(Some(value)),
                PaginationPolicy::Ordinary
            );
        }
        assert_eq!(
            PaginationPolicy::from_paginate_as(None),
            PaginationPolicy::Ordinary
        );
        Ok(())
    }

    #[test]
    fn arbitrary_case_distinct_definition_and_run_overlays_resolve_from_current_values(
    ) -> Result<(), ModelError> {
        let mut project = scriptcraft_fdx::instantiate_builtin_template(
            scriptcraft_fdx::DEFAULT_BUILTIN_TEMPLATE_ID,
        )
        .map(|document| document.project)
        .unwrap_or_else(|_| unreachable!("packaged Screenplay fixture is valid"));
        let mut definitions = project
            .element_definitions()
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let mut arbitrary = definitions
            .iter()
            .find(|definition| definition.key.as_str() == "Action")
            .cloned()
            .ok_or(ModelError::InvalidNumber)?;
        arbitrary.key = ElementTypeKey::new("action")?;
        arbitrary.font.family = "Courier Final Draft".to_owned();
        arbitrary.font.size = 16;
        arbitrary.font.color = Some("#112233445566".to_owned());
        arbitrary.behavior.paginate_as = Some("Scene Heading".to_owned());
        arbitrary.paragraph.alignment = ParagraphAlignment::Right;
        definitions.push(arbitrary);
        project.replace_element_definitions(definitions)?;

        let key = ElementTypeKey::new("action")?;
        let env = environment()?;
        let style = resolve_effective_text_style(&project, &key, &env)
            .map_err(|_| ModelError::InvalidNumber)?;
        assert_eq!(style.declared_family, "Courier Final Draft");
        assert_eq!(style.size, 16);
        assert_eq!(
            style.color,
            scriptcraft_model::typography::EffectiveTextColor::ExplicitRgb8 {
                red: 0x11,
                green: 0x33,
                blue: 0x55,
            }
        );
        assert!(
            !project
                .element_semantics(&key)
                .ok_or(ModelError::InvalidNumber)?
                .capabilities()
                .scene_boundary
        );
        let paragraph = resolve_effective_paragraph(
            &project,
            &key,
            &StableId::new("lowercase-action")?,
            &[],
            &env,
        )
        .map_err(|_| ModelError::InvalidNumber)?;
        assert_eq!(paragraph.alignment, ParagraphAlignment::Right);
        assert_eq!(paragraph.pagination, PaginationPolicy::KeepWithNext);

        let direct_run = TextRun {
            text: "sample".to_owned(),
            attributes: vec![
                scriptcraft_model::RunAttribute {
                    name: "Font".to_owned(),
                    value: "Arial".to_owned(),
                },
                scriptcraft_model::RunAttribute {
                    name: "Size".to_owned(),
                    value: "22".to_owned(),
                },
                scriptcraft_model::RunAttribute {
                    name: "Style".to_owned(),
                    value: "Bold+Italic+Underline".to_owned(),
                },
                scriptcraft_model::RunAttribute {
                    name: "Color".to_owned(),
                    value: "not-a-color".to_owned(),
                },
            ],
        };
        let run_style = resolve_effective_run_text_style(&project, &key, &direct_run, &env)
            .map_err(|_| ModelError::InvalidNumber)?;
        assert_eq!(run_style.declared_family, "Arial");
        assert_eq!(run_style.size, 22);
        assert!(run_style.bold && run_style.italic && run_style.underline);
        assert_eq!(
            run_style.color,
            scriptcraft_model::typography::EffectiveTextColor::UnsupportedExplicit
        );
        assert_eq!(run_style.source_color.as_deref(), Some("not-a-color"));
        Ok(())
    }

    #[test]
    fn unavailable_inline_style_prefers_the_element_face_before_family_cascade(
    ) -> Result<(), ModelError> {
        let mut project = scriptcraft_fdx::instantiate_builtin_template(
            scriptcraft_fdx::DEFAULT_BUILTIN_TEMPLATE_ID,
        )
        .map(|document| document.project)
        .unwrap_or_else(|_| unreachable!("packaged Screenplay fixture is valid"));
        let mut definitions = project
            .element_definitions()
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let action = definitions
            .iter_mut()
            .find(|definition| definition.key.as_str() == "Action")
            .ok_or(ModelError::InvalidNumber)?;
        action.font.family = "Futura".to_owned();
        action.font.bold = false;
        action.font.italic = true;
        project.replace_element_definitions(definitions)?;

        let environment = environment_with_partial_futura()?;
        let action_key = ElementTypeKey::new("Action")?;
        let element_style = resolve_effective_text_style(&project, &action_key, &environment)
            .map_err(|_| ModelError::InvalidNumber)?;
        assert_eq!(element_style.resolved_font_id.as_str(), "futura-Italic");

        let bold_italic_override = TextRun {
            text: "To tell your story".to_owned(),
            attributes: vec![scriptcraft_model::RunAttribute {
                name: "Style".to_owned(),
                value: "Bold+Italic".to_owned(),
            }],
        };
        let effective = resolve_effective_run_text_style(
            &project,
            &action_key,
            &bold_italic_override,
            &environment,
        )
        .map_err(|_| ModelError::InvalidNumber)?;
        assert!(effective.bold && effective.italic);
        assert_eq!(effective.resolved_font_id, element_style.resolved_font_id);

        let family_cascade =
            environment
                .fonts
                .resolve_face_id("Futura", FaceStyle::BoldItalic, None)?;
        assert_eq!(family_cascade.as_str(), "futura-Bold");
        Ok(())
    }

    #[test]
    fn typed_and_typeless_titles_use_definition_and_explicit_neutral_style(
    ) -> Result<(), ModelError> {
        let mut project = scriptcraft_fdx::instantiate_builtin_template(
            scriptcraft_fdx::DEFAULT_BUILTIN_TEMPLATE_ID,
        )
        .map(|document| document.project)
        .unwrap_or_else(|_| unreachable!("packaged Screenplay fixture is valid"));
        let mut definitions = project
            .element_definitions()
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let mut summary = definitions
            .iter()
            .find(|definition| definition.key.as_str() == "Action")
            .cloned()
            .unwrap_or_else(|| unreachable!("packaged Screenplay includes Action"));
        summary.key = ElementTypeKey::new("Hidden Summary Fixture")?;
        summary.font.family = "Arial".to_owned();
        summary.font.hidden_text = true;
        summary.paragraph.left_indent = FixedPoint::from_raw(6_336);
        summary.paragraph.right_indent = FixedPoint::from_points(500);
        definitions.push(summary);
        project.replace_element_definitions(definitions)?;
        let env = environment()?;
        let typed = TitlePageParagraph::new("typed", "TITLE", Default::default())?;
        let mut typed = typed;
        typed.element_type = Some(ElementTypeKey::new("Hidden Summary Fixture")?);
        typed.formatting.alignment =
            Some(scriptcraft_model::title_page::TitlePageAlignment::Center);
        typed.formatting.starts_new_page = Some(true);
        let (typed_paragraph, typed_style) = resolve_effective_title(&project, &typed, &env)
            .map_err(|_| ModelError::InvalidNumber)?;
        assert_eq!(typed_style.declared_family, "Arial");
        assert!(typed_style.hidden_text);
        assert_eq!(typed_paragraph.left.raw(), 6_336);
        assert_eq!(typed_paragraph.alignment, ParagraphAlignment::Center);
        assert!(typed_paragraph.starts_new_page);

        let typeless = TitlePageParagraph::new("plain", "TITLE", Default::default())?;
        let (plain_paragraph, plain_style) = resolve_effective_title(&project, &typeless, &env)
            .map_err(|_| ModelError::InvalidNumber)?;
        assert_eq!(plain_style.declared_family, "Courier Prime");
        assert!(!plain_style.hidden_text);
        assert_eq!(plain_paragraph.left, FixedPoint::from_points(108));
        Ok(())
    }
}
