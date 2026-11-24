//! Raw, exact-path Beat import. Detached output reconciliation belongs to WO-003.

use crate::content::direct_paragraph_overrides;
use crate::diagnostics::{map_model_error, FdxError};
use crate::document::BeatBoardViewState;
use crate::identity::raw_identity_attribute;
use crate::reader::{attribute, RawElement, RawNode};
use scriptcraft_model::preservation::{PreservedElement, PreservedNode};
use scriptcraft_model::{
    BeatBoard, BeatCard, BeatParagraph, BeatPoint, BeatRect,
    BoardFixed, Project, RunAttribute, TextRun, DEFAULT_BOARD_HEIGHT,
    DEFAULT_BOARD_WIDTH, DEFAULT_CARD_HEIGHT, DEFAULT_CARD_WIDTH, MAX_BOARD_DIMENSION,
    MAX_CARD_HEIGHT, MAX_CARD_WIDTH, MIN_CARD_HEIGHT, MIN_CARD_WIDTH,
};
use std::collections::BTreeMap;

fn children<'a>(parent: &'a RawElement, name: &'a str, ns: Option<&'a str>) -> impl Iterator<Item = &'a RawElement> {
    parent.children.iter().filter_map(move |node| match node {
        RawNode::Element(element)
            if element.name.local_name == name && element.name.namespace_uri.as_deref() == ns => Some(element),
        _ => None,
    })
}

pub(crate) fn winning_board(root: &RawElement) -> Option<&RawElement> {
    let ns = root.name.namespace_uri.as_deref();
    let boards = children(root, "DisplayBoards", ns).next()?;
    children(boards, "DisplayBoard", ns).find(|board| attribute(board, "Type") == Some("Beat"))
}

/// Returns only supported winning-board ListItems, in Item stacking order.
pub(crate) fn winning_items<'a>(root: &'a RawElement) -> Result<Vec<(&'a RawElement, &'a RawElement)>, FdxError> {
    let ns = root.name.namespace_uri.as_deref();
    let Some(board) = winning_board(root) else { return Ok(Vec::new()) };
    let mut list_items = BTreeMap::new();
    if let Some(container) = children(root, "ListItems", ns).next() {
        for item in children(container, "ListItem", ns) {
            if let Some(id) = raw_identity_attribute(item) {
                if list_items.insert(id, item).is_some() {
                    return Err(FdxError::AmbiguousIdentity);
                }
            }
        }
    }
    let mut result = Vec::new();
    for item in children(board, "Item", ns) {
        let id = raw_identity_attribute(item).ok_or(FdxError::DanglingReference)?;
        let list_item = list_items.get(id).copied().ok_or(FdxError::DanglingReference)?;
        if attribute(list_item, "Type") != Some("Beat") {
            continue;
        }
        result.push((item, list_item));
    }
    Ok(result)
}

pub(crate) fn paragraphs<'a>(list_item: &'a RawElement, ns: Option<&'a str>) -> Vec<&'a RawElement> {
    children(list_item, "Content", ns).next()
        .map(|content| children(content, "Paragraph", ns).collect())
        .unwrap_or_default()
}

fn retained_children<'a>(parent: &'a PreservedElement, name: &'a str, ns: Option<&'a str>) -> impl Iterator<Item = &'a PreservedElement> {
    parent.children.iter().filter_map(move |node| match node {
        PreservedNode::Element(element)
            if element.name.local_name == name && element.name.namespace_uri.as_deref() == ns => Some(element),
        _ => None,
    })
}

fn retained_attribute<'a>(element: &'a PreservedElement, name: &str) -> Option<&'a str> {
    let mut matches = element.attributes.iter().filter(|attribute| {
        attribute.name.namespace_uri.is_none() && attribute.name.local_name == name
    });
    let value = matches.next()?.value.as_str();
    matches.next().is_none().then_some(value)
}

fn retained_identity(element: &PreservedElement) -> Option<&str> {
    let mut matches = element.attributes.iter().filter(|attribute| {
        attribute.name.namespace_uri.is_none() && attribute.name.local_name.eq_ignore_ascii_case("id")
    });
    let value = matches.next()?.value.as_str();
    matches.next().is_none().then_some(value)
}

