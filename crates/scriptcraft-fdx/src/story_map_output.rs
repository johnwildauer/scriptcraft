//! Detached, bounded projection of Story Map semantics onto the retained FDX tree.

use crate::context::{SaveContext, SaveOperation};
use crate::diagnostics::FdxError;
use crate::document::StoryMapViewState;
use crate::tree::{new_lineage_element, plain_source_name};
use scriptcraft_model::preservation::{PreservedElement, PreservedNode, PreservedSection};
use scriptcraft_model::{BeatCard, BoardFixed, Project, StoryMapPlacement};
use std::collections::{BTreeMap, BTreeSet};

fn element(node: &PreservedNode) -> Option<&PreservedElement> {
    if let PreservedNode::Element(value) = node { Some(value) } else { None }
}

fn element_mut(node: &mut PreservedNode) -> Option<&mut PreservedElement> {
    if let PreservedNode::Element(value) = node { Some(value) } else { None }
}

fn is_child(element: &PreservedElement, name: &str, namespace: Option<&str>) -> bool {
    element.name.local_name == name && element.name.namespace_uri.as_deref() == namespace
}

fn attribute_values<'a>(element: &'a PreservedElement, name: &str) -> Vec<&'a str> {
    element.attributes.iter().filter(|attribute| {
        attribute.name.namespace_uri.is_none() && attribute.name.local_name == name
    }).map(|attribute| attribute.value.as_str()).collect()
}

fn unique_attribute<'a>(element: &'a PreservedElement, name: &str) -> Option<&'a str> {
    let mut values = attribute_values(element, name).into_iter();
    let value = values.next()?;
    values.next().is_none().then_some(value)
}

fn identity(element: &PreservedElement) -> Option<&str> {
    let mut values = element.attributes.iter().filter(|attribute| {
        attribute.name.namespace_uri.is_none() && attribute.name.local_name.eq_ignore_ascii_case("id")
    }).map(|attribute| attribute.value.as_str());
    let value = values.next()?;
    values.next().is_none().then_some(value)
}

fn set_attribute(element: &mut PreservedElement, name: &str, value: String) -> Result<(), FdxError> {
    let indexes = element.attributes.iter().enumerate().filter_map(|(index, attribute)| {
        (attribute.name.namespace_uri.is_none() && attribute.name.local_name == name).then_some(index)
    }).collect::<Vec<_>>();
    match indexes.as_slice() {
        [] => element.attributes.push(scriptcraft_model::preservation::PreservedAttribute {
            name: plain_source_name(name), value,
        }),
        [index] => element.attributes[*index].value = value,
        _ => return Err(FdxError::UnsupportedConstruct),
    }
    Ok(())
}

fn set_identity(element: &mut PreservedElement, value: &str) -> Result<(), FdxError> {
    let indexes = element.attributes.iter().enumerate().filter_map(|(index, attribute)| {
        (attribute.name.namespace_uri.is_none() && attribute.name.local_name.eq_ignore_ascii_case("id"))
            .then_some(index)
    }).collect::<Vec<_>>();
    match indexes.as_slice() {
        [] => set_attribute(element, "Id", value.to_owned()),
        [index] => { element.attributes[*index].value = value.to_owned(); Ok(()) }
        _ => Err(FdxError::UnsupportedConstruct),
    }
}

fn remove_attribute(element: &mut PreservedElement, name: &str) -> Result<(), FdxError> {
    let indexes = element.attributes.iter().enumerate().filter_map(|(index, attribute)| {
        (attribute.name.namespace_uri.is_none() && attribute.name.local_name == name).then_some(index)
    }).collect::<Vec<_>>();
    match indexes.as_slice() {
        [] => Ok(()),
        [index] => { element.attributes.remove(*index); Ok(()) }
        _ => Err(FdxError::UnsupportedConstruct),
    }
}

fn target_value(element: &PreservedElement) -> Result<String, FdxError> {
    let mut text = String::new();
    for node in &element.children {
        match node {
            PreservedNode::Text(value) | PreservedNode::CData(value) => text.push_str(value),
            _ => return Err(FdxError::UnsupportedConstruct),
        }
    }
    Ok(text)
}

