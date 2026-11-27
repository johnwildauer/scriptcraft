use scriptcraft_model::{
    element_type::ElementTypeKey,
    formatted_text::{InlineColor, InlineFont, InlineSize, RunStyle},
    formatting::{ParagraphAlignment, ParagraphFormattingPatch, ParagraphLeading, ParagraphSpacing},
    ModelError,
};
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BeatCardIdValue(scriptcraft_model::StableId);
impl BeatCardIdValue {
    pub fn new(value: impl Into<String>) -> Result<Self, ModelError> { scriptcraft_model::StableId::new(value).map(Self) }
    pub fn from_core(value: scriptcraft_model::StableId) -> Self { Self(value) }
    pub fn as_core(&self) -> &scriptcraft_model::StableId { &self.0 }
}

/// Durable Story Map lane identity accepted by the command boundary.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StoryMapLaneIdValue(String);

impl StoryMapLaneIdValue {
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidStoryMapLaneId> {
        let value = value.into();
        if value.is_empty()
            || value.len() > scriptcraft_model::MAX_STORY_MAP_STRING_BYTES
            || !value.chars().all(is_xml_compatible_story_map_character)
        {
            return Err(InvalidStoryMapLaneId);
        }
        Ok(Self(value))
    }

    pub fn from_core(value: &str) -> Result<Self, InvalidStoryMapLaneId> {
        Self::new(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_core(self) -> String {
        self.0
    }
}

impl Serialize for StoryMapLaneIdValue {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for StoryMapLaneIdValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(String::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidStoryMapLaneId;

impl std::fmt::Display for InvalidStoryMapLaneId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Story Map lane ID must be nonempty, bounded, and XML-compatible")
    }
}

impl std::error::Error for InvalidStoryMapLaneId {}

fn is_xml_compatible_story_map_character(character: char) -> bool {
    matches!(character, '\u{9}' | '\u{A}' | '\u{D}')
        || ('\u{20}'..='\u{D7FF}').contains(&character)
        || ('\u{E000}'..='\u{FFFD}').contains(&character)
        || ('\u{10000}'..='\u{10FFFF}').contains(&character)
}

const MAX_STORY_MAP_COMMAND_EIGHTHS: u32 =
    (scriptcraft_model::MAX_STORY_MAP_TARGET_PAGES as u64
        * scriptcraft_model::STORY_MAP_EIGHTHS_PER_PAGE) as u32;

/// Nonnegative, target-bounded primary start position in exact eighths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct StoryMapStartEighthsValue(u32);

impl StoryMapStartEighthsValue {
    pub const fn new(value: u32) -> Option<Self> {
        if value <= MAX_STORY_MAP_COMMAND_EIGHTHS { Some(Self(value)) } else { None }
    }

    pub const fn get(self) -> u32 { self.0 }
}

impl<'de> Deserialize<'de> for StoryMapStartEighthsValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = u32::deserialize(deserializer)?;
        Self::new(value).ok_or_else(|| de::Error::custom("Story Map start eighths exceed the supported target bound"))
    }
}

/// Positive, target-bounded primary length in exact eighths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct StoryMapLengthEighthsValue(u32);

impl StoryMapLengthEighthsValue {
    pub const fn new(value: u32) -> Option<Self> {
        if value > 0 && value <= MAX_STORY_MAP_COMMAND_EIGHTHS { Some(Self(value)) } else { None }
    }

    pub const fn get(self) -> u32 { self.0 }
}

impl<'de> Deserialize<'de> for StoryMapLengthEighthsValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = u32::deserialize(deserializer)?;
        Self::new(value).ok_or_else(|| de::Error::custom("Story Map length eighths must be positive and within the supported target bound"))
    }
}

/// Stable ScriptNote identity accepted by the command wire format.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ScriptNoteIdValue(scriptcraft_model::StableId);

impl ScriptNoteIdValue {
    pub fn new(value: impl Into<String>) -> Result<Self, ModelError> {
        scriptcraft_model::StableId::new(value).map(Self)
    }

