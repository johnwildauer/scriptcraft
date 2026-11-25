//! Shared source-preserving whitespace formatting for FDX writer paths.
//! The FDX writer owns formatting generated structure while preserving source whitespace.

use crate::constants::FDX_ROOT;
use scriptcraft_model::preservation::{
    PreservedElement, PreservedNode, PreservedSection, SourceOwnership,
};
use std::collections::BTreeSet;

pub(crate) fn format_generated_structure(nodes: &mut [PreservedNode]) {
    for node in nodes {
        if let PreservedNode::Element(element) = node {
            format_generated_element(element, 0, false);
        }
    }
}

fn format_generated_element(
    element: &mut PreservedElement,
    depth: usize,
    inherited_space_preserve: bool,
) {
    let space_preserve = xml_space_preserve(element, inherited_space_preserve);
    if space_preserve {
        return;
    }

    // Generated elements are formatted only when their children are known to
    // be element-only. Character data, comments, PIs and CDATA are never
    // surrounded with serializer whitespace.
    if element.children.is_empty()
        || !element
            .children
            .iter()
            .all(|child| matches!(child, PreservedNode::Element(_)))
    {
        return;
    }

    for child in &mut element.children {
        if let PreservedNode::Element(child) = child {
            format_generated_element(child, depth.saturating_add(1), space_preserve);
        }
    }

    let root_children = depth == 0 && element.name.local_name == FDX_ROOT;
    let child_indent = "  ".repeat(depth.saturating_add(1));
    let closing_indent = "  ".repeat(depth);
    let mut formatted = Vec::with_capacity(element.children.len().saturating_mul(2) + 1);
    for child in std::mem::take(&mut element.children) {
        formatted.push(PreservedNode::Text(if root_children {
            format!("\n\n{child_indent}").into()
        } else {
            format!("\n{child_indent}").into()
        }));
        formatted.push(child);
    }
    formatted.push(PreservedNode::Text(format!("\n{closing_indent}").into()));
    element.children = formatted.into();
}

fn xml_space_preserve(element: &PreservedElement, inherited: bool) -> bool {
    element
        .attributes
        .iter()
        .find(|attribute| {
            attribute.name.local_name == "space"
                && (attribute.name.namespace_uri.as_deref()
                    == Some("http://www.w3.org/XML/1998/namespace")
                    || attribute.name.qualified_name.as_deref() == Some("xml:space"))
        })
        .map_or(inherited, |attribute| match attribute.value.as_str() {
            "default" => false,
            "preserve" => true,
            _ => true,
        })
}

pub(crate) fn collect_inserted_managed_paragraph_ids(
    sections: &[PreservedSection],
    original_ids: &BTreeSet<String>,
) -> BTreeSet<String> {
    fn visit(
        nodes: &[PreservedNode],
        original_ids: &BTreeSet<String>,
        found: &mut BTreeSet<String>,
    ) {
        for node in nodes {
            let PreservedNode::Element(element) = node else {
                continue;
            };
            if element.name.local_name == "Paragraph"
                && matches!(element.ownership, SourceOwnership::Managed(_))
            {
                if let Some(id) = element
                    .attributes
                    .iter()
                    .find(|attribute| attribute.name.local_name.eq_ignore_ascii_case("id"))
                    .map(|attribute| attribute.value.as_str())
                {
                    if !original_ids.contains(id) {
                        found.insert(id.to_owned());
                    }
                }
            }
            visit(&element.children, original_ids, found);
        }
    }

    let mut found = BTreeSet::new();
    for section in sections {
        visit(&section.children, original_ids, &mut found);
    }
    found
}

pub(crate) fn format_inserted_paragraph_whitespace(
    sections: &mut [PreservedSection],
    inserted_ids: &BTreeSet<String>,
) {
    if inserted_ids.is_empty() {
        return;
    }

    fn visit(
        nodes: &mut [PreservedNode],
        inherited_space_preserve: bool,
        inserted_ids: &BTreeSet<String>,
    ) {
        for node in nodes {
            let PreservedNode::Element(root) = node else {
                continue;
            };
            let root_preserve = xml_space_preserve(root, inherited_space_preserve);
            if root.name.local_name == FDX_ROOT {
                for child in &mut root.children {
                    let PreservedNode::Element(content) = child else {
                        continue;
                    };
                    if content.name.local_name == "Content" {
                        let content_preserve = xml_space_preserve(content, root_preserve);
                        if !content_preserve {
                            format_content_insertions(&mut content.children, inserted_ids);
                        }
                    }
                    visit(std::slice::from_mut(child), root_preserve, inserted_ids);
                }
            } else {
                visit(&mut root.children, root_preserve, inserted_ids);
            }
        }
    }

    for section in sections {
        visit(&mut section.children, false, inserted_ids);
    }
}

