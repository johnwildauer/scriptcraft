#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

//! Canonical screenplay values shared by document and codec packages.

pub mod dual_dialogue;
pub mod cast_list;
pub mod layout_inputs;
pub mod element;
pub mod formatting;
pub mod formatted_text;
pub mod error;
pub mod identity;
pub mod position;
pub mod paragraph_id;
pub mod project;
pub mod preservation;
pub mod scene_metadata;
pub mod title_page;
pub mod validation;
mod legacy_json;

pub use dual_dialogue::DualDialogueError;
pub use layout_inputs::{LayoutInputs, RoleBox};
pub use element::{
    BodyOwner, Role, RunAttribute, ScreenplayElement, SourceFormat, SourceProvenance,
    SourceSection, TextRun,
};
pub use error::ModelError;
pub use project::{NativeProjectEnvelope, Project};
pub use validation::{validate_layout_inputs, MAX_ELEMENTS};
pub use legacy_json::{CURRENT_SCHEMA_VERSION, MAX_JSON_BYTES, MAX_STRING_BYTES, POINTS_PER_INCH};
pub use identity::{StableId, StableIdAllocator};
pub use position::{Position, Selection};
pub use title_page::{TitlePageAlignment, TitlePageParagraph, TitlePageParagraphFormatting};

pub const FOUNDATION_STATUS: &str = "scriptcraft foundation ready";
pub const MAX_TEXT_BYTES: usize = 262_144;

pub const fn foundation_status() -> &'static str {
    FOUNDATION_STATUS
}
