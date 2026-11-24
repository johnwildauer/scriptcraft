//! Detached projection of the supported winning Beat board and its members.
//! The semantic board owns its supported content; unrelated source stays opaque.

use crate::context::SaveContext;
use crate::diagnostics::{map_preservation_error, FdxError};
use crate::reader::{RawAttribute, RawElement, RawNode};
use crate::tree::{new_lineage_element, plain_source_name};
use scriptcraft_model::preservation::{AnchorTarget, BoardEntityKind, ForeignIdDomain, ForeignIdentity, NamespaceBinding, PreservationState, PreservedAttribute, PreservedElement, PreservedNode, PreservedSection, SourceName};
use scriptcraft_model::{BeatBoard, BeatCard, BeatParagraph, BoardFixed, Project};
use std::collections::{BTreeMap, BTreeSet};

fn cards_in_creation_order(board: &BeatBoard) -> Result<Vec<&BeatCard>, FdxError> {
    let cards = board.cards.iter().map(|card| (&card.id, card)).collect::<BTreeMap<_, _>>();
    if cards.len() != board.cards.len() || board.card_creation_order.len() != cards.len() {
        return Err(FdxError::AmbiguousIdentity);
    }
    let mut seen = BTreeSet::new();
    board.card_creation_order.iter().map(|id| {
        if !seen.insert(id) { return Err(FdxError::AmbiguousIdentity); }
        cards.get(id).copied().ok_or(FdxError::AmbiguousIdentity)
    }).collect()
}

type RunNames = BTreeMap<String, SourceName>;

fn collect_run_names(nodes: &[PreservedNode], names: &mut RunNames) {
    for node in nodes {
        if let PreservedNode::Element(element) = node {
            for attribute in &element.attributes {
                if let Some(qualified) = attribute.name.qualified_name.as_ref().filter(|name| name.contains(':')) {
                    names.entry(qualified.clone()).or_insert_with(|| attribute.name.clone());
                }
            }
            collect_run_names(&element.children, names);
        }
    }
}

fn generated_paragraph(parent: &SourceName, paragraph: &BeatParagraph, names: &RunNames) -> Result<PreservedElement, FdxError> {
    let settings = paragraph.direct_settings;
    let spec = settings.paragraph;
    let (Some(alignment), Some(first), Some(leading), Some(left), Some(right),
        Some(before), Some(spacing), Some(new_page)) = (
        spec.alignment, spec.first_indent, spec.leading, spec.left_indent,
        spec.right_indent, spec.space_before, spec.spacing, settings.starts_new_page,
    ) else { return Err(FdxError::UnsupportedConstruct); };
    let id = paragraph.foreign_id.as_deref().ok_or(FdxError::AmbiguousIdentity)?;
    let mut node = new_lineage_element(parent, "Paragraph", vec![
        ("Alignment".into(), alignment.fdx_value().into()),
        ("FirstIndent".into(), generated_indent(first)),
        ("Leading".into(), leading.fdx_value().into()),
        ("LeftIndent".into(), generated_indent(left)),
        ("OutlineLevel".into(), "0".into()),
        ("RightIndent".into(), generated_indent(right)),
        ("SpaceBefore".into(), before.decimal_string()),
        ("Spacing".into(), spacing.fdx_value().into()),
        ("StartsNewPage".into(), if new_page { "Yes" } else { "No" }.into()),
        ("id".into(), id.to_owned()),
    ]);
    reconcile_beat_text(&mut node, paragraph, names)?;
    Ok(node)
}

fn generated_card(parent: &SourceName, card: &BeatCard, names: &RunNames) -> Result<PreservedElement, FdxError> {
    let mut item = new_lineage_element(parent, "ListItem", vec![
        ("Type".into(), "Beat".into()), ("Id".into(), card.foreign_id.clone()),
        ("Title".into(), card.title.clone()),
    ]);
    item.source_type = Some("Beat".to_owned());
    let mut content = new_lineage_element(&item.name, "Content", Vec::new());
    for paragraph in &card.paragraphs {
        content.children.push(PreservedNode::Element(generated_paragraph(&content.name, paragraph, names)?));
    }
    item.children.push(PreservedNode::Element(content));
    Ok(item)
}

fn generated_item(parent: &SourceName, card: &BeatCard) -> PreservedElement {
    new_lineage_element(parent, "Item", vec![
        ("Id".into(), card.foreign_id.clone()), ("Left".into(), decimal(card.rect.left)),
        ("Top".into(), decimal(card.rect.top)), ("Width".into(), decimal(card.rect.width)),
        ("Height".into(), decimal(card.rect.height)),
    ])
}

/// Generate the same supported shapes used by source-backed insertion.
pub(crate) fn append_generated_board(
    root: &mut PreservedElement, project: &Project, context: &SaveContext,
) -> Result<(), FdxError> {
    let semantic = project.beat_board();
    let creation_order = cards_in_creation_order(semantic)?;
    let board_id = semantic.foreign_id.as_deref().ok_or(FdxError::AmbiguousIdentity)?;
    let mut list = new_lineage_element(&root.name, "ListItems", Vec::new());
    let mut boards = new_lineage_element(&root.name, "DisplayBoards", Vec::new());
    let view = context.beat_board_view_state();
    let origin = view.scroll_origin();
    if origin.x > semantic.width || origin.y > semantic.height { return Err(FdxError::InvalidValue); }
    let zoom = view.zoom_milli_percent();
    let mut board = new_lineage_element(&boards.name, "DisplayBoard", vec![
        ("Type".into(), "Beat".into()), ("Id".into(), board_id.to_owned()),
        ("Height".into(), decimal(semantic.height)), ("Width".into(), decimal(semantic.width)),
        ("ScrollOrigin".into(), format!("{},{}", decimal(origin.x), decimal(origin.y))),
        ("ZoomLevel".into(), format!("{}.{:03}", zoom / 1_000, zoom % 1_000)),
    ]);
    board.source_type = Some("Beat".to_owned());
    for card in creation_order {
        list.children.push(PreservedNode::Element(generated_card(&list.name, card, &RunNames::new())?));
    }
    for card in &semantic.cards {
        board.children.push(PreservedNode::Element(generated_item(&board.name, card)));
    }
    boards.children.push(PreservedNode::Element(board));
    root.children.push(PreservedNode::Element(list));
    root.children.push(PreservedNode::Element(boards));
    Ok(())
}

