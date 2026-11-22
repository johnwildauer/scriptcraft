use super::{
    geometry::{LayoutError, LayoutErrorKind, LineVerticalMetrics},
    FixedRect,
};
use scriptcraft_model::{
    formatting::{FixedPoint, ParagraphAlignment},
    layout_inputs::{DocumentLeading, PageLayoutInputs},
    typography::{
        DocumentFontEnvironment, EffectiveTextStyle as ResolvedTextStyle, FaceStyle,
        LeadingResolverKind,
    },
    ElementDefinition, ElementTypeKey, ModelError, Project, StableId, TextRun, TitlePageParagraph,
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

/// Selects which document surface is laid out.
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

pub(super) fn validate_page_profile(inputs: &PageLayoutInputs) -> Result<(), LayoutError> {
    if inputs.document_leading == DocumentLeading::Unsupported {
        return Err(LayoutError {
            kind: LayoutErrorKind::UnsupportedDocumentLeading,
        });
    }
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
    pub space_before_units: u32,
    pub base_leading: FixedPoint,
    pub starts_new_page: bool,
    pub pagination: PaginationPolicy,
}

/// A fully resolved paragraph value. Definition values are the base; matching
/// stable-ID direct values override them; page bounds are the only neutral
/// paragraph fallback. `base_leading` belongs to the element setting, while
/// wrapped lines retain their own actual-style maximum. No Role/static
/// formatting row participates.
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
    _runs: &[TextRun],
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
    match paragraph.leading.as_str() {
        "Regular" | "Single" => (),
        _ => {
            return Err(LayoutError {
                kind: LayoutErrorKind::Model(ModelError::InvalidNumber),
            })
        }
    }
    let spacing_single = direct
        .paragraph
        .spacing_single
        .unwrap_or(paragraph.spacing == 1);
    if !spacing_single {
        return Err(LayoutError {
            kind: LayoutErrorKind::Model(ModelError::InvalidNumber),
        });
    }
    let base_leading = resolve_style_leading(&style, environment)?;
    let source_size = style.size;
    let definition_space_before = paragraph.space_before;
    let (space_before_units, space_before) = match direct.paragraph.space_before {
        Some(value) => derive_space_before(value, source_size, base_leading)
            .or_else(|_| derive_space_before(definition_space_before, source_size, base_leading))?,
        None => derive_space_before(definition_space_before, source_size, base_leading)?,
    };
    let pagination = PaginationPolicy::from_paginate_as(definition.behavior.paginate_as.as_deref());
    Ok(EffectiveParagraph {
        left,
        right,
        first_indent,
        alignment: direct.paragraph.alignment.unwrap_or(paragraph.alignment),
        space_before,
        space_before_units,
        base_leading,
        starts_new_page: direct.starts_new_page.unwrap_or(paragraph.starts_new_page),
        pagination,
    })
}

fn resolve_page_baseline_paragraph(
    formatting: scriptcraft_model::title_page::TitlePageParagraphFormatting,
    _runs: &[TextRun],
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
    if formatting.spacing_single == Some(false) {
        return Err(LayoutError {
            kind: LayoutErrorKind::Model(ModelError::InvalidNumber),
        });
    }
    let base_leading = resolve_style_leading(style, environment)?;
    let (space_before_units, space_before) = match formatting.space_before {
        Some(value) => derive_space_before(value, style.size, base_leading)
            .unwrap_or((0, FixedPoint::default())),
        None => derive_space_before(FixedPoint::default(), style.size, base_leading)?,
    };
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
        space_before,
        space_before_units,
        base_leading,
        starts_new_page: formatting.starts_new_page.unwrap_or(false),
        pagination: PaginationPolicy::Ordinary,
    })
}

pub(super) fn resolve_style_leading(
    style: &ResolvedTextStyle,
    environment: &LayoutEnvironment,
) -> Result<FixedPoint, LayoutError> {
    let face = environment.fonts.require_face(&style.resolved_font_id)?;
    let ratio = match face.leading_resolver {
        LeadingResolverKind::Courier => 100_i64,
        LeadingResolverKind::Proportional => 120_i64,
    };
    let base = i64::from(style.size)
        .checked_mul(ratio)
        .ok_or(LayoutError { kind: LayoutErrorKind::InvalidEffectiveLeading })?;
    let adjustment = match environment.page.document_leading {
        DocumentLeading::Normal => 0,
        DocumentLeading::Loose => FixedPoint::SCALE,
        DocumentLeading::Tight => -FixedPoint::SCALE,
        DocumentLeading::VeryTight => -2 * FixedPoint::SCALE,
        DocumentLeading::Unsupported => {
            return Err(LayoutError { kind: LayoutErrorKind::UnsupportedDocumentLeading })
        }
    };
    let raw = base
        .checked_add(adjustment)
        .filter(|value| *value > 0)
        .ok_or(LayoutError { kind: LayoutErrorKind::InvalidEffectiveLeading })?;
    Ok(FixedPoint::from_raw(raw))
}

fn derive_space_before(
    value: FixedPoint,
    base_size: u16,
    base_leading: FixedPoint,
) -> Result<(u32, FixedPoint), LayoutError> {
    let size_raw = i64::from(base_size)
        .checked_mul(FixedPoint::SCALE)
        .filter(|size| *size > 0)
        .ok_or(LayoutError { kind: LayoutErrorKind::InvalidSpaceBefore })?;
    let source_raw = value.raw();
    if !(0..=79_200).contains(&source_raw) || source_raw % size_raw != 0 {
        return Err(LayoutError { kind: LayoutErrorKind::InvalidSpaceBefore });
    }
    let units = u32::try_from(source_raw / size_raw)
        .map_err(|_| LayoutError { kind: LayoutErrorKind::InvalidSpaceBefore })?;
    let gap_raw = i64::from(units)
        .checked_mul(base_leading.raw())
        .ok_or(LayoutError { kind: LayoutErrorKind::InvalidSpaceBefore })?;
    Ok((units, FixedPoint::from_raw(gap_raw)))
}

pub(super) fn resolve_line_vertical_metrics(
    base: &ResolvedTextStyle,
    scalars: &[super::wrap::ResolvedDisplayScalar],
    environment: &LayoutEnvironment,
) -> Result<LineVerticalMetrics, LayoutError> {
    let mut leading = resolve_style_leading(base, environment)?;
    let mut natural_ascent = FixedPoint::from_raw(i64::MIN);
    let mut natural_descent = FixedPoint::from_raw(i64::MAX);
    let mut participants: Vec<&ResolvedTextStyle> = Vec::new();
    if scalars.is_empty() {
        participants.push(base);
    } else {
        for scalar in scalars {
            if !participants.contains(&&scalar.text_style) {
                participants.push(&scalar.text_style);
            }
        }
    }
    for style in participants {
        leading = leading.max(resolve_style_leading(style, environment)?);
        let face = environment.fonts.require_face(&style.resolved_font_id)?;
        natural_ascent = natural_ascent.max(face.metrics.scale_ascent(style.size)?);
        natural_descent = natural_descent.min(face.metrics.scale_descent(style.size)?);
    }
    Ok(LineVerticalMetrics { leading, natural_ascent, natural_descent })
}