fn project_target(root: &mut PreservedElement, project: &Project, original: Option<&Project>) -> Result<(), FdxError> {
    let namespace = root.name.namespace_uri.as_deref();
    let indexes = root.children.iter().enumerate().filter_map(|(index, node)| {
        element(node).filter(|child| is_child(child, "TargetScriptLength", namespace)).map(|_| index)
    }).collect::<Vec<_>>();
    let text = project.story_map().target_pages.to_string();
    match indexes.as_slice() {
        [] => {
            let mut target = new_lineage_element(&root.name, "TargetScriptLength", Vec::new());
            target.children.push(PreservedNode::Text(text.into()));
            let index = root.children.iter().position(|node| {
                element(node).is_some_and(|child| is_child(child, "DisplayBoards", namespace))
            }).unwrap_or(root.children.len());
            root.children.insert(index, PreservedNode::Element(target));
        }
        [index] => {
            let target = element_mut(root.children.get_mut(*index).ok_or(FdxError::InvalidValue)?)
                .ok_or(FdxError::InvalidValue)?;
            let source_text = target_value(target)?;
            if let Some(original) = original {
                let imported = source_text.trim().parse::<u32>().ok()
                    .filter(|value| (1..=scriptcraft_model::MAX_STORY_MAP_TARGET_PAGES).contains(value));
                if imported != Some(original.story_map().target_pages) {
                    return Err(FdxError::UnsupportedConstruct);
                }
            }
            target.children = vec![PreservedNode::Text(text.into())].into();
        }
        _ => return Err(FdxError::UnsupportedConstruct),
    }
    Ok(())
}

fn winning_board_index(root: &PreservedElement) -> Result<Option<(usize, usize)>, FdxError> {
    let namespace = root.name.namespace_uri.as_deref();
    let Some(container_index) = root.children.iter().position(|node| {
        element(node).is_some_and(|child| is_child(child, "DisplayBoards", namespace))
    }) else { return Ok(None) };
    let container = element(root.children.get(container_index).ok_or(FdxError::InvalidValue)?)
        .ok_or(FdxError::InvalidValue)?;
    for (board_index, node) in container.children.iter().enumerate() {
        let Some(board) = element(node).filter(|board| is_child(board, "DisplayBoard", namespace)) else { continue };
        let types = attribute_values(board, "Type");
        if types.iter().any(|value| *value == "StoryMap") {
            if types.len() != 1 { return Err(FdxError::UnsupportedConstruct); }
            return Ok(Some((container_index, board_index)));
        }
    }
    Ok(None)
}

pub(crate) fn decimal_milli(value: BoardFixed) -> String {
    let raw = value.raw();
    let whole = raw / 1_000;
    let fraction = raw % 1_000;
    if fraction == 0 { return whole.to_string(); }
    let mut text = format!("{whole}.{fraction:03}");
    while text.ends_with('0') { text.pop(); }
    text
}

pub(crate) fn decimal_eighths(value: u64) -> String {
    const FRACTIONS: [&str; 8] = ["", ".125", ".25", ".375", ".5", ".625", ".75", ".875"];
    format!("{}{}", value / 8, FRACTIONS[(value % 8) as usize])
}

fn parse_decimal_eighths(value: &str) -> Option<u64> {
    let (mantissa, exponent) = match value.find(|character| matches!(character, 'e' | 'E')) {
        Some(index) => (&value[..index], value[index + 1..].parse::<i32>().ok()?),
        None => (value, 0),
    };
    if mantissa.starts_with('-') { return None; }
    let mantissa = mantissa.strip_prefix('+').unwrap_or(mantissa);
    let mut parts = mantissa.split('.');
    let whole = parts.next()?;
    let fraction = parts.next().unwrap_or("");
    if parts.next().is_some() || (whole.is_empty() && fraction.is_empty())
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || !fraction.bytes().all(|byte| byte.is_ascii_digit()) { return None; }
    let coefficient = format!("{whole}{fraction}").parse::<u128>().ok()?;
    let scale = i32::try_from(fraction.len()).ok()?.checked_sub(exponent)?;
    let eighths = if scale > 0 {
        let divisor = 10_u128.checked_pow(u32::try_from(scale).ok()?)?;
        let numerator = coefficient.checked_mul(8)?;
        if numerator % divisor != 0 { return None; }
        numerator / divisor
    } else {
        coefficient.checked_mul(10_u128.checked_pow(scale.unsigned_abs())?)?.checked_mul(8)?
    };
    u64::try_from(eighths).ok()
}