fn child_index(parent: &PreservedElement, name: &str) -> Option<usize> {
    parent.children.iter().position(|node| matches!(node,
        PreservedNode::Element(child)
            if child.name.local_name == name && child.name.namespace_uri == parent.name.namespace_uri))
}

fn child_mut(parent: &mut PreservedElement, index: usize) -> Result<&mut PreservedElement, FdxError> {
    match parent.children.get_mut(index) {
        Some(PreservedNode::Element(element)) => Ok(element),
        _ => Err(FdxError::InvalidValue),
    }
}

// Only the first supported direct-root container participates in Beat import.
// Compare later boards to the original source before discarding their subtrees;
// a changed or ambiguously referenced duplicate is not ours to normalize.
pub(crate) fn normalize_transactional_duplicates(
    sections: &mut [PreservedSection], state: &PreservationState,
    original: &PreservationState, winning_id: Option<&str>,
) -> Result<BTreeSet<ForeignIdentity>, FdxError> {
    fn duplicates(sections: &[PreservedSection]) -> Result<Vec<PreservedElement>, FdxError> {
        let mut roots = sections.iter().flat_map(|section| &section.children).filter_map(|node| match node {
            PreservedNode::Element(root) if root.name.local_name == "FinalDraft" => Some(root),
            _ => None,
        });
        let root = roots.next().ok_or(FdxError::InvalidRoot)?;
        if roots.next().is_some() { return Err(FdxError::InvalidRoot); }
        let Some(index) = child_index(root, "DisplayBoards") else { return Ok(Vec::new()) };
        let PreservedNode::Element(boards) = &root.children[index] else { return Err(FdxError::InvalidValue); };
        Ok(boards.children.iter().filter_map(|node| match node {
            PreservedNode::Element(board) if board.name.local_name == "DisplayBoard"
                && board.name.namespace_uri == boards.name.namespace_uri
                && attribute(board, "Type") == Some("Beat") => Some(board.clone()),
            _ => None,
        }).skip(1).collect())
    }

    let before = duplicates(&original.sections)?;
    let current = duplicates(sections)?;
    if before != current { return Err(FdxError::AmbiguousIdentity); }
    let mut removed = BTreeSet::new();
    for board in &current {
        let id = identity(board).ok_or(FdxError::AmbiguousIdentity)?;
        if Some(id) == winning_id { return Err(FdxError::AmbiguousIdentity); }
        let id = ForeignIdentity::new(ForeignIdDomain::Board, id.to_owned()).map_err(map_preservation_error)?;
        if !removed.insert(id) {
            return Err(FdxError::AmbiguousIdentity);
        }
    }
    if removed.is_empty() { return Ok(removed); }
    if state.identities.iter().any(|id| removed.contains(id)) {
        return Err(FdxError::AmbiguousIdentity);
    }
    let mut registered = BTreeSet::new();
    for entity in &state.board_graph.entities {
        if removed.contains(&entity.identity)
            && (entity.kind != BoardEntityKind::DisplayBoard || entity.anchor.is_some()
                || !registered.insert(entity.identity.clone())) {
            return Err(FdxError::AmbiguousIdentity);
        }
    }
    if registered != removed { return Err(FdxError::AmbiguousIdentity); }
    if state.anchors.iter().any(|anchor| matches!(&anchor.target, AnchorTarget::Foreign(target) if removed.contains(target)))
        || state.references.iter().any(|reference| removed.contains(&reference.source) || removed.contains(&reference.target))
        || state.board_graph.references.iter().any(|reference| removed.contains(&reference.target)) {
        return Err(FdxError::DanglingReference);
    }
    let root = sections.iter_mut().flat_map(|section| &mut section.children).find_map(|node| match node {
        PreservedNode::Element(root) if root.name.local_name == "FinalDraft" => Some(root),
        _ => None,
    }).ok_or(FdxError::InvalidRoot)?;
    let index = child_index(root, "DisplayBoards").ok_or(FdxError::InvalidRoot)?;
    let boards = child_mut(root, index)?;
    let namespace = boards.name.namespace_uri.clone();
    let mut seen_winner = false;
    boards.children.retain(|node| {
        let PreservedNode::Element(board) = node else { return true };
        if board.name.local_name != "DisplayBoard" || board.name.namespace_uri != namespace
            || attribute(board, "Type") != Some("Beat") { return true; }
        if !seen_winner { seen_winner = true; true } else { false }
    });
    Ok(removed)
}

fn attribute<'a>(element: &'a PreservedElement, name: &str) -> Option<&'a str> {
    let mut matches = element.attributes.iter().filter(|attribute| {
        attribute.name.namespace_uri.is_none() && attribute.name.local_name == name
    });
    let value = matches.next()?.value.as_str();
    matches.next().is_none().then_some(value)
}

fn identity(element: &PreservedElement) -> Option<&str> {
    let mut matches = element.attributes.iter().filter(|attribute| {
        attribute.name.namespace_uri.is_none() && attribute.name.local_name.eq_ignore_ascii_case("id")
    });
    let value = matches.next()?.value.as_str();
    matches.next().is_none().then_some(value)
}

fn set_attribute(element: &mut PreservedElement, name: &str, value: String) -> Result<(), FdxError> {
    let indexes = element.attributes.iter().enumerate().filter_map(|(index, attribute)| {
        (attribute.name.namespace_uri.is_none() && attribute.name.local_name == name).then_some(index)
    }).collect::<Vec<_>>();
    match indexes.as_slice() {
        [] => element.attributes.push(PreservedAttribute { name: plain_source_name(name), value }),
        [index] => element.attributes[*index].value = value,
        _ => return Err(FdxError::AmbiguousIdentity),
    }
    Ok(())
}

fn reconcile_setting<T: PartialEq>(
    element: &mut PreservedElement, name: &str, old: Option<T>, new: Option<T>,
    format: impl FnOnce(T) -> String,
) -> Result<(), FdxError> {
    if old == new { return Ok(()); }
    let indexes = element.attributes.iter().enumerate().filter_map(|(index, attribute)| {
        (attribute.name.namespace_uri.is_none() && attribute.name.local_name == name).then_some(index)
    }).collect::<Vec<_>>();
    // When the semantic setting changes it owns this direct attribute, even if
    // the previous spelling was not recognized on import.
    if indexes.len() > 1 {
        return Err(FdxError::UnsupportedConstruct);
    }
    match new {
        Some(value) => set_attribute(element, name, format(value)),
        None => {
            if let Some(index) = indexes.first() { element.attributes.remove(*index); }
            Ok(())
        }
    }
}