fn retained_winning_items<'a>(root: &'a PreservedElement, board: &'a PreservedElement) -> Result<Vec<(&'a PreservedElement, &'a PreservedElement)>, FdxError> {
    let ns = root.name.namespace_uri.as_deref();
    let mut list_items = BTreeMap::new();
    if let Some(container) = retained_children(root, "ListItems", ns).next() {
        for item in retained_children(container, "ListItem", ns) {
            if let Some(id) = retained_identity(item) {
                if list_items.insert(id, item).is_some() {
                    return Err(FdxError::AmbiguousIdentity);
                }
            }
        }
    }
    let mut result = Vec::new();
    for item in retained_children(board, "Item", ns) {
        let id = retained_identity(item).ok_or(FdxError::DanglingReference)?;
        let list_item = list_items.get(id).copied().ok_or(FdxError::DanglingReference)?;
        if retained_attribute(list_item, "Type") == Some("Beat") {
            result.push((item, list_item));
        }
    }
    Ok(result)
}

/// Extract the same direct text and attributes as semantic Text import, without
/// flattening opaque inline descendants. Empty slots still consume an ordinal.
fn retained_text_run(element: &PreservedElement) -> Option<TextRun> {
    let mut text = String::new();
    for child in &element.children {
        match child {
            PreservedNode::Text(value) | PreservedNode::CData(value) => text.push_str(value),
            PreservedNode::Element(_) | PreservedNode::Comment(_)
            | PreservedNode::ProcessingInstruction { .. } => {}
        }
    }
    if text.is_empty() { return None; }
    let attributes = element.attributes.iter().map(|attribute| RunAttribute {
        name: attribute.name.qualified_name.clone()
            .unwrap_or_else(|| attribute.name.local_name.clone()),
        value: attribute.value.clone(),
    }).collect();
    Some(TextRun { text, attributes })
}

fn decimal_milli(value: &str) -> Option<i64> {
    let value = value.trim().parse::<f64>().ok()?;
    let scaled = value * 1_000.0;
    if !scaled.is_finite() || scaled < i64::MIN as f64 || scaled >= i64::MAX as f64 {
        return None;
    }
    Some(scaled.round() as i64)
}

fn dimension(element: &PreservedElement, name: &str, fallback: BoardFixed, minimum: BoardFixed) -> BoardFixed {
    retained_attribute(element, name).and_then(decimal_milli)
        .filter(|raw| *raw >= minimum.raw() && *raw <= MAX_BOARD_DIMENSION.raw())
        .map(BoardFixed::from_milli_pixels).unwrap_or(fallback)
}

fn rectangle(item: &PreservedElement, width: BoardFixed, height: BoardFixed) -> BeatRect {
    let size = |name, fallback: BoardFixed, minimum: BoardFixed, maximum: BoardFixed, bound: BoardFixed| {
        let raw = retained_attribute(item, name).and_then(decimal_milli).filter(|value| *value > 0)
            .unwrap_or(fallback.raw()).clamp(minimum.raw(), maximum.raw().min(bound.raw()));
        BoardFixed::from_milli_pixels(raw)
    };
    let card_width = size("Width", DEFAULT_CARD_WIDTH, MIN_CARD_WIDTH, MAX_CARD_WIDTH, width);
    let card_height = size("Height", DEFAULT_CARD_HEIGHT, MIN_CARD_HEIGHT, MAX_CARD_HEIGHT, height);
    let position = |name, bound: BoardFixed, size: BoardFixed| {
        BoardFixed::from_milli_pixels(retained_attribute(item, name).and_then(decimal_milli).unwrap_or(0)
            .clamp(0, bound.raw() - size.raw()))
    };
    BeatRect { left: position("Left", width, card_width), top: position("Top", height, card_height), width: card_width, height: card_height }
}

fn view_state(board: &PreservedElement, width: BoardFixed, height: BoardFixed) -> BeatBoardViewState {
    let coordinate = |value: &str, bound: BoardFixed| {
        let value = value.trim().parse::<f64>().ok()?;
        value.is_finite().then(|| (value.clamp(0.0, bound.raw() as f64 / 1_000.0) * 1_000.0).round() as i64)
    };
    let origin = retained_attribute(board, "ScrollOrigin").and_then(|value| {
        let (x, y) = value.split_once(',')?;
        Some(BeatPoint {
            x: BoardFixed::from_milli_pixels(coordinate(x, width)?),
            y: BoardFixed::from_milli_pixels(coordinate(y, height)?),
        })
    }).unwrap_or_default();
    let zoom = retained_attribute(board, "ZoomLevel").and_then(|value| value.trim().parse::<f64>().ok())
        .filter(|value| value.is_finite())
        .map(|value| (value.clamp(5.0, 300.0) * 1_000.0).round() as u32).unwrap_or(100_000);
    BeatBoardViewState::new(origin, zoom).unwrap_or_default()
}