fn projected_view_attributes(board: &mut PreservedElement, view: StoryMapViewState) -> Result<(), FdxError> {
    set_attribute(board, "Height", view.height().to_string())?;
    if let Some(width) = view.width() {
        set_attribute(board, "Width", decimal_milli(width))?;
    } else {
        remove_attribute(board, "Width")?;
    }
    let scroll = view.scroll_origin();
    set_attribute(board, "ScrollOrigin", format!("{},0", decimal_milli(scroll)))?;
    let zoom = view.zoom_milli_percent();
    set_attribute(board, "ZoomLevel", format!("{}.{:03}", zoom / 1_000, zoom % 1_000))?;
    Ok(())
}

fn lane_element(parent: &scriptcraft_model::preservation::SourceName, lane: &scriptcraft_model::StoryMapLane) -> PreservedElement {
    let mut element = new_lineage_element(parent, "Lane", vec![
        ("Id".into(), lane.uuid.clone()), ("Label".into(), lane.label.clone()),
        ("Level".into(), lane.level.to_string()), ("Type".into(), lane.lane_type.clone()),
    ]);
    element.source_type = Some(lane.lane_type.clone());
    element
}

fn update_lane(element: &mut PreservedElement, lane: &scriptcraft_model::StoryMapLane) -> Result<(), FdxError> {
    set_identity(element, &lane.uuid)?;
    set_attribute(element, "Label", lane.label.clone())?;
    set_attribute(element, "Level", lane.level.to_string())?;
    set_attribute(element, "Type", lane.lane_type.clone())?;
    element.source_type = Some(lane.lane_type.clone());
    Ok(())
}

fn project_lanes(board: &mut PreservedElement, project: &Project, original: Option<&Project>) -> Result<(), FdxError> {
    let namespace = board.name.namespace_uri.as_deref();
    let containers = board.children.iter().enumerate().filter_map(|(index, node)| {
        element(node).filter(|child| is_child(child, "Lanes", namespace)).map(|_| index)
    }).collect::<Vec<_>>();
    if containers.is_empty() {
        let mut lanes = new_lineage_element(&board.name, "Lanes", Vec::new());
        lanes.children = project.story_map().lanes.iter().map(|lane| {
            PreservedNode::Element(lane_element(&lanes.name, lane))
        }).collect::<Vec<_>>().into();
        let content_index = board.children.iter().position(|node| {
            element(node).is_some_and(|child| is_child(child, "Item", namespace))
        }).unwrap_or(board.children.len());
        board.children.insert(content_index, PreservedNode::Element(lanes));
        return Ok(());
    }
    let index = containers[0];
    let lanes_container = element_mut(board.children.get_mut(index).ok_or(FdxError::InvalidValue)?)
        .ok_or(FdxError::InvalidValue)?;
    let original_lanes = original.map(|project| project.story_map().lanes.as_slice()).unwrap_or_default();
    let mut prior = original_lanes.iter().map(|lane| lane.uuid.as_str()).collect::<BTreeSet<_>>();
    let mut existing = BTreeMap::<String, PreservedElement>::new();
    for node in &lanes_container.children {
        let Some(lane) = element(node).filter(|lane| is_child(lane, "Lane", namespace)) else { continue };
        let Some(id) = identity(lane) else { return Err(FdxError::UnsupportedConstruct) };
        if !prior.contains(id) || existing.insert(id.to_owned(), lane.clone()).is_some() {
            return Err(FdxError::UnsupportedConstruct);
        }
        let old_lane = original_lanes.iter().find(|old| old.uuid == id)
            .ok_or(FdxError::UnsupportedConstruct)?;
        let source_level = unique_attribute(lane, "Level").and_then(|value| value.parse::<i64>().ok());
        if unique_attribute(lane, "Label") != Some(old_lane.label.as_str())
            || source_level != Some(old_lane.level)
            || unique_attribute(lane, "Type") != Some(old_lane.lane_type.as_str())
        {
            return Err(FdxError::UnsupportedConstruct);
        }
    }
    let empty_default_collection = existing.is_empty() && original_lanes.len() == 2
        && original_lanes[0].label == "Outline 1" && original_lanes[0].lane_type == "Outline 1"
        && original_lanes[0].level == 7
        && original_lanes[1].label == "Outline 2" && original_lanes[1].lane_type == "Outline 2"
        && original_lanes[1].level == 5;
    if empty_default_collection {
        prior.clear();
    } else if existing.len() != prior.len() {
        return Err(FdxError::UnsupportedConstruct);
    }
    let mut ordered = Vec::with_capacity(project.story_map().lanes.len());
    for lane in &project.story_map().lanes {
        let mut node = existing.remove(&lane.uuid).unwrap_or_else(|| lane_element(&lanes_container.name, lane));
        update_lane(&mut node, lane)?;
        ordered.push(PreservedNode::Element(node));
    }
    let mut next = ordered.into_iter();
    let mut children = Vec::with_capacity(lanes_container.children.len().max(project.story_map().lanes.len()));
    let mut saw_lane = false;
    for node in std::mem::take(&mut lanes_container.children) {
        if element(&node).is_some_and(|lane| is_child(lane, "Lane", namespace)) {
            saw_lane = true;
            if let Some(replacement) = next.next() { children.push(replacement); }
        } else { children.push(node); }
    }
    if !saw_lane {
        children.extend(next);
    } else {
        // Any new lanes follow the last managed slot, before unrelated trailing nodes.
        if next.len() > 0 {
            let insert_at = children.iter().rposition(|node| element(node).is_some_and(|lane| is_child(lane, "Lane", namespace)))
                .map_or(children.len(), |index| index + 1);
            children.splice(insert_at..insert_at, next);
        }
    }
    lanes_container.children = children.into();
    Ok(())
}

