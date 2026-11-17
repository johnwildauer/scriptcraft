#![forbid(unsafe_code)]
#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

//! Transitional facade for editing, storage, layout, export, and FDX.
//! FDX compatibility values remain exposed through `fdx` and `text_state` adapters.

pub mod cast_list;
pub mod dual_dialogue;
pub mod editing;
pub mod exports;
pub mod fdx;
pub mod fdx_storage;
pub mod formatted_text;
pub mod formatting;
pub mod layout;
mod paragraph_id;
pub mod persistence {
    pub use scriptcraft_session::persistence::*;
}
pub mod preservation;
pub mod scene_metadata;
pub mod title_page;
pub mod text_state;

pub use scriptcraft_model::{
    dual_dialogue::DualDialogueError,
    element::{
        BodyOwner, Role, RunAttribute, ScreenplayElement, SourceFormat, SourceProvenance,
        SourceSection, TextRun,
    },
    error::ModelError,
    identity::{StableId, StableIdAllocator},
    layout_inputs::{LayoutInputs, RoleBox},
    project::{NativeProjectEnvelope, Project},
    validation::{validate_layout_inputs, MAX_ELEMENTS},
    foundation_status, FOUNDATION_STATUS, MAX_TEXT_BYTES,
};
pub use scriptcraft_model::{
    CURRENT_SCHEMA_VERSION, MAX_JSON_BYTES, MAX_STRING_BYTES, POINTS_PER_INCH,
};
pub(crate) use scriptcraft_model::element::{
    MAX_PROVENANCE_STRING_BYTES, MAX_RUNS_PER_ELEMENT, MAX_RUN_ATTRIBUTES,
    MAX_RUN_ATTRIBUTE_BYTES,
};

pub use formatting::{
    EffectiveTextStyle, FixedPoint, FontFallback, ParagraphAlignment, PrintVisibility,
    SourceFontSpec, SourceLayoutSettings, SourceParagraphOverrides, SourceParagraphSpec,
};
pub use formatted_text::{
    clear_paragraph_run_styles, clear_supported_styles_for_range,
    clear_supported_styles_over_ranges, parse_style_attribute, project_lit_styles,
    project_paragraph_alignment, set_paragraph_alignments, styled_display_spans,
    toggle_paragraph_run_style, toggle_paragraph_run_styles, toggle_style_for_range,
    toggle_style_over_ranges, project_lit_styles_over_ranges, project_style_at_caret,
    DisplaySourceSegment, FormattedTextError, ParagraphAlignmentProjection,
    ParagraphTarget, RunStyle, RunStyleEdit, RunStyleFlags, RunBoundaryAffinity,
    RunStyleProjection, RunStyleRange, StyledDisplaySpan,
};
pub use title_page::{TitlePageAlignment, TitlePageParagraph, TitlePageParagraphFormatting};
