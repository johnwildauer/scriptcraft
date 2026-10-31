#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

//! Dependency-free semantic screenplay model and native JSON schema.

pub mod editing;
pub mod exports;
pub mod layout;
pub mod persistence;

use std::collections::BTreeSet;

pub const FOUNDATION_STATUS: &str = "scriptcraft foundation ready";
pub const CURRENT_SCHEMA_VERSION: u32 = 1;
pub const MAX_JSON_BYTES: usize = 1_048_576;
pub const MAX_ELEMENTS: usize = 10_000;
pub const MAX_TEXT_BYTES: usize = 262_144;
pub const MAX_STRING_BYTES: usize = 262_144;
pub const POINTS_PER_INCH: u32 = 72;

const LEGACY_ROLE_BOX_COUNT: usize = 7;
const ADDITIONAL_ROLE_BOX_COUNT: usize = 10;
const MAX_RUNS_PER_ELEMENT: usize = 1_024;
const MAX_RUN_ATTRIBUTES: usize = 128;
const MAX_RUN_ATTRIBUTE_BYTES: usize = 4_096;
const MAX_PROVENANCE_STRING_BYTES: usize = 4_096;

const fn points_from_eighths_of_an_inch(eighths: u32) -> u32 {
    eighths * (POINTS_PER_INCH / 8)
}

