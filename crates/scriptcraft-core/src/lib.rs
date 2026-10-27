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

const ROLE_BOX_COUNT: usize = 7;

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
    InvalidUtf8Boundary,
    InvalidNumber,
    InvalidJson,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    SceneHeading,
    Action,
    Character,
    Dialogue,
    Parenthetical,
    Transition,
    Note,
}

impl Role {
    fn as_str(self) -> &'static str {
        match self {
            Self::SceneHeading => "scene_heading",
            Self::Action => "action",
            Self::Character => "character",
            Self::Dialogue => "dialogue",
            Self::Parenthetical => "parenthetical",
            Self::Transition => "transition",
            Self::Note => "note",
        }
    }
    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "scene_heading" => Self::SceneHeading,
            "action" => Self::Action,
            "character" => Self::Character,
            "dialogue" => Self::Dialogue,
            "parenthetical" => Self::Parenthetical,
            "transition" => Self::Transition,
            "note" => Self::Note,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenplayElement {
    pub id: StableId,
    pub role: Role,
    pub text: String,
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
        Ok(Self { id, role, text })
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

#[derive(Debug, Clone, PartialEq, Eq)]
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
    pub role_boxes: [RoleBox; ROLE_BOX_COUNT],
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
            font_family: "Courier".into(),
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
        let start = ScreenplayElement::new("element-1", Role::SceneHeading, "")?;
        Ok(Self {
            id: StableId::new(id)?,
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
fn decode_element(v: &Json) -> Result<ScreenplayElement, ModelError> {
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

fn decode_role_boxes(v: &Json) -> Result<[RoleBox; ROLE_BOX_COUNT], ModelError> {
    let Json::Array(items) = v else {
        return Err(ModelError::WrongType("role_boxes"));
    };
    if items.len() != ROLE_BOX_COUNT {
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
        assert_eq!(p.page_profile.font_family, "Courier");
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
        let missing = json.replace(",\"font_family\":\"Courier\"", "");
        assert_eq!(
            Project::from_json(&missing),
            Err(ModelError::MissingField("font_family"))
        );
        let malformed = json.replace("\"font_family\":\"Courier\"", "\"font_family\":1");
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
    fn all_roles_encode() {
        let roles = [
            Role::SceneHeading,
            Role::Action,
            Role::Character,
            Role::Dialogue,
            Role::Parenthetical,
            Role::Transition,
            Role::Note,
        ];
        let mut p = Project::new("p", "t", "a").unwrap_or_else(|_| unreachable!());
        p.elements = roles
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
}