fn reconcile_paragraph_settings(element: &mut PreservedElement, paragraph: &BeatParagraph) -> Result<(), FdxError> {
    let old = crate::content::direct_paragraph_overrides(&raw_element(element)).unwrap_or_default();
    let new = paragraph.direct_settings;
    reconcile_setting(element, "Alignment", old.paragraph.alignment, new.paragraph.alignment, |v| v.fdx_value().into())?;
    reconcile_setting(element, "FirstIndent", old.paragraph.first_indent, new.paragraph.first_indent, crate::content::fixed_inches_string)?;
    reconcile_setting(element, "Leading", old.paragraph.leading, new.paragraph.leading, |v| v.fdx_value().into())?;
    reconcile_setting(element, "LeftIndent", old.paragraph.left_indent, new.paragraph.left_indent, crate::content::fixed_inches_string)?;
    reconcile_setting(element, "RightIndent", old.paragraph.right_indent, new.paragraph.right_indent, crate::content::fixed_inches_string)?;
    reconcile_setting(element, "SpaceBefore", old.paragraph.space_before, new.paragraph.space_before, |v| v.decimal_string())?;
    reconcile_setting(element, "Spacing", old.paragraph.spacing, new.paragraph.spacing, |v| v.fdx_value().into())?;
    reconcile_setting(element, "StartsNewPage", old.starts_new_page, new.starts_new_page, |v| if v { "Yes".into() } else { "No".into() })?;
    if crate::content::direct_paragraph_overrides(&raw_element(element)).unwrap_or_default() != new {
        return Err(FdxError::InvalidValue);
    }
    Ok(())
}

fn decimal(value: BoardFixed) -> String {
    let raw = value.raw();
    let magnitude = raw.unsigned_abs();
    let sign = if raw < 0 { "-" } else { "" };
    let whole = magnitude / 1_000;
    let fraction = magnitude % 1_000;
    if fraction == 0 {
        format!("{sign}{whole}")
    } else {
        let mut result = format!("{sign}{whole}.{fraction:03}");
        while result.ends_with('0') { result.pop(); }
        result
    }
}

fn generated_indent(value: scriptcraft_model::formatting::FixedPoint) -> String {
    if value.raw() == 0 { "0.00".into() } else { crate::content::fixed_inches_string(value) }
}

pub(crate) fn raw_element(element: &PreservedElement) -> RawElement {
    RawElement {
        name: element.name.clone(),
        attributes: element.attributes.iter().map(|attribute| RawAttribute {
            name: attribute.name.clone(), value: attribute.value.clone(),
        }).collect(),
        namespace_bindings: element.namespace_bindings.clone(),
        children: element.children.iter().map(|child| match child {
            PreservedNode::Element(element) => RawNode::Element(raw_element(element)),
            PreservedNode::Text(text) => RawNode::Text(text.to_string()),
            PreservedNode::CData(text) => RawNode::CData(text.to_string()),
            PreservedNode::Comment(text) => RawNode::Comment(text.to_string()),
            PreservedNode::ProcessingInstruction { target, data } => RawNode::ProcessingInstruction {
                target: target.to_string(), data: data.to_string(),
            },
        }).collect(),
    }
}

