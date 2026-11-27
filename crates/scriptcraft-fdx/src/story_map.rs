//! Bounded, source-preserving Story Map import and disposable view state.

use crate::diagnostics::{map_model_error, FdxError};
use crate::document::StoryMapViewState;
use crate::reader::RawElement;
use scriptcraft_model::preservation::{PreservedElement, PreservedNode};
use scriptcraft_model::{
    BeatBoard, BoardFixed, Project, StoryMap, StoryMapLane, StoryMapPlacement,
    DEFAULT_STORY_MAP_TARGET_PAGES, MAX_BOARD_DIMENSION, MAX_STORY_MAP_LANES,
    MAX_STORY_MAP_STRING_BYTES, MAX_STORY_MAP_TARGET_PAGES, MAX_TEXT_BYTES,
};
use std::collections::BTreeSet;

fn children<'a>(
    parent: &'a PreservedElement,
    name: &'a str,
    namespace: Option<&'a str>,
) -> impl Iterator<Item = &'a PreservedElement> {
    parent.children.iter().filter_map(move |node| match node {
        PreservedNode::Element(element)
            if element.name.local_name == name
                && element.name.namespace_uri.as_deref() == namespace =>
        {
            Some(element)
        }
        _ => None,
    })
}

fn attribute<'a>(element: &'a PreservedElement, name: &str) -> Option<&'a str> {
    let mut matches = element.attributes.iter().filter(|attribute| {
        attribute.name.namespace_uri.is_none() && attribute.name.local_name == name
    });
    let value = matches.next()?.value.as_str();
    matches.next().is_none().then_some(value)
}

fn attribute_values<'a>(element: &'a PreservedElement, name: &str) -> Vec<&'a str> {
    element
        .attributes
        .iter()
        .filter(|attribute| {
            attribute.name.namespace_uri.is_none() && attribute.name.local_name == name
        })
        .map(|attribute| attribute.value.as_str())
        .collect()
}

fn identity_attribute(element: &PreservedElement) -> Option<&str> {
    let mut matches = element.attributes.iter().filter(|attribute| {
        attribute.name.namespace_uri.is_none()
            && attribute.name.local_name.eq_ignore_ascii_case("Id")
    });
    let value = matches.next()?.value.as_str();
    matches.next().is_none().then_some(value)
}

fn retained_root<'a>(root: &RawElement, project: &'a Project) -> Result<&'a PreservedElement, FdxError> {
    let preservation = project.preservation.as_ref().ok_or(FdxError::InvalidValue)?;
    let mut roots = preservation
        .sections
        .iter()
        .flat_map(|section| &section.children)
        .filter_map(|node| match node {
            PreservedNode::Element(element) if element.name == root.name => Some(element),
            _ => None,
        });
    let retained = roots.next().ok_or(FdxError::InvalidRoot)?;
    if roots.next().is_some() {
        return Err(FdxError::InvalidRoot);
    }
    Ok(retained)
}

fn winning_board<'a>(root: &'a PreservedElement) -> Option<&'a PreservedElement> {
    let namespace = root.name.namespace_uri.as_deref();
    let boards = children(root, "DisplayBoards", namespace).next()?;
    children(boards, "DisplayBoard", namespace).find(|board| {
        let types = attribute_values(board, "Type");
        types.len() == 1 && types[0] == "StoryMap"
    })
}

fn target_pages(root: &PreservedElement) -> u32 {
    let namespace = root.name.namespace_uri.as_deref();
    let mut targets = children(root, "TargetScriptLength", namespace);
    let Some(target) = targets.next() else {
        return DEFAULT_STORY_MAP_TARGET_PAGES;
    };
    if targets.next().is_some() {
        return DEFAULT_STORY_MAP_TARGET_PAGES;
    }
    let mut text = String::new();
    for child in &target.children {
        match child {
            PreservedNode::Text(value) | PreservedNode::CData(value) => text.push_str(value),
            PreservedNode::Element(_)
            | PreservedNode::Comment(_)
            | PreservedNode::ProcessingInstruction { .. } => {
                return DEFAULT_STORY_MAP_TARGET_PAGES;
            }
        }
    }
    text.trim()
        .parse::<u32>()
        .ok()
        .filter(|value| (1..=MAX_STORY_MAP_TARGET_PAGES).contains(value))
        .unwrap_or(DEFAULT_STORY_MAP_TARGET_PAGES)
}

