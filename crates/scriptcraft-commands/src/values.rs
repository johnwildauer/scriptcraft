use scriptcraft_model::{
    element_type::ElementTypeKey,
    formatted_text::{InlineColor, InlineFont, InlineSize, RunStyle},
    formatting::{ParagraphAlignment, ParagraphFormattingPatch, ParagraphLeading, ParagraphSpacing},
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
pub enum InlineColorValue { ElementDefault, Black, White, Red, Green, Blue }

impl<'de> Deserialize<'de> for InlineColorValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        match String::deserialize(deserializer)?.as_str() {
            "element_default" => Ok(Self::ElementDefault), "black" => Ok(Self::Black),
            "white" => Ok(Self::White), "red" => Ok(Self::Red), "green" => Ok(Self::Green),
            "blue" => Ok(Self::Blue), value => Err(de::Error::custom(format!("unsupported inline color: {value}"))),
        }
    }
}

impl InlineColorValue {
    pub const fn as_core(self) -> Option<InlineColor> {
        match self {
            Self::ElementDefault => None, Self::Black => Some(InlineColor::Black),
            Self::White => Some(InlineColor::White), Self::Red => Some(InlineColor::Red),
            Self::Green => Some(InlineColor::Green), Self::Blue => Some(InlineColor::Blue),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunFormattingPatchValue {
    #[serde(default)] pub font: Option<InlineFontValue>,
    #[serde(default)] pub size: Option<InlineSizeValue>,
    #[serde(default)] pub color: Option<InlineColorValue>,
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
pub enum ParagraphLeadingValue { Regular, Tight, Loose }
impl<'de> Deserialize<'de> for ParagraphLeadingValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        match String::deserialize(deserializer)?.as_str() {
            "regular" => Ok(Self::Regular), "tight" => Ok(Self::Tight), "loose" => Ok(Self::Loose),
            value => Err(de::Error::custom(format!("unknown paragraph leading: {value}"))),
        }
    }
}
impl ParagraphLeadingValue {
    pub const fn as_core(self) -> ParagraphLeading {
        match self { Self::Regular => ParagraphLeading::Regular, Self::Tight => ParagraphLeading::Tight, Self::Loose => ParagraphLeading::Loose }
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
