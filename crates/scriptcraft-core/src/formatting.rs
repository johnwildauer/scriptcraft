//! Transitional compatibility re-exports for model formatting values.

pub use scriptcraft_model::formatting::{
    formatting_for_role, formatting_in_native_order, Behavior, EffectiveTextStyle, FixedPoint,
    FontFallback, FontSpec, Outline, ParagraphAlignment, ParagraphSpec, PrintVisibility,
    RoleFormatting, SourceFontSpec, SourceLayoutSettings, SourceParagraphOverrides,
    SourceParagraphSpec, ROLE_FORMATTING_COUNT, ROLE_FORMATTING_DEFAULTS,
};
pub(crate) use scriptcraft_model::formatting::role_index;
