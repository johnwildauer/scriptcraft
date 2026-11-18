//! Root WindowState XML projection and reconciliation.

use crate::constants::FDX_ROOT;
use crate::document::WindowState;
use crate::reader::{RawElement, RawNode};
use crate::tree::new_lineage_element;
use crate::FdxError;
use scriptcraft_model::preservation::{PreservedElement, PreservedNode, PreservedSection, SourceName};
use std::collections::BTreeMap;

/// Project one unambiguous direct-root WindowState from bounded raw XML.
pub(crate) fn raw_root_window_state(root: &RawElement) -> Option<WindowState> {
    let root_namespace = root.name.namespace_uri.as_deref();
    let mut windows = root.children.iter().filter_map(|node| match node {
        RawNode::Element(element) if is_fdx_element(element, "WindowState", root_namespace) => {
            Some(element)
        }
        _ => None,
    });
    let window = windows.next()?;
    if windows.next().is_some() {
        return None;
    }

    let mut parsed = BTreeMap::new();
    for attribute in &window.attributes {
        if attribute.name.namespace_uri.is_none()
            && matches!(attribute.name.local_name.as_str(), "Left" | "Top" | "Width" | "Height")
            && parsed
                .insert(attribute.name.local_name.as_str(), attribute.value.as_str())
                .is_some()
        {
            return None;
        }
    }
    let parse = |name: &'static str| parsed.get(name)?.parse::<i32>().ok();
    WindowState::new(parse("Left")?, parse("Top")?, parse("Width")?, parse("Height")?)
}

/// Generate the canonical root WindowState block.
pub(crate) fn window_state_element(parent: &SourceName, state: WindowState) -> PreservedElement {
    new_lineage_element(
        parent,
        "WindowState",
        vec![
            ("Height".to_owned(), state.height.to_string()),
            ("Left".to_owned(), state.left.to_string()),
            ("Mode".to_owned(), "Normal".to_owned()),
            ("Top".to_owned(), state.top.to_string()),
            ("Width".to_owned(), state.width.to_string()),
        ],
    )
}

/// Replace direct root WindowState duplicates on a detached writer candidate.
pub(crate) fn reconcile_root_window_state(
    sections: &mut [PreservedSection],
    state: WindowState,
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
    let window_indices = root
        .children
        .iter()
        .enumerate()
        .filter_map(|(index, node)| match node {
            PreservedNode::Element(element)
                if element.name.local_name == "WindowState"
                    && element.name.namespace_uri == root.name.namespace_uri =>
            {
                Some(index)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let canonical = window_state_element(&root.name, state);
    if let Some(first) = window_indices.first().copied() {
        root.children[first] = PreservedNode::Element(canonical);
        for index in window_indices.into_iter().skip(1).rev() {
            root.children.remove(index);
        }
        return Ok(());
    }

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
    let anchor = page_layout
        .map(|index| index + 1)
        .or(settings)
        .unwrap_or(root.children.len());
    // Keep existing inter-element whitespace on the leading side; where the
    // source is compact, add only the generated block's local separator.
    let mut insertion = anchor;
    if page_layout.is_some() {
        while insertion < root.children.len()
            && matches!(root.children.get(insertion), Some(PreservedNode::Text(text)) if text.trim().is_empty())
        {
            insertion += 1;
        }
    } else if settings.is_some() {
        while insertion > 0
            && matches!(root.children.get(insertion - 1), Some(PreservedNode::Text(text)) if text.trim().is_empty())
        {
            insertion -= 1;
        }
    }
    let needs_leading_space = insertion == 0
        || matches!(
            insertion.checked_sub(1).and_then(|index| root.children.get(index)),
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