/// Bind only source paragraphs whose ownership is provable before allocation.
/// Existing identified paragraphs are matched within their winning-board card;
/// an id-less source is bound by its imported card-local source coordinate.
/// An unmatched id-less source remains unbound: it is a deleted occurrence,
/// not an invitation to bind an identical newly created paragraph.
/// The caller supplies a detached preservation candidate and both pre- and
/// post-allocation boards. Unmatched identified source nodes remain untouched.
pub(crate) fn bind_candidate_paragraph_identities(
    state: &mut PreservationState,
    before: &BeatBoard,
    after: &BeatBoard,
    reserved_source_ids: &BTreeSet<String>,
    original: &BeatBoard,
) -> Result<(), FdxError> {
    let mut roots = state.sections.iter_mut().flat_map(|section| &mut section.children).filter_map(|node| match node {
        PreservedNode::Element(root) if root.name.local_name == "FinalDraft" => Some(root),
        _ => None,
    });
    let root = roots.next().ok_or(FdxError::InvalidRoot)?;
    if roots.next().is_some() { return Err(FdxError::InvalidRoot); }
    let winning = child_index(root, "DisplayBoards").and_then(|index| match &root.children[index] {
        PreservedNode::Element(boards) => boards.children.iter().find_map(|node| match node {
            PreservedNode::Element(board) if board.name.local_name == "DisplayBoard"
                && board.name.namespace_uri == boards.name.namespace_uri
                && attribute(board, "Type") == Some("Beat") => Some(board),
            _ => None,
        }),
        _ => None,
    });
    let mut winning_items = BTreeMap::<String, usize>::new();
    if let Some(board) = winning {
        let board_id = identity(board).ok_or(FdxError::AmbiguousIdentity)?;
        if before.foreign_id.as_deref() != Some(board_id) { return Err(FdxError::AmbiguousIdentity); }
        for node in &board.children {
            let PreservedNode::Element(item) = node else { continue };
            if item.name.local_name == "Item" && item.name.namespace_uri == board.name.namespace_uri {
                let id = identity(item).ok_or(FdxError::DanglingReference)?;
                let count = winning_items.entry(id.to_owned()).or_default();
                *count = count.checked_add(1).ok_or(FdxError::ResourceLimit)?;
            }
        }
    } else if before.foreign_id.as_ref().is_some_and(|id| reserved_source_ids.contains(id)) {
        return Err(FdxError::AmbiguousIdentity);
    }

    let mut semantic_ids = BTreeSet::new();
    if let Some(id) = &after.foreign_id { semantic_ids.insert(id.clone()); }
    for card in &after.cards {
        if !semantic_ids.insert(card.foreign_id.clone()) { return Err(FdxError::AmbiguousIdentity); }
        for paragraph in &card.paragraphs {
            let id = paragraph.foreign_id.as_ref().ok_or(FdxError::AmbiguousIdentity)?;
            if !semantic_ids.insert(id.clone()) { return Err(FdxError::AmbiguousIdentity); }
        }
    }

    let mut list_indexes = BTreeMap::new();
    if let Some(index) = child_index(root, "ListItems") {
        let list = child_mut(root, index)?;
        for (index, node) in list.children.iter().enumerate() {
            let PreservedNode::Element(item) = node else { continue };
            if item.name.local_name != "ListItem" || item.name.namespace_uri != list.name.namespace_uri { continue; }
            if let Some(id) = identity(item) {
                if list_indexes.insert(id.to_owned(), index).is_some() { return Err(FdxError::AmbiguousIdentity); }
            } else if item.attributes.iter().filter(|attribute| attribute.name.namespace_uri.is_none()
                && attribute.name.local_name.eq_ignore_ascii_case("id")).count() > 1 {
                return Err(FdxError::AmbiguousIdentity);
            }
        }
    }
    for id in winning_items.keys() {
        if !list_indexes.contains_key(id) { return Err(FdxError::DanglingReference); }
    }

    let list_index = child_index(root, "ListItems");
    let mut additions = Vec::new();
    let after_cards = after.cards.iter().map(|card| (&card.id, card)).collect::<BTreeMap<_, _>>();
    let before_card_ids = before.cards.iter().map(|card| &card.id).collect::<BTreeSet<_>>();
    if after_cards.len() != after.cards.len() || before_card_ids.len() != before.cards.len()
        || before_card_ids.iter().any(|id| !after_cards.contains_key(*id)) {
        return Err(FdxError::AmbiguousIdentity);
    }
    // New cards have no retained source owner to bind. Their semantic durable
    // identities and graph registrations are already allocated by the Session;
    // register paragraph identities here on the detached save candidate only.
    for card in after.cards.iter().filter(|card| !before_card_ids.contains(&card.id)) {
        if card.foreign_id.is_empty() {
            return Err(FdxError::AmbiguousIdentity);
        }
        for paragraph in &card.paragraphs {
            let id = paragraph.foreign_id.as_ref().ok_or(FdxError::AmbiguousIdentity)?;
            let identity = ForeignIdentity::new(ForeignIdDomain::Paragraph, id.clone()).map_err(map_preservation_error)?;
            if !state.identities.contains(&identity) { additions.push(identity); }
        }
    }
    for old_card in &before.cards {
        let new_card = after_cards.get(&old_card.id).copied().ok_or(FdxError::AmbiguousIdentity)?;
        let after_paragraphs = new_card.paragraphs.iter().map(|p| (&p.id, p)).collect::<BTreeMap<_, _>>();
        let before_paragraph_ids = old_card.paragraphs.iter().map(|p| &p.id).collect::<BTreeSet<_>>();
        if after_paragraphs.len() != new_card.paragraphs.len()
            || before_paragraph_ids.len() != old_card.paragraphs.len()
            || before_paragraph_ids.len() != after_paragraphs.len()
            || before_paragraph_ids.iter().any(|id| !after_paragraphs.contains_key(*id)) {
            return Err(FdxError::AmbiguousIdentity);
        }
        if old_card.foreign_id != new_card.foreign_id && !old_card.foreign_id.is_empty() {
            return Err(FdxError::AmbiguousIdentity);
        }
        let retained = winning_items.contains_key(&old_card.foreign_id);
        if !retained {
            if !old_card.foreign_id.is_empty() && reserved_source_ids.contains(&old_card.foreign_id) {
                return Err(FdxError::AmbiguousIdentity);
            }
            for paragraph in &old_card.paragraphs {
                if paragraph.source_paragraph_ordinal.is_some()
                    || paragraph.foreign_id.as_ref().is_some_and(|id| reserved_source_ids.contains(id)) {
                    return Err(FdxError::AmbiguousIdentity);
                }
            }
            continue;
        }
        if winning_items.get(&old_card.foreign_id) != Some(&1) { return Err(FdxError::AmbiguousIdentity); }
        let mut original_cards = original.cards.iter().filter(|card| card.foreign_id == old_card.foreign_id);
        let original_card = original_cards.next().ok_or(FdxError::AmbiguousIdentity)?;
        if original_cards.next().is_some() || original_card.id != old_card.id {
            return Err(FdxError::AmbiguousIdentity);
        }
        // A surviving imported owner cannot discard or change its baseline
        // coordinate and thereby masquerade as a generated paragraph. New
        // owners may not claim an imported coordinate (checked below).
        let original_paragraphs = original_card.paragraphs.iter()
            .map(|paragraph| (&paragraph.id, paragraph))
            .collect::<BTreeMap<_, _>>();
        if original_paragraphs.len() != original_card.paragraphs.len() {
            return Err(FdxError::AmbiguousIdentity);
        }
        for paragraph in &old_card.paragraphs {
            if let Some(original) = original_paragraphs.get(&paragraph.id) {
                if paragraph.source_paragraph_ordinal != original.source_paragraph_ordinal
                    || original.foreign_id.as_ref().is_some_and(|id| paragraph.foreign_id.as_ref() != Some(id)) {
                    return Err(FdxError::AmbiguousIdentity);
                }
            }
        }
        let list = child_mut(root, list_index.ok_or(FdxError::DanglingReference)?)?;
        let item = child_mut(list, *list_indexes.get(&old_card.foreign_id).ok_or(FdxError::DanglingReference)?)?;
        if attribute(item, "Type") != Some("Beat") { return Err(FdxError::AmbiguousIdentity); }
        let Some(content_index) = child_index(item, "Content") else {
            for paragraph in &old_card.paragraphs {
                if paragraph.source_paragraph_ordinal.is_some()
                    || paragraph.foreign_id.as_ref().is_some_and(|id| reserved_source_ids.contains(id)) {
                    return Err(FdxError::AmbiguousIdentity);
                }
            }
            continue;
        };
        let content = child_mut(item, content_index)?;
        let indexes = content.children.iter().enumerate().filter_map(|(index, node)| match node {
            PreservedNode::Element(p) if p.name.local_name == "Paragraph"
                && p.name.namespace_uri == content.name.namespace_uri => Some(index),
            _ => None,
        }).collect::<Vec<_>>();
        let mut identified = BTreeMap::new();
        let mut unidentified = Vec::new();
        let mut source_ordinals = BTreeMap::new();
        for (ordinal, index) in indexes.into_iter().enumerate() {
            source_ordinals.insert(index, ordinal);
            let PreservedNode::Element(source) = &content.children[index] else { return Err(FdxError::InvalidValue); };
            let count = source.attributes.iter().filter(|attribute| attribute.name.namespace_uri.is_none()
                && attribute.name.local_name.eq_ignore_ascii_case("id")).count();
            if count > 1 { return Err(FdxError::AmbiguousIdentity); }
            if let Some(id) = identity(source) {
                if identified.insert(id.to_owned(), index).is_some() { return Err(FdxError::AmbiguousIdentity); }
            } else {
                unidentified.push(index);
            }
        }
        let mut claimed_ordinals = BTreeSet::new();
        for paragraph in &old_card.paragraphs {
            if let Some(ordinal) = paragraph.source_paragraph_ordinal {
                if !claimed_ordinals.insert(ordinal) || ordinal >= source_ordinals.len() {
                    return Err(FdxError::AmbiguousIdentity);
                }
                let source_owner = original_card.paragraphs.get(ordinal).ok_or(FdxError::AmbiguousIdentity)?;
                if source_owner.source_paragraph_ordinal != Some(ordinal) || source_owner.id != paragraph.id {
                    return Err(FdxError::AmbiguousIdentity);
                }
                if let Some(id) = &paragraph.foreign_id {
                    if identified.get(id).and_then(|index| source_ordinals.get(index)) != Some(&ordinal) {
                        return Err(FdxError::AmbiguousIdentity);
                    }
                }
            }
            if let Some(id) = &paragraph.foreign_id {
                if identified.contains_key(id) && paragraph.source_paragraph_ordinal.is_none() {
                    return Err(FdxError::AmbiguousIdentity);
                }
                if !identified.contains_key(id) && reserved_source_ids.contains(id) {
                    return Err(FdxError::AmbiguousIdentity);
                }
            }
        }
        let by_ordinal = old_card.paragraphs.iter()
            .filter(|paragraph| paragraph.foreign_id.is_none())
            .filter_map(|paragraph| {
            paragraph.source_paragraph_ordinal.map(|ordinal| (ordinal, &paragraph.id))
        }).collect::<BTreeMap<_, _>>();
        let mut used = BTreeSet::new();
        for index in unidentified {
            let ordinal = *source_ordinals.get(&index).ok_or(FdxError::AmbiguousIdentity)?;
            let Some(stable_id) = by_ordinal.get(&ordinal).copied() else { continue };
            if !used.insert(stable_id) { return Err(FdxError::AmbiguousIdentity); }
            let id = after_paragraphs.get(stable_id).and_then(|p| p.foreign_id.as_ref())
                .ok_or(FdxError::AmbiguousIdentity)?;
            let source = child_mut(content, index)?;
            set_attribute(source, "id", id.clone())?;
            additions.push(ForeignIdentity::new(ForeignIdDomain::Paragraph, id.clone()).map_err(map_preservation_error)?);
        }
    }
    state.identities.extend(additions);
    state.rebuild_indexes();
    Ok(())
}