fn page_goal(card: &mut PreservedElement, placement: &StoryMapPlacement, project: &Project) -> Result<(), FdxError> {
    let lane = project.story_map().lanes.iter().find(|lane| lane.uuid == placement.lane_uuid)
        .ok_or(FdxError::InvalidValue)?;
    let namespace = card.name.namespace_uri.as_deref();
    let indexes = card.children.iter().enumerate().filter_map(|(index, node)| {
        element(node).filter(|child| is_child(child, "SubordinateTo", namespace)
            && attribute_values(child, "Type").iter().any(|value| *value == "PageGoal"))
            .map(|_| index)
    }).collect::<Vec<_>>();
    if indexes.len() > 1 { return Err(FdxError::UnsupportedConstruct); }
    if let Some(index) = indexes.first().copied() {
        let node = element_mut(card.children.get_mut(index).ok_or(FdxError::InvalidValue)?)
            .ok_or(FdxError::InvalidValue)?;
        if unique_attribute(node, "Type") != Some("PageGoal") || unique_attribute(node, "Order") != Some("0")
            || unique_attribute(node, "Level").is_none()
            || unique_attribute(node, "Number").is_none()
            || unique_attribute(node, "Length").is_none() {
            return Err(FdxError::UnsupportedConstruct);
        }
        set_attribute(node, "Type", "PageGoal".into())?;
        set_attribute(node, "Order", "0".into())?;
        set_attribute(node, "Level", lane.lane_type.clone())?;
        set_attribute(node, "Number", decimal_eighths(placement.start_eighths))?;
        set_attribute(node, "Length", decimal_eighths(placement.length_eighths))?;
    } else {
        let content_index = card.children.iter().position(|node| {
            element(node).is_some_and(|child| is_child(child, "Content", namespace))
        }).ok_or(FdxError::UnsupportedConstruct)?;
        let node = new_lineage_element(&card.name, "SubordinateTo", vec![
            ("Type".into(), "PageGoal".into()), ("Order".into(), "0".into()),
            ("Level".into(), lane.lane_type.clone()),
            ("Number".into(), decimal_eighths(placement.start_eighths)),
            ("Length".into(), decimal_eighths(placement.length_eighths)),
        ]);
        let mut node = node;
        node.source_type = Some("PageGoal".into());
        card.children.insert(content_index, PreservedNode::Element(node));
    }
    Ok(())
}

