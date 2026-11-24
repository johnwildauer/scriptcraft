//! DocumentRef/XRef source projection and transactional lineage reconciliation.

use crate::constants::FDX_ROOT;
use crate::context::{valid_document_ref_datetime, SaveContext, SaveOperation};
use crate::diagnostics::{map_preservation_error, FdxError, SaveDiagnosticCode};
use crate::identity::identity_domain;
use crate::reader::{RawElement, RawNode};
use crate::tree::{new_lineage_element, plain_source_name};
use scriptcraft_model::preservation::{
    ForeignIdentity, PreservedAttribute, PreservedElement, PreservedNode, PreservedSection,
};
use std::collections::BTreeSet;

/// Project a single valid direct-root DocumentRef DateTime from bounded raw XML.
pub(crate) fn raw_document_ref_datetime(root: &RawElement) -> Option<String> {
    let root_namespace = root.name.namespace_uri.as_deref();
    let mut document_refs = root.children.iter().filter_map(|node| match node {
        RawNode::Element(element) if element.name.local_name == "DocumentRef" => Some(element),
        _ => None,
    });
    let document_ref = document_refs.next()?;
    if document_refs.next().is_some()
        || document_ref.name.namespace_uri.as_deref() != root_namespace
    {
        return None;
    }
    let mut datetimes = document_ref.attributes.iter().filter(|attribute| {
        attribute.name.namespace_uri.is_none() && attribute.name.local_name == "DateTime"
    });
    let value = datetimes.next()?.value.as_str();
    if datetimes.next().is_some() || !valid_document_ref_datetime(value) {
        return None;
    }
    Some(value.to_owned())
}