fn append_missing_board(
    root: &mut PreservedElement, project: &Project, context: &SaveContext,
) -> Result<(), FdxError> {
    if child_index(root, "ListItems").is_none() {
        root.children.push(PreservedNode::Element(new_lineage_element(&root.name, "ListItems", Vec::new())));
    }
    if child_index(root, "DisplayBoards").is_none() {
        root.children.push(PreservedNode::Element(new_lineage_element(&root.name, "DisplayBoards", Vec::new())));
    }
    let boards_index = child_index(root, "DisplayBoards").ok_or(FdxError::InvalidValue)?;
    let boards = child_mut(root, boards_index)?;
    let semantic = project.beat_board();
    let id = semantic.foreign_id.as_deref().ok_or(FdxError::AmbiguousIdentity)?;
    let view = context.beat_board_view_state();
    let origin = view.scroll_origin();
    if origin.x > semantic.width || origin.y > semantic.height { return Err(FdxError::InvalidValue); }
    let zoom = view.zoom_milli_percent();
    let mut board = new_lineage_element(&boards.name, "DisplayBoard", vec![
        ("Type".into(), "Beat".into()), ("Id".into(), id.to_owned()),
        ("Width".into(), decimal(semantic.width)), ("Height".into(), decimal(semantic.height)),
        ("ScrollOrigin".into(), format!("{},{}", decimal(origin.x), decimal(origin.y))),
        ("ZoomLevel".into(), format!("{}.{:03}", zoom / 1_000, zoom % 1_000)),
    ]);
    board.source_type = Some("Beat".into());
    boards.children.push(PreservedNode::Element(board));
    Ok(())
}

#[derive(Default)]
pub(crate) struct ParagraphIdentityChanges {
    pub(crate) added: Vec<ForeignIdentity>,
    pub(crate) removed: BTreeSet<ForeignIdentity>,
}

/// An output-only projection. It is not a replacement for the live retained
/// baseline: the writer must still verify its graph and reread its XML.
pub(crate) struct BeatSourceProjection {
    pub(crate) sections: Vec<PreservedSection>,
    pub(crate) paragraphs: ParagraphIdentityChanges,
    pub(crate) removed_boards: BTreeSet<ForeignIdentity>,
}

/// Project Beat changes onto detached sections without mutating any caller
/// state, including when reconciliation or duplicate normalization fails.
pub(crate) fn project_beat_source(
    sections: &[PreservedSection],
    project: &Project,
    context: &SaveContext,
    state: &PreservationState,
    original: &BeatBoard,
    original_state: &PreservationState,
    normalize_duplicates: bool,
) -> Result<BeatSourceProjection, FdxError> {
    let mut names = RunNames::new();
    for section in sections { collect_run_names(&section.children, &mut names); }
    let mut sections = sections.to_vec();
    let paragraphs = reconcile_existing_board(
        &mut sections, project, context, state, original, original_state, normalize_duplicates, &names,
    )?;
    let removed_boards = if normalize_duplicates {
        normalize_transactional_duplicates(
            &mut sections, state, original_state, original.foreign_id.as_deref(),
        )?
    } else {
        BTreeSet::new()
    };
    Ok(BeatSourceProjection { sections, paragraphs, removed_boards })
}

fn paragraph_identity(id: &str) -> Result<ForeignIdentity, FdxError> {
    ForeignIdentity::new(ForeignIdDomain::Paragraph, id.to_owned()).map_err(map_preservation_error)
}

fn ensure_unreferenced(state: &PreservationState, target: &ForeignIdentity) -> Result<(), FdxError> {
    if state.references.iter().chain(&state.board_graph.references).any(|reference| &reference.target == target) {
        return Err(FdxError::DanglingReference);
    }
    Ok(())
}