fn original_page_goal_matches(card: &PreservedElement, old: &BeatCard, original: &Project) -> Result<usize, FdxError> {
    let namespace = card.name.namespace_uri.as_deref();
    let candidates = card.children.iter().enumerate().filter_map(|(index, node)| {
        element(node).filter(|child| is_child(child, "SubordinateTo", namespace)
            && attribute_values(child, "Type").iter().any(|value| *value == "PageGoal"))
            .map(|child| (index, child))
    }).collect::<Vec<_>>();
    let Some(expected) = &old.story_map_placement else { return Err(FdxError::InvalidValue) };
    let matches = candidates.iter().filter(|(_, child)| {
        let (Some(typ), Some(order), Some(level), Some(number), Some(length)) = (
            unique_attribute(child, "Type"), unique_attribute(child, "Order"), unique_attribute(child, "Level"),
            unique_attribute(child, "Number"), unique_attribute(child, "Length"),
        ) else { return false };
        if typ != "PageGoal" || order != "0" { return false; }
        let mut lanes = original.story_map().lanes.iter().filter(|lane| lane.lane_type == level);
        let Some(lane) = lanes.next() else { return false };
        lanes.next().is_none() && lane.uuid == expected.lane_uuid
            && parse_decimal_eighths(number) == Some(expected.start_eighths)
            && parse_decimal_eighths(length) == Some(expected.length_eighths)
    }).collect::<Vec<_>>();
    if matches.len() != 1 || candidates.len() != 1 { return Err(FdxError::UnsupportedConstruct); }
    Ok(matches[0].0)
}

fn reconcile_card_placement(
    item: &mut PreservedElement,
    card: &BeatCard,
    original_card: Option<&BeatCard>,
    project: &Project,
    original: Option<&Project>,
) -> Result<(), FdxError> {
    match (original_card.and_then(|card| card.story_map_placement.as_ref()), card.story_map_placement.as_ref()) {
        (Some(_), Some(placement)) => {
            let old_card = original_card.ok_or(FdxError::InvalidValue)?;
            let original_project = original.ok_or(FdxError::InvalidValue)?;
            let index = original_page_goal_matches(item, old_card, original_project)?;
            let child = element_mut(item.children.get_mut(index).ok_or(FdxError::InvalidValue)?)
                .ok_or(FdxError::InvalidValue)?;
            let lane = project.story_map().lanes.iter().find(|lane| lane.uuid == placement.lane_uuid)
                .ok_or(FdxError::InvalidValue)?;
            set_attribute(child, "Type", "PageGoal".into())?;
            set_attribute(child, "Order", "0".into())?;
            set_attribute(child, "Level", lane.lane_type.clone())?;
            set_attribute(child, "Number", decimal_eighths(placement.start_eighths))?;
            set_attribute(child, "Length", decimal_eighths(placement.length_eighths))
        }
        (Some(_), None) => {
            let old_card = original_card.ok_or(FdxError::InvalidValue)?;
            let original_project = original.ok_or(FdxError::InvalidValue)?;
            let index = original_page_goal_matches(item, old_card, original_project)?;
            item.children.remove(index);
            Ok(())
        }
        (None, Some(placement)) => {
            if original_card.is_some() {
                let namespace = item.name.namespace_uri.as_deref();
                let has_page_goal = item.children.iter().any(|node| {
                    element(node).is_some_and(|child| is_child(child, "SubordinateTo", namespace)
                        && attribute_values(child, "Type").iter().any(|value| *value == "PageGoal"))
                });
                if has_page_goal { return Err(FdxError::UnsupportedConstruct); }
            }
            page_goal(item, placement, project)
        }
        (None, None) => Ok(()),
    }
}

fn projected_board(parent: &scriptcraft_model::preservation::SourceName, project: &Project, context: &SaveContext) -> Result<PreservedElement, FdxError> {
    let id = project.story_map().foreign_id.as_deref().ok_or(FdxError::AmbiguousIdentity)?;
    let mut board = new_lineage_element(parent, "DisplayBoard", vec![("Type".into(), "StoryMap".into()), ("Id".into(), id.into())]);
    board.source_type = Some("StoryMap".into());
    if context.operation() != SaveOperation::DirectSerialization {
        projected_view_attributes(&mut board, context.story_map_view_state())?;
    }
    let mut lanes = new_lineage_element(&board.name, "Lanes", Vec::new());
    lanes.children = project.story_map().lanes.iter().map(|lane| {
        PreservedNode::Element(lane_element(&lanes.name, lane))
    }).collect::<Vec<_>>().into();
    board.children.push(PreservedNode::Element(lanes));
    Ok(board)
}