fn valid_lane_text(value: &str, allow_empty: bool) -> bool {
    (allow_empty || !value.is_empty())
        && value.len() <= MAX_STORY_MAP_STRING_BYTES
        && crate::reader::validate_xml_chars(value).is_ok()
}

fn lane_collection(board: Option<&PreservedElement>) -> Option<Vec<StoryMapLane>> {
    let board = board?;
    let namespace = board.name.namespace_uri.as_deref();
    let container = children(board, "Lanes", namespace).next()?;
    let source_lanes = children(container, "Lane", namespace).collect::<Vec<_>>();
    if source_lanes.is_empty() || source_lanes.len() > MAX_STORY_MAP_LANES {
        return None;
    }
    let mut lanes = Vec::with_capacity(source_lanes.len());
    let mut uuids = BTreeSet::new();
    for lane in source_lanes {
        let uuid = identity_attribute(lane)?;
        let label = attribute(lane, "Label")?;
        let level = attribute(lane, "Level")?.parse::<i64>().ok()?;
        let lane_type = attribute(lane, "Type")?;
        if !valid_lane_text(uuid, false)
            || !valid_lane_text(label, true)
            || !valid_lane_text(lane_type, false)
            || !uuids.insert(uuid.to_owned())
        {
            return None;
        }
        lanes.push(StoryMapLane {
            uuid: uuid.to_owned(),
            label: label.to_owned(),
            level,
            lane_type: lane_type.to_owned(),
        });
    }
    let aggregate_bytes = lanes.iter().try_fold(
        identity_attribute(board).map_or(0usize, str::len),
        |total, lane| {
            total
                .checked_add(lane.uuid.len())?
                .checked_add(lane.label.len())?
                .checked_add(lane.lane_type.len())
        },
    )?;
    if aggregate_bytes > MAX_TEXT_BYTES {
        return None;
    }
    Some(lanes)
}

fn collect_source_ids(root: &PreservedElement, reserved: &mut BTreeSet<String>) {
    fn visit(element: &PreservedElement, reserved: &mut BTreeSet<String>) {
        for attribute in &element.attributes {
            if attribute.name.namespace_uri.is_none()
                && attribute.name.local_name.eq_ignore_ascii_case("Id")
            {
                reserved.insert(attribute.value.clone());
            }
        }
        for child in &element.children {
            if let PreservedNode::Element(element) = child {
                visit(element, reserved);
            }
        }
    }
    visit(root, reserved);
}

fn allocate_uuid(reserved: &mut BTreeSet<String>) -> Result<String, FdxError> {
    for _ in 0..16 {
        let mut bytes = [0_u8; 16];
        getrandom::fill(&mut bytes).map_err(|_| FdxError::InvalidValue)?;
        bytes[6] = (bytes[6] & 0x0f) | 0x40;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        let candidate = uuid::Uuid::from_bytes(bytes).hyphenated().to_string();
        if reserved.insert(candidate.clone()) {
            return Ok(candidate);
        }
    }
    Err(FdxError::InvalidValue)
}

fn default_lanes(reserved: &mut BTreeSet<String>) -> Result<Vec<StoryMapLane>, FdxError> {
    let mut lanes = Vec::with_capacity(2);
    for (label, level) in [("Outline 1", 7), ("Outline 2", 5)] {
        lanes.push(StoryMapLane {
            uuid: allocate_uuid(reserved)?,
            label: label.to_owned(),
            level,
            lane_type: label.to_owned(),
        });
    }
    Ok(lanes)
}

