use super::*;
use crate::formatting::SourceLayoutSettings;
use crate::StableId;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct PreservationState {
    pub source: SourceSnapshot,
    /// Ephemeral projection of recognized source layout settings for runtime
    /// behavior. The complete source values in `sections` and
    /// `source.original_bytes` remain authoritative for serialization.
    pub(super) source_layout_settings: SourceLayoutSettings,
    pub sections: SharedVec<PreservedSection>,
    pub anchors: Vec<SourceAnchor>,
    pub identities: Vec<ForeignIdentity>,
    pub references: Vec<ForeignReference>,
    pub board_graph: BoardGraph,
    pub(super) managed_paragraph_index: Option<BTreeMap<StableId, Vec<ManagedParagraphLocation>>>,
    pub(super) source_identity_use_index: Option<SourceIdentityUseIndex>,
    pub(super) source_text_bytes: usize,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ManagedParagraphLocation {
    pub(super) section_index: usize,
    pub(super) node_path: Vec<usize>,
}

impl PartialEq for PreservationState {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source
            && self.source_layout_settings == other.source_layout_settings
            && self.sections == other.sections
            && self.anchors == other.anchors
            && self.identities == other.identities
            && self.references == other.references
            && self.board_graph == other.board_graph
    }
}

impl Eq for PreservationState {}

impl PreservationState {
    /// Constructs source-backed preservation from codec-owned source parts.
    /// Transient indexes are intentionally initialized empty and rebuilt by
    /// the model's preservation operations when needed.
    pub fn from_source_parts(
        source: SourceSnapshot,
        source_layout_settings: SourceLayoutSettings,
        sections: Vec<PreservedSection>,
        anchors: Vec<SourceAnchor>,
        identities: Vec<ForeignIdentity>,
        references: Vec<ForeignReference>,
        board_graph: BoardGraph,
    ) -> Self {
        Self {
            source,
            source_layout_settings,
            sections: sections.into(),
            anchors,
            identities,
            references,
            board_graph,
            managed_paragraph_index: None,
            source_identity_use_index: None,
            source_text_bytes: 0,
        }
    }

pub fn source_layout_settings(&self) -> &SourceLayoutSettings {
        &self.source_layout_settings
    }
}