pub const fn foundation_status() -> &'static str {
    FOUNDATION_STATUS
}

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
    InvalidUtf8Boundary,
    InvalidNumber,
    InvalidJson,
    UnsupportedJsonState,
    MissingField(&'static str),
    WrongType(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StableId(String);

impl StableId {
    pub fn new(value: impl Into<String>) -> Result<Self, ModelError> {
        let value = value.into();
        if value.is_empty() {
            return Err(ModelError::EmptyId);
        }
        if value.len() > 128
            || !value
                .bytes()
                .all(|b| b.is_ascii_graphic() && b != b'"' && b != b'\\')
        {
            return Err(ModelError::InvalidId);
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Allocates deterministic IDs that do not collide with any reserved ID.
/// Foreign source IDs must be kept in their own namespace and are not reserved here.
#[derive(Debug, Clone)]
pub struct StableIdAllocator {
    reserved: BTreeSet<StableId>,
    next_serial: u64,
}

impl Default for StableIdAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl StableIdAllocator {
    pub fn new() -> Self {
        Self {
            reserved: BTreeSet::new(),
            next_serial: 1,
        }
    }

    pub fn reserve(&mut self, id: StableId) -> Result<(), ModelError> {
        if !self.reserved.insert(id) {
            return Err(ModelError::DuplicateId);
        }
        Ok(())
    }

    pub fn allocate(&mut self, prefix: &str) -> Result<StableId, ModelError> {
        loop {
            let serial = self.next_serial;
            self.next_serial = self
                .next_serial
                .checked_add(1)
                .ok_or(ModelError::InvalidNumber)?;
            let candidate = StableId::new(format!("{prefix}-{serial}"))?;
            if self.reserved.insert(candidate.clone()) {
                return Ok(candidate);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Role {
    SceneHeading,
    Action,
    Character,
    Dialogue,
    Parenthetical,
    Transition,
    Note,
    NewAct,
    EndOfAct,
    CastList,
    General,
    Shot,
    Sequence,
    Outline1,
    Outline2,
    Outline3,
    Summary,
}

impl Role {
    pub const ALL: [Self; 17] = [
        Self::SceneHeading,
        Self::Action,
        Self::Character,
        Self::Dialogue,
        Self::Parenthetical,
        Self::Transition,
        Self::Note,
        Self::NewAct,
        Self::EndOfAct,
        Self::CastList,
        Self::General,
        Self::Shot,
        Self::Sequence,
        Self::Outline1,
        Self::Outline2,
        Self::Outline3,
        Self::Summary,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SceneHeading => "scene_heading",
            Self::Action => "action",
            Self::Character => "character",
            Self::Dialogue => "dialogue",
            Self::Parenthetical => "parenthetical",
            Self::Transition => "transition",
            Self::Note => "note",
            Self::NewAct => "new_act",
            Self::EndOfAct => "end_of_act",
            Self::CastList => "cast_list",
            Self::General => "general",
            Self::Shot => "shot",
            Self::Sequence => "sequence",
            Self::Outline1 => "outline_1",
            Self::Outline2 => "outline_2",
            Self::Outline3 => "outline_3",
            Self::Summary => "summary",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "scene_heading" => Self::SceneHeading,
            "action" => Self::Action,
            "character" => Self::Character,
            "dialogue" => Self::Dialogue,
            "parenthetical" => Self::Parenthetical,
            "transition" => Self::Transition,
            "note" => Self::Note,
            "new_act" => Self::NewAct,
            "end_of_act" => Self::EndOfAct,
            "cast_list" => Self::CastList,
            "general" => Self::General,
            "shot" => Self::Shot,
            "sequence" => Self::Sequence,
            "outline_1" => Self::Outline1,
            "outline_2" => Self::Outline2,
            "outline_3" => Self::Outline3,
            "summary" => Self::Summary,
            _ => return None,
        })
    }

    const fn legacy_profile_index(self) -> Option<usize> {
        match self {
            Self::SceneHeading => Some(0),
            Self::Action => Some(1),
            Self::Character => Some(2),
            Self::Dialogue => Some(3),
            Self::Parenthetical => Some(4),
            Self::Transition => Some(5),
            Self::Note => Some(6),
            _ => None,
        }
    }

    const fn additional_profile_index(self) -> Option<usize> {
        match self {
            Self::NewAct => Some(0),
            Self::EndOfAct => Some(1),
            Self::CastList => Some(2),
            Self::General => Some(3),
            Self::Shot => Some(4),
            Self::Sequence => Some(5),
            Self::Outline1 => Some(6),
            Self::Outline2 => Some(7),
            Self::Outline3 => Some(8),
            Self::Summary => Some(9),
            _ => None,
        }
    }

    const fn is_legacy(self) -> bool {
        self.legacy_profile_index().is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextRun {
    pub text: String,
    /// Ordered source attributes are retained as bounded opaque name/value pairs.
    pub attributes: Vec<RunAttribute>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunAttribute {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFormat {
    Fdx,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceSection {
    Content,
    TitlePage,
}

/// Minimal source coordinates for later ordered preservation; this is not the
/// source tree or a claim that the source can yet be safely rewritten.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceProvenance {
    pub format: SourceFormat,
    pub section: SourceSection,
    pub ordinal: u32,
    pub foreign_id: Option<String>,
    pub foreign_role: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenplayElement {
    pub id: StableId,
    pub role: Role,
    /// Plain-text compatibility projection used by existing editing/layout clients.
    pub text: String,
    /// Empty for legacy plain-text elements; populated runs remain source ordered.
    pub runs: Vec<TextRun>,
    pub source_provenance: Option<SourceProvenance>,
}

impl ScreenplayElement {
    pub fn new(
        id: impl Into<String>,
        role: Role,
        text: impl Into<String>,
    ) -> Result<Self, ModelError> {
        let id = StableId::new(id)?;
        let text = text.into();
        if text.len() > MAX_TEXT_BYTES {
            return Err(ModelError::TextTooLong);
        }
        Ok(Self {
            id,
            role,
            text,
            runs: Vec::new(),
            source_provenance: None,
        })
    }

    pub fn with_runs(
        id: impl Into<String>,
        role: Role,
        runs: Vec<TextRun>,
        source_provenance: Option<SourceProvenance>,
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
        let element = Self {
            id: StableId::new(id)?,
            role,
            text,
            runs,
            source_provenance,
        };
        validate_element(&element)?;
        Ok(element)
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
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Metadata {
    pub title: String,
    pub author: String,
    pub contact: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoleBox {
    pub x: u32,
    pub width: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageProfile {
    pub name: String,
    pub format_version: u32,
    pub paper_width: u32,
    pub paper_height: u32,
    pub margin_top: u32,
    pub margin_bottom: u32,
    pub margin_left: u32,
    pub margin_right: u32,
    pub chars_per_line: u32,
    pub lines_per_page: u32,
    pub line_advance: u32,
    pub boundary_gap: u32,
    pub uppercase_cues: bool,
    pub auto_parenthetical: bool,
    pub strict_transition_boundaries: bool,
    pub font_family: String,
    pub font_size: u32,
    pub char_width: u32,
    /// The original seven role boxes retain their established indices and values.
    pub role_boxes: [RoleBox; LEGACY_ROLE_BOX_COUNT],
    /// Added role geometry is explicitly optional until separately specified.
    pub additional_role_boxes: [Option<RoleBox>; ADDITIONAL_ROLE_BOX_COUNT],
}

impl PageProfile {
    pub fn role_box(&self, role: Role) -> Option<&RoleBox> {
        if let Some(index) = role.legacy_profile_index() {
            self.role_boxes.get(index)
        } else {
            role.additional_profile_index()
                .and_then(|index| self.additional_role_boxes.get(index))
                .and_then(Option::as_ref)
        }
    }
}

impl Default for PageProfile {
    fn default() -> Self {
        Self {
            name: "us_letter_screenplay".into(),
            format_version: 1,
            paper_width: points_from_eighths_of_an_inch(68),
            paper_height: points_from_eighths_of_an_inch(88),
            margin_top: points_from_eighths_of_an_inch(8),
            margin_bottom: points_from_eighths_of_an_inch(8),
            margin_left: points_from_eighths_of_an_inch(12),
            margin_right: points_from_eighths_of_an_inch(8),
            chars_per_line: 55,
            lines_per_page: 55,
            line_advance: 12,
            boundary_gap: 12,
            uppercase_cues: true,
            auto_parenthetical: true,
            strict_transition_boundaries: true,
            font_family: "Courier Prime".into(),
            font_size: 12,
            char_width: 7,
            role_boxes: [
                RoleBox { x: 108, width: 432 },
                RoleBox { x: 108, width: 432 },
                RoleBox { x: 259, width: 175 },
                RoleBox { x: 180, width: 216 },
                RoleBox { x: 216, width: 180 },
                RoleBox { x: 432, width: 108 },
                RoleBox { x: 108, width: 432 },
            ],
            additional_role_boxes: [None; ADDITIONAL_ROLE_BOX_COUNT],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub id: StableId,
    pub schema_version: u32,
    pub metadata: Metadata,
    pub title_page: Vec<ScreenplayElement>,
    pub page_profile: PageProfile,
    pub elements: Vec<ScreenplayElement>,
}

impl Project {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        author: impl Into<String>,
    ) -> Result<Self, ModelError> {
        let id = StableId::new(id)?;
        let mut ids = StableIdAllocator::new();
        ids.reserve(id.clone())?;
        let start = ScreenplayElement::new(
            ids.allocate("element")?.as_str().to_owned(),
            Role::SceneHeading,
            "",
        )?;
        Ok(Self {
            id,
            schema_version: CURRENT_SCHEMA_VERSION,
            metadata: Metadata {
                title: title.into(),
                author: author.into(),
                ..Metadata::default()
            },
            title_page: Vec::new(),
            page_profile: PageProfile::default(),
            elements: vec![start],
        })
    }
    pub fn validate(&self) -> Result<(), ModelError> {
        if self.schema_version != CURRENT_SCHEMA_VERSION {
            return Err(ModelError::InvalidSchemaVersion);
        }
        validate_strings(&self.metadata)?;
        validate_profile(&self.page_profile)?;
        if self.elements.len() > MAX_ELEMENTS || self.title_page.len() > MAX_ELEMENTS {
            return Err(ModelError::TooManyElements);
        }
        let mut ids = BTreeSet::new();
        ids.insert(&self.id);
        for item in self.title_page.iter().chain(self.elements.iter()) {
            validate_element(item)?;
            if !ids.insert(&item.id) {
                return Err(ModelError::DuplicateId);
            }
        }
        Ok(())
    }
    pub fn to_json(&self) -> Result<String, ModelError> {
        self.validate()?;
        ensure_legacy_json_compatible(self)?;
        let json = encode_envelope(self);
        if json.len() > MAX_JSON_BYTES {
            return Err(ModelError::StringTooLong);
        }
        Ok(json)
    }
    pub fn from_json(input: &str) -> Result<Self, ModelError> {
        if input.len() > MAX_JSON_BYTES {
            return Err(ModelError::StringTooLong);
        }
        let value = Parser::new(input).parse()?;
        let project = decode_project(value)?;
        project.validate()?;
        Ok(project)
    }

    /// Allocates an application element ID distinct from the project, title-page,
    /// and body IDs. Foreign source IDs intentionally remain a separate namespace.
    pub fn allocate_element_id(&self) -> Result<StableId, ModelError> {
        let mut allocator = StableIdAllocator::new();
        allocator.reserve(self.id.clone())?;
        for element in self.title_page.iter().chain(self.elements.iter()) {
            allocator.reserve(element.id.clone())?;
        }
        allocator.allocate("element")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeProjectEnvelope {
    pub schema_version: u32,
    pub project: Project,
}

impl NativeProjectEnvelope {
    pub fn new(project: Project) -> Result<Self, ModelError> {
        project.validate()?;
        Ok(Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            project,
        })
    }
    pub fn to_json(&self) -> Result<String, ModelError> {
        if self.schema_version != CURRENT_SCHEMA_VERSION {
            return Err(ModelError::InvalidSchemaVersion);
        }
        self.project.to_json()
    }
    pub fn from_json(input: &str) -> Result<Self, ModelError> {
        let project = Project::from_json(input)?;
        Ok(Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            project,
        })
    }
}

fn validate_strings(m: &Metadata) -> Result<(), ModelError> {
    for s in [
        &m.title,
        &m.author,
        &m.contact,
        &m.created_at,
        &m.updated_at,
    ] {
        if s.len() > MAX_STRING_BYTES {
            return Err(ModelError::StringTooLong);
        }
    }
    Ok(())
}
fn validate_profile(p: &PageProfile) -> Result<(), ModelError> {
    if p.name.len() > MAX_STRING_BYTES
        || p.format_version == 0
        || p.paper_width == 0
        || p.paper_height == 0
        || p.chars_per_line == 0
        || p.lines_per_page == 0
        || p.line_advance == 0
        || p.line_advance > 1_000
        || p.boundary_gap > 1_000
        || !matches!(
            p.font_family.as_str(),
            "Courier" | "Courier New" | "Courier Prime"
        )
        || p.font_size == 0
        || p.font_size > 72
        || p.char_width == 0
        || p.char_width > 100
        || p.role_boxes
            .iter()
            .any(|box_| box_.width == 0 || box_.width > 1_000 || box_.x > 10_000)
        || p.additional_role_boxes
            .iter()
            .flatten()
            .any(|box_| box_.width == 0 || box_.width > 1_000 || box_.x > 10_000)
    {
        return Err(ModelError::InvalidNumber);
    }
    Ok(())
}
fn validate_element(e: &ScreenplayElement) -> Result<(), ModelError> {
    StableId::new(e.id.0.clone())?;
    if e.text.len() > MAX_TEXT_BYTES {
        return Err(ModelError::TextTooLong);
    }
    if e.runs.len() > MAX_RUNS_PER_ELEMENT {
        return Err(ModelError::TooManyRuns);
    }
    let mut run_text = String::new();
    let mut attribute_count = 0usize;
    for run in &e.runs {
        if run_text.len().saturating_add(run.text.len()) > MAX_TEXT_BYTES {
            return Err(ModelError::TextTooLong);
        }
        run_text.push_str(&run.text);
        attribute_count = attribute_count.saturating_add(run.attributes.len());
        if run.attributes.iter().any(|attribute| {
            attribute.name.is_empty()
                || attribute.name.len() > MAX_RUN_ATTRIBUTE_BYTES
                || attribute.value.len() > MAX_RUN_ATTRIBUTE_BYTES
        }) {
            return Err(ModelError::StringTooLong);
        }
    }
    if attribute_count > MAX_RUN_ATTRIBUTES {
        return Err(ModelError::TooManyRunAttributes);
    }
    if !e.runs.is_empty() && run_text != e.text {
        return Err(ModelError::InvalidRunProjection);
    }
    if e.source_provenance.as_ref().is_some_and(|source| {
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

fn ensure_legacy_json_compatible(project: &Project) -> Result<(), ModelError> {
    let richer_profile = project
        .page_profile
        .additional_role_boxes
        .iter()
        .any(Option::is_some);
    let richer_elements = project
        .title_page
        .iter()
        .chain(project.elements.iter())
        .any(|element| {
            !element.role.is_legacy()
                || !element.runs.is_empty()
                || element.source_provenance.is_some()
        });
    if richer_profile || richer_elements {
        return Err(ModelError::UnsupportedJsonState);
    }
    Ok(())
}

fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
fn element_json(e: &ScreenplayElement) -> String {
    format!(
        "{{\"id\":{},\"role\":{},\"text\":{}}}",
        quote(e.id.as_str()),
        quote(e.role.as_str()),
        quote(&e.text)
    )
}
fn elements_json(items: &[ScreenplayElement]) -> String {
    items.iter().map(element_json).collect::<Vec<_>>().join(",")
}
fn encode_envelope(p: &Project) -> String {
    let role_boxes = p
        .page_profile
        .role_boxes
        .iter()
        .map(|box_| format!("{{\"x\":{},\"width\":{}}}", box_.x, box_.width))
        .collect::<Vec<_>>()
        .join(",");
    format!("{{\"schema_version\":{},\"project\":{{\"id\":{},\"schema_version\":{},\"metadata\":{{\"title\":{},\"author\":{},\"contact\":{},\"created_at\":{},\"updated_at\":{}}},\"title_page\":[{}],\"page_profile\":{{\"name\":{},\"format_version\":{},\"paper_width\":{},\"paper_height\":{},\"margin_top\":{},\"margin_bottom\":{},\"margin_left\":{},\"margin_right\":{},\"chars_per_line\":{},\"lines_per_page\":{},\"line_advance\":{},\"boundary_gap\":{},\"uppercase_cues\":{},\"auto_parenthetical\":{},\"strict_transition_boundaries\":{},\"font_family\":{},\"font_size\":{},\"char_width\":{},\"role_boxes\":[{}]}},\"elements\":[{}]}}}}", CURRENT_SCHEMA_VERSION, quote(p.id.as_str()), p.schema_version, quote(&p.metadata.title), quote(&p.metadata.author), quote(&p.metadata.contact), quote(&p.metadata.created_at), quote(&p.metadata.updated_at), elements_json(&p.title_page), quote(&p.page_profile.name), p.page_profile.format_version, p.page_profile.paper_width, p.page_profile.paper_height, p.page_profile.margin_top, p.page_profile.margin_bottom, p.page_profile.margin_left, p.page_profile.margin_right, p.page_profile.chars_per_line, p.page_profile.lines_per_page, p.page_profile.line_advance, p.page_profile.boundary_gap, p.page_profile.uppercase_cues, p.page_profile.auto_parenthetical, p.page_profile.strict_transition_boundaries, quote(&p.page_profile.font_family), p.page_profile.font_size, p.page_profile.char_width, role_boxes, elements_json(&p.elements))
}

#[derive(Debug, Clone, PartialEq)]
enum Json {
    Object(Vec<(String, Json)>),
    Array(Vec<Json>),
    String(String),
    Number(u32),
    Bool(bool),
}
struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}
impl<'a> Parser<'a> {
    fn new(s: &'a str) -> Self {
        Self {
            bytes: s.as_bytes(),
            pos: 0,
        }
    }
    fn parse(mut self) -> Result<Json, ModelError> {
        let v = self.value()?;
        self.ws();
        if self.pos != self.bytes.len() {
            return Err(ModelError::InvalidJson);
        }
        Ok(v)
    }
    fn ws(&mut self) {
        while self
            .bytes
            .get(self.pos)
            .is_some_and(|b| b.is_ascii_whitespace())
        {
            self.pos += 1;
        }
    }
    fn value(&mut self) -> Result<Json, ModelError> {
        self.ws();
        match self.bytes.get(self.pos) {
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => Ok(Json::String(self.string()?)),
            Some(b'0'..=b'9') => self.number(),
            Some(b't') if self.bytes.get(self.pos..self.pos.saturating_add(4)) == Some(b"true") => {
                self.pos += 4;
                Ok(Json::Bool(true))
            }
            Some(b'f')
                if self.bytes.get(self.pos..self.pos.saturating_add(5)) == Some(b"false") =>
            {
                self.pos += 5;
                Ok(Json::Bool(false))
            }
            _ => Err(ModelError::InvalidJson),
        }
    }
    fn object(&mut self) -> Result<Json, ModelError> {
        self.pos += 1;
        let mut out = Vec::new();
        self.ws();
        if self.bytes.get(self.pos) == Some(&b'}') {
            self.pos += 1;
            return Ok(Json::Object(out));
        }
        loop {
            self.ws();
            let key = self.string()?;
            self.ws();
            if self.bytes.get(self.pos) != Some(&b':') {
                return Err(ModelError::InvalidJson);
            }
            self.pos += 1;
            out.push((key, self.value()?));
            self.ws();
            match self.bytes.get(self.pos) {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(Json::Object(out));
                }
                _ => return Err(ModelError::InvalidJson),
            }
        }
    }
    fn array(&mut self) -> Result<Json, ModelError> {
        self.pos += 1;
        let mut out = Vec::new();
        self.ws();
        if self.bytes.get(self.pos) == Some(&b']') {
            self.pos += 1;
            return Ok(Json::Array(out));
        }
        loop {
            out.push(self.value()?);
            self.ws();
            match self.bytes.get(self.pos) {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    return Ok(Json::Array(out));
                }
                _ => return Err(ModelError::InvalidJson),
            }
        }
    }
    fn number(&mut self) -> Result<Json, ModelError> {
        let start = self.pos;
        while self.bytes.get(self.pos).is_some_and(u8::is_ascii_digit) {
            self.pos += 1;
        }
        let s = std::str::from_utf8(&self.bytes[start..self.pos])
            .map_err(|_| ModelError::InvalidNumber)?;
        s.parse()
            .map(Json::Number)
            .map_err(|_| ModelError::InvalidNumber)
    }
    fn string(&mut self) -> Result<String, ModelError> {
        if self.bytes.get(self.pos) != Some(&b'"') {
            return Err(ModelError::InvalidJson);
        }
        self.pos += 1;
        let mut out = String::new();
        while let Some(&b) = self.bytes.get(self.pos) {
            self.pos += 1;
            match b {
                b'"' => return Ok(out),
                b'\\' => {
                    let esc = *self.bytes.get(self.pos).ok_or(ModelError::InvalidJson)?;
                    self.pos += 1;
                    match esc {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        _ => return Err(ModelError::InvalidJson),
                    }
                }
                0..=31 => return Err(ModelError::InvalidJson),
                _ => {
                    let start = self.pos - 1;
                    while self
                        .bytes
                        .get(self.pos)
                        .is_some_and(|c| *c >= 32 && *c != b'"' && *c != b'\\')
                    {
                        self.pos += 1;
                    }
                    let part = std::str::from_utf8(&self.bytes[start..self.pos])
                        .map_err(|_| ModelError::InvalidJson)?;
                    out.push_str(part);
                }
            }
            if out.len() > MAX_STRING_BYTES {
                return Err(ModelError::StringTooLong);
            }
        }
        Err(ModelError::InvalidJson)
    }
}

fn obj<'a>(v: &'a Json, key: &'static str) -> Result<&'a Json, ModelError> {
    if let Json::Object(fields) = v {
        fields
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
            .ok_or(ModelError::MissingField(key))
    } else {
        Err(ModelError::WrongType(key))
    }
}
fn str_field(v: &Json, key: &'static str) -> Result<String, ModelError> {
    match obj(v, key)? {
        Json::String(s) => Ok(s.clone()),
        _ => Err(ModelError::WrongType(key)),
    }
}
fn num_field(v: &Json, key: &'static str) -> Result<u32, ModelError> {
    match obj(v, key)? {
        Json::Number(n) => Ok(*n),
        _ => Err(ModelError::WrongType(key)),
    }
}
fn optional_bool_field(v: &Json, key: &'static str, default: bool) -> Result<bool, ModelError> {
    let Json::Object(fields) = v else {
        return Err(ModelError::WrongType(key));
    };
    match fields.iter().find(|(name, _)| name == key) {
        Some((_, Json::Bool(value))) => Ok(*value),
        Some(_) => Err(ModelError::WrongType(key)),
        None => Ok(default),
    }
}
fn object_has_field(v: &Json, key: &str) -> bool {
    matches!(v, Json::Object(fields) if fields.iter().any(|(name, _)| name == key))
}

fn decode_element(v: &Json) -> Result<ScreenplayElement, ModelError> {
    if object_has_field(v, "runs") || object_has_field(v, "source_provenance") {
        return Err(ModelError::UnsupportedJsonState);
    }
    let role = Role::parse(&str_field(v, "role")?).ok_or(ModelError::InvalidJson)?;
    ScreenplayElement::new(str_field(v, "id")?, role, str_field(v, "text")?)
}
fn decode_elements(v: &Json) -> Result<Vec<ScreenplayElement>, ModelError> {
    match v {
        Json::Array(a) => {
            if a.len() > MAX_ELEMENTS {
                return Err(ModelError::TooManyElements);
            }
            a.iter().map(decode_element).collect()
        }
        _ => Err(ModelError::WrongType("elements")),
    }
}

fn decode_role_boxes(v: &Json) -> Result<[RoleBox; LEGACY_ROLE_BOX_COUNT], ModelError> {
    let Json::Array(items) = v else {
        return Err(ModelError::WrongType("role_boxes"));
    };
    if items.len() != LEGACY_ROLE_BOX_COUNT {
        return Err(ModelError::InvalidNumber);
    }
    let boxes = items
        .iter()
        .map(|item| {
            Ok(RoleBox {
                x: num_field(item, "x")?,
                width: num_field(item, "width")?,
            })
        })
        .collect::<Result<Vec<_>, ModelError>>()?;
    boxes.try_into().map_err(|_| ModelError::InvalidNumber)
}
fn decode_project(root: Json) -> Result<Project, ModelError> {
    let envelope_schema = num_field(&root, "schema_version")?;
    if envelope_schema != CURRENT_SCHEMA_VERSION {
        return Err(ModelError::InvalidSchemaVersion);
    }
    let envelope = obj(&root, "project")?;
    let schema = num_field(envelope, "schema_version")?;
    let m = obj(envelope, "metadata")?;
    let profile = obj(envelope, "page_profile")?;
    Ok(Project {
        id: StableId::new(str_field(envelope, "id")?)?,
        schema_version: schema,
        metadata: Metadata {
            title: str_field(m, "title")?,
            author: str_field(m, "author")?,
            contact: str_field(m, "contact")?,
            created_at: str_field(m, "created_at")?,
            updated_at: str_field(m, "updated_at")?,
        },
        title_page: decode_elements(obj(envelope, "title_page")?)?,
        page_profile: PageProfile {
            name: str_field(profile, "name")?,
            format_version: num_field(profile, "format_version")?,
            paper_width: num_field(profile, "paper_width")?,
            paper_height: num_field(profile, "paper_height")?,
            margin_top: num_field(profile, "margin_top")?,
            margin_bottom: num_field(profile, "margin_bottom")?,
            margin_left: num_field(profile, "margin_left")?,
            margin_right: num_field(profile, "margin_right")?,
            chars_per_line: num_field(profile, "chars_per_line")?,
            lines_per_page: num_field(profile, "lines_per_page")?,
            line_advance: num_field(profile, "line_advance")?,
            boundary_gap: num_field(profile, "boundary_gap")?,
            uppercase_cues: optional_bool_field(profile, "uppercase_cues", true)?,
            auto_parenthetical: optional_bool_field(profile, "auto_parenthetical", true)?,
            strict_transition_boundaries: optional_bool_field(
                profile,
                "strict_transition_boundaries",
                true,
            )?,
            font_family: str_field(profile, "font_family")?,
            font_size: num_field(profile, "font_size")?,
            char_width: num_field(profile, "char_width")?,
            role_boxes: decode_role_boxes(obj(profile, "role_boxes")?)?,
            additional_role_boxes: if object_has_field(profile, "additional_role_boxes") {
                return Err(ModelError::UnsupportedJsonState);
            } else {
                [None; ADDITIONAL_ROLE_BOX_COUNT]
            },
        },
        elements: decode_elements(obj(envelope, "elements")?)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn new_project_has_valid_start() {
        let p = Project::new("p-1", "题名", "作者").unwrap_or_else(|_| unreachable!());
        assert_eq!(p.elements.len(), 1);
        assert_eq!(p.elements[0].id.as_str(), "element-1");
        assert_eq!(p.elements[0].role, Role::SceneHeading);
        assert!(p.elements[0].text.is_empty());
        assert_eq!(p.page_profile.font_family, "Courier Prime");
        assert!(p.page_profile.uppercase_cues);
        assert!(p.validate().is_ok());
    }

    #[test]
    fn new_project_session_starts_with_valid_scene_heading_caret() {
        let project = Project::new("p-1", "Title", "Author").unwrap_or_else(|_| unreachable!());
        let session = editing::EditingSession::new(project).unwrap_or_else(|_| unreachable!());

        assert_eq!(
            session.selection(),
            editing::Selection::caret(editing::Position {
                element: 0,
                offset: 0,
            })
        );
        assert_eq!(session.project().elements.len(), 1);
        assert_eq!(session.project().elements[0].role, Role::SceneHeading);
        assert!(session.project().elements[0].text.is_empty());
        assert!(session.validate().is_ok());
    }

    #[test]
    fn legacy_profile_missing_uppercase_policy_defaults_true() {
        let project = Project::new("p", "Title", "A").unwrap_or_else(|_| unreachable!());
        let json = project
            .to_json()
            .unwrap_or_else(|_| unreachable!())
            .replace(",\"uppercase_cues\":true", "");
        assert!(
            Project::from_json(&json)
                .unwrap_or_else(|_| unreachable!())
                .page_profile
                .uppercase_cues
        );
    }

    #[test]
    fn supported_profile_fonts_round_trip_without_rewriting() {
        for family in ["Courier", "Courier New", "Courier Prime"] {
            let mut project = Project::new("p", "Title", "A").unwrap_or_else(|_| unreachable!());
            project.page_profile.font_family = family.into();
            let json = project.to_json().unwrap_or_else(|_| unreachable!());
            let restored = Project::from_json(&json).unwrap_or_else(|_| unreachable!());
            assert_eq!(restored.page_profile.font_family, family);
        }
    }

    #[test]
    fn missing_or_malformed_font_fields_remain_typed_errors() {
        let project = Project::new("p", "Title", "A").unwrap_or_else(|_| unreachable!());
        let json = project.to_json().unwrap_or_else(|_| unreachable!());
        let missing = json.replace(",\"font_family\":\"Courier Prime\"", "");
        assert_eq!(
            Project::from_json(&missing),
            Err(ModelError::MissingField("font_family"))
        );
        let malformed = json.replace("\"font_family\":\"Courier Prime\"", "\"font_family\":1");
        assert_eq!(
            Project::from_json(&malformed),
            Err(ModelError::WrongType("font_family"))
        );
    }
    #[test]
    fn deterministic_unicode_round_trip() {
        let mut p = Project::new("p", "Title", "A").unwrap_or_else(|_| unreachable!());
        p.elements[0].text = "naïve café 漢字".into();
        let a = p.to_json().unwrap_or_else(|_| unreachable!());
        let b = Project::from_json(&a).unwrap_or_else(|_| unreachable!());
        assert_eq!(a, b.to_json().unwrap_or_else(|_| unreachable!()));
        assert!(b.elements[0].is_boundary("naïve".len()));
        assert!(!b.elements[0].is_boundary("na".len() + 1));
    }

    #[test]
    fn named_profile_round_trips_bounded_alternate_geometry() {
        let mut project = Project::new("p", "Title", "A").unwrap_or_else(|_| unreachable!());
        project.page_profile.name = "alternate_screenplay".into();
        project.page_profile.format_version = 2;
        project.page_profile.font_family = "Courier Prime".into();
        project.page_profile.font_size = 12;
        project.page_profile.char_width = 8;
        project.page_profile.line_advance = 14;
        project.page_profile.boundary_gap = 10;
        project.page_profile.role_boxes[1] = RoleBox { x: 120, width: 400 };
        let json = project.to_json().unwrap_or_else(|_| unreachable!());
        let restored = Project::from_json(&json).unwrap_or_else(|_| unreachable!());
        assert_eq!(restored, project);
        assert_eq!(json, restored.to_json().unwrap_or_else(|_| unreachable!()));
    }

    #[test]
    fn profile_rejects_unbounded_role_geometry() {
        let mut project = Project::new("p", "Title", "A").unwrap_or_else(|_| unreachable!());
        project.page_profile.role_boxes[0].width = 0;
        assert_eq!(project.to_json(), Err(ModelError::InvalidNumber));
        project.page_profile = PageProfile::default();
        project.page_profile.role_boxes[0].width = 1_001;
        assert_eq!(project.to_json(), Err(ModelError::InvalidNumber));
    }
    #[test]
    fn hostile_bounds_are_typed_errors_without_panics() {
        let p = Project::new("p", "t", "a").unwrap_or_else(|_| unreachable!());
        let mut s = p.to_json().unwrap_or_else(|_| unreachable!());
        s.push_str(&"x".repeat(MAX_JSON_BYTES));
        assert_eq!(Project::from_json(&s), Err(ModelError::StringTooLong));
        assert_eq!(StableId::new(""), Err(ModelError::EmptyId));
    }
    #[test]
    fn unsupported_outer_schema_version_is_a_typed_error() {
        let p = Project::new("p", "t", "a").unwrap_or_else(|_| unreachable!());
        let json = p.to_json().unwrap_or_else(|_| unreachable!()).replacen(
            "\"schema_version\":1",
            "\"schema_version\":999",
            1,
        );
        assert_eq!(
            Project::from_json(&json),
            Err(ModelError::InvalidSchemaVersion)
        );
    }
    #[test]
    fn legacy_roles_keep_json_round_trip() {
        assert_eq!(Role::ALL.len(), 17);
        let labels = Role::ALL.map(Role::as_str);
        let unique = labels.iter().copied().collect::<BTreeSet<_>>();
        assert_eq!(unique.len(), 17);
        for role in Role::ALL {
            assert_eq!(Role::parse(role.as_str()), Some(role));
        }

        let mut p = Project::new("p", "t", "a").unwrap_or_else(|_| unreachable!());
        p.elements = Role::ALL[..LEGACY_ROLE_BOX_COUNT]
            .iter()
            .enumerate()
            .map(|(i, r)| {
                ScreenplayElement::new(format!("e-{i}"), *r, "text")
                    .unwrap_or_else(|_| unreachable!())
            })
            .collect();
        assert_eq!(
            p,
            Project::from_json(&p.to_json().unwrap_or_else(|_| unreachable!()))
                .unwrap_or_else(|_| unreachable!())
        );
    }

    #[test]
    fn expanded_roles_are_distinct_and_transitional_json_refuses_them() {
        for (index, role) in Role::ALL[LEGACY_ROLE_BOX_COUNT..].iter().enumerate() {
            let mut project = Project::new("p", "t", "a").unwrap_or_else(|_| unreachable!());
            project.elements[0].role = *role;
            assert!(project.validate().is_ok());
            assert_eq!(Role::parse(role.as_str()), Some(*role));
            assert_eq!(Role::ALL[LEGACY_ROLE_BOX_COUNT + index], *role);
            assert_eq!(project.to_json(), Err(ModelError::UnsupportedJsonState));
        }
    }

    #[test]
    fn runs_and_source_provenance_keep_order_unicode_and_case() {
        let provenance = SourceProvenance {
            format: SourceFormat::Fdx,
            section: SourceSection::Content,
            ordinal: 9,
            foreign_id: Some("Foreign-ID-9".into()),
            foreign_role: Some("nEw Act".into()),
        };
        let element = ScreenplayElement::with_runs(
            "internal-1",
            Role::NewAct,
            vec![
                TextRun {
                    text: "naïve ".into(),
                    attributes: vec![RunAttribute {
                        name: "Style".into(),
                        value: "Bold".into(),
                    }],
                },
                TextRun {
                    text: "漢字🙂".into(),
                    attributes: Vec::new(),
                },
            ],
            Some(provenance.clone()),
        )
        .unwrap_or_else(|_| unreachable!());
        assert_eq!(element.text, "naïve 漢字🙂");
        assert_eq!(element.runs[0].attributes[0].value, "Bold");
        assert_eq!(element.source_provenance, Some(provenance));
        assert!(Project {
            id: StableId::new("p").unwrap_or_else(|_| unreachable!()),
            schema_version: CURRENT_SCHEMA_VERSION,
            metadata: Metadata::default(),
            title_page: Vec::new(),
            page_profile: PageProfile::default(),
            elements: vec![element],
        }
        .validate()
        .is_ok());
    }

    #[test]
    fn richer_model_and_json_payloads_are_rejected_by_transitional_json() {
        let mut project = Project::new("p", "t", "a").unwrap_or_else(|_| unreachable!());
        project.elements[0] = ScreenplayElement::with_runs(
            "rich",
            Role::Action,
            vec![TextRun {
                text: "text".into(),
                attributes: Vec::new(),
            }],
            None,
        )
        .unwrap_or_else(|_| unreachable!());
        assert_eq!(project.to_json(), Err(ModelError::UnsupportedJsonState));

        let legacy = Project::new("p", "t", "a")
            .unwrap_or_else(|_| unreachable!())
            .to_json()
            .unwrap_or_else(|_| unreachable!());
        let with_runs = legacy.replace(
            "\"role\":\"scene_heading\",\"text\":\"\"",
            "\"role\":\"scene_heading\",\"text\":\"\",\"runs\":[]",
        );
        assert_eq!(
            Project::from_json(&with_runs),
            Err(ModelError::UnsupportedJsonState)
        );

        let mut profile = Project::new("p", "t", "a").unwrap_or_else(|_| unreachable!());
        profile.page_profile.additional_role_boxes[0] = Some(RoleBox { x: 10, width: 20 });
        assert_eq!(profile.to_json(), Err(ModelError::UnsupportedJsonState));
    }

    #[test]
    fn model_rejects_run_projection_and_resource_limit_violations() {
        let mut element = ScreenplayElement::with_runs(
            "e",
            Role::Action,
            vec![TextRun {
                text: "x".into(),
                attributes: Vec::new(),
            }],
            None,
        )
        .unwrap_or_else(|_| unreachable!());
        element.text.push('y');
        assert_eq!(
            validate_element(&element),
            Err(ModelError::InvalidRunProjection)
        );

        let too_many_runs = vec![
            TextRun {
                text: String::new(),
                attributes: Vec::new(),
            };
            MAX_RUNS_PER_ELEMENT + 1
        ];
        assert_eq!(
            ScreenplayElement::with_runs("e", Role::Action, too_many_runs, None),
            Err(ModelError::TooManyRuns)
        );

        let too_many_attributes = (0..MAX_RUN_ATTRIBUTES + 1)
            .map(|index| RunAttribute {
                name: format!("attr-{index}"),
                value: String::new(),
            })
            .collect();
        assert_eq!(
            ScreenplayElement::with_runs(
                "e",
                Role::Action,
                vec![TextRun {
                    text: String::new(),
                    attributes: too_many_attributes,
                }],
                None,
            ),
            Err(ModelError::TooManyRunAttributes)
        );

        assert_eq!(
            ScreenplayElement::with_runs(
                "e",
                Role::Action,
                vec![TextRun {
                    text: String::new(),
                    attributes: vec![RunAttribute {
                        name: "x".repeat(MAX_RUN_ATTRIBUTE_BYTES + 1),
                        value: String::new(),
                    }],
                }],
                None,
            ),
            Err(ModelError::StringTooLong)
        );
        assert_eq!(
            ScreenplayElement::with_runs(
                "e",
                Role::Action,
                Vec::new(),
                Some(SourceProvenance {
                    format: SourceFormat::Fdx,
                    section: SourceSection::Content,
                    ordinal: 0,
                    foreign_id: Some("x".repeat(MAX_PROVENANCE_STRING_BYTES + 1)),
                    foreign_role: None,
                }),
            ),
            Err(ModelError::StringTooLong)
        );
    }

    #[test]
    fn role_profile_expansion_leaves_new_geometry_unassigned_by_default() {
        let mut profile = PageProfile::default();
        assert_eq!(
            profile.role_box(Role::SceneHeading),
            Some(&profile.role_boxes[0])
        );
        assert_eq!(profile.role_box(Role::NewAct), None);
        profile.additional_role_boxes[0] = Some(RoleBox { x: 120, width: 200 });
        assert_eq!(
            profile.role_box(Role::NewAct),
            profile.additional_role_boxes[0].as_ref()
        );
        assert!(validate_profile(&profile).is_ok());
        profile.additional_role_boxes[0] = Some(RoleBox { x: 120, width: 0 });
        assert_eq!(validate_profile(&profile), Err(ModelError::InvalidNumber));
    }

    #[test]
    fn all_roles_reach_layout_without_role_coercion() {
        let mut project = Project::new("p", "t", "a").unwrap_or_else(|_| unreachable!());
        project.elements = Role::ALL
            .iter()
            .enumerate()
            .map(|(index, role)| {
                ScreenplayElement::new(format!("role-{index}"), *role, "text")
                    .unwrap_or_else(|_| unreachable!())
            })
            .collect();
        let layout = layout::ScreenplayLayout::build(&project).unwrap_or_else(|_| unreachable!());
        for role in Role::ALL {
            assert!(layout
                .pages
                .iter()
                .flat_map(|page| page.lines.iter())
                .any(|line| line.role == role));
        }
    }

    #[test]
    fn project_and_element_ids_share_an_application_collision_domain() {
        let project = Project::new("element-1", "t", "a").unwrap_or_else(|_| unreachable!());
        assert_eq!(project.elements[0].id.as_str(), "element-2");
        assert!(project.validate().is_ok());

        let mut duplicate = Project::new("p", "t", "a").unwrap_or_else(|_| unreachable!());
        duplicate.elements[0].id = duplicate.id.clone();
        assert_eq!(duplicate.validate(), Err(ModelError::DuplicateId));

        let mut with_title = Project::new("p", "t", "a").unwrap_or_else(|_| unreachable!());
        with_title.title_page.push(
            ScreenplayElement::new("element-2", Role::Action, "title")
                .unwrap_or_else(|_| unreachable!()),
        );
        assert_eq!(
            with_title
                .allocate_element_id()
                .unwrap_or_else(|_| unreachable!())
                .as_str(),
            "element-3"
        );
    }
}
