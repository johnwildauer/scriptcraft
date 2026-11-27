#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

//! Canonical screenplay values shared by document and codec packages.

pub mod beat_board;
pub mod body_offsets;
pub mod cast_list;
pub mod dialogue_alts;
pub mod dual_dialogue;
pub mod element;
pub mod element_definition;
pub mod element_definitions;
pub mod element_semantics;
pub mod element_type;
pub mod error;
pub mod formatted_text;
pub mod formatting;
pub mod identity;
pub mod layout_inputs;
pub mod paragraph_id;
pub mod position;
pub mod preservation;
pub mod project;
pub mod scene_metadata;
pub mod script_notes;
pub mod story_map;
pub mod title_page;
pub mod typography;
pub mod validation;

pub use beat_board::{
    BeatBoard, BeatCard, BeatParagraph, BeatPoint, BeatRect, BoardFixed, BOARD_SNAP_INCREMENT,
    DEFAULT_BOARD_HEIGHT, DEFAULT_BOARD_WIDTH, DEFAULT_CARD_HEIGHT, DEFAULT_CARD_WIDTH,
    MAX_BOARD_DIMENSION, MAX_CARD_HEIGHT, MAX_CARD_WIDTH, MIN_CARD_HEIGHT, MIN_CARD_WIDTH,
};
pub use body_offsets::{
    flat_document_len, flat_offset_to_position, paragraph_flat_start, position_to_flat_offset,
    selection_to_flat_offsets,
};
pub use dual_dialogue::DualDialogueError;
pub use dialogue_alts::{
    DialogueAltAddIds, DialogueAltDirection, DialogueAltEdit, DialogueAltError, DialogueAltId,
    DialogueAltIdParseError, DialogueAlternative, DialogueAlternatives,
};
pub use element::{
    BodyOwner, RunAttribute, ScreenplayElement, SourceFormat, SourceProvenance, SourceSection,
    TextRun,
};
pub use element_definition::{
    ElementBehavior, ElementDefinition, ElementFontSpec, ElementOutline, ElementParagraphSpec,
    SemanticCapabilities, MAX_ELEMENT_DEFINITION_STRING_BYTES,
};
pub use element_definitions::{DocumentElementDefinitions, MAX_DOCUMENT_ELEMENT_DEFINITIONS};
pub use element_semantics::{BehaviorTargetResolution, ElementSemantics, ParagraphBehaviorKey};
pub use element_type::{ElementTypeKey, MAX_ELEMENT_TYPE_KEY_BYTES};
pub use error::ModelError;
pub use formatting::{ParagraphFormattingPatch, ParagraphLeading, ParagraphSpacing};
pub use identity::{StableId, StableIdAllocator};
pub use layout_inputs::{DocumentLeading, DocumentPageLayout, LayoutInputs, PageLayoutInputs};
pub use position::{Position, Selection};
pub use project::Project;
pub use script_notes::{
    ScriptNote, ScriptNoteIdentity, ScriptNoteParagraph, ScriptNoteRange, ScriptNotes,
    DEFAULT_SCRIPT_NOTE_COLOR_RAW,
};
pub use story_map::{
    StoryMap, StoryMapLane, StoryMapPlacement, DEFAULT_STORY_MAP_TARGET_PAGES,
    MAX_STORY_MAP_LANES, MAX_STORY_MAP_STRING_BYTES, MAX_STORY_MAP_TARGET_PAGES,
    STORY_MAP_EIGHTHS_PER_PAGE,
};
pub use title_page::{TitlePageAlignment, TitlePageParagraph, TitlePageParagraphFormatting};
pub use typography::{
    DocumentFontEnvironment, EffectiveTextColor, EffectiveTextStyle, FaceStyle, FontResolutionKind,
    RequestedFontResolution, ResolvedDecorationMetrics, ResolvedFaceMetrics, ResolvedFontFace,
    ResolvedFontId, ResolvedFontSourceKind, ScalarAdvance, ScaledDecorationGeometry,
};
pub use validation::{validate_layout_inputs, MAX_ELEMENTS};

pub const MAX_TEXT_BYTES: usize = 262_144;
