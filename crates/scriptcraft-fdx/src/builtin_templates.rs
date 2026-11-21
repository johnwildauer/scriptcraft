//! Compile-time built-in template catalog and common full-document loader.

use crate::diagnostics::FdxError;
use crate::document::FdxDocument;

const SCREENPLAY_BYTES: &[u8] = include_bytes!("../resources/Screenplay.fdxt");
const STAGEPLAY_BYTES: &[u8] = include_bytes!("../resources/Stageplay.fdxt");
const DRAMATISTS_GUILD_MUSICAL_BYTES: &[u8] =
    include_bytes!("../resources/Dramatists Guild Musical.fdxt");

/// Stable identity for one of the deliberately closed built-in template choices.
/// This catalog identity does not restrict document element-type keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltinTemplateId {
    Screenplay,
    Stageplay,
    DramatistsGuildMusical,
}

impl BuiltinTemplateId {
    /// Returns the stable bounded identifier used by catalog consumers.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Screenplay => "screenplay",
            Self::Stageplay => "stageplay",
            Self::DramatistsGuildMusical => "dramatists-guild-musical",
        }
    }

    /// Resolves an exact stable catalog identifier.
    pub fn from_stable_id(value: &str) -> Option<Self> {
        match value {
            "screenplay" => Some(Self::Screenplay),
            "stageplay" => Some(Self::Stageplay),
            "dramatists-guild-musical" => Some(Self::DramatistsGuildMusical),
            _ => None,
        }
    }
}

/// Presentation metadata for one built-in template, in deterministic catalog order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuiltinTemplateMetadata {
    pub id: BuiltinTemplateId,
    pub display_name: &'static str,
    pub description: &'static str,
    pub order: u8,
}

/// Stable identity selected for default new-document creation.
pub const DEFAULT_BUILTIN_TEMPLATE_ID: BuiltinTemplateId = BuiltinTemplateId::Screenplay;

const BUILTIN_TEMPLATES: [BuiltinTemplateMetadata; 3] = [
    BuiltinTemplateMetadata {
        id: BuiltinTemplateId::Screenplay,
        display_name: "Screenplay",
        description: "The industry standard.",
        order: 0,
    },
    BuiltinTemplateMetadata {
        id: BuiltinTemplateId::Stageplay,
        display_name: "Stageplay",
        description:
            "An alternative for modern plays with looser dialogue margins in Times New Roman.",
        order: 1,
    },
    BuiltinTemplateMetadata {
        id: BuiltinTemplateId::DramatistsGuildMusical,
        display_name: "Dramatists Guild Musical",
        description: "The Guild's modern format for musicals in Times New Roman.",
        order: 2,
    },
];

/// Returns the sole ordered catalog of built-in template presentation metadata.
pub fn builtin_templates() -> &'static [BuiltinTemplateMetadata] {
    &BUILTIN_TEMPLATES
}

/// Looks up deterministic presentation metadata by stable template identity.
pub fn builtin_template_metadata(id: BuiltinTemplateId) -> &'static BuiltinTemplateMetadata {
    // Each enum identity has exactly one entry in the compile-time catalog.
    match id {
        BuiltinTemplateId::Screenplay => &BUILTIN_TEMPLATES[0],
        BuiltinTemplateId::Stageplay => &BUILTIN_TEMPLATES[1],
        BuiltinTemplateId::DramatistsGuildMusical => &BUILTIN_TEMPLATES[2],
    }
}

/// Loads a fresh full document through the same bounded Version 6 reader used
/// for ordinary source-backed FDX documents.
pub fn instantiate_builtin_template(id: BuiltinTemplateId) -> Result<FdxDocument, FdxError> {
    crate::reader::read_v6(template_bytes(id))
}

/// Shared by required-base completion so Screenplay has one byte owner.
pub(crate) const fn screenplay_bytes() -> &'static [u8] {
    SCREENPLAY_BYTES
}

fn template_bytes(id: BuiltinTemplateId) -> &'static [u8] {
    match id {
        BuiltinTemplateId::Screenplay => SCREENPLAY_BYTES,
        BuiltinTemplateId::Stageplay => STAGEPLAY_BYTES,
        BuiltinTemplateId::DramatistsGuildMusical => DRAMATISTS_GUILD_MUSICAL_BYTES,
    }
}

