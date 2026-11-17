//! Reversible ScriptCraft mapping for disposable FDX TextState offsets.

use crate::reader::{RawElement, RawNode};
use crate::tree::new_lineage_element;
use crate::constants::FDX_ROOT;
use crate::FdxError;
use scriptcraft_model::{
    position::{Position, Selection},
    preservation::{PreservedNode, PreservedSection, SourceName},
    Project,
};

pub const MIN_TEXT_SCALING: u16 = 75;
pub const MAX_TEXT_SCALING: u16 = 300;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextState {
    pub scaling: u16,
    pub selection: Selection,
}

impl TextState {
    pub const DEFAULT: Self = Self {
        scaling: 100,
        selection: Selection::caret(Position {
            element: 0,
            offset: 0,
        }),
    };

    pub fn normalized_scaling(value: u64) -> Option<u16> {
        (value > 0)
            .then_some(value.clamp(u64::from(MIN_TEXT_SCALING), u64::from(MAX_TEXT_SCALING)) as u16)
    }
}

/// Map a flat UTF-16 code-unit offset into a canonical body paragraph and UTF-8 byte boundary.
/// One code unit separates adjacent semantic body paragraphs.
pub fn flat_offset_to_position(project: &Project, offset: usize) -> Option<Position> {
    if project.elements.is_empty() {
        return None;
    }
    let mut base = 0usize;
    for (element_index, element) in project.elements.iter().enumerate() {
        if offset == base {
            return Some(Position { element: element_index, offset: 0 });
        }
        for (byte, character) in element.text.char_indices() {
            let next = base.checked_add(character.len_utf16())?;
            if offset < next {
                return None;
            }
            if offset == next {
                return Some(Position {
                    element: element_index,
                    offset: byte + character.len_utf8(),
                });
            }
            base = next;
        }
        if element_index + 1 < project.elements.len() {
            base = base.checked_add(1)?;
        }
    }
    (offset == base).then(|| Position {
        element: project.elements.len() - 1,
        offset: project.elements.last().map_or(0, |element| element.text.len()),
    })
}

/// Map an editor selection to ordered flat UTF-16 offsets without changing its direction.
pub fn selection_to_flat_offsets(
    project: &Project,
    selection: Selection,
) -> Option<(usize, usize)> {
    fn position_to_flat(project: &Project, position: Position) -> Option<usize> {
        let element = project.elements.get(position.element)?;
        if position.offset > element.text.len() || !element.text.is_char_boundary(position.offset) {
            return None;
        }
        let mut flat = 0usize;
        for (index, candidate) in project.elements.iter().enumerate() {
            if index == position.element {
                return flat.checked_add(candidate.text[..position.offset].encode_utf16().count());
            }
            flat = flat.checked_add(candidate.text.encode_utf16().count())?;
            flat = flat.checked_add(1)?;
        }
        None
    }

    let anchor = position_to_flat(project, selection.anchor)?;
    let focus = position_to_flat(project, selection.focus)?;
    Some((anchor.min(focus), anchor.max(focus)))
}

/// Parse exactly two nonnegative decimal UTF-16 offsets and map them atomically.
pub fn parse_flat_selection(project: &Project, value: &str) -> Option<Selection> {
    let (start, end) = value.split_once(',')?;
    if end.contains(',') {
        return None;
    }
    if start.is_empty()
        || end.is_empty()
        || !start.bytes().all(|byte| byte.is_ascii_digit())
        || !end.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let start = start.parse::<usize>().ok()?;
    let end = end.parse::<usize>().ok()?;
    if start > end {
        return None;
    }
    Some(Selection {
        anchor: flat_offset_to_position(project, start)?,
        focus: flat_offset_to_position(project, end)?,
    })
}

/// Project a unique direct-root TextState into bounded root zoom/selection
/// values. Nested title-page TextState remains opaque.
pub(crate) fn raw_root_text_state(root: &RawElement, project: &Project) -> TextState {
    let namespace = root.name.namespace_uri.as_deref();
    let mut blocks = root.children.iter().filter_map(|node| match node {
        RawNode::Element(element) if is_fdx_element(element, "TextState", namespace) => {
            Some(element)
        }
        _ => None,
    });
    let Some(block) = blocks.next() else {
        return TextState::DEFAULT;
    };
    if blocks.next().is_some() {
        return TextState::DEFAULT;
    }
    let mut scaling = None;
    let mut selection = None;
    let mut scaling_ambiguous = false;
    let mut selection_ambiguous = false;
    for attribute in &block.attributes {
        if attribute.name.namespace_uri.is_some() {
            continue;
        }
        match attribute.name.local_name.as_str() {
            "Scaling" => {
                if scaling.replace(attribute.value.as_str()).is_some() {
                    scaling_ambiguous = true;
                }
            }
            "Selection" if selection.replace(attribute.value.as_str()).is_some() => {
                selection_ambiguous = true;
            }
            _ => {}
        }
    }
    let scale = if scaling_ambiguous {
        100
    } else {
        scaling
            .and_then(|value| value.parse::<u64>().ok())
            .and_then(TextState::normalized_scaling)
            .unwrap_or(100)
    };
    let mapped = if selection_ambiguous {
        None
    } else {
        selection.and_then(|value| parse_flat_selection(project, value))
    };
    TextState {
        scaling: scale,
        selection: mapped.unwrap_or(TextState::DEFAULT.selection),
    }
}