/// Retained paragraphs with nonempty direct text, used only to choose defaults
/// for the first typed run in an empty paragraph, never to authorize an edit.
pub(crate) fn paragraphs_with_source_text(state: &PreservationState) -> BTreeSet<String> {
    let mut result = BTreeSet::new();
    let Some(root) = state.sections.iter().flat_map(|section| &section.children).find_map(|node| match node {
        PreservedNode::Element(root) if root.name.local_name == "FinalDraft" => Some(root), _ => None,
    }) else { return result };
    let Some(list) = child_index(root, "ListItems").and_then(|index| match &root.children[index] {
        PreservedNode::Element(list) => Some(list), _ => None,
    }) else { return result };
    for node in &list.children {
        let PreservedNode::Element(item) = node else { continue };
        if item.name.local_name != "ListItem" || item.name.namespace_uri != list.name.namespace_uri { continue; }
        let Some(content) = child_index(item, "Content").and_then(|index| match &item.children[index] {
            PreservedNode::Element(content) => Some(content), _ => None,
        }) else { continue };
        for node in &content.children {
            let PreservedNode::Element(paragraph) = node else { continue };
            if paragraph.name.local_name != "Paragraph" || paragraph.name.namespace_uri != content.name.namespace_uri { continue; }
            let Some(id) = identity(paragraph) else { continue };
            if paragraph.children.iter().any(|node| match node {
                PreservedNode::Element(text) if text.name.local_name == "Text"
                    && text.name.namespace_uri == paragraph.name.namespace_uri => text.children.iter().any(|child| {
                        matches!(child, PreservedNode::Text(value) | PreservedNode::CData(value) if !value.is_empty())
                    }),
                _ => false,
            }) { result.insert(id.to_owned()); }
        }
    }
    result
}

fn reconcile_beat_text(paragraph: &mut PreservedElement, semantic: &BeatParagraph, names: &RunNames) -> Result<(), FdxError> {
    let source_runs = crate::content::semantic_text_runs(&raw_element(paragraph), false, false)?;
    if source_runs == semantic.runs { return Ok(()); }
    let templates = paragraph.children.iter().filter_map(|node| match node {
        PreservedNode::Element(text) if text.name.local_name == "Text"
            && text.name.namespace_uri == paragraph.name.namespace_uri => Some(text.clone()),
        _ => None,
    }).collect::<Vec<_>>();
    let mut runs = semantic.runs.iter();
    let mut children = Vec::new();
    for node in std::mem::take(&mut paragraph.children) {
        let PreservedNode::Element(mut text) = node else { children.push(node); continue };
        if text.name.local_name != "Text" || text.name.namespace_uri != paragraph.name.namespace_uri {
            children.push(PreservedNode::Element(text));
            continue;
        }
        if let Some(run) = runs.next() {
            write_beat_run(&mut text, run, &templates, names);
            children.push(PreservedNode::Element(text));
        } else {
            // A discarded semantic run does not discard unrelated inline payload.
            text.children = text.children.into_iter().filter(|node| !matches!(node,
                PreservedNode::Text(_) | PreservedNode::CData(_))).collect::<Vec<_>>().into();
            if !text.children.is_empty() || !text.namespace_bindings.is_empty() || !text.attributes.is_empty() {
                children.push(PreservedNode::Element(text));
            }
        }
    }
    for run in runs {
        let mut text = new_lineage_element(&paragraph.name, "Text", Vec::new());
        write_beat_run(&mut text, run, &templates, names);
        children.push(PreservedNode::Element(text));
    }
    paragraph.children = children.into();
    Ok(())
}

// The semantic run is authoritative. Reuse source names and namespace bindings
// when available, but never require a matching source occurrence for an edit.
fn write_beat_run(text: &mut PreservedElement, run: &scriptcraft_model::TextRun, templates: &[PreservedElement], names: &RunNames) {
    text.attributes = run.attributes.iter().map(|attribute| {
        let source = templates.iter().flat_map(|template| &template.attributes).find(|source| {
            source.name.qualified_name.as_deref().unwrap_or(&source.name.local_name) == attribute.name
        });
        PreservedAttribute {
            name: source.map(|source| source.name.clone()).or_else(|| names.get(&attribute.name).cloned())
                .unwrap_or_else(|| plain_source_name(&attribute.name)),
            value: attribute.value.clone(),
        }
    }).collect();
    text.source_type = text.attributes.iter().find(|attribute| {
        attribute.name.namespace_uri.is_none() && attribute.name.local_name == "Type"
    }).map(|attribute| attribute.value.clone());
    for attribute in &text.attributes {
        if let (Some((prefix, _)), Some(uri)) = (
            attribute.name.qualified_name.as_deref().and_then(|name| name.split_once(':')),
            attribute.name.namespace_uri.as_ref(),
        ) {
            if prefix != "xml" && !text.namespace_bindings.iter().any(|binding| binding.prefix == prefix) {
                text.namespace_bindings.push(NamespaceBinding { prefix: prefix.to_owned(), namespace_uri: uri.clone() });
            }
        }
    }
    for template in templates {
        for binding in &template.namespace_bindings {
            if run.attributes.iter().any(|attribute| attribute.name.starts_with(&format!("{}:", binding.prefix)))
                && !text.namespace_bindings.iter().any(|existing| existing.prefix == binding.prefix) {
                text.namespace_bindings.push(binding.clone());
            }
        }
    }
    let mut inserted = false;
    let mut children = Vec::new();
    for child in std::mem::take(&mut text.children) {
        match child {
            PreservedNode::Text(_) | PreservedNode::CData(_) => {
                if !inserted { children.push(PreservedNode::Text(run.text.clone().into())); inserted = true; }
            }
            other => children.push(other),
        }
    }
    if !inserted { children.insert(0, PreservedNode::Text(run.text.clone().into())); }
    text.children = children.into();
}