fn decimal_eighths(value: &str) -> Option<u64> {
    let (mantissa, exponent) = match value.find(|character| matches!(character, 'e' | 'E')) {
        Some(index) => {
            if value[index + 1..]
                .bytes()
                .any(|byte| matches!(byte, b'e' | b'E'))
            {
                return None;
            }
            let exponent = value[index + 1..].parse::<i32>().ok()?;
            (&value[..index], exponent)
        }
        None => (value, 0),
    };
    if mantissa.starts_with('-') {
        return None;
    }
    let mantissa = mantissa.strip_prefix('+').unwrap_or(mantissa);
    let mut pieces = mantissa.split('.');
    let whole = pieces.next()?;
    let fraction = pieces.next().unwrap_or("");
    if pieces.next().is_some()
        || (whole.is_empty() && fraction.is_empty())
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let digits = format!("{whole}{fraction}");
    let coefficient = digits.parse::<u128>().ok()?;
    let scale = i32::try_from(fraction.len()).ok()?.checked_sub(exponent)?;
    let eighths = if scale > 0 {
        let divisor = 10_u128.checked_pow(u32::try_from(scale).ok()?)?;
        let numerator = coefficient.checked_mul(8)?;
        if numerator % divisor != 0 {
            return None;
        }
        numerator / divisor
    } else {
        coefficient
            .checked_mul(10_u128.checked_pow(scale.unsigned_abs())?)?
            .checked_mul(8)?
    };
    u64::try_from(eighths).ok()
}

fn page_goal_placement(
    list_item: &PreservedElement,
    namespace: Option<&str>,
    lanes: &[StoryMapLane],
    target_eighths: u64,
) -> Option<StoryMapPlacement> {
    let page_goals = children(list_item, "SubordinateTo", namespace)
        .filter(|node| {
            attribute_values(node, "Type")
                .iter()
                .any(|value| *value == "PageGoal")
        })
        .collect::<Vec<_>>();
    if page_goals.len() != 1 {
        return None;
    }
    let goal = page_goals[0];
    let types = attribute_values(goal, "Type");
    if types.len() != 1 || types[0] != "PageGoal" || attribute(goal, "Order")? != "0" {
        return None;
    }
    let lane_type = attribute(goal, "Level")?;
    let mut matching_lanes = lanes.iter().filter(|lane| lane.lane_type == lane_type);
    let lane = matching_lanes.next()?;
    if matching_lanes.next().is_some() {
        return None;
    }
    let start_eighths = decimal_eighths(attribute(goal, "Number")?)?;
    let length_eighths = decimal_eighths(attribute(goal, "Length")?)?;
    if length_eighths == 0 {
        return None;
    }
    let end = start_eighths.checked_add(length_eighths)?;
    if end > target_eighths {
        return None;
    }
    Some(StoryMapPlacement {
        lane_uuid: lane.uuid.clone(),
        start_eighths,
        length_eighths,
    })
}

fn fixed_milli(value: &str, positive: bool) -> Option<i64> {
    let value = value.trim().parse::<f64>().ok()?;
    let maximum = MAX_BOARD_DIMENSION.raw() as f64 / 1_000.0;
    if !value.is_finite() || (positive && value <= 0.0) || value < 0.0 || value > maximum {
        return None;
    }
    let raw = (value * 1_000.0).round();
    (raw >= 0.0
        && raw <= MAX_BOARD_DIMENSION.raw() as f64
        && (!positive || raw >= 1.0))
    .then_some(raw as i64)
}

