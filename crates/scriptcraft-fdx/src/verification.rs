//! Semantic, identity, budget, and retained-source verification for FDX output.

use crate::diagnostics::{FdxError, SaveDiagnosticCode};
use crate::reader::validate_xml_chars;
use crate::writer::validate_qname;
use crate::{
    MAX_ATTRIBUTES_PER_ELEMENT, MAX_SOURCE_BYTES, MAX_SOURCE_EVENTS, MAX_SOURCE_NAME_OR_VALUE_BYTES,
    MAX_SOURCE_TEXT_BYTES,
};
use scriptcraft_model::dual_dialogue::DualDialogueGroup;
use scriptcraft_model::preservation::{
    BoardEntityKind, BoardGraph, DependencyConfidence, ForeignIdDomain, ForeignIdentity,
    PreservedNode, PreservedSection, ReferencePolicy, SourceAnchorId, SourceName, SourceOwnership,
};
use scriptcraft_model::{BeatBoard, BodyOwner, Project, TitlePageParagraph};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IdentityDomainSummary {
    Document,
    Paragraph,
    Scene,
    Beat,
    CastMember,
    Character,
    ListItem,
    Board,
    Image,
    Revision,
    Tag,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct IdentitySetDifference {
    domain: IdentityDomainSummary,
    missing: bool,
    unexpected: bool,
}

pub(crate) fn same_non_paragraph_identities(
    left: &[ForeignIdentity],
    right: &[ForeignIdentity],
) -> bool {
    left.iter()
        .filter(|identity| identity.domain != ForeignIdDomain::Paragraph)
        .eq(right
            .iter()
            .filter(|identity| identity.domain != ForeignIdDomain::Paragraph))
}

pub(crate) fn identity_set(identities: &[ForeignIdentity]) -> BTreeSet<ForeignIdentity> {
    identities.iter().cloned().collect()
}

fn beat_card_ids(board: &BeatBoard) -> BTreeSet<ForeignIdentity> {
    board.cards.iter().map(|card| ForeignIdentity {
        domain: ForeignIdDomain::ListItem,
        value: card.foreign_id.clone(),
    }).collect()
}

/// Compare the graph outside the precisely identified managed changes. Entity
/// source ordinals are recomputed on reread; relative order and all other
/// entity data remain significant. Winning-board edges to managed cards may
/// change, and any board's Item edge to a card whose ListItem is being deleted
/// may be detached with that owner. Outgoing edges from authorized removed
/// duplicate boards may also change.
pub(crate) fn same_unrelated_board_graph(
    original: &BoardGraph,
    current: &BoardGraph,
    old_board: &BeatBoard,
    new_board: &BeatBoard,
    removed_duplicates: &BTreeSet<ForeignIdentity>,
) -> bool {
    let old_cards = beat_card_ids(old_board);
    let new_cards = beat_card_ids(new_board);
    let changing_cards = old_cards.symmetric_difference(&new_cards).cloned().collect::<BTreeSet<_>>();
    let deleted_cards = old_cards.difference(&new_cards).cloned().collect::<BTreeSet<_>>();
    let mut changing_entities = changing_cards;
    if old_board.foreign_id != new_board.foreign_id {
        for id in old_board.foreign_id.iter().chain(&new_board.foreign_id) {
            changing_entities.insert(ForeignIdentity { domain: ForeignIdDomain::Board, value: id.clone() });
        }
    }
    changing_entities.extend(removed_duplicates.iter().cloned());
    let entities = |graph: &BoardGraph| {
        graph.entities.iter().filter(|entity| !changing_entities.contains(&entity.identity))
            .map(|entity| {
                let mut entity = entity.clone();
                entity.source_order = 0;
                entity
            }).collect::<Vec<_>>()
    };
    let managed_cards = old_cards.union(&new_cards).cloned().collect::<BTreeSet<_>>();
    let boards = old_board.foreign_id.iter().chain(&new_board.foreign_id)
        .map(|id| ForeignIdentity { domain: ForeignIdDomain::Board, value: id.clone() })
        .collect::<BTreeSet<_>>();
    let references = |graph: &BoardGraph| {
        graph.references.iter().filter(|reference| {
            !removed_duplicates.contains(&reference.source)
                && !(boards.contains(&reference.source) && managed_cards.contains(&reference.target))
                && !(reference.source.domain == ForeignIdDomain::Board
                    && deleted_cards.contains(&reference.target))
        }).cloned().collect::<Vec<_>>() 
    };
    entities(original) == entities(current) && references(original) == references(current)
}

/// Check durable Beat owners against their actual graph domains, rather than
/// treating ListItems or DisplayBoards as screenplay paragraph identities.
pub(crate) fn verify_managed_beat_graph(
    board: &BeatBoard,
    identities: &[ForeignIdentity],
    graph: &BoardGraph,
) -> Result<(), FdxError> {
    let mismatch = || SaveDiagnosticCode::WriteOriginalIdentityGraphMismatch.error();
    let board_id = ForeignIdentity {
        domain: ForeignIdDomain::Board,
        value: board.foreign_id.clone().ok_or_else(mismatch)?,
    };
    let card_ids = beat_card_ids(board);
    if card_ids.len() != board.cards.len() { return Err(mismatch()); }
    let mut owners = BTreeSet::new();
    owners.insert(board_id.clone());
    owners.extend(card_ids.iter().cloned());
    for paragraph in board.cards.iter().flat_map(|card| &card.paragraphs) {
        let identity = ForeignIdentity {
            domain: ForeignIdDomain::Paragraph,
            value: paragraph.foreign_id.clone().ok_or_else(mismatch)?,
        };
        if !owners.insert(identity) { return Err(mismatch()); }
    }
    let mut occurrences = BTreeMap::<ForeignIdentity, usize>::new();
    for identity in identities.iter().chain(graph.entities.iter().map(|entity| &entity.identity)) {
        let count = occurrences.entry(identity.clone()).or_default();
        *count = count.checked_add(1).ok_or(FdxError::ResourceLimit)?;
    }
    let entity_kinds = graph.entities.iter().map(|entity| (entity.identity.clone(), entity.kind))
        .collect::<BTreeMap<_, _>>();
    let mut values = BTreeSet::new();
    for owner in &owners {
        if !values.insert(owner.value.as_str()) { return Err(mismatch()); }
        if occurrences.get(owner) != Some(&1) { return Err(mismatch()); }
        let expected_kind = match &owner.domain {
            ForeignIdDomain::Board => Some(BoardEntityKind::DisplayBoard),
            ForeignIdDomain::ListItem => Some(BoardEntityKind::ListItem),
            ForeignIdDomain::Paragraph => None,
            _ => return Err(mismatch()),
        };
        if entity_kinds.get(owner).copied() != expected_kind { return Err(mismatch()); }
    }
    let actual = graph.references.iter().filter(|reference| {
        reference.source == board_id && card_ids.contains(&reference.target)
    }).collect::<Vec<_>>();
    if actual.len() != board.cards.len() || actual.iter().zip(&board.cards).any(|(reference, card)| {
        reference.target.value != card.foreign_id || reference.policy != ReferencePolicy::RequiredInternal
    }) { return Err(mismatch()); }
    Ok(())
}

/// Collect the complete graph from the actual detached output tree, after all
/// reconciliation and optional duplicate normalization, before XML emission.
pub(crate) fn intended_output_graph(
    nodes: &[PreservedNode],
    transactional: bool,
    project: &Project,
) -> Result<(Vec<ForeignIdentity>, BoardGraph), FdxError> {
    let mut roots = nodes.iter().filter_map(|node| match node {
        PreservedNode::Element(root) => Some(root),
        _ => None,
    });
    let root = roots.next().ok_or(FdxError::InvalidRoot)?;
    if roots.next().is_some() || root.name.local_name != "FinalDraft" {
        return Err(FdxError::InvalidRoot);
    }
    let raw = crate::beat_board_output::raw_element(root);
    if transactional {
        let namespace = raw.name.namespace_uri.as_deref();
        let boards = raw.children.iter().find_map(|node| match node {
            crate::reader::RawNode::Element(element)
                if element.name.local_name == "DisplayBoards"
                    && element.name.namespace_uri.as_deref() == namespace => Some(element),
            _ => None,
        }).ok_or(FdxError::InvalidRoot)?;
        let count = boards.children.iter().filter(|node| matches!(node,
            crate::reader::RawNode::Element(board)
                if board.name.local_name == "DisplayBoard"
                    && board.name.namespace_uri.as_deref() == namespace
                    && crate::reader::attribute(board, "Type") == Some("Beat")
        )).count();
        if count != 1 { return Err(FdxError::AmbiguousIdentity); }
    }
    crate::identity::collect_identities(&raw, project)
}

/// Generated nodes have no imported ownership or anchors. Compare their XML
/// projection with the reread tree while ignoring only those in-memory fields.
pub(crate) fn same_generated_source_tree(expected: &[PreservedNode], actual: &[PreservedNode]) -> bool {
    fn xml_projection(nodes: &[PreservedNode]) -> Vec<PreservedNode> {
        let mut nodes = normalize_source_tree(nodes);
        let mut stack = nodes.iter_mut().collect::<Vec<_>>();
        while let Some(node) = stack.pop() {
            if let PreservedNode::Element(element) = node {
                element.source_type = None;
                element.ownership = SourceOwnership::Opaque;
                element.dependency_confidence = DependencyConfidence::Unknown;
                element.anchor = None;
                if element.name.qualified_name.as_deref() == Some(element.name.local_name.as_str()) {
                    element.name.qualified_name = None;
                }
                for attribute in &mut element.attributes {
                    if attribute.name.qualified_name.as_deref() == Some(attribute.name.local_name.as_str()) {
                        attribute.name.qualified_name = None;
                    }
                }
                stack.extend(element.children.iter_mut());
            }
        }
        nodes
    }
    xml_projection(expected) == xml_projection(actual)
}

pub(crate) fn first_identity_set_difference(
    expected: &BTreeSet<ForeignIdentity>,
    actual: &BTreeSet<ForeignIdentity>,
) -> Option<IdentitySetDifference> {
    if let Some(identity) = expected.difference(actual).next() {
        return Some(IdentitySetDifference {
            domain: identity_domain_summary(&identity.domain),
            missing: true,
            unexpected: false,
        });
    }
    actual
        .difference(expected)
        .next()
        .map(|identity| IdentitySetDifference {
            domain: identity_domain_summary(&identity.domain),
            missing: false,
            unexpected: true,
        })
}

fn identity_domain_summary(domain: &ForeignIdDomain) -> IdentityDomainSummary {
    match domain {
        ForeignIdDomain::Document => IdentityDomainSummary::Document,
        ForeignIdDomain::Paragraph => IdentityDomainSummary::Paragraph,
        ForeignIdDomain::Scene => IdentityDomainSummary::Scene,
        ForeignIdDomain::Beat => IdentityDomainSummary::Beat,
        ForeignIdDomain::CastMember => IdentityDomainSummary::CastMember,
        ForeignIdDomain::Character => IdentityDomainSummary::Character,
        ForeignIdDomain::ListItem => IdentityDomainSummary::ListItem,
        ForeignIdDomain::Board => IdentityDomainSummary::Board,
        ForeignIdDomain::Image => IdentityDomainSummary::Image,
        ForeignIdDomain::Revision => IdentityDomainSummary::Revision,
        ForeignIdDomain::Tag => IdentityDomainSummary::Tag,
        ForeignIdDomain::Other(_) => IdentityDomainSummary::Other,
    }
}

pub(crate) fn same_semantic_elements(
    left: &[std::sync::Arc<BodyOwner>],
    right: &[std::sync::Arc<BodyOwner>],
) -> bool {
    left.len() == right.len()
        && left.iter().zip(right).all(|(left, right)| {
            left.element_type() == right.element_type()
                && left.text == right.text
                && (!scriptcraft_model::SemanticCapabilities::for_key(&left.element_type()).dialogue
                    || left.dialogue_alternatives == right.dialogue_alternatives)
                && if left.runs.is_empty() {
                    right.runs.iter().all(|run| run.attributes.is_empty())
                } else {
                    left.runs == right.runs
                }
        })
}

pub(crate) fn same_semantic_element_definitions(left: &Project, right: &Project) -> bool {
    left.element_definitions() == right.element_definitions()
}

pub fn same_dual_dialogue_projection(left: &Project, right: &Project) -> bool {
    fn member_positions(project: &Project, group: &DualDialogueGroup) -> Option<Vec<usize>> {
        group
            .member_ids()
            .map(|id| {
                project
                    .elements
                    .iter()
                    .position(|element| &element.id == id)
            })
            .collect()
    }

    fn active_side_positions(
        project: &Project,
        group: &DualDialogueGroup,
    ) -> Option<(Vec<usize>, Vec<usize>)> {
        let projection =
            scriptcraft_model::dual_dialogue::active_dual_dialogue_projection(project, group)?;
        let positions = |side: &scriptcraft_model::dual_dialogue::DualDialogueSide| {
            side.member_ids()
                .map(|id| {
                    project
                        .elements
                        .iter()
                        .position(|element| &element.id == id)
                })
                .collect::<Option<Vec<_>>>()
        };
        Some((positions(&projection.left)?, positions(&projection.right)?))
    }

    if left.dual_dialogue_groups.len() != right.dual_dialogue_groups.len() {
        return false;
    }
    for (left_group, right_group) in left
        .dual_dialogue_groups
        .iter()
        .zip(&right.dual_dialogue_groups)
    {
        // Member stable IDs are session-local and may be reallocated on reread; ordered occurrences and foreign source identity are the semantic comparison authorities.
        if left_group.members.len() != right_group.members.len()
            || active_side_positions(left, left_group) != active_side_positions(right, right_group)
            || member_positions(left, left_group) != member_positions(right, right_group)
        {
            return false;
        }
        if left.preservation.is_some()
            && left_group.wrapper.outer_foreign_id != right_group.wrapper.outer_foreign_id
        {
            return false;
        }
        // Wrapper ordinals are recomputable provenance coordinates, not durable serialized identity.
        if let Some(expected) = left_group.wrapper.source.as_ref() {
            if !right_group.wrapper.source.as_ref().is_some_and(|actual| {
                expected.format == actual.format
                    && expected.section == actual.section
                    && expected.foreign_id == actual.foreign_id
                    && expected.foreign_role == actual.foreign_role
            }) {
                return false;
            }
        }
        let left_overrides = left.dual_dialogue_paragraph_overrides.get(&left_group.id);
        let right_overrides = right.dual_dialogue_paragraph_overrides.get(&right_group.id);
        if left_overrides != right_overrides {
            return false;
        }
        if let Some(positions) = member_positions(left, left_group) {
            for position in positions {
                let Some(left_element) = left.elements.get(position) else {
                    return false;
                };
                let Some(right_element) = right.elements.get(position) else {
                    return false;
                };
                let expected_source = left_element.source_provenance.as_ref();
                let actual_source = right_element.source_provenance.as_ref();
                if let Some(expected_source) = expected_source {
                    if let Some(expected_foreign_id) = expected_source.foreign_id.as_deref() {
                        if !actual_source.is_some_and(|actual| {
                            actual.foreign_id.as_deref() == Some(expected_foreign_id)
                                && actual.section == expected_source.section
                        }) {
                            return false;
                        }
                    }
                }
                if left.direct_paragraph_overrides.get(&left_element.id)
                    != right.direct_paragraph_overrides.get(&right_element.id)
                {
                    return false;
                }
            }
        }
    }
    left.elements.len() == right.elements.len()
}

pub(crate) fn same_semantic_title_paragraphs(
    left: &[TitlePageParagraph],
    right: &[TitlePageParagraph],
) -> bool {
    left.len() == right.len()
        && left.iter().zip(right).all(|(left, right)| {
            left.element_type() == right.element_type()
                && left.text == right.text
                && left.formatting == right.formatting
                && if left.runs.is_empty() {
                    right.runs.iter().all(|run| run.attributes.is_empty())
                } else {
                    left.runs == right.runs
                }
        })
}

/// Compare durable Beat content, excluding session-local IDs and viewport state.
pub fn same_beat_board_projection(left: &Project, right: &Project) -> bool {
    let left = left.beat_board();
    let right = right.beat_board();
    let creation_ids = |board: &scriptcraft_model::BeatBoard| -> Option<Vec<String>> {
        let cards = board.cards.iter().map(|card| (&card.id, &card.foreign_id))
            .collect::<std::collections::BTreeMap<_, _>>();
        if cards.len() != board.cards.len() || board.card_creation_order.len() != cards.len() {
            return None;
        }
        let mut seen = BTreeSet::new();
        board.card_creation_order.iter().map(|id| {
            if !seen.insert(id) { return None; }
            cards.get(id).map(|id| id.as_str().to_owned())
        }).collect()
    };
    let Some(left_creation) = creation_ids(left) else { return false };
    let Some(right_creation) = creation_ids(right) else { return false };
    left.foreign_id == right.foreign_id
        && left.width == right.width
        && left.height == right.height
        && left_creation == right_creation
        && left.cards.len() == right.cards.len()
        && left.cards.iter().zip(&right.cards).all(|(left, right)| {
            left.foreign_id == right.foreign_id
                && left.title == right.title
                && left.rect == right.rect
                && left.paragraphs.len() == right.paragraphs.len()
                && left.paragraphs.iter().zip(&right.paragraphs).all(|(left, right)| {
                    left.foreign_id == right.foreign_id
                        && left.text == right.text
                        && left.runs == right.runs
                        && left.direct_settings == right.direct_settings
                })
        })
}

/// Compare the complete durable ScriptNote projection, excluding only
/// session-local stable IDs and import-coordinate ordinals.
pub fn same_script_notes_projection(left: &Project, right: &Project) -> bool {
    fn note_key(note: &scriptcraft_model::ScriptNote) -> Option<String> {
        note.ref_id.as_deref().filter(|value| !value.is_empty()).map(|value| format!("R:{value}"))
            .or_else(|| note.numeric_id.as_deref().filter(|value| !value.is_empty()).map(|value| format!("I:{value}")))
    }
    let left_notes = left.script_notes().iter().collect::<Vec<_>>();
    let right_notes = right.script_notes().iter().collect::<Vec<_>>();
    if left_notes.len() != right_notes.len() { return false; }
    for (expected, actual) in left_notes.iter().zip(&right_notes) {
        if note_key(expected) != note_key(actual)
            || expected.numeric_id != actual.numeric_id
            || expected.ref_id != actual.ref_id
            || expected.range != actual.range
            || expected.color != actual.color
            || expected.title != actual.title
            || expected.note_type != actual.note_type
            || expected.writer_id != actual.writer_id
            || expected.writer_name != actual.writer_name
            || expected.date_time != actual.date_time
            || expected.date_modified != actual.date_modified
            || expected.paragraphs.len() != actual.paragraphs.len()
        { return false; }
        for (expected_paragraph, actual_paragraph) in expected.paragraphs.iter().zip(&actual.paragraphs) {
            if expected_paragraph.durable_id != actual_paragraph.durable_id
                || expected_paragraph.text != actual_paragraph.text
                || expected_paragraph.runs != actual_paragraph.runs
                || expected_paragraph.direct_settings != actual_paragraph.direct_settings
            { return false; }
        }
    }
    true
}

pub(crate) fn same_writer_semantic_projection(left: &Project, right: &Project) -> bool {
    left.page_layout() == right.page_layout()
        && same_semantic_element_definitions(left, right)
        && same_semantic_title_paragraphs(&left.title_page, &right.title_page)
        && same_semantic_elements(&left.elements, &right.elements)
        && same_dual_dialogue_projection(left, right)
        && same_beat_board_projection(left, right)
        && same_script_notes_projection(left, right)
}

pub(crate) fn verify_writer_semantic_projection(
    expected: &Project,
    actual: &Project,
) -> Result<(), FdxError> {
    if same_writer_semantic_projection(expected, actual) {
        Ok(())
    } else {
        Err(SaveDiagnosticCode::WriteSemanticRereadMismatch.error())
    }
}


pub(crate) fn validate_project_output_budgets(project: &Project) -> Result<(), FdxError> {
    let mut text_bytes = 0usize;
    let mut payload_bytes = 0usize;
    let mut event_count = 3usize;
    // Account for the board and its generated containers, ListItems, Content,
    // and Item nodes as well as their title/identity attributes.
    event_count = event_count.checked_add(3).ok_or(FdxError::ResourceLimit)?;
    let board = project.beat_board();
    if let Some(id) = &board.foreign_id {
        if id.len() > MAX_SOURCE_NAME_OR_VALUE_BYTES {
            return Err(FdxError::ResourceLimit);
        }
        validate_xml_chars(id)?;
        payload_bytes = payload_bytes.checked_add(id.len()).ok_or(FdxError::ResourceLimit)?;
    }
    for card in &board.cards {
        event_count = event_count.checked_add(3).ok_or(FdxError::ResourceLimit)?;
        for value in [&card.foreign_id, &card.title] {
            if value.len() > MAX_SOURCE_NAME_OR_VALUE_BYTES {
                return Err(FdxError::ResourceLimit);
            }
            validate_xml_chars(value)?;
            payload_bytes = payload_bytes.checked_add(value.len()).ok_or(FdxError::ResourceLimit)?;
        }
        for paragraph in &card.paragraphs {
            if let Some(id) = &paragraph.foreign_id {
                if id.len() > MAX_SOURCE_NAME_OR_VALUE_BYTES {
                    return Err(FdxError::ResourceLimit);
                }
                validate_xml_chars(id)?;
                payload_bytes = payload_bytes.checked_add(id.len()).ok_or(FdxError::ResourceLimit)?;
            }
        }
        if event_count > MAX_SOURCE_EVENTS {
            return Err(FdxError::ResourceLimit);
        }
    }
    for note in project.script_notes().iter() {
        event_count = event_count.checked_add(1).and_then(|value| value.checked_add(10))
            .ok_or(FdxError::ResourceLimit)?;
        let range = format!("{},{}", note.range.start(), note.range.end());
        let values = [note.color.as_str(), note.date_modified.as_str(), note.date_time.as_str(),
            note.numeric_id.as_deref().unwrap_or(""), note.title.as_str(), range.as_str(),
            note.ref_id.as_deref().unwrap_or(""), note.note_type.as_deref().unwrap_or(""),
            note.writer_id.as_str(), note.writer_name.as_str()];
        for value in values {
            if value.len() > MAX_SOURCE_NAME_OR_VALUE_BYTES { return Err(FdxError::ResourceLimit); }
            validate_xml_chars(value)?;
            payload_bytes = payload_bytes.checked_add(value.len()).ok_or(FdxError::ResourceLimit)?;
        }
        for paragraph in &note.paragraphs {
            event_count = event_count.checked_add(12).ok_or(FdxError::ResourceLimit)?;
            if let Some(id) = &paragraph.durable_id {
                if id.len() > MAX_SOURCE_NAME_OR_VALUE_BYTES { return Err(FdxError::ResourceLimit); }
                validate_xml_chars(id)?;
                payload_bytes = payload_bytes.checked_add(id.len()).ok_or(FdxError::ResourceLimit)?;
            }
            validate_xml_chars(&paragraph.text)?;
            text_bytes = text_bytes.checked_add(paragraph.text.len()).ok_or(FdxError::ResourceLimit)?;
            payload_bytes = payload_bytes.checked_add(paragraph.text.len()).ok_or(FdxError::ResourceLimit)?;
            for run in &paragraph.runs {
                event_count = event_count.checked_add(1).ok_or(FdxError::ResourceLimit)?;
                if !run.text.is_empty() {
                    event_count = event_count.checked_add(1).ok_or(FdxError::ResourceLimit)?;
                }
                for attribute in &run.attributes {
                    if attribute.name.len() > MAX_SOURCE_NAME_OR_VALUE_BYTES
                        || attribute.value.len() > MAX_SOURCE_NAME_OR_VALUE_BYTES
                    { return Err(FdxError::ResourceLimit); }
                    validate_qname(&attribute.name)?;
                    validate_xml_chars(&attribute.value)?;
                    payload_bytes = payload_bytes.checked_add(attribute.name.len())
                        .and_then(|size| size.checked_add(attribute.value.len()))
                        .ok_or(FdxError::ResourceLimit)?;
                    event_count = event_count.checked_add(1).ok_or(FdxError::ResourceLimit)?;
                }
            }
        }
        if text_bytes > MAX_SOURCE_TEXT_BYTES || payload_bytes > MAX_SOURCE_BYTES
            || event_count > MAX_SOURCE_EVENTS { return Err(FdxError::ResourceLimit); }
    }
    for (text, runs) in project
        .title_page
        .iter()
        .map(|element| (element.text.as_str(), element.runs.as_slice()))
        .chain(
            project
                .elements
                .iter()
                .map(|element| (element.text.as_str(), element.runs.as_slice())),
        )
        .chain(board.cards.iter().flat_map(|card| {
            card.paragraphs.iter().map(|paragraph| (paragraph.text.as_str(), paragraph.runs.as_slice()))
        }))
    {
        validate_xml_chars(text)?;
        event_count = event_count.checked_add(1).ok_or(FdxError::ResourceLimit)?;
        text_bytes = text_bytes.checked_add(text.len()).ok_or(FdxError::ResourceLimit)?;
        payload_bytes = payload_bytes.checked_add(text.len()).ok_or(FdxError::ResourceLimit)?;
        let run_count = runs.len().max(usize::from(runs.is_empty()));
        event_count = event_count.checked_add(run_count).ok_or(FdxError::ResourceLimit)?;
        if runs.is_empty() && !text.is_empty() {
            event_count = event_count.checked_add(1).ok_or(FdxError::ResourceLimit)?;
        }
        for run in runs {
            if !run.text.is_empty() {
                event_count = event_count.checked_add(1).ok_or(FdxError::ResourceLimit)?;
            }
            if run.attributes.len() > MAX_ATTRIBUTES_PER_ELEMENT {
                return Err(FdxError::ResourceLimit);
            }
            event_count = event_count.checked_add(run.attributes.len()).ok_or(FdxError::ResourceLimit)?;
            for attribute in &run.attributes {
                if attribute.name.len() > MAX_SOURCE_NAME_OR_VALUE_BYTES
                    || attribute.value.len() > MAX_SOURCE_NAME_OR_VALUE_BYTES
                {
                    return Err(FdxError::ResourceLimit);
                }
                validate_qname(&attribute.name)?;
                validate_xml_chars(&attribute.value)?;
                payload_bytes = payload_bytes.checked_add(attribute.name.len())
                    .and_then(|size| size.checked_add(attribute.value.len()))
                    .ok_or(FdxError::ResourceLimit)?;
            }
        }
        if text_bytes > MAX_SOURCE_TEXT_BYTES || payload_bytes > MAX_SOURCE_BYTES
            || event_count > MAX_SOURCE_EVENTS {
            return Err(FdxError::ResourceLimit);
        }
    }
    if project.elements.iter().any(|owner| owner.dialogue_alternatives.as_ref()
        .is_some_and(|alternatives| !alternatives.inactive.is_empty()))
    {
        event_count = event_count.checked_add(1).ok_or(FdxError::ResourceLimit)?;
    }
    for owner in &project.elements {
        let Some(alternatives) = &owner.dialogue_alternatives else { continue };
        event_count = event_count.checked_add(1).ok_or(FdxError::ResourceLimit)?;
        let current = alternatives.current.to_string();
        if current.len() > MAX_SOURCE_NAME_OR_VALUE_BYTES { return Err(FdxError::ResourceLimit); }
        validate_xml_chars(&current)?;
        payload_bytes = payload_bytes.checked_add("Current".len()).and_then(|bytes| bytes.checked_add(current.len()))
            .ok_or(FdxError::ResourceLimit)?;
        event_count = event_count.checked_add(1).ok_or(FdxError::ResourceLimit)?;
        let definition = project.element_definitions().get(&owner.element_type())
            .filter(|definition| definition.capabilities().dialogue)
            .or_else(|| project.element_definitions().iter().find(|definition| definition.key.as_str() == "Dialogue"));
        for alternative in &alternatives.inactive {
            let id = alternative.id.to_string();
            if id.len() > MAX_SOURCE_NAME_OR_VALUE_BYTES { return Err(FdxError::ResourceLimit); }
            validate_xml_chars(&id)?;
            // The numeric ID appears once on Alt and once inline. Account for
            // both values, their nodes, and the fixed Paragraph boilerplate.
            payload_bytes = payload_bytes.checked_add("Id".len()).and_then(|bytes| bytes.checked_add(id.len()))
                .and_then(|bytes| bytes.checked_add(id.len()))
                .ok_or(FdxError::ResourceLimit)?;
            text_bytes = text_bytes.checked_add(id.len()).ok_or(FdxError::ResourceLimit)?;
            event_count = event_count.checked_add(15).ok_or(FdxError::ResourceLimit)?;
            for (name, value) in [
                ("Alignment", "Left"), ("FirstIndent", "0.00"),
                ("Leading", "Regular"), ("LeftIndent", "2.50"),
                ("OutlineLevel", "1"), ("RightIndent", "-1.11"),
                ("SpaceBefore", "0"), ("Spacing", "1"),
                ("StartsNewPage", "No"),
            ] {
                payload_bytes = payload_bytes.checked_add(name.len()).and_then(|bytes| bytes.checked_add(value.len()))
                    .ok_or(FdxError::ResourceLimit)?;
            }
            let durable_owner_id = owner.source_provenance.as_ref()
                .and_then(|source| source.foreign_id.as_deref()).unwrap_or("00000000-0000-0000-0000-000000000000");
            if durable_owner_id.len() > MAX_SOURCE_NAME_OR_VALUE_BYTES { return Err(FdxError::ResourceLimit); }
            validate_xml_chars(durable_owner_id)?;
            payload_bytes = payload_bytes.checked_add("id".len())
                .and_then(|bytes| bytes.checked_add(durable_owner_id.len()))
                .ok_or(FdxError::ResourceLimit)?;
            validate_xml_chars(&alternative.text)?;
            text_bytes = text_bytes.checked_add(alternative.text.len()).ok_or(FdxError::ResourceLimit)?;
            payload_bytes = payload_bytes.checked_add(alternative.text.len()).ok_or(FdxError::ResourceLimit)?;
            let projected_runs = match definition {
                Some(definition) if definition.capabilities().dialogue => {
                    crate::dialogue_alts_output::complete_alt_output_runs(
                        definition, &alternative.text, &alternative.runs,
                    )?
                }
                _ => alternative.runs.clone(),
            };
            event_count = event_count.checked_add(projected_runs.len().max(usize::from(projected_runs.is_empty() && !alternative.text.is_empty())))
                .ok_or(FdxError::ResourceLimit)?;
            for run in &projected_runs {
                if !run.text.is_empty() { event_count = event_count.checked_add(1).ok_or(FdxError::ResourceLimit)?; }
                if run.attributes.len() > MAX_ATTRIBUTES_PER_ELEMENT { return Err(FdxError::ResourceLimit); }
                for attribute in &run.attributes {
                    if attribute.name.len() > MAX_SOURCE_NAME_OR_VALUE_BYTES
                        || attribute.value.len() > MAX_SOURCE_NAME_OR_VALUE_BYTES
                    { return Err(FdxError::ResourceLimit); }
                    validate_qname(&attribute.name)?;
                    validate_xml_chars(&attribute.name)?;
                    validate_xml_chars(&attribute.value)?;
                    payload_bytes = payload_bytes.checked_add(attribute.name.len())
                        .and_then(|size| size.checked_add(attribute.value.len()))
                        .ok_or(FdxError::ResourceLimit)?;
                    event_count = event_count.checked_add(1).ok_or(FdxError::ResourceLimit)?;
                }
            }
            if text_bytes > MAX_SOURCE_TEXT_BYTES || payload_bytes > MAX_SOURCE_BYTES
                || event_count > MAX_SOURCE_EVENTS { return Err(FdxError::ResourceLimit); }
        }
    }
    if text_bytes > MAX_SOURCE_TEXT_BYTES || payload_bytes > MAX_SOURCE_BYTES
        || event_count > MAX_SOURCE_EVENTS
    { return Err(FdxError::ResourceLimit); }
    Ok(())
}
pub(crate) fn normalize_sections(sections: &[PreservedSection]) -> Vec<PreservedSection> {
    sections
        .iter()
        .cloned()
        .map(|mut section| {
            section.children = normalize_source_tree(&section.children).into();
            section
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StructuralNodeKind {
    Element,
    Text,
    CData,
    Comment,
    ProcessingInstruction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StructuralNodeSummary {
    kind: StructuralNodeKind,
    namespace_uri: Option<String>,
    local_name: Option<String>,
    child_count: usize,
    attribute_count: usize,
}

/// Private, value-free first-difference metadata for unit tests. The path is a
/// sequence of ordered child indexes prefixed by the source-section index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceTreeDifference {
    node_path: Vec<usize>,
    field: &'static str,
    expected_detail: Option<String>,
    actual_detail: Option<String>,
    expected: Option<StructuralNodeSummary>,
    actual: Option<StructuralNodeSummary>,
}

impl SourceTreeDifference {
    pub(crate) fn public_detail(&self) -> String {
        const MAX_DETAIL_BYTES: usize = 240;
        const MAX_PATH_DEPTH: usize = 12;
        let mut path = String::from("section[");
        path.push_str(&self.node_path.first().copied().unwrap_or(0).to_string());
        path.push(']');
        for (depth, index) in self.node_path.iter().skip(1).enumerate() {
            if depth >= MAX_PATH_DEPTH {
                path.push_str("/…");
                break;
            }
            path.push_str("/node[");
            path.push_str(&index.to_string());
            path.push(']');
        }
        let expected = self.expected_detail.as_deref().unwrap_or("absent");
        let actual = self.actual_detail.as_deref().unwrap_or("absent");
        let detail = format!(
            "path={path}; field={}; expected={expected}; actual={actual}",
            self.field
        );
        if detail.len() <= MAX_DETAIL_BYTES {
            detail
        } else {
            let mut bounded = detail;
            bounded.truncate(MAX_DETAIL_BYTES.saturating_sub(3));
            bounded.push_str("...");
            bounded
        }
    }
}

fn difference(
    node_path: Vec<usize>,
    field: &'static str,
    expected_detail: Option<String>,
    actual_detail: Option<String>,
    expected: Option<StructuralNodeSummary>,
    actual: Option<StructuralNodeSummary>,
) -> SourceTreeDifference {
    SourceTreeDifference {
        node_path,
        field,
        expected_detail,
        actual_detail,
        expected,
        actual,
    }
}

fn safe_xml_name(name: &str) -> String {
    const MAX_NAME_BYTES: usize = 36;
    let mut safe = String::new();
    for character in name.chars() {
        if safe.len() >= MAX_NAME_BYTES {
            safe.push_str("...");
            break;
        }
        safe.push(
            if character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.') {
                character
            } else {
                '_'
            },
        );
    }
    safe
}

fn safe_name_detail(value: Option<&str>) -> Option<String> {
    value.map(safe_xml_name)
}

fn source_name_difference(
    expected: &SourceName,
    actual: &SourceName,
) -> Option<(&'static str, Option<String>, Option<String>)> {
    if expected.local_name != actual.local_name {
        return Some((
            "element.local-name",
            safe_name_detail(Some(&expected.local_name)),
            safe_name_detail(Some(&actual.local_name)),
        ));
    }
    if expected.namespace_uri != actual.namespace_uri {
        return Some((
            "element.namespace",
            Some(
                if expected.namespace_uri.is_some() {
                    "present"
                } else {
                    "absent"
                }
                .into(),
            ),
            Some(
                if actual.namespace_uri.is_some() {
                    "present"
                } else {
                    "absent"
                }
                .into(),
            ),
        ));
    }
    if expected.qualified_name != actual.qualified_name {
        return Some((
            "element.qualified-name",
            Some(
                if expected.qualified_name.is_some() {
                    "present"
                } else {
                    "absent"
                }
                .into(),
            ),
            Some(
                if actual.qualified_name.is_some() {
                    "present"
                } else {
                    "absent"
                }
                .into(),
            ),
        ));
    }
    None
}

fn ownership_category(ownership: &SourceOwnership) -> &'static str {
    match ownership {
        SourceOwnership::Opaque => "opaque",
        SourceOwnership::Managed(_) => "managed",
        SourceOwnership::DualDialogueWrapper(_) => "dual_dialogue_wrapper",
        SourceOwnership::Attached(_) => "attached",
    }
}

fn anchor_category(anchor: &Option<SourceAnchorId>) -> &'static str {
    if anchor.is_some() {
        "present"
    } else {
        "absent"
    }
}

fn text_classification(text: &str) -> String {
    let class = if text.is_empty() {
        "empty"
    } else if text.chars().all(char::is_whitespace) {
        "whitespace-only"
    } else {
        "contains-non-whitespace"
    };
    format!("length={},{}", text.chars().count(), class)
}

fn node_description(node: &PreservedNode) -> String {
    match node {
        PreservedNode::Element(element) => format!(
            "element:{};children={};attributes={}",
            safe_xml_name(&element.name.local_name),
            element.children.len(),
            element.attributes.len()
        ),
        PreservedNode::Text(text) => format!("text:{}", text_classification(text)),
        PreservedNode::CData(text) => format!("cdata:{}", text_classification(text)),
        PreservedNode::Comment(text) => format!("comment:{}", text_classification(text)),
        PreservedNode::ProcessingInstruction { .. } => "processing-instruction".into(),
    }
}

fn structural_node_summary(node: &PreservedNode) -> StructuralNodeSummary {
    match node {
        PreservedNode::Element(element) => StructuralNodeSummary {
            kind: StructuralNodeKind::Element,
            namespace_uri: element.name.namespace_uri.clone(),
            local_name: Some(element.name.local_name.clone()),
            child_count: element.children.len(),
            attribute_count: element.attributes.len(),
        },
        PreservedNode::Text(_) => StructuralNodeSummary {
            kind: StructuralNodeKind::Text,
            namespace_uri: None,
            local_name: None,
            child_count: 0,
            attribute_count: 0,
        },
        PreservedNode::CData(_) => StructuralNodeSummary {
            kind: StructuralNodeKind::CData,
            namespace_uri: None,
            local_name: None,
            child_count: 0,
            attribute_count: 0,
        },
        PreservedNode::Comment(_) => StructuralNodeSummary {
            kind: StructuralNodeKind::Comment,
            namespace_uri: None,
            local_name: None,
            child_count: 0,
            attribute_count: 0,
        },
        PreservedNode::ProcessingInstruction { .. } => StructuralNodeSummary {
            kind: StructuralNodeKind::ProcessingInstruction,
            namespace_uri: None,
            local_name: None,
            child_count: 0,
            attribute_count: 0,
        },
    }
}

pub(crate) fn first_source_tree_difference(
    expected: &[PreservedSection],
    actual: &[PreservedSection],
) -> Option<SourceTreeDifference> {
    let section_count = expected.len().max(actual.len());
    for section_index in 0..section_count {
        let expected_section = expected.get(section_index);
        let actual_section = actual.get(section_index);
        let Some(expected_section) = expected_section else {
            return Some(difference(
                vec![section_index],
                "section.presence",
                None,
                actual_section
                    .map(|section| format!("kind={:?},ordinal={}", section.kind, section.ordinal)),
                None,
                actual_section
                    .and_then(|section| section.children.first())
                    .map(structural_node_summary),
            ));
        };
        let Some(actual_section) = actual_section else {
            return Some(difference(
                vec![section_index],
                "section.presence",
                Some(format!(
                    "kind={:?},ordinal={}",
                    expected_section.kind, expected_section.ordinal
                )),
                None,
                expected_section
                    .children
                    .first()
                    .map(structural_node_summary),
                None,
            ));
        };
        let section_path = vec![section_index];
        if let Some((field, left, right)) =
            source_name_difference(&expected_section.name, &actual_section.name)
        {
            let field = match field {
                "element.local-name" => "section.local-name",
                "element.namespace" => "section.namespace",
                _ => "section.qualified-name",
            };
            return Some(difference(
                section_path,
                field,
                left,
                right,
                expected_section
                    .children
                    .first()
                    .map(structural_node_summary),
                actual_section.children.first().map(structural_node_summary),
            ));
        }
        if expected_section.kind != actual_section.kind {
            return Some(difference(
                section_path,
                "section.kind",
                Some(format!("{:?}", expected_section.kind)),
                Some(format!("{:?}", actual_section.kind)),
                expected_section
                    .children
                    .first()
                    .map(structural_node_summary),
                actual_section.children.first().map(structural_node_summary),
            ));
        }
        if expected_section.ordinal != actual_section.ordinal {
            return Some(difference(
                section_path,
                "section.ordinal",
                Some(expected_section.ordinal.to_string()),
                Some(actual_section.ordinal.to_string()),
                expected_section
                    .children
                    .first()
                    .map(structural_node_summary),
                actual_section.children.first().map(structural_node_summary),
            ));
        }
        if let Some(difference) = first_node_sequence_difference(
            &expected_section.children,
            &actual_section.children,
            vec![section_index],
        ) {
            return Some(difference);
        }
    }
    None
}

fn first_node_sequence_difference(
    expected: &[PreservedNode],
    actual: &[PreservedNode],
    path: Vec<usize>,
) -> Option<SourceTreeDifference> {
    let node_count = expected.len().max(actual.len());
    for child_index in 0..node_count {
        let mut child_path = path.clone();
        child_path.push(child_index);
        let expected_node = expected.get(child_index);
        let actual_node = actual.get(child_index);
        let (Some(expected_node), Some(actual_node)) = (expected_node, actual_node) else {
            let expected_detail = expected_node.map_or_else(
                || format!("absent;sibling-count={}", expected.len()),
                |node| {
                    format!(
                        "{};sibling-count={}",
                        node_description(node),
                        expected.len()
                    )
                },
            );
            let actual_detail = actual_node.map_or_else(
                || format!("absent;sibling-count={}", actual.len()),
                |node| format!("{};sibling-count={}", node_description(node), actual.len()),
            );
            return Some(difference(
                child_path,
                "child.presence",
                Some(expected_detail),
                Some(actual_detail),
                expected_node.map(structural_node_summary),
                actual_node.map(structural_node_summary),
            ));
        };
        if expected_node != actual_node
            && (actual[child_index + 1..].contains(expected_node)
                || expected[child_index + 1..].contains(actual_node))
        {
            return Some(difference(
                child_path,
                "child.order",
                Some(node_description(expected_node)),
                Some(node_description(actual_node)),
                Some(structural_node_summary(expected_node)),
                Some(structural_node_summary(actual_node)),
            ));
        }
        match (expected_node, actual_node) {
            (PreservedNode::Element(expected_element), PreservedNode::Element(actual_element)) => {
                let expected_summary = Some(structural_node_summary(expected_node));
                let actual_summary = Some(structural_node_summary(actual_node));
                if let Some((field, left, right)) =
                    source_name_difference(&expected_element.name, &actual_element.name)
                {
                    return Some(difference(
                        child_path,
                        field,
                        left,
                        right,
                        expected_summary,
                        actual_summary,
                    ));
                }
                let common_attributes = expected_element
                    .attributes
                    .len()
                    .min(actual_element.attributes.len());
                for attribute_index in 0..common_attributes {
                    let left = &expected_element.attributes[attribute_index];
                    let right = &actual_element.attributes[attribute_index];
                    if left.name != right.name {
                        return Some(difference(
                            child_path,
                            "attribute.name-or-order",
                            safe_name_detail(Some(&left.name.local_name)),
                            safe_name_detail(Some(&right.name.local_name)),
                            expected_summary,
                            actual_summary,
                        ));
                    }
                    if left.value != right.value {
                        return Some(difference(
                            child_path,
                            "attribute.value",
                            safe_name_detail(Some(&left.name.local_name)),
                            safe_name_detail(Some(&right.name.local_name)),
                            expected_summary,
                            actual_summary,
                        ));
                    }
                }
                if expected_element.attributes.len() != actual_element.attributes.len() {
                    let left = expected_element
                        .attributes
                        .get(common_attributes)
                        .map(|attribute| safe_xml_name(&attribute.name.local_name));
                    let right = actual_element
                        .attributes
                        .get(common_attributes)
                        .map(|attribute| safe_xml_name(&attribute.name.local_name));
                    return Some(difference(
                        child_path,
                        "attribute.count-or-presence",
                        left,
                        right,
                        expected_summary,
                        actual_summary,
                    ));
                }
                let common_bindings = expected_element
                    .namespace_bindings
                    .len()
                    .min(actual_element.namespace_bindings.len());
                for binding_index in 0..common_bindings {
                    let left = &expected_element.namespace_bindings[binding_index];
                    let right = &actual_element.namespace_bindings[binding_index];
                    if left.prefix != right.prefix {
                        return Some(difference(
                            child_path,
                            "namespace-binding.prefix-or-order",
                            Some(if left.prefix.is_empty() {
                                "default".into()
                            } else {
                                safe_xml_name(&left.prefix)
                            }),
                            Some(if right.prefix.is_empty() {
                                "default".into()
                            } else {
                                safe_xml_name(&right.prefix)
                            }),
                            expected_summary,
                            actual_summary,
                        ));
                    }
                    if left.namespace_uri != right.namespace_uri {
                        return Some(difference(
                            child_path,
                            "namespace-binding.namespace",
                            Some("present".into()),
                            Some("present".into()),
                            expected_summary,
                            actual_summary,
                        ));
                    }
                }
                if expected_element.namespace_bindings.len()
                    != actual_element.namespace_bindings.len()
                {
                    return Some(difference(
                        child_path,
                        "namespace-binding.count-or-presence",
                        expected_element
                            .namespace_bindings
                            .get(common_bindings)
                            .map(|binding| safe_xml_name(&binding.prefix)),
                        actual_element
                            .namespace_bindings
                            .get(common_bindings)
                            .map(|binding| safe_xml_name(&binding.prefix)),
                        expected_summary,
                        actual_summary,
                    ));
                }
                if expected_element.source_type != actual_element.source_type {
                    return Some(difference(
                        child_path,
                        "source-type",
                        Some(if expected_element.source_type.is_some() {
                            "present".into()
                        } else {
                            "absent".into()
                        }),
                        Some(if actual_element.source_type.is_some() {
                            "present".into()
                        } else {
                            "absent".into()
                        }),
                        expected_summary,
                        actual_summary,
                    ));
                }
                if expected_element.ownership != actual_element.ownership {
                    return Some(difference(
                        child_path,
                        "ownership",
                        Some(ownership_category(&expected_element.ownership).into()),
                        Some(ownership_category(&actual_element.ownership).into()),
                        expected_summary,
                        actual_summary,
                    ));
                }
                if expected_element.dependency_confidence != actual_element.dependency_confidence {
                    return Some(difference(
                        child_path,
                        "dependency-confidence",
                        Some(format!("{:?}", expected_element.dependency_confidence)),
                        Some(format!("{:?}", actual_element.dependency_confidence)),
                        expected_summary,
                        actual_summary,
                    ));
                }
                // Anchor identifiers are in-memory keys and are allocated
                // again when the emitted XML is re-read. Retain the semantic
                // source projection check: a winning settings node must stay
                // anchored and an inert duplicate must stay unanchored.
                if expected_element.anchor.is_some() != actual_element.anchor.is_some() {
                    return Some(difference(
                        child_path,
                        "anchor",
                        Some(anchor_category(&expected_element.anchor).into()),
                        Some(anchor_category(&actual_element.anchor).into()),
                        expected_summary,
                        actual_summary,
                    ));
                }
                if let Some(difference) = first_node_sequence_difference(
                    &expected_element.children,
                    &actual_element.children,
                    child_path,
                ) {
                    return Some(difference);
                }
            }
            (PreservedNode::Text(expected_text), PreservedNode::Text(actual_text))
                if expected_text == actual_text => {}
            (PreservedNode::Text(expected_text), PreservedNode::Text(actual_text)) => {
                return Some(difference(
                    child_path,
                    "text.content",
                    Some(text_classification(expected_text)),
                    Some(text_classification(actual_text)),
                    Some(structural_node_summary(expected_node)),
                    Some(structural_node_summary(actual_node)),
                ));
            }
            (PreservedNode::CData(expected_text), PreservedNode::CData(actual_text))
                if expected_text == actual_text => {}
            (PreservedNode::CData(expected_text), PreservedNode::CData(actual_text)) => {
                return Some(difference(
                    child_path,
                    "cdata.content",
                    Some(text_classification(expected_text)),
                    Some(text_classification(actual_text)),
                    Some(structural_node_summary(expected_node)),
                    Some(structural_node_summary(actual_node)),
                ));
            }
            (PreservedNode::Comment(expected_text), PreservedNode::Comment(actual_text))
                if expected_text == actual_text => {}
            (PreservedNode::Comment(expected_text), PreservedNode::Comment(actual_text)) => {
                return Some(difference(
                    child_path,
                    "comment.content",
                    Some(text_classification(expected_text)),
                    Some(text_classification(actual_text)),
                    Some(structural_node_summary(expected_node)),
                    Some(structural_node_summary(actual_node)),
                ));
            }
            (
                PreservedNode::ProcessingInstruction {
                    target: expected_target,
                    data: expected_data,
                },
                PreservedNode::ProcessingInstruction {
                    target: actual_target,
                    data: actual_data,
                },
            ) if expected_target == actual_target && expected_data == actual_data => {}
            (
                PreservedNode::ProcessingInstruction {
                    target: expected_target,
                    data: expected_data,
                },
                PreservedNode::ProcessingInstruction {
                    target: actual_target,
                    data: actual_data,
                },
            ) => {
                let (field, left, right) = if expected_target != actual_target {
                    (
                        "processing-instruction.target",
                        Some("present".into()),
                        Some("present".into()),
                    )
                } else {
                    (
                        "processing-instruction.data",
                        Some(text_classification(expected_data)),
                        Some(text_classification(actual_data)),
                    )
                };
                return Some(difference(
                    child_path,
                    field,
                    left,
                    right,
                    Some(structural_node_summary(expected_node)),
                    Some(structural_node_summary(actual_node)),
                ));
            }
            _ => {
                return Some(difference(
                    child_path,
                    "node.kind-or-order",
                    Some(node_description(expected_node)),
                    Some(node_description(actual_node)),
                    Some(structural_node_summary(expected_node)),
                    Some(structural_node_summary(actual_node)),
                ));
            }
        }
    }
    None
}

fn normalize_source_tree(nodes: &[PreservedNode]) -> Vec<PreservedNode> {
    let mut normalized = normalize_text_nodes(nodes);
    let normalized_definition_anchor = SourceAnchorId::for_element_definition(0).ok();
    let mut stack = normalized.iter_mut().collect::<Vec<_>>();
    while let Some(node) = stack.pop() {
        if let PreservedNode::Element(element) = node {
            if matches!(
                element.ownership,
                SourceOwnership::Managed(_) | SourceOwnership::DualDialogueWrapper(_)
            ) {
                // Managed and wrapper ownership IDs and their source-anchor IDs
                // are session-local; they are not serialized FDX fields and may
                // be reallocated on reread. Compare the source XML structure,
                // not these keys.
                element.ownership = SourceOwnership::Opaque;
                element.anchor = None;
            } else if element.name.local_name == "ElementSettings" {
                // Definition anchors are not serialized. Normalize only their
                // allocation identity while preserving presence, so winners
                // remain distinguishable from inert, unanchored duplicates.
                if element.anchor.is_some() {
                    element.anchor = normalized_definition_anchor
                        .clone()
                        .or_else(|| element.anchor.clone());
                }
            }
            stack.extend(element.children.iter_mut());
        }
    }
    normalized
}

fn normalize_text_nodes(nodes: &[PreservedNode]) -> Vec<PreservedNode> {
    let mut normalized = Vec::<PreservedNode>::new();
    for node in nodes {
        let normalized_node = match node {
            PreservedNode::Element(element) => {
                let mut element = element.clone();
                element.children = normalize_text_nodes(&element.children).into();
                PreservedNode::Element(element)
            }
            other => other.clone(),
        };
        if matches!(&normalized_node, PreservedNode::Text(text) if text.is_empty()) {
            // The XML writer emits no bytes for an empty text node, so a reader
            // cannot reconstruct it. Omit it only at this comparison boundary.
            continue;
        }
        if let PreservedNode::Text(text) = normalized_node {
            if let Some(PreservedNode::Text(previous)) = normalized.last_mut() {
                previous.push_str(&text);
            } else {
                normalized.push(PreservedNode::Text(text));
            }
        } else {
            normalized.push(normalized_node);
        }
    }
    normalized
}