/// Generate the canonical root TextState projection with deterministic
/// attribute order.
pub(crate) fn text_state_element(
    parent: &SourceName,
    project: &Project,
    state: TextState,
) -> Result<scriptcraft_model::preservation::PreservedElement, FdxError> {
    let (start, end) = selection_to_flat_offsets(project, state.selection)
        .ok_or(FdxError::InvalidValue)?;
    Ok(new_lineage_element(
        parent,
        "TextState",
        vec![
            (
                "Scaling".to_owned(),
                state
                    .scaling
                    .clamp(MIN_TEXT_SCALING, MAX_TEXT_SCALING)
                    .to_string(),
            ),
            ("Selection".to_owned(), format!("{start},{end}")),
            ("ShowInvisibles".to_owned(), "No".to_owned()),
        ],
    ))
}

/// Replace only direct root TextState entries on a detached writer candidate.
/// Selection mapping uses the body Project; nested title-page blocks are not
/// traversed.
pub(crate) fn reconcile_root_text_state(
    sections: &mut [PreservedSection],
    project: &Project,
    state: TextState,
) -> Result<(), FdxError> {
    let mut roots = sections
        .iter_mut()
        .flat_map(|section| &mut section.children)
        .filter_map(|node| match node {
            PreservedNode::Element(element) if element.name.local_name == FDX_ROOT => Some(element),
            _ => None,
        });
    let root = roots.next().ok_or(FdxError::InvalidRoot)?;
    if roots.next().is_some() {
        return Err(FdxError::InvalidRoot);
    }
    let indices = root
        .children
        .iter()
        .enumerate()
        .filter_map(|(index, node)| match node {
            PreservedNode::Element(element)
                if element.name.local_name == "TextState"
                    && element.name.namespace_uri == root.name.namespace_uri =>
            {
                Some(index)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let canonical = text_state_element(&root.name, project, state)?;
    if let Some(first) = indices.first().copied() {
        root.children[first] = PreservedNode::Element(canonical);
        for index in indices.into_iter().skip(1).rev() {
            root.children.remove(index);
        }
        return Ok(());
    }

    let window = root.children.iter().position(|node| {
        matches!(node,
            PreservedNode::Element(element)
                if element.name.local_name == "WindowState"
                    && element.name.namespace_uri == root.name.namespace_uri
        )
    });
    let page_layout = root.children.iter().position(|node| {
        matches!(node,
            PreservedNode::Element(element)
                if element.name.local_name == "PageLayout"
                    && element.name.namespace_uri == root.name.namespace_uri
        )
    });
    let settings = root.children.iter().position(|node| {
        matches!(node,
            PreservedNode::Element(element)
                if element.name.local_name == "ElementSettings"
                    && element.name.namespace_uri == root.name.namespace_uri
        )
    });
    let mut insertion = window
        .map(|index| index + 1)
        .or_else(|| page_layout.map(|index| index + 1))
        .or(settings)
        .unwrap_or(root.children.len());
    while insertion < root.children.len()
        && matches!(root.children.get(insertion), Some(PreservedNode::Text(text)) if text.trim().is_empty())
    {
        insertion += 1;
    }
    let needs_leading_space = insertion == 0
        || matches!(
            insertion
                .checked_sub(1)
                .and_then(|index| root.children.get(index)),
            Some(PreservedNode::Element(_))
        );
    if needs_leading_space {
        root.children
            .insert(insertion, PreservedNode::Text("\n\n  ".into()));
        insertion += 1;
    }
    root.children
        .insert(insertion, PreservedNode::Element(canonical));
    let after = insertion + 1;
    if after == root.children.len()
        || matches!(root.children.get(after), Some(PreservedNode::Element(_)))
    {
        root.children
            .insert(after, PreservedNode::Text("\n\n  ".into()));
    }
    Ok(())
}

fn is_fdx_element(element: &RawElement, name: &str, root_ns: Option<&str>) -> bool {
    element.name.local_name == name && element.name.namespace_uri.as_deref() == root_ns
}