fn reconcile_card(
    source: &mut PreservedElement, card: &BeatCard, state: &PreservationState,
    changes: &mut ParagraphIdentityChanges, names: &RunNames,
) -> Result<(), FdxError> {
    set_attribute(source, "Title", card.title.clone())?;
    if child_index(source, "Content").is_none() {
        source.children.push(PreservedNode::Element(new_lineage_element(&source.name, "Content", Vec::new())));
    }
    let content_index = child_index(source, "Content").ok_or(FdxError::InvalidValue)?;
    let content = child_mut(source, content_index)?;
    let indexes = content.children.iter().enumerate().filter_map(|(index, node)| match node {
        PreservedNode::Element(element) if element.name.local_name == "Paragraph"
            && element.name.namespace_uri == content.name.namespace_uri => Some(index),
        _ => None,
    }).collect::<Vec<_>>();
    let mut retained = BTreeMap::new();
    for &index in &indexes {
        let PreservedNode::Element(paragraph) = &content.children[index] else { return Err(FdxError::InvalidValue); };
        // A surviving imported id-less paragraph was bound during finalization.
        // Any remaining id-less node is outside the semantic paragraph sequence.
        if let Some(id) = identity(paragraph) {
            if retained.insert(id.to_owned(), paragraph.clone()).is_some() { return Err(FdxError::AmbiguousIdentity); }
        }
    }
    let mut desired = Vec::with_capacity(card.paragraphs.len());
    for paragraph in &card.paragraphs {
        let id = paragraph.foreign_id.as_deref().ok_or(FdxError::AmbiguousIdentity)?;
        let node = if let Some(mut source) = retained.remove(id) {
            reconcile_paragraph_settings(&mut source, paragraph)?;
            reconcile_beat_text(&mut source, paragraph, names)?;
            source
        } else {
            changes.added.push(paragraph_identity(id)?);
            generated_paragraph(&content.name, paragraph, names)?
        };
        desired.push(PreservedNode::Element(node));
    }
    for id in retained.keys() {
        let id = paragraph_identity(id)?;
        ensure_unreferenced(state, &id)?;
        changes.removed.insert(id);
    }
    let slots = indexes.iter().copied().collect::<BTreeSet<_>>();
    let last = indexes.last().copied();
    let mut desired = desired.into_iter();
    let mut children = Vec::new();
    for (index, node) in std::mem::take(&mut content.children).into_iter().enumerate() {
        if slots.contains(&index) {
            if let Some(replacement) = desired.next() { children.push(replacement); }
        } else {
            children.push(node);
        }
        if Some(index) == last { children.extend(desired.by_ref()); }
    }
    if last.is_none() { children.extend(desired); }
    content.children = children.into();
    Ok(())
}