/// Add deterministic indentation only around newly generated ScriptNote
/// subtrees. Existing note/container whitespace and opaque source are untouched.
pub(crate) fn format_script_note_insertions(
    sections: &mut [PreservedSection],
    project: &scriptcraft_model::Project,
) {
    let generated = project.script_notes().iter()
        .filter(|note| note.source_note_ordinal.is_none())
        .filter_map(|note| note.numeric_id.as_deref())
        .collect::<BTreeSet<_>>();
    if generated.is_empty() { return; }
    let Some(root) = sections.iter_mut().flat_map(|section| &mut section.children).find_map(|node| match node {
        PreservedNode::Element(root) if root.name.local_name == FDX_ROOT => Some(root),
        _ => None,
    }) else { return; };
    let namespace = root.name.namespace_uri.clone();
    let Some(container) = root.children.iter_mut().find_map(|node| match node {
        PreservedNode::Element(container) if container.name.local_name == "ScriptNotes"
            && container.name.namespace_uri == namespace => Some(container),
        _ => None,
    }) else { return; };
    let mut formatted = Vec::new();
    let mut inserted = false;
    for node in std::mem::take(&mut container.children) {
        let is_generated = matches!(&node,
            PreservedNode::Element(note) if note.name.local_name == "ScriptNote"
                && note.attributes.iter().any(|attribute| attribute.name.namespace_uri.is_none()
                    && attribute.name.local_name == "Id" && generated.contains(attribute.value.as_str())));
        if is_generated {
            formatted.push(PreservedNode::Text("\n    ".into()));
            formatted.push(node);
            inserted = true;
        } else { formatted.push(node); }
    }
    if inserted { formatted.push(PreservedNode::Text("\n  ".into())); }
    container.children = formatted.into();
}

fn format_content_insertions(children: &mut Vec<PreservedNode>, inserted_ids: &BTreeSet<String>) {
    // Only the unambiguous element-only Content shape is eligible. Comments,
    // PIs, CDATA, non-whitespace text and non-Paragraph children make the
    // context mixed/opaque, so source order and all whitespace remain intact.
    if children.iter().any(|child| match child {
        PreservedNode::Text(text) => !text.chars().all(char::is_whitespace),
        PreservedNode::Element(element) => element.name.local_name != "Paragraph",
        _ => true,
    }) {
        return;
    }
    let has_inserted = children.iter().any(|child| match child {
        PreservedNode::Element(element) if element.name.local_name == "Paragraph" => element
            .attributes
            .iter()
            .find(|attribute| attribute.name.local_name.eq_ignore_ascii_case("id"))
            .is_some_and(|attribute| inserted_ids.contains(&attribute.value)),
        _ => false,
    });
    if !has_inserted {
        return;
    }

    let mut formatted = Vec::with_capacity(children.len().saturating_mul(2) + 1);
    let mut pending_whitespace = Vec::new();
    let mut previous_was_inserted = false;
    let mut saw_paragraph = false;
    for child in std::mem::take(children) {
        if matches!(&child, PreservedNode::Text(text) if text.chars().all(char::is_whitespace)) {
            pending_whitespace.push(child);
            continue;
        }
        let PreservedNode::Element(element) = &child else {
            // Already excluded above; keep defensive fallback lossless.
            formatted.append(&mut pending_whitespace);
            formatted.push(child);
            continue;
        };
        let is_inserted = element
            .attributes
            .iter()
            .find(|attribute| attribute.name.local_name.eq_ignore_ascii_case("id"))
            .is_some_and(|attribute| inserted_ids.contains(&attribute.value));
        if is_inserted || (saw_paragraph && previous_was_inserted) {
            formatted.push(PreservedNode::Text("\n    ".into()));
        } else {
            formatted.append(&mut pending_whitespace);
        }
        pending_whitespace.clear();
        formatted.push(child);
        saw_paragraph = true;
        previous_was_inserted = is_inserted;
    }
    if previous_was_inserted {
        formatted.push(PreservedNode::Text("\n  ".into()));
    } else {
        formatted.append(&mut pending_whitespace);
    }
    *children = formatted;
}