fn view_state(board: Option<&PreservedElement>) -> StoryMapViewState {
    let Some(board) = board else {
        return StoryMapViewState::default();
    };
    let height = attribute(board, "Height")
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|value| (90..=1_000_000).contains(value))
        .unwrap_or(90);
    let width = attribute(board, "Width")
        .and_then(|value| fixed_milli(value, true))
        .map(BoardFixed::from_milli_pixels);
    let origin = attribute(board, "ScrollOrigin")
        .and_then(|value| value.split_once(',').map(|(x, _)| x))
        .and_then(|value| fixed_milli(value, false))
        .map(BoardFixed::from_milli_pixels)
        .unwrap_or_default();
    let zoom = attribute(board, "ZoomLevel")
        .and_then(|value| value.trim().parse::<f64>().ok())
        .filter(|value| {
            value.is_finite()
                && *value > 0.0
                && *value <= 120_000.0
                && (*value * 1_000.0).round() >= 1.0
        })
        .map(|value| (value * 1_000.0).round() as u32)
        .unwrap_or(100_000);
    StoryMapViewState::new(height, width, origin, zoom).unwrap_or_default()
}

fn retained_list_item<'a>(
    root: &'a PreservedElement,
    foreign_id: &str,
) -> Option<&'a PreservedElement> {
    let namespace = root.name.namespace_uri.as_deref();
    let list_items = children(root, "ListItems", namespace).next()?;
    children(list_items, "ListItem", namespace).find(|item| {
        let ids = item
            .attributes
            .iter()
            .filter(|attribute| {
                attribute.name.namespace_uri.is_none()
                    && attribute.name.local_name.eq_ignore_ascii_case("Id")
            })
            .map(|attribute| attribute.value.as_str())
            .collect::<Vec<_>>();
        ids.len() == 1 && ids[0] == foreign_id
    })
}

/// Imports the winner's semantics onto the existing Beat cards and returns its
/// disposable view value. The retained tree is read-only throughout this path.
pub(crate) fn import(
    raw_root: &RawElement,
    project: &mut Project,
    beat_board: &mut BeatBoard,
) -> Result<StoryMapViewState, FdxError> {
    let root = retained_root(raw_root, project)?.clone();
    let board = winning_board(&root);
    let target_pages = target_pages(&root);
    let mut reserved = BTreeSet::new();
    collect_source_ids(&root, &mut reserved);
    for element in &project.elements {
        reserved.insert(element.id.as_str().to_owned());
    }
    for group in &project.dual_dialogue_groups {
        reserved.insert(group.id.as_str().to_owned());
    }
    for paragraph in &project.title_page {
        reserved.insert(paragraph.id.as_str().to_owned());
    }
    for card in &beat_board.cards {
        reserved.insert(card.id.as_str().to_owned());
        reserved.insert(card.foreign_id.clone());
        for paragraph in &card.paragraphs {
            reserved.insert(paragraph.id.as_str().to_owned());
            reserved.extend(paragraph.foreign_id.iter().cloned());
        }
    }
    for note in project.script_notes().iter() {
        reserved.insert(note.id.as_str().to_owned());
        for paragraph in &note.paragraphs {
            reserved.insert(paragraph.id.as_str().to_owned());
            reserved.extend(paragraph.durable_id.iter().cloned());
        }
    }
    if let Some(id) = &project.story_map().foreign_id {
        reserved.insert(id.clone());
    }
    for lane in &project.story_map().lanes {
        reserved.insert(lane.uuid.clone());
    }
    if let Some(id) = board.and_then(identity_attribute) {
        reserved.insert(id.to_owned());
    }
    reserved.extend(beat_board.foreign_id.iter().cloned());
    let lanes = match lane_collection(board) {
        Some(lanes) => lanes,
        None => default_lanes(&mut reserved)?,
    };
    let story_map = StoryMap {
        foreign_id: board.and_then(identity_attribute).map(str::to_owned),
        target_pages,
        lanes,
    };
    project
        .replace_story_map(story_map)
        .map_err(map_model_error)?;

    let namespace = root.name.namespace_uri.as_deref();
    let target_eighths = target_pages
        .checked_mul(8)
        .map(u64::from)
        .ok_or(FdxError::InvalidValue)?;
    for card in &mut beat_board.cards {
        card.story_map_placement = retained_list_item(&root, &card.foreign_id)
            .and_then(|item| page_goal_placement(item, namespace, &project.story_map().lanes, target_eighths));
    }
    Ok(view_state(board))
}