fn project_board(
    root: &mut PreservedElement,
    project: &Project,
    context: &SaveContext,
    original: Option<&Project>,
) -> Result<(), FdxError> {
    let namespace = root.name.namespace_uri.clone();
    project_target(root, project, original)?;
    let (container_index, board_index) = match winning_board_index(root)? {
        Some(indexes) => indexes,
        None => {
            let container_index = match root.children.iter().position(|node| {
                element(node).is_some_and(|child| is_child(child, "DisplayBoards", namespace.as_deref()))
            }) {
                Some(index) => index,
                None => {
                    let container = new_lineage_element(&root.name, "DisplayBoards", Vec::new());
                    root.children.push(PreservedNode::Element(container));
                    root.children.len() - 1
                }
            };
            let container = element_mut(root.children.get_mut(container_index).ok_or(FdxError::InvalidValue)?)
                .ok_or(FdxError::InvalidValue)?;
            let board = projected_board(&container.name, project, context)?;
            container.children.push(PreservedNode::Element(board));
            return project_list_items(root, project, original);
        }
    };
    let container = element_mut(root.children.get_mut(container_index).ok_or(FdxError::InvalidValue)?)
        .ok_or(FdxError::InvalidValue)?;
    let board = element_mut(container.children.get_mut(board_index).ok_or(FdxError::InvalidValue)?)
        .ok_or(FdxError::InvalidValue)?;
    set_identity(board, project.story_map().foreign_id.as_deref().ok_or(FdxError::AmbiguousIdentity)?)?;
    if context.operation() != SaveOperation::DirectSerialization {
        projected_view_attributes(board, context.story_map_view_state())?;
    }
    project_lanes(board, project, original)?;
    project_list_items(root, project, original)
}

fn project_list_items(root: &mut PreservedElement, project: &Project, original: Option<&Project>) -> Result<(), FdxError> {
    let namespace = root.name.namespace_uri.as_deref();
    let Some(list_index) = root.children.iter().position(|node| {
        element(node).is_some_and(|child| is_child(child, "ListItems", namespace))
    }) else {
        if project.beat_board().cards.iter().any(|card| card.story_map_placement.is_some()) {
            return Err(FdxError::UnsupportedConstruct);
        }
        return Ok(());
    };
    let list = element_mut(root.children.get_mut(list_index).ok_or(FdxError::InvalidValue)?)
        .ok_or(FdxError::InvalidValue)?;
    let old_cards = original.map(|value| value.beat_board().cards.iter().map(|card| (card.foreign_id.as_str(), card))
        .collect::<BTreeMap<_, _>>()).unwrap_or_default();
    let mut seen = BTreeSet::new();
    for node in &mut list.children {
        let Some(item) = element_mut(node) else { continue };
        if !is_child(item, "ListItem", namespace) { continue; }
        let Some(id) = identity(item) else { continue };
        let Some(card) = project.beat_board().cards.iter().find(|card| card.foreign_id == id) else { continue };
        if !seen.insert(id.to_owned()) { return Err(FdxError::AmbiguousIdentity); }
        reconcile_card_placement(item, card, old_cards.get(id).copied(), project, original)?;
    }
    if project.beat_board().cards.iter().any(|card| card.story_map_placement.is_some() && !seen.contains(&card.foreign_id)) {
        return Err(FdxError::UnsupportedConstruct);
    }
    Ok(())
}

/// Project current semantics onto one detached source-backed tree. Direct
/// serialization deliberately retains existing disposable board attributes.
pub(crate) fn project_source_backed(
    sections: &mut [PreservedSection],
    project: &Project,
    context: &SaveContext,
    original: &Project,
) -> Result<(), FdxError> {
    let roots = sections.iter_mut().flat_map(|section| &mut section.children).filter_map(|node| {
        element_mut(node).filter(|root| root.name.local_name == "FinalDraft")
    }).collect::<Vec<_>>();
    if roots.len() != 1 { return Err(FdxError::InvalidRoot); }
    project_board(roots.into_iter().next().ok_or(FdxError::InvalidRoot)?, project, context, Some(original))
}

/// Add the generated Story Map projection after the generated ListItems and
/// Beat board are present in a preservation-free document.
pub(crate) fn project_generated(
    nodes: &mut [PreservedNode],
    project: &Project,
    context: &SaveContext,
) -> Result<(), FdxError> {
    let roots = nodes.iter_mut().filter_map(|node| {
        element_mut(node).filter(|root| root.name.local_name == "FinalDraft")
    }).collect::<Vec<_>>();
    if roots.len() != 1 { return Err(FdxError::InvalidRoot); }
    project_board(roots.into_iter().next().ok_or(FdxError::InvalidRoot)?, project, context, None)
}
