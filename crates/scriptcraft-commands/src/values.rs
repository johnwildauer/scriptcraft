use scriptcraft_model::{element::Role, formatted_text::RunStyle, formatting::ParagraphAlignment};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

/// Phase 1 compatibility payload for command ID 22, serialized with the exact
/// stable role spelling. Phase 6 owns transition to open element-type commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoleValue(Role);

impl RoleValue {
    pub const fn new(role: Role) -> Self {
        Self(role)
    }

    pub const fn as_core(self) -> Role {
        self.0
    }

    pub const fn as_str(self) -> &'static str {
        self.0.as_str()
    }
}

impl Serialize for RoleValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for RoleValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Role::parse(&value)
            .map(Self::new)
            .ok_or_else(|| de::Error::custom(format!("unknown screenplay role: {value}")))
    }
}

/// Semantic editing surface. Body is the compatibility default for payloads
/// written before surface-aware commands were introduced.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EditorSurface {
    #[default]
    Body,
    TitlePage,
}

impl<'de> Deserialize<'de> for EditorSurface {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match String::deserialize(deserializer)?.as_str() {
            "body" => Ok(Self::Body),
            "title_page" => Ok(Self::TitlePage),
            value => Err(de::Error::custom(format!(
                "unknown editor surface: {value}"
            ))),
        }
    }
}

impl EditorSurface {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Body => "body",
            Self::TitlePage => "title_page",
        }
    }
}

/// Bounded, typed inline Text style accepted by title and body commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TitleTextStyle {
    Bold,
    Italics,
    Underline,
    AllCaps,
    Strikethrough,
}

impl<'de> Deserialize<'de> for TitleTextStyle {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match String::deserialize(deserializer)?.as_str() {
            "bold" => Ok(Self::Bold),
            "italics" => Ok(Self::Italics),
            "underline" => Ok(Self::Underline),
            "all_caps" => Ok(Self::AllCaps),
            "strikethrough" => Ok(Self::Strikethrough),
            value => Err(de::Error::custom(format!(
                "unknown title text style: {value}"
            ))),
        }
    }
}

impl TitleTextStyle {
    pub const fn as_core(self) -> RunStyle {
        match self {
            Self::Bold => RunStyle::Bold,
            Self::Italics => RunStyle::Italics,
            Self::Underline => RunStyle::Underline,
            Self::AllCaps => RunStyle::AllCaps,
            Self::Strikethrough => RunStyle::Strikethrough,
        }
    }
}

/// Bounded direct paragraph alignment value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ParagraphAlignmentValue {
    Left,
    Right,
    Center,
    Justified,
}

impl<'de> Deserialize<'de> for ParagraphAlignmentValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match String::deserialize(deserializer)?.as_str() {
            "left" => Ok(Self::Left),
            "right" => Ok(Self::Right),
            "center" => Ok(Self::Center),
            "justified" => Ok(Self::Justified),
            value => Err(de::Error::custom(format!(
                "unknown paragraph alignment: {value}"
            ))),
        }
    }
}

impl ParagraphAlignmentValue {
    pub const fn as_core(self) -> ParagraphAlignment {
        match self {
            Self::Left => ParagraphAlignment::Left,
            Self::Right => ParagraphAlignment::Right,
            Self::Center => ParagraphAlignment::Center,
            Self::Justified => ParagraphAlignment::Justified,
        }
    }
}

/// Transient PDF request options; never serialized into a native document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PdfExportOptions {
    pub include_title_page: bool,
}

impl Default for PdfExportOptions {
    fn default() -> Self {
        Self {
            include_title_page: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportResolution {
    Continue,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PointerPhase {
    Press,
    Drag,
    Release,
    DoubleClick,
    TripleClick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HorizontalDirection {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerticalDirection {
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineEdge {
    Start,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Appearance {
    System,
    Day,
    Night,
    Midnight,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptViewMode {
    Normal,
    #[default]
    Page,
    Speed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SprintTimerMode {
    Countdown,
    Stopwatch,
}
