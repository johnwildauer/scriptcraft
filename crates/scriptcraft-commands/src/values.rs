use scriptcraft_model::{
    element_type::ElementTypeKey, formatted_text::RunStyle, formatting::ParagraphAlignment,
    ModelError,
};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

/// Owned, exact element-type key carried by command 22.
///
/// This value is open: it preserves arbitrary labels and case distinctions and
/// never normalizes or interprets a key through a closed vocabulary.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ElementTypeValue(ElementTypeKey);

impl ElementTypeValue {
    pub fn new(value: impl Into<String>) -> Result<Self, ModelError> {
        ElementTypeKey::new(value).map(Self)
    }

    pub fn from_key(key: ElementTypeKey) -> Self {
        Self(key)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn as_key(&self) -> &ElementTypeKey {
        &self.0
    }

    pub fn into_key(self) -> ElementTypeKey {
        self.0
    }

    pub fn into_string(self) -> String {
        self.0.into_string()
    }
}

impl Serialize for ElementTypeValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ElementTypeValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(|_| de::Error::custom("invalid bounded element-type key"))
    }
}

/// Closed identity for the packaged built-in template catalog. This value is
/// deliberately distinct from open document element-type keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BuiltinTemplateValue {
    #[serde(rename = "screenplay")]
    Screenplay,
    #[serde(rename = "stageplay")]
    Stageplay,
    #[serde(rename = "dramatists-guild-musical")]
    DramatistsGuildMusical,
}

impl BuiltinTemplateValue {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Screenplay => "screenplay",
            Self::Stageplay => "stageplay",
            Self::DramatistsGuildMusical => "dramatists-guild-musical",
        }
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
