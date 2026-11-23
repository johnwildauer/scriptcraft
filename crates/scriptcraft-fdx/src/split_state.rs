//! Bounded ScriptPanel display-mode projection and detached reconciliation.

use crate::constants::FDX_ROOT;
use crate::document::ScriptPanelDisplayMode;
use crate::reader::{RawElement, RawNode};
use crate::tree::new_lineage_element;
use crate::FdxError;
use scriptcraft_model::preservation::{
    PreservedAttribute, PreservedElement, PreservedNode, PreservedSection, SourceName,
};

/// Read the first root-namespace SplitState/ScriptPanel pair. Unusable disposable
/// view state falls back to Page without invalidating the document.
pub(crate) fn raw_root_display_mode(root: &RawElement) -> ScriptPanelDisplayMode {
    let root_namespace = root.name.namespace_uri.as_deref();
    let Some(split_state) = root.children.iter().find_map(|node| match node {
        RawNode::Element(element) if is_fdx_element(element, "SplitState", root_namespace) => {
            Some(element)
        }
        _ => None,
    }) else {
        return ScriptPanelDisplayMode::Page;
    };
    let Some(script_panel) = split_state.children.iter().find_map(|node| match node {
        RawNode::Element(element) if is_fdx_element(element, "ScriptPanel", root_namespace) => {
            Some(element)
        }
        _ => None,
    }) else {
        return ScriptPanelDisplayMode::Page;
    };

    let mut values = script_panel.attributes.iter().filter_map(|attribute| {
        (attribute.name.namespace_uri.is_none() && attribute.name.local_name == "DisplayMode")
            .then_some(attribute.value.as_str())
    });
    let Some(value) = values.next() else {
        return ScriptPanelDisplayMode::Page;
    };
    if values.next().is_some() {
        return ScriptPanelDisplayMode::Page;
    }
    ScriptPanelDisplayMode::from_fdx_str(value).unwrap_or(ScriptPanelDisplayMode::Page)
}

/// Create the minimal generated ScriptPanel for the given FDX parent.
pub(crate) fn script_panel_element(
    parent: &SourceName,
    mode: ScriptPanelDisplayMode,
) -> PreservedElement {
    new_lineage_element(
        parent,
        "ScriptPanel",
        vec![("DisplayMode".to_owned(), mode.as_fdx_str().to_owned())],
    )
}

/// Reconcile only the first direct-root SplitState projection on detached save
/// output. All other source nodes and later SplitState occurrences remain inert.
pub(crate) fn reconcile_root_display_mode(
    sections: &mut [PreservedSection],
    mode: ScriptPanelDisplayMode,
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

    let root_namespace = root.name.namespace_uri.clone();
    let split_index = root.children.iter().position(|node| {
        matches!(node,
            PreservedNode::Element(element)
                if element.name.local_name == "SplitState"
                    && element.name.namespace_uri == root_namespace)
    });
    if let Some(split_index) = split_index {
        let PreservedNode::Element(split_state) = &mut root.children[split_index] else {
            return Err(FdxError::InvalidValue);
        };
        if let Some(script_panel) = split_state.children.iter_mut().find_map(|node| match node {
            PreservedNode::Element(element)
                if element.name.local_name == "ScriptPanel"
                    && element.name.namespace_uri == root_namespace =>
            {
                Some(element)
            }
            _ => None,
        }) {
            reconcile_display_mode_attribute(script_panel, mode);
        } else {
            let panel = script_panel_element(&split_state.name, mode);
            insert_generated_child(&mut split_state.children, PreservedNode::Element(panel), "    ", "  ");
        }
        return Ok(());
    }

    let mut split_state = new_lineage_element(&root.name, "SplitState", Vec::new());
    split_state
        .children
        .push(PreservedNode::Element(script_panel_element(&split_state.name, mode)));
    let window_state = root.children.iter().rposition(|node| {
        matches!(node,
            PreservedNode::Element(element)
                if element.name.local_name == "WindowState"
                    && element.name.namespace_uri == root_namespace)
    });
    let text_state = root.children.iter().rposition(|node| {
        matches!(node,
            PreservedNode::Element(element)
                if element.name.local_name == "TextState"
                    && element.name.namespace_uri == root_namespace)
    });
    let settings = root.children.iter().position(|node| {
        matches!(node,
            PreservedNode::Element(element)
                if element.name.local_name == "ElementSettings"
                    && element.name.namespace_uri == root_namespace)
    });
    let view_anchor = text_state.or(window_state);
    let mut insertion = view_anchor
        .map(|index| index + 1)
        .or(settings)
        .unwrap_or(root.children.len());
    if view_anchor.is_some() {
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
    insert_root_child(
        &mut root.children,
        insertion,
        PreservedNode::Element(split_state),
    );
    Ok(())
}

fn reconcile_display_mode_attribute(element: &mut PreservedElement, mode: ScriptPanelDisplayMode) {
    let first = element.attributes.iter().position(|attribute| {
        attribute.name.namespace_uri.is_none() && attribute.name.local_name == "DisplayMode"
    });
    if let Some(first_index) = first {
        element.attributes[first_index].value = mode.as_fdx_str().to_owned();
        let mut seen_first = false;
        element.attributes.retain(|attribute| {
            if attribute.name.namespace_uri.is_none() && attribute.name.local_name == "DisplayMode" {
                if !seen_first {
                    seen_first = true;
                    true
                } else {
                    false
                }
            } else {
                true
            }
        });
    } else {
        element.attributes.push(PreservedAttribute {
            name: SourceName {
                namespace_uri: None,
                local_name: "DisplayMode".to_owned(),
                qualified_name: Some("DisplayMode".to_owned()),
            },
            value: mode.as_fdx_str().to_owned(),
        });
    }
}

fn insert_generated_child(
    children: &mut Vec<PreservedNode>,
    child: PreservedNode,
    child_indent: &str,
    closing_indent: &str,
) {
    let mut insertion = children.len();
    while insertion > 0
        && matches!(children.get(insertion - 1), Some(PreservedNode::Text(text)) if text.trim().is_empty())
    {
        insertion -= 1;
    }
    let needs_leading_space = insertion == 0
        || matches!(insertion.checked_sub(1).and_then(|index| children.get(index)), Some(PreservedNode::Element(_)));
    if needs_leading_space {
        children.insert(insertion, PreservedNode::Text(format!("\n{child_indent}").into()));
        insertion += 1;
    }
    children.insert(insertion, child);
    if insertion + 1 == children.len() {
        children.push(PreservedNode::Text(format!("\n{closing_indent}").into()));
    }
}

fn insert_root_child(children: &mut Vec<PreservedNode>, mut insertion: usize, child: PreservedNode) {
    let needs_leading_space = insertion == 0
        || matches!(insertion.checked_sub(1).and_then(|index| children.get(index)), Some(PreservedNode::Element(_)));
    if needs_leading_space {
        children.insert(insertion, PreservedNode::Text("\n\n  ".into()));
        insertion += 1;
    }
    children.insert(insertion, child);
    let after = insertion + 1;
    if after == children.len() || matches!(children.get(after), Some(PreservedNode::Element(_))) {
        children.insert(after, PreservedNode::Text("\n\n  ".into()));
    }
}

fn is_fdx_element(element: &RawElement, name: &str, root_namespace: Option<&str>) -> bool {
    element.name.local_name == name && element.name.namespace_uri.as_deref() == root_namespace
}