/// Reconcile DocumentRef and XRef records on a detached transactional candidate.
pub(crate) fn reconcile_document_lineage(
    sections: &mut [PreservedSection],
    existing_identities: &[ForeignIdentity],
    project: &scriptcraft_model::Project,
    context: &mut SaveContext,
) -> Result<Vec<ForeignIdentity>, FdxError> {
    if !valid_document_ref_datetime(context.timestamp()) {
        return Err(FdxError::InvalidValue);
    }
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

    let mut reserved = existing_identities
        .iter()
        .map(|identity| identity.value.clone())
        .collect::<BTreeSet<_>>();
    if let Some(id) = &project.beat_board().foreign_id { reserved.insert(id.clone()); }
    for card in &project.beat_board().cards {
        reserved.insert(card.foreign_id.clone());
        reserved.extend(card.paragraphs.iter().filter_map(|paragraph| paragraph.foreign_id.clone()));
    }
    collect_preserved_id_attributes(&root.children, &mut reserved);

    let document_ref_indices = root
        .children
        .iter()
        .enumerate()
        .filter_map(|(index, node)| match node {
            PreservedNode::Element(element) if element.name.local_name == "DocumentRef" => {
                Some(index)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    if document_ref_indices.len() > 1 {
        return Err(SaveDiagnosticCode::LineageAmbiguousDocumentRef.error());
    }

    let mut additions = Vec::new();
    let document_ref_index = if let Some(index) = document_ref_indices.first().copied() {
        let PreservedNode::Element(document_ref) = &root.children[index] else {
            return Err(SaveDiagnosticCode::LineageAmbiguousDocumentRef.error());
        };
        if document_ref.name.namespace_uri != root.name.namespace_uri {
            return Err(SaveDiagnosticCode::LineageAmbiguousDocumentRef.error());
        }
        index
    } else {
        let document_ref_id = context.allocate_lineage_id(&mut reserved)?;
        additions.push(
            ForeignIdentity::new(identity_domain("DocumentRef"), document_ref_id.clone())
                .map_err(map_preservation_error)?,
        );
        let document_ref = new_lineage_element(
            &root.name,
            "DocumentRef",
            vec![
                ("DateTime".to_owned(), context.timestamp().to_owned()),
                ("id".to_owned(), document_ref_id),
            ],
        );
        let content_index = root
            .children
            .iter()
            .position(|node| {
                matches!(node,
                    PreservedNode::Element(element)
                        if element.name.local_name == "Content"
                            && element.name.namespace_uri == root.name.namespace_uri
                )
            })
            .ok_or(FdxError::MissingContent)?;
        root.children
            .insert(content_index, PreservedNode::Element(document_ref));
        content_index
    };

    let PreservedNode::Element(document_ref) = &mut root.children[document_ref_index] else {
        return Err(SaveDiagnosticCode::LineageAmbiguousDocumentRef.error());
    };
    let datetime_indices = document_ref
        .attributes
        .iter()
        .enumerate()
        .filter_map(|(index, attribute)| {
            (attribute.name.namespace_uri.is_none() && attribute.name.local_name == "DateTime")
                .then_some(index)
        })
        .collect::<Vec<_>>();
    if datetime_indices.len() > 1 {
        return Err(SaveDiagnosticCode::LineageAmbiguousDocumentRefDateTime.error());
    }
    let xref_datetime = context
        .session_open_timestamp()
        .ok_or(FdxError::InvalidValue)?
        .to_owned();

    if let Some(index) = datetime_indices.first().copied() {
        document_ref.attributes[index].value = context.timestamp().to_owned();
    } else {
        let insert_at = document_ref
            .attributes
            .iter()
            .position(|attribute| {
                attribute.name.namespace_uri.is_none()
                    && attribute.name.local_name.eq_ignore_ascii_case("id")
            })
            .unwrap_or(0);
        document_ref.attributes.insert(
            insert_at,
            PreservedAttribute {
                name: plain_source_name("DateTime"),
                value: context.timestamp().to_owned(),
            },
        );
    }

    let has_direct_xref = document_ref.children.iter().any(|node| {
        matches!(node,
            PreservedNode::Element(element)
                if element.name.local_name == "XRef"
                    && element.name.namespace_uri == document_ref.name.namespace_uri
        )
    });
    let should_append_xref = match context.operation() {
        SaveOperation::DirectSerialization => false,
        SaveOperation::Save => !has_direct_xref,
        SaveOperation::SaveAs => true,
    };
    if should_append_xref {
        let xref_id = context.allocate_lineage_id(&mut reserved)?;
        let xref_identity = ForeignIdentity::new(identity_domain("XRef"), xref_id.clone())
            .map_err(map_preservation_error)?;
        let xref = new_lineage_element(
            &document_ref.name,
            "XRef",
            vec![
                ("DateTime".to_owned(), xref_datetime),
                ("id".to_owned(), xref_id),
            ],
        );
        append_lineage_xref(root, document_ref_index, xref)?;
        additions.push(xref_identity);
    }
    Ok(additions)
}

fn collect_preserved_id_attributes(nodes: &[PreservedNode], values: &mut BTreeSet<String>) {
    for node in nodes {
        let PreservedNode::Element(element) = node else {
            continue;
        };
        values.extend(
            element
                .attributes
                .iter()
                .filter(|&attribute| {
                    attribute.name.namespace_uri.is_none()
                        && attribute.name.local_name.eq_ignore_ascii_case("id")
                })
                .map(|attribute| attribute.value.clone()),
        );
        collect_preserved_id_attributes(&element.children, values);
    }
}

fn append_lineage_xref(
    root: &mut PreservedElement,
    document_ref_index: usize,
    xref: PreservedElement,
) -> Result<(), FdxError> {
    let root_preserves_space = root.attributes.iter().any(|attribute| {
        attribute.name.namespace_uri.as_deref() == Some("http://www.w3.org/XML/1998/namespace")
            && attribute.name.local_name == "space"
            && attribute.value == "preserve"
    });
    let PreservedNode::Element(document_ref) = root
        .children
        .get_mut(document_ref_index)
        .ok_or(SaveDiagnosticCode::LineageAmbiguousDocumentRef.error())?
    else {
        return Err(SaveDiagnosticCode::LineageAmbiguousDocumentRef.error());
    };
    let document_ref_preserves_space = document_ref.attributes.iter().any(|attribute| {
        attribute.name.namespace_uri.as_deref() == Some("http://www.w3.org/XML/1998/namespace")
            && attribute.name.local_name == "space"
            && attribute.value == "preserve"
    });
    if root_preserves_space || document_ref_preserves_space {
        return append_xref_without_formatting(document_ref, xref);
    }
    let format_safe = document_ref.children.iter().all(|node| match node {
        PreservedNode::Element(element) => {
            element.name.namespace_uri == document_ref.name.namespace_uri
        }
        PreservedNode::Text(text) => text.chars().all(char::is_whitespace),
        _ => false,
    });
    if !format_safe {
        return append_xref_without_formatting(document_ref, xref);
    }
    append_xref_with_formatting(document_ref, xref)
}

fn append_xref_without_formatting(
    document_ref: &mut PreservedElement,
    xref: PreservedElement,
) -> Result<(), FdxError> {
    let index = lineage_xref_insert_index(document_ref)?;
    document_ref
        .children
        .insert(index, PreservedNode::Element(xref));
    Ok(())
}

fn append_xref_with_formatting(
    document_ref: &mut PreservedElement,
    xref: PreservedElement,
) -> Result<(), FdxError> {
    let index = lineage_xref_insert_index(document_ref)?;
    let has_following_node = index < document_ref.children.len();
    let following_is_whitespace = matches!(
        document_ref.children.get(index),
        Some(PreservedNode::Text(text)) if text.chars().all(char::is_whitespace)
    );
    document_ref
        .children
        .insert(index, PreservedNode::Text("\n    ".into()));
    document_ref
        .children
        .insert(index + 1, PreservedNode::Element(xref));
    if has_following_node && !following_is_whitespace {
        document_ref
            .children
            .insert(index + 2, PreservedNode::Text("\n    ".into()));
    } else if !has_following_node {
        document_ref
            .children
            .push(PreservedNode::Text("\n  ".into()));
    }
    Ok(())
}

fn lineage_xref_insert_index(document_ref: &PreservedElement) -> Result<usize, FdxError> {
    let mut last_xref = None;
    for (index, node) in document_ref.children.iter().enumerate() {
        if let PreservedNode::Element(element) = node {
            if element.name.local_name == "XRef" {
                if element.name.namespace_uri != document_ref.name.namespace_uri {
                    return Err(SaveDiagnosticCode::LineageAmbiguousDocumentRef.error());
                }
                last_xref = Some(index);
            }
        }
    }
    Ok(last_xref.map_or_else(
        || {
            document_ref
                .children
                .iter()
                .rposition(|node| !matches!(node, PreservedNode::Text(text) if text.chars().all(char::is_whitespace)))
                .map_or(0, |index| index + 1)
        },
        |index| index + 1,
    ))
}