    pub fn from_core(value: scriptcraft_model::StableId) -> Self { Self(value) }
    pub fn as_core(&self) -> &scriptcraft_model::StableId { &self.0 }
}

impl Serialize for ScriptNoteIdValue {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.0.as_str())
    }
}

impl<'de> Deserialize<'de> for ScriptNoteIdValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(String::deserialize(deserializer)?).map_err(|_| de::Error::custom("invalid ScriptNote ID"))
    }
}

/// ScriptNote property colors, plus the wire-only theme-default sentinel.
/// White and ElementDefault remain intentionally excluded from explicit colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptNoteColorValue {
    Black, Pink, Red, Orange, Yellow, Olive, Green, Mint, Blue, Violet, Purple, Brown, Gray,
    #[serde(rename = "none")]
    ThemeDefault,
}

impl ScriptNoteColorValue {
    pub const fn as_core(self) -> Option<InlineColor> {
        match self {
            Self::Black => Some(InlineColor::Black), Self::Pink => Some(InlineColor::Pink),
            Self::Red => Some(InlineColor::Red), Self::Orange => Some(InlineColor::Orange),
            Self::Yellow => Some(InlineColor::Yellow), Self::Olive => Some(InlineColor::Olive),
            Self::Green => Some(InlineColor::Green), Self::Mint => Some(InlineColor::Mint),
            Self::Blue => Some(InlineColor::Blue), Self::Violet => Some(InlineColor::Violet),
            Self::Purple => Some(InlineColor::Purple), Self::Brown => Some(InlineColor::Brown),
            Self::Gray => Some(InlineColor::Gray),
            Self::ThemeDefault => None,
        }
    }

    pub const fn from_core(color: InlineColor) -> Option<Self> {
        match color {
            InlineColor::Black => Some(Self::Black), InlineColor::Pink => Some(Self::Pink),
            InlineColor::Red => Some(Self::Red), InlineColor::Orange => Some(Self::Orange),
            InlineColor::Yellow => Some(Self::Yellow), InlineColor::Olive => Some(Self::Olive),
            InlineColor::Green => Some(Self::Green), InlineColor::Mint => Some(Self::Mint),
            InlineColor::Blue => Some(Self::Blue), InlineColor::Violet => Some(Self::Violet),
            InlineColor::Purple => Some(Self::Purple), InlineColor::Brown => Some(Self::Brown),
            InlineColor::Gray => Some(Self::Gray), InlineColor::White => None,
        }
    }
}

/// Maximum Type string accepted at the command boundary.
pub const MAX_SCRIPT_NOTE_TYPE_BYTES: usize = 1024;
/// Maximum text payload accepted by a single ScriptNote text command.
pub const MAX_SCRIPT_NOTE_TEXT_BYTES: usize = 1_048_576;

fn validate_script_note_type(value: Option<String>) -> Result<Option<String>, &'static str> {
    if let Some(value) = &value {
        if value.is_empty() || value.len() > MAX_SCRIPT_NOTE_TYPE_BYTES
            || value.chars().any(char::is_control)
        {
            return Err("ScriptNote Type must be nonempty, bounded, and contain no control characters");
        }
    }
    Ok(value)
}

fn deserialize_script_note_type<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<String>, D::Error> {
    validate_script_note_type(Option::<String>::deserialize(deserializer)?)
        .map_err(de::Error::custom)
}

/// A bounded local-wall-clock timestamp in YYYYMMDDTHHMMSS form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct ScriptNoteTimestampValue(String);

