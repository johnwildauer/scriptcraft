#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

//! Canonical screenplay values shared by document and codec packages.

pub mod cast_list;
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
pub mod title_page;
pub mod typography;
pub mod validation;

pub use dual_dialogue::DualDialogueError;
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
pub use identity::{StableId, StableIdAllocator};
pub use layout_inputs::{DocumentLeading, DocumentPageLayout, LayoutInputs, PageLayoutInputs};
pub use position::{Position, Selection};
pub use project::Project;
pub use title_page::{TitlePageAlignment, TitlePageParagraph, TitlePageParagraphFormatting};
pub use typography::{
    DocumentFontEnvironment, EffectiveTextColor, EffectiveTextStyle, FaceStyle, FontResolutionKind,
    RequestedFontResolution, ResolvedDecorationMetrics, ResolvedFaceMetrics, ResolvedFontFace,
    ResolvedFontId, ResolvedFontSourceKind, ScalarAdvance, ScaledDecorationGeometry,
};
pub use validation::{validate_layout_inputs, MAX_ELEMENTS};

pub const MAX_TEXT_BYTES: usize = 262_144;
