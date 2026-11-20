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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn load(id: BuiltinTemplateId) -> Option<FdxDocument> {
        instantiate_builtin_template(id).ok()
    }

    #[test]
    fn catalog_has_exact_ids_metadata_default_and_order() {
        let entries = builtin_templates();
        assert_eq!(entries.len(), 3);
        assert_eq!(DEFAULT_BUILTIN_TEMPLATE_ID, BuiltinTemplateId::Screenplay);
        assert_eq!(
            entries.iter().map(|entry| entry.id).collect::<Vec<_>>(),
            vec![
                BuiltinTemplateId::Screenplay,
                BuiltinTemplateId::Stageplay,
                BuiltinTemplateId::DramatistsGuildMusical,
            ]
        );
        assert_eq!(
            entries.iter().map(|entry| entry.order).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        assert_eq!(entries[0].display_name, "Screenplay");
        assert_eq!(entries[0].description, "The industry standard.");
        assert_eq!(entries[1].display_name, "Stageplay");
        assert_eq!(
            entries[1].description,
            "An alternative for modern plays with looser dialogue margins in Times New Roman."
        );
        assert_eq!(entries[2].display_name, "Dramatists Guild Musical");
        assert_eq!(
            entries[2].description,
            "The Guild's modern format for musicals in Times New Roman."
        );
        for entry in entries {
            assert_eq!(
                BuiltinTemplateId::from_stable_id(entry.id.as_str()),
                Some(entry.id)
            );
            assert_eq!(builtin_template_metadata(entry.id), entry);
        }
        assert_eq!(BuiltinTemplateId::from_stable_id("Screenplay"), None);
        assert_eq!(BuiltinTemplateId::from_stable_id("unknown"), None);
    }

    #[test]
    fn each_embedded_resource_parses_as_fresh_source_backed_full_content() {
        for entry in builtin_templates() {
            let Some(first) = load(entry.id) else {
                assert!(
                    false,
                    "each embedded built-in resource must parse through read_v6"
                );
                return;
            };
            let Some(second) = load(entry.id) else {
                assert!(
                    false,
                    "each embedded built-in resource must parse independently"
                );
                return;
            };
            assert!(!first.project.elements.is_empty());
            assert!(!first.project.title_page.is_empty());
            assert!(first
                .project
                .elements
                .iter()
                .all(|owner| owner.source_provenance.is_some()));
            assert!(first
                .project
                .title_page
                .iter()
                .all(|paragraph| paragraph.source_provenance.is_some()));
            assert!(first.project.preservation.is_some());
            assert!(!Arc::ptr_eq(
                first.project.layout_identity(),
                second.project.layout_identity()
            ));

            let Some(preservation) = first.project.preservation.as_ref() else {
                assert!(false, "read_v6 must retain the embedded source snapshot");
                return;
            };
            assert_eq!(
                preservation.source.original_bytes.as_slice(),
                template_bytes(entry.id)
            );
            assert!(std::str::from_utf8(&preservation.source.original_bytes)
                .is_ok_and(|source| source.contains("Template=\"Yes\"")));
        }
    }

    #[test]
    fn common_reader_keeps_template_specific_arbitrary_types_and_fonts_editable() {
        let Some(stageplay) = load(BuiltinTemplateId::Stageplay) else {
            assert!(false, "Stageplay resource must parse");
            return;
        };
        assert!(stageplay.project.elements.iter().any(|owner| {
            owner.element_type().as_str() == "Notations" && owner.text.contains("DR. JUDITH BROWNE")
        }));
        let stage_action = stageplay
            .project
            .element_definitions()
            .get_by_label("Action");
        assert_eq!(
            stage_action.map(|definition| definition.font.family.as_str()),
            Some("Times New Roman")
        );

        let Some(musical) = load(BuiltinTemplateId::DramatistsGuildMusical) else {
            assert!(false, "Dramatists Guild Musical resource must parse");
            return;
        };
        assert!(musical.project.elements.iter().any(|owner| {
            owner.element_type().as_str() == "Lyrics"
                && owner.text.contains("Lyrics are in all caps")
        }));
        assert!(musical
            .project
            .elements
            .iter()
            .any(|owner| owner.element_type().as_str() == "Scene Citation"));
        let musical_action = musical.project.element_definitions().get_by_label("Action");
        assert_eq!(
            musical_action.map(|definition| definition.font.family.as_str()),
            Some("Times New Roman")
        );
    }

    #[test]
    fn screenplay_loader_retains_its_source_title_page_sequence() {
        let Some(screenplay) = load(BuiltinTemplateId::Screenplay) else {
            assert!(false, "Screenplay resource must parse");
            return;
        };
        assert!(!screenplay.project.elements.is_empty());
        assert!(screenplay.project.element_definitions().len() > 9);
        assert_eq!(screenplay.project.title_page.len(), 54);
        assert!(screenplay.project.title_page.iter().any(|paragraph| {
            paragraph.text.contains("Title") || paragraph.text.contains("Writer")
        }));
    }
}
