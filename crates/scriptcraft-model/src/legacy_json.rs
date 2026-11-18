use std::collections::BTreeSet;
use std::sync::Arc;

use crate::{
    element::{BodyOwner, Role, ScreenplayElement},
    identity::StableId,
    layout_inputs::{LayoutInputs, RoleBox},
    project::{LegacyJsonMetadata, LegacyJsonState, Project},
    title_page::TitlePageParagraph,
    validation::MAX_ELEMENTS,
    ModelError,
};

pub const CURRENT_SCHEMA_VERSION: u32 = 1;
pub const MAX_JSON_BYTES: usize = 1_048_576;
pub const MAX_STRING_BYTES: usize = 262_144;
pub const POINTS_PER_INCH: u32 = 72;

impl Project {
    pub fn to_json(&self) -> Result<String, ModelError> {
        self.validate()?;
        ensure_legacy_json_compatible(self)?;
        let legacy = self
            .legacy_json_state
            .as_ref()
            .ok_or(ModelError::UnsupportedJsonState)?;
        let legacy_title_page = self
            .title_page
            .iter()
            .map(TitlePageParagraph::legacy_projection)
            .collect::<Result<Vec<_>, _>>()?;
        let json = encode_envelope(self, legacy, &legacy_title_page);
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

fn ensure_legacy_json_compatible(project: &Project) -> Result<(), ModelError> {
    let richer_profile = project.legacy_json_state.as_ref().is_some_and(|legacy| {
        legacy
            .layout_inputs
            .additional_role_boxes
            .iter()
            .any(Option::is_some)
    });
    let title_compatible = project
        .title_page
        .iter()
        .all(|element| element.legacy_projection().is_ok());
    let richer_elements = !title_compatible
        || !project.direct_paragraph_overrides.is_empty()
        || !project.dual_dialogue_groups.is_empty()
        || !project.dual_dialogue_paragraph_overrides.is_empty()
        || project.elements.iter().any(|element| {
            !element.role.is_legacy()
                || !element.runs.is_empty()
                || element.source_provenance.is_some()
        })
        || project.preservation.is_some();
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
fn body_elements_json(items: &[Arc<BodyOwner>]) -> String {
    items
        .iter()
        .map(|owner| element_json(owner))
        .collect::<Vec<_>>()
        .join(",")
}
fn encode_envelope(
    p: &Project,
    legacy: &LegacyJsonState,
    legacy_title_page: &[ScreenplayElement],
) -> String {
    let layout_inputs = &legacy.layout_inputs;
    let role_boxes = layout_inputs
        .role_boxes
        .iter()
        .map(|box_| format!("{{\"x\":{},\"width\":{}}}", box_.x, box_.width))
        .collect::<Vec<_>>()
        .join(",");
    // Emit legacy identity/metadata only for an unchanged JSON-origin session.
    format!("{{\"schema_version\":{},\"project\":{{\"id\":{},\"schema_version\":{},\"metadata\":{{\"title\":{},\"author\":{},\"contact\":{},\"created_at\":{},\"updated_at\":{}}},\"title_page\":[{}],\"page_profile\":{{\"name\":{},\"format_version\":{},\"paper_width\":{},\"paper_height\":{},\"margin_top\":{},\"margin_bottom\":{},\"margin_left\":{},\"margin_right\":{},\"chars_per_line\":{},\"lines_per_page\":{},\"line_advance\":{},\"boundary_gap\":{},\"uppercase_cues\":true,\"auto_parenthetical\":true,\"strict_transition_boundaries\":true,\"font_family\":{},\"font_size\":{},\"char_width\":{},\"role_boxes\":[{}]}},\"elements\":[{}]}}}}", CURRENT_SCHEMA_VERSION, quote(legacy.project_id.as_str()), legacy.project_schema_version, quote(&legacy.metadata.title), quote(&legacy.metadata.author), quote(&legacy.metadata.contact), quote(&legacy.metadata.created_at), quote(&legacy.metadata.updated_at), elements_json(legacy_title_page), quote(&layout_inputs.name), layout_inputs.format_version, layout_inputs.paper_width, layout_inputs.paper_height, layout_inputs.margin_top, layout_inputs.margin_bottom, layout_inputs.margin_left, layout_inputs.margin_right, layout_inputs.chars_per_line, layout_inputs.lines_per_page, layout_inputs.line_advance, layout_inputs.boundary_gap, quote(&layout_inputs.font_family), layout_inputs.font_size, layout_inputs.char_width, role_boxes, body_elements_json(&p.elements))
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

fn ensure_known_unique_fields(v: &Json, known: &[&str]) -> Result<(), ModelError> {
    let Json::Object(fields) = v else {
        return Err(ModelError::InvalidJson);
    };
    let mut seen = BTreeSet::new();
    if fields
        .iter()
        .any(|(name, _)| !known.contains(&name.as_str()) || !seen.insert(name.as_str()))
    {
        return Err(ModelError::UnsupportedJsonState);
    }
    Ok(())
}

fn decode_element(v: &Json) -> Result<ScreenplayElement, ModelError> {
    ensure_known_unique_fields(v, &["id", "role", "text"])?;
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

fn decode_role_boxes(
    v: &Json,
) -> Result<[RoleBox; crate::layout_inputs::LEGACY_ROLE_BOX_COUNT], ModelError> {
    let Json::Array(items) = v else {
        return Err(ModelError::WrongType("role_boxes"));
    };
    if items.len() != crate::layout_inputs::LEGACY_ROLE_BOX_COUNT {
        return Err(ModelError::InvalidNumber);
    }
    let boxes = items
        .iter()
        .map(|item| {
            ensure_known_unique_fields(item, &["x", "width"])?;
            Ok(RoleBox {
                x: num_field(item, "x")?,
                width: num_field(item, "width")?,
            })
        })
        .collect::<Result<Vec<_>, ModelError>>()?;
    boxes.try_into().map_err(|_| ModelError::InvalidNumber)
}
fn decode_project(root: Json) -> Result<Project, ModelError> {
    ensure_known_unique_fields(&root, &["schema_version", "project"])?;
    let envelope_schema = num_field(&root, "schema_version")?;
    if envelope_schema != CURRENT_SCHEMA_VERSION {
        return Err(ModelError::InvalidSchemaVersion);
    }
    let envelope = obj(&root, "project")?;
    ensure_known_unique_fields(
        envelope,
        &[
            "id",
            "schema_version",
            "metadata",
            "title_page",
            "page_profile",
            "elements",
        ],
    )?;
    if object_has_field(envelope, "preservation") {
        return Err(ModelError::UnsupportedJsonState);
    }
    let schema = num_field(envelope, "schema_version")?;
    let m = obj(envelope, "metadata")?;
    ensure_known_unique_fields(
        m,
        &["title", "author", "contact", "created_at", "updated_at"],
    )?;
    let profile = obj(envelope, "page_profile")?;
    ensure_known_unique_fields(
        profile,
        &[
            "name",
            "format_version",
            "paper_width",
            "paper_height",
            "margin_top",
            "margin_bottom",
            "margin_left",
            "margin_right",
            "chars_per_line",
            "lines_per_page",
            "line_advance",
            "boundary_gap",
            "uppercase_cues",
            "auto_parenthetical",
            "strict_transition_boundaries",
            "font_family",
            "font_size",
            "char_width",
            "role_boxes",
            "role_print_visibility",
            "role_starts_new_page",
            "additional_role_boxes",
        ],
    )?;
    if object_has_field(profile, "role_print_visibility")
        || object_has_field(profile, "role_starts_new_page")
    {
        return Err(ModelError::UnsupportedJsonState);
    }
    if !optional_bool_field(profile, "uppercase_cues", true)?
        || !optional_bool_field(profile, "auto_parenthetical", true)?
        || !optional_bool_field(profile, "strict_transition_boundaries", true)?
    {
        return Err(ModelError::UnsupportedJsonState);
    }
    let layout_inputs = LayoutInputs {
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
        font_family: str_field(profile, "font_family")?,
        font_size: num_field(profile, "font_size")?,
        char_width: num_field(profile, "char_width")?,
        role_boxes: decode_role_boxes(obj(profile, "role_boxes")?)?,
        additional_role_boxes: if object_has_field(profile, "additional_role_boxes") {
            return Err(ModelError::UnsupportedJsonState);
        } else {
            [None; crate::layout_inputs::ADDITIONAL_ROLE_BOX_COUNT]
        },
    };
    let legacy_json_state = LegacyJsonState {
        project_id: StableId::new(str_field(envelope, "id")?)?,
        project_schema_version: schema,
        metadata: LegacyJsonMetadata {
            title: str_field(m, "title")?,
            author: str_field(m, "author")?,
            contact: str_field(m, "contact")?,
            created_at: str_field(m, "created_at")?,
            updated_at: str_field(m, "updated_at")?,
        },
        layout_inputs,
    };
    Ok(Project {
        title_page: decode_elements(obj(envelope, "title_page")?)?
            .into_iter()
            .map(TitlePageParagraph::from_legacy_element)
            .collect::<Result<Vec<_>, _>>()?,
        elements: decode_elements(obj(envelope, "elements")?)?
            .into_iter()
            .map(BodyOwner::shared)
            .collect(),
        dual_dialogue_groups: Vec::new(),
        dual_dialogue_paragraph_overrides: std::collections::BTreeMap::new(),
        direct_paragraph_overrides: std::collections::BTreeMap::new(),
        preservation: None,
        legacy_json_state: Some(legacy_json_state),
        layout_identity: Arc::new(()),
    })
}