impl ScriptNoteTimestampValue {
    pub fn new(value: impl Into<String>) -> Result<Self, InvalidScriptNoteTimestamp> {
        let value = value.into();
        let bytes = value.as_bytes();
        let digits = |range: std::ops::Range<usize>| -> Option<u32> {
            bytes.get(range)?.iter().try_fold(0_u32, |number, byte| {
                byte.is_ascii_digit().then(|| number * 10 + u32::from(*byte - b'0'))
            })
        };
        let valid = bytes.len() == 15 && bytes.get(8) == Some(&b'T')
            && bytes.iter().enumerate().all(|(index, byte)| index == 8 || byte.is_ascii_digit())
            && digits(4..6).is_some_and(|month| (1..=12).contains(&month))
            && digits(6..8).is_some_and(|day| (1..=31).contains(&day))
            && digits(9..11).is_some_and(|hour| hour <= 23)
            && digits(11..13).is_some_and(|minute| minute <= 59)
            && digits(13..15).is_some_and(|second| second <= 59);
        valid.then_some(Self(value)).ok_or(InvalidScriptNoteTimestamp)
    }

    pub fn as_str(&self) -> &str { &self.0 }
}

impl<'de> Deserialize<'de> for ScriptNoteTimestampValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(String::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidScriptNoteTimestamp;

impl std::fmt::Display for InvalidScriptNoteTimestamp {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("timestamp must have bounded YYYYMMDDTHHMMSS fields")
    }
}
impl std::error::Error for InvalidScriptNoteTimestamp {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptNoteCreateValue {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<ScriptNoteColorValue>,
    #[serde(default, deserialize_with = "deserialize_script_note_type")]
    pub note_type: Option<String>,
    pub timestamp: ScriptNoteTimestampValue,
}

impl ScriptNoteCreateValue {
    pub fn validate(&self) -> Result<(), &'static str> {
        validate_script_note_type(self.note_type.clone()).map(|_| ())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptNotePropertiesValue {
    pub note: ScriptNoteIdValue,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<ScriptNoteColorValue>,
    #[serde(default, deserialize_with = "deserialize_script_note_type")]
    pub note_type: Option<String>,
    pub modified: ScriptNoteTimestampValue,
}

impl ScriptNotePropertiesValue {
    pub fn validate(&self) -> Result<(), &'static str> {
        validate_script_note_type(self.note_type.clone()).map(|_| ())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptNoteTextPositionValue { pub paragraph: u32, pub offset: u32 }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptNoteFieldValue { Title, Body }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ScriptNoteSelectionValue {
    None,
    Text {
        note: ScriptNoteIdValue,
        field: ScriptNoteFieldValue,
        anchor: ScriptNoteTextPositionValue,
        focus: ScriptNoteTextPositionValue,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum ScriptNoteTextOperationValue {
    Insert { text: String }, Paste { text: String }, DeleteBackward, DeleteForward, Enter,
}

impl ScriptNoteTextOperationValue {
    pub fn validate(&self) -> Result<(), &'static str> {
        let text = match self { Self::Insert { text } | Self::Paste { text } => text, _ => return Ok(()) };
        if text.len() > MAX_SCRIPT_NOTE_TEXT_BYTES { Err("ScriptNote text payload exceeds its byte bound") } else { Ok(()) }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ScriptNoteNavigationValue {
    Horizontal { direction: HorizontalDirection },
    LineEdge { edge: LineEdge },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptNoteTextEditValue {
    pub operation: ScriptNoteTextOperationValue,
    pub modified: ScriptNoteTimestampValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptNotePresentValue {
    pub note: ScriptNoteIdValue,
    pub move_caret: bool,
    pub focus_body: bool,
}
impl Serialize for BeatCardIdValue {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> { serializer.serialize_str(self.0.as_str()) }
}
impl<'de> Deserialize<'de> for BeatCardIdValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(String::deserialize(deserializer)?).map_err(|_| de::Error::custom("invalid Beat card ID"))
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BeatPointValue { pub x: i64, pub y: i64 }
impl BeatPointValue {
    pub fn to_core(self) -> scriptcraft_model::BeatPoint { scriptcraft_model::BeatPoint { x: scriptcraft_model::BoardFixed::from_milli_pixels(self.x), y: scriptcraft_model::BoardFixed::from_milli_pixels(self.y) } }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BeatRectValue { pub left: i64, pub top: i64, pub width: i64, pub height: i64 }
impl BeatRectValue {
    pub fn to_core(self) -> scriptcraft_model::BeatRect { scriptcraft_model::BeatRect { left: scriptcraft_model::BoardFixed::from_milli_pixels(self.left), top: scriptcraft_model::BoardFixed::from_milli_pixels(self.top), width: scriptcraft_model::BoardFixed::from_milli_pixels(self.width), height: scriptcraft_model::BoardFixed::from_milli_pixels(self.height) } }
    pub fn from_core(rect: scriptcraft_model::BeatRect) -> Self { Self { left: rect.left.raw(), top: rect.top.raw(), width: rect.width.raw(), height: rect.height.raw() } }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BeatSelectionValue {
    None,
    Card { card: BeatCardIdValue },
    Text { card: BeatCardIdValue, field: BeatTextFieldValue, anchor: BeatTextPositionValue, focus: BeatTextPositionValue },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BeatTextFieldValue { Title, Body }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BeatTextPositionValue { pub paragraph: u32, pub offset: u32 }

impl BeatTextPositionValue {
    pub fn to_core(self) -> Result<scriptcraft_session::editing::BeatTextPosition, crate::PositionError> {
        Ok(scriptcraft_session::editing::BeatTextPosition {
            paragraph: usize::try_from(self.paragraph).map_err(|_| crate::PositionError::IndexOutOfRange)?,
            offset: usize::try_from(self.offset).map_err(|_| crate::PositionError::IndexOutOfRange)?,
        })
    }
    pub fn from_core(value: scriptcraft_session::editing::BeatTextPosition) -> Result<Self, crate::PositionError> {
        Ok(Self { paragraph: u32::try_from(value.paragraph).map_err(|_| crate::PositionError::IndexOutOfRange)?,
            offset: u32::try_from(value.offset).map_err(|_| crate::PositionError::IndexOutOfRange)? })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum BeatTextEditValue { Insert { text: String }, Paste { text: String }, DeleteBackward, DeleteForward, Enter }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BeatNavigationValue { Horizontal { direction: HorizontalDirection }, LineEdge { edge: LineEdge } }

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InlineFontValue {
    ElementDefault,
    CourierFinalDraft,
    CourierPrime,
    Arial,
    TimesNewRoman,
}

impl<'de> Deserialize<'de> for InlineFontValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        match String::deserialize(deserializer)?.as_str() {
            "element_default" => Ok(Self::ElementDefault),
            "courier_final_draft" => Ok(Self::CourierFinalDraft),
            "courier_prime" => Ok(Self::CourierPrime),
            "arial" => Ok(Self::Arial),
            "times_new_roman" => Ok(Self::TimesNewRoman),
            value => Err(de::Error::custom(format!("unsupported inline font: {value}"))),
        }
    }
}

impl InlineFontValue {
    pub const fn as_core(self) -> Option<InlineFont> {
        match self {
            Self::ElementDefault => None,
            Self::CourierFinalDraft => Some(InlineFont::CourierFinalDraft),
            Self::CourierPrime => Some(InlineFont::CourierPrime),
            Self::Arial => Some(InlineFont::Arial),
            Self::TimesNewRoman => Some(InlineFont::TimesNewRoman),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineSizeValue { Seven, Eight, Nine, Ten, Twelve, Fourteen, Eighteen, TwentyFour }

impl Serialize for InlineSizeValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        serializer.serialize_u8(self.as_core().points())
    }
}

impl<'de> Deserialize<'de> for InlineSizeValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        match u8::deserialize(deserializer)? {
            7 => Ok(Self::Seven), 8 => Ok(Self::Eight), 9 => Ok(Self::Nine), 10 => Ok(Self::Ten),
            12 => Ok(Self::Twelve), 14 => Ok(Self::Fourteen), 18 => Ok(Self::Eighteen), 24 => Ok(Self::TwentyFour),
            value => Err(de::Error::custom(format!("unsupported inline size: {value}"))),
        }
    }
}

impl InlineSizeValue {
    pub const fn as_core(self) -> InlineSize {
        match self {
            Self::Seven => InlineSize::Seven, Self::Eight => InlineSize::Eight,
            Self::Nine => InlineSize::Nine, Self::Ten => InlineSize::Ten,
            Self::Twelve => InlineSize::Twelve, Self::Fourteen => InlineSize::Fourteen,
            Self::Eighteen => InlineSize::Eighteen, Self::TwentyFour => InlineSize::TwentyFour,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InlineColorValue {
    ElementDefault,
    Black,
    White,
    Pink,
    Red,
    Orange,
    Yellow,
    Olive,
    Green,
    Mint,
    Blue,
    Violet,
    Purple,
    Brown,
    Gray,
}

impl<'de> Deserialize<'de> for InlineColorValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        match String::deserialize(deserializer)?.as_str() {
            "element_default" => Ok(Self::ElementDefault), "black" => Ok(Self::Black),
            "white" => Ok(Self::White), "pink" => Ok(Self::Pink), "red" => Ok(Self::Red),
            "orange" => Ok(Self::Orange), "yellow" => Ok(Self::Yellow), "olive" => Ok(Self::Olive),
            "green" => Ok(Self::Green), "mint" => Ok(Self::Mint), "blue" => Ok(Self::Blue),
            "violet" => Ok(Self::Violet), "purple" => Ok(Self::Purple), "brown" => Ok(Self::Brown),
            "gray" => Ok(Self::Gray), value => Err(de::Error::custom(format!("unsupported inline color: {value}"))),
        }
    }
}

impl InlineColorValue {
    pub const fn as_core(self) -> Option<InlineColor> {
        match self {
            Self::ElementDefault => None, Self::Black => Some(InlineColor::Black),
            Self::White => Some(InlineColor::White), Self::Pink => Some(InlineColor::Pink),
            Self::Red => Some(InlineColor::Red), Self::Orange => Some(InlineColor::Orange),
            Self::Yellow => Some(InlineColor::Yellow), Self::Olive => Some(InlineColor::Olive),
            Self::Green => Some(InlineColor::Green), Self::Mint => Some(InlineColor::Mint),
            Self::Blue => Some(InlineColor::Blue), Self::Violet => Some(InlineColor::Violet),
            Self::Purple => Some(InlineColor::Purple), Self::Brown => Some(InlineColor::Brown),
            Self::Gray => Some(InlineColor::Gray),
        }
    }

    pub const fn from_core(color: InlineColor) -> Self {
        match color {
            InlineColor::Black => Self::Black,
            InlineColor::White => Self::White,
            InlineColor::Pink => Self::Pink,
            InlineColor::Red => Self::Red,
            InlineColor::Orange => Self::Orange,
            InlineColor::Yellow => Self::Yellow,
            InlineColor::Olive => Self::Olive,
            InlineColor::Green => Self::Green,
            InlineColor::Mint => Self::Mint,
            InlineColor::Blue => Self::Blue,
            InlineColor::Violet => Self::Violet,
            InlineColor::Purple => Self::Purple,
            InlineColor::Brown => Self::Brown,
            InlineColor::Gray => Self::Gray,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunFormattingPatchValue {
    #[serde(default)] pub font: Option<InlineFontValue>,
    #[serde(default)] pub size: Option<InlineSizeValue>,
    #[serde(default)] pub color: Option<InlineColorValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub background: Option<InlineColorValue>,
    #[serde(default, skip_serializing_if = "is_false")] pub clear_size: bool,
}

impl<'de> Deserialize<'de> for RunFormattingPatchValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            #[serde(default)] font: Option<InlineFontValue>,
            #[serde(default)] size: Option<InlineSizeValue>,
            #[serde(default)] color: Option<InlineColorValue>,
            #[serde(default)] background: Option<InlineColorValue>,
            #[serde(default)] clear_size: bool,
        }

        let fields = Fields::deserialize(deserializer)?;
        if fields.size.is_some() && fields.clear_size {
            return Err(de::Error::custom("size and clear_size are mutually exclusive"));
        }
        Ok(Self {
            font: fields.font,
            size: fields.size,
            color: fields.color,
            background: fields.background,
            clear_size: fields.clear_size,
        })
    }
}

const fn is_false(value: &bool) -> bool { !*value }

impl RunFormattingPatchValue {
    pub const fn as_core(self) -> scriptcraft_model::formatted_text::InlineRunFormattingPatch {
        scriptcraft_model::formatted_text::InlineRunFormattingPatch {
            font: match self.font { Some(value) => Some(value.as_core()), None => None },
            size: if self.clear_size { Some(None) } else { match self.size { Some(value) => Some(Some(value.as_core())), None => None } },
            color: match self.color { Some(value) => Some(value.as_core()), None => None },
            background: match self.background { Some(value) => Some(value.as_core()), None => None },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ParagraphSpacingValue { Single, OneAndHalf, Double }
impl<'de> Deserialize<'de> for ParagraphSpacingValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        match String::deserialize(deserializer)?.as_str() {
            "single" => Ok(Self::Single), "one_and_half" => Ok(Self::OneAndHalf), "double" => Ok(Self::Double),
            value => Err(de::Error::custom(format!("unknown paragraph spacing: {value}"))),
        }
    }
}
impl ParagraphSpacingValue {
    pub const fn as_core(self) -> ParagraphSpacing {
        match self { Self::Single => ParagraphSpacing::Single, Self::OneAndHalf => ParagraphSpacing::OneAndHalf, Self::Double => ParagraphSpacing::Double }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ParagraphLeadingValue { VeryTight, Regular, Tight, Loose }
impl<'de> Deserialize<'de> for ParagraphLeadingValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        match String::deserialize(deserializer)?.as_str() {
            "very_tight" => Ok(Self::VeryTight), "regular" => Ok(Self::Regular), "tight" => Ok(Self::Tight), "loose" => Ok(Self::Loose),
            value => Err(de::Error::custom(format!("unknown paragraph leading: {value}"))),
        }
    }
}
impl ParagraphLeadingValue {
    pub const fn as_core(self) -> ParagraphLeading {
        match self { Self::VeryTight => ParagraphLeading::VeryTight, Self::Regular => ParagraphLeading::Regular, Self::Tight => ParagraphLeading::Tight, Self::Loose => ParagraphLeading::Loose }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct SpaceBeforeLines(u8);
impl SpaceBeforeLines {
    pub const fn new(value: u8) -> Option<Self> {
        if value <= 3 { Some(Self(value)) } else { None }
    }

    pub const fn get(self) -> u8 { self.0 }
}
impl<'de> Deserialize<'de> for SpaceBeforeLines {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        let value = u8::deserialize(deserializer)?;
        if value <= 3 { Ok(Self(value)) } else { Err(de::Error::custom("space-before lines must be 0..=3")) }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParagraphFormattingPatchValue {
    #[serde(default)] pub spacing: Option<ParagraphSpacingValue>,
    #[serde(default)] pub space_before_lines: Option<SpaceBeforeLines>,
    #[serde(default)] pub leading: Option<ParagraphLeadingValue>,
}
impl ParagraphFormattingPatchValue {
    pub fn as_core(self) -> ParagraphFormattingPatch {
        ParagraphFormattingPatch {
            spacing: match self.spacing { Some(value) => Some(value.as_core()), None => None },
            space_before_lines: match self.space_before_lines { Some(value) => Some(value.get()), None => None },
            leading: match self.leading { Some(value) => Some(value.as_core()), None => None },
            ..ParagraphFormattingPatch::default()
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