pub(crate) fn import(root: &RawElement, project: &Project) -> Result<(BeatBoard, BeatBoardViewState), FdxError> {
    // Use the retained original for both shell selection and each Text slot;
    // raw source identity validation has already run before this import.
    let preservation = project.preservation.as_ref().ok_or(FdxError::InvalidValue)?;
    let mut roots = preservation.sections.iter().flat_map(|section| &section.children).filter_map(|node| match node {
        PreservedNode::Element(element) if element.name == root.name => Some(element),
        _ => None,
    });
    let retained_root = roots.next().ok_or(FdxError::InvalidRoot)?;
    if roots.next().is_some() { return Err(FdxError::InvalidRoot); }
    let ns = retained_root.name.namespace_uri.as_deref();
    let source = retained_children(retained_root, "DisplayBoards", ns).next()
        .and_then(|boards| retained_children(boards, "DisplayBoard", ns)
            .find(|board| retained_attribute(board, "Type") == Some("Beat")));
    let Some(source) = source else {
        return Ok((BeatBoard::default(), BeatBoardViewState::default()));
    };
    let items = retained_winning_items(retained_root, source)?;
    let smallest = BoardFixed::from_milli_pixels(1);
    let width = dimension(source, "Width", DEFAULT_BOARD_WIDTH, if items.is_empty() { smallest } else { MIN_CARD_WIDTH });
    let height = dimension(source, "Height", DEFAULT_BOARD_HEIGHT, if items.is_empty() { smallest } else { MIN_CARD_HEIGHT });
    let mut allocator = project.stable_id_allocator().map_err(map_model_error)?;
    let mut cards = Vec::new();
    for (item, list_item) in items {
        let id = allocator.allocate("beat-card").map_err(map_model_error)?;
        let foreign_id = retained_identity(list_item).ok_or(FdxError::DanglingReference)?.to_owned();
        let mut body = Vec::new();
        let paragraphs = retained_children(list_item, "Content", ns).next()
            .map(|content| retained_children(content, "Paragraph", ns).collect::<Vec<_>>())
            .unwrap_or_default();
        for (ordinal, paragraph) in paragraphs.into_iter().enumerate() {
            let paragraph_id = allocator.allocate("beat-paragraph").map_err(map_model_error)?;
            let mut runs = Vec::new();
            for text_element in retained_children(paragraph, "Text", ns) {
                if let Some(run) = retained_text_run(text_element) {
                    runs.push(run);
                }
            }
            let text = runs.iter().map(|run| run.text.as_str()).collect();
            body.push(BeatParagraph {
                id: paragraph_id,
                foreign_id: retained_identity(paragraph).map(str::to_owned),
                source_paragraph_ordinal: Some(ordinal),
                text, runs,
                direct_settings: direct_paragraph_overrides(&crate::beat_board_output::raw_element(paragraph)).unwrap_or_default(),
            });
        }
        if body.is_empty() {
            body.push(BeatParagraph {
                id: allocator.allocate("beat-paragraph").map_err(map_model_error)?,
                foreign_id: None, source_paragraph_ordinal: None,
                text: String::new(), runs: Vec::new(), direct_settings: Default::default(),
            });
        }
        cards.push(BeatCard { id, foreign_id, title: retained_attribute(list_item, "Title").unwrap_or("").to_owned(), paragraphs: body, rect: rectangle(item, width, height) });
    }
    let by_foreign_id = cards.iter().map(|card| (card.foreign_id.as_str(), card.id.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut card_creation_order = Vec::with_capacity(cards.len());
    if let Some(list) = retained_children(retained_root, "ListItems", ns).next() {
        for item in retained_children(list, "ListItem", ns) {
            if let Some(id) = retained_identity(item).and_then(|id| by_foreign_id.get(id)) {
                card_creation_order.push(id.clone());
            }
        }
    }
    if card_creation_order.len() != cards.len() { return Err(FdxError::AmbiguousIdentity); }
    Ok((BeatBoard { foreign_id: retained_identity(source).map(str::to_owned), width, height, cards, card_creation_order }, view_state(source, width, height)))
}