fn reconcile_existing_board(
    sections: &mut [PreservedSection], project: &Project, context: &SaveContext,
    state: &PreservationState, original: &BeatBoard, original_state: &PreservationState,
    transactional: bool, names: &RunNames,
) -> Result<ParagraphIdentityChanges, FdxError> {
    let mut roots = sections.iter_mut().flat_map(|section| &mut section.children).filter_map(|node| match node {
        PreservedNode::Element(root) if root.name.local_name == "FinalDraft" => Some(root),
        _ => None,
    });
    let root = roots.next().ok_or(FdxError::InvalidRoot)?;
    if roots.next().is_some() { return Err(FdxError::InvalidRoot); }
    let board_index = child_index(root, "DisplayBoards").and_then(|index| match &root.children[index] {
        PreservedNode::Element(boards) => boards.children.iter().position(|node| matches!(node,
            PreservedNode::Element(board) if board.name.local_name == "DisplayBoard"
                && board.name.namespace_uri == boards.name.namespace_uri && attribute(board, "Type") == Some("Beat"))),
        _ => None,
    });
    if board_index.is_some() != original.foreign_id.is_some() {
        return Err(FdxError::AmbiguousIdentity);
    }
    if board_index.is_none() { append_missing_board(root, project, context)?; }
    let boards_index = child_index(root, "DisplayBoards").ok_or(FdxError::InvalidValue)?;
    let board_index = match board_index {
        Some(index) => index,
        None => child_mut(root, boards_index)?.children.len().checked_sub(1).ok_or(FdxError::InvalidValue)?,
    };
    let semantic = project.beat_board();
    let managed = semantic.cards.iter().map(|card| (card.foreign_id.as_str(), card)).collect::<BTreeMap<_, _>>();
    if managed.len() != semantic.cards.len() { return Err(FdxError::AmbiguousIdentity); }
    let creation_order = cards_in_creation_order(semantic)?;
    let mut changes = ParagraphIdentityChanges::default();
    let mut list_items = BTreeMap::new();
    if let Some(index) = child_index(root, "ListItems") {
        let list = child_mut(root, index)?;
        for node in &list.children {
            let PreservedNode::Element(item) = node else { continue };
            if item.name.local_name != "ListItem" || item.name.namespace_uri != list.name.namespace_uri { continue; }
            if let Some(id) = identity(item) {
                if list_items.insert(id.to_owned(), item.clone()).is_some() { return Err(FdxError::AmbiguousIdentity); }
            } else if item.attributes.iter().filter(|attribute| attribute.name.namespace_uri.is_none()
                && attribute.name.local_name.eq_ignore_ascii_case("id")).count() > 1 {
                return Err(FdxError::AmbiguousIdentity);
            }
        }
    }
    let mut retained_items = BTreeMap::new();
    let mut slots = BTreeSet::new();
    {
        let boards = child_mut(root, boards_index)?;
        let board = child_mut(boards, board_index)?;
        if identity(board) != semantic.foreign_id.as_deref() { return Err(FdxError::AmbiguousIdentity); }
        if original.foreign_id.is_some() && identity(board) != original.foreign_id.as_deref() {
            return Err(FdxError::AmbiguousIdentity);
        }
        set_attribute(board, "Width", decimal(semantic.width))?;
        set_attribute(board, "Height", decimal(semantic.height))?;
        if transactional {
            let view = context.beat_board_view_state();
            let origin = view.scroll_origin();
            if origin.x > semantic.width || origin.y > semantic.height { return Err(FdxError::InvalidValue); }
            set_attribute(board, "ScrollOrigin", format!("{},{}", decimal(origin.x), decimal(origin.y)))?;
            let zoom = view.zoom_milli_percent();
            set_attribute(board, "ZoomLevel", format!("{}.{:03}", zoom / 1_000, zoom % 1_000))?;
        }
        for (index, node) in board.children.iter().enumerate() {
            let PreservedNode::Element(item) = node else { continue };
            if item.name.local_name != "Item" || item.name.namespace_uri != board.name.namespace_uri { continue; }
            let Some(id) = identity(item) else { return Err(FdxError::DanglingReference); };
            let list_item = list_items.get(id).ok_or(FdxError::DanglingReference)?;
            if attribute(list_item, "Type") == Some("Beat") {
                slots.insert(index);
                if retained_items.insert(id.to_owned(), item.clone()).is_some() { return Err(FdxError::AmbiguousIdentity); }
            }
        }
    }
    let original_cards = original.cards.iter().map(|card| card.foreign_id.as_str()).collect::<BTreeSet<_>>();
    let source_cards = retained_items.keys().map(String::as_str).collect::<BTreeSet<_>>();
    if original_cards.len() != original.cards.len() || original_cards != source_cards {
        return Err(FdxError::AmbiguousIdentity);
    }
    let original_root = original_state.sections.iter().flat_map(|section| &section.children).find_map(|node| match node {
        PreservedNode::Element(root) if root.name.local_name == "FinalDraft" => Some(root),
        _ => None,
    }).ok_or(FdxError::InvalidRoot)?;
    // The retained list is the baseline, not an editable creation-order ledger.
    // Require the entire supported ListItem sequence to remain in source order;
    // only the detached reconciliation below may append or remove managed cards.
    let list_ids = |root: &PreservedElement| -> Result<Vec<Option<String>>, FdxError> {
        let Some(index) = child_index(root, "ListItems") else { return Ok(Vec::new()) };
        let PreservedNode::Element(list) = &root.children[index] else { return Err(FdxError::InvalidValue) };
        list.children.iter().filter_map(|node| match node {
            PreservedNode::Element(item) if item.name.local_name == "ListItem"
                && item.name.namespace_uri == list.name.namespace_uri => Some(item),
            _ => None,
        }).map(|item| {
            if item.attributes.iter().filter(|attribute| attribute.name.namespace_uri.is_none()
                && attribute.name.local_name.eq_ignore_ascii_case("id")).count() > 1 {
                Err(FdxError::AmbiguousIdentity)
            } else {
                Ok(identity(item).map(str::to_owned))
            }
        }).collect()
    };
    let original_list_order = list_ids(original_root)?;
    if original_list_order != list_ids(root)? { return Err(FdxError::AmbiguousIdentity); }
    let surviving_source_order = original_list_order.iter().filter_map(Option::as_deref)
        .filter(|id| original_cards.contains(*id) && managed.contains_key(*id))
        .collect::<Vec<_>>();
    let mut ledger_source_order = Vec::new();
    let mut saw_new = false;
    for card in &creation_order {
        if original_cards.contains(card.foreign_id.as_str()) {
            if saw_new { return Err(FdxError::AmbiguousIdentity); }
            ledger_source_order.push(card.foreign_id.as_str());
        } else {
            saw_new = true;
        }
    }
    if ledger_source_order != surviving_source_order { return Err(FdxError::AmbiguousIdentity); }
    let removed_cards = retained_items.keys().filter(|id| !managed.contains_key(id.as_str())).cloned().collect::<BTreeSet<_>>();
    let board_id = ForeignIdentity::new(ForeignIdDomain::Board, semantic.foreign_id.clone().ok_or(FdxError::AmbiguousIdentity)?)
        .map_err(map_preservation_error)?;
    for id in &removed_cards {
        let card_id = ForeignIdentity::new(ForeignIdDomain::ListItem, id.clone()).map_err(map_preservation_error)?;
        if state.references.iter().any(|reference| reference.target == card_id)
            || state.board_graph.references.iter().any(|reference| reference.target == card_id && reference.source != board_id) {
            return Err(FdxError::DanglingReference);
        }
        let item = list_items.get(id).ok_or(FdxError::DanglingReference)?;
        if let Some(index) = child_index(item, "Content") {
            let PreservedNode::Element(content) = &item.children[index] else { return Err(FdxError::InvalidValue); };
            for node in &content.children {
                let PreservedNode::Element(paragraph) = node else { continue };
                if paragraph.name.local_name != "Paragraph" || paragraph.name.namespace_uri != content.name.namespace_uri { continue; }
                let Some(id) = identity(paragraph) else { continue };
                let id = paragraph_identity(id)?;
                ensure_unreferenced(state, &id)?;
                if !changes.removed.insert(id) { return Err(FdxError::AmbiguousIdentity); }
            }
        }
    }
    {
        let boards = child_mut(root, boards_index)?;
        let board = child_mut(boards, board_index)?;
        let mut desired_order = Vec::new();
        for card in &semantic.cards {
            let Some(mut item) = retained_items.remove(&card.foreign_id) else {
                if list_items.contains_key(&card.foreign_id) { return Err(FdxError::AmbiguousIdentity); }
                desired_order.push(PreservedNode::Element(generated_item(&board.name, card)));
                continue;
            };
            set_attribute(&mut item, "Left", decimal(card.rect.left))?;
            set_attribute(&mut item, "Top", decimal(card.rect.top))?;
            set_attribute(&mut item, "Width", decimal(card.rect.width))?;
            set_attribute(&mut item, "Height", decimal(card.rect.height))?;
            desired_order.push(PreservedNode::Element(item));
        }
        // Managed slots are stacking slots, not ownership claims. Retained
        // Items are selected by durable identity above; generated Items may
        // occupy any slot after later activation/reordering. Opaque siblings
        // remain in their original relative order and excess Items append.
        let mut replacements = desired_order.into_iter();
        let mut children = Vec::new();
        for (index, node) in std::mem::take(&mut board.children).into_iter().enumerate() {
            if slots.contains(&index) {
                if let Some(replacement) = replacements.next() { children.push(replacement); }
            } else { children.push(node); }
        }
        children.extend(replacements);
        board.children = children.into();
    }
    if child_index(root, "ListItems").is_none() {
        root.children.push(PreservedNode::Element(new_lineage_element(&root.name, "ListItems", Vec::new())));
    }
    let list_index = child_index(root, "ListItems").ok_or(FdxError::InvalidValue)?;
    let list = child_mut(root, list_index)?;
    let mut seen = BTreeSet::new();
    let mut children = Vec::new();
    for node in std::mem::take(&mut list.children) {
        let PreservedNode::Element(mut item) = node else { children.push(node); continue };
        if item.name.local_name == "ListItem" && item.name.namespace_uri == list.name.namespace_uri {
            let Some(id) = identity(&item).map(str::to_owned) else {
                children.push(PreservedNode::Element(item));
                continue;
            };
            if removed_cards.contains(&id) { continue; }
            if let Some(card) = managed.get(id.as_str()) {
                if attribute(&item, "Type") != Some("Beat") || !seen.insert(id) { return Err(FdxError::AmbiguousIdentity); }
                reconcile_card(&mut item, card, state, &mut changes, names)?;
            }
        }
        children.push(PreservedNode::Element(item));
    }
    for card in creation_order {
        if seen.contains(&card.foreign_id) { continue; }
        for paragraph in &card.paragraphs {
            changes.added.push(paragraph_identity(paragraph.foreign_id.as_deref().ok_or(FdxError::AmbiguousIdentity)?)?);
        }
        children.push(PreservedNode::Element(generated_card(&list.name, card, names)?));
    }
    list.children = children.into();
    Ok(changes)
}
