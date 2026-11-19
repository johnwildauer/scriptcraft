//! Low-level conversions between raw FDX XML and retained model nodes.

use crate::diagnostics::FdxError;
use crate::reader::{attribute, RawElement, RawNode};
use scriptcraft_model::preservation::SharedVec;
use scriptcraft_model::preservation::{
    DependencyConfidence, PreservedAttribute, PreservedElement, PreservedNode, SourceName,
    SourceOwnership,
};

pub(crate) fn new_lineage_element(
    parent: &SourceName,
    local_name: &str,
    attributes: Vec<(String, String)>,
) -> PreservedElement {
    let prefix = parent
        .qualified_name
        .as_deref()
        .and_then(|qualified| qualified.rsplit_once(':').map(|(prefix, _)| prefix));
    let qualified_name = prefix.map_or_else(
        || local_name.to_owned(),
        |prefix| format!("{prefix}:{local_name}"),
    );
    PreservedElement {
        name: SourceName {
            namespace_uri: parent.namespace_uri.clone(),
            local_name: local_name.to_owned(),
            qualified_name: Some(qualified_name),
        },
        attributes: attributes
            .into_iter()
            .map(|(name, value)| PreservedAttribute {
                name: plain_source_name(&name),
                value,
            })
            .collect(),
        namespace_bindings: Vec::new(),
        source_type: None,
        ownership: SourceOwnership::Opaque,
        dependency_confidence: DependencyConfidence::Unknown,
        anchor: None,
        children: SharedVec::default(),
    }
}

pub(crate) fn plain_source_name(local_name: &str) -> SourceName {
    SourceName {
        namespace_uri: None,
        local_name: local_name.to_owned(),
        qualified_name: Some(local_name.to_owned()),
    }
}

pub(crate) fn preserved_element_owned(
    name: &str,
    attributes: Vec<(String, String)>,
) -> PreservedElement {
    let mut element = preserved_element(name, Vec::new());
    element.attributes = attributes
        .into_iter()
        .map(|(name, value)| PreservedAttribute {
            name: SourceName {
                namespace_uri: None,
                local_name: name.clone(),
                qualified_name: Some(name),
            },
            value,
        })
        .collect();
    element
}

pub(crate) fn preserved_element(name: &str, attributes: Vec<(&str, &str)>) -> PreservedElement {
    PreservedElement {
        name: SourceName {
            namespace_uri: None,
            local_name: name.to_owned(),
            qualified_name: Some(name.to_owned()),
        },
        attributes: attributes
            .into_iter()
            .map(|(name, value)| PreservedAttribute {
                name: SourceName {
                    namespace_uri: None,
                    local_name: name.to_owned(),
                    qualified_name: Some(name.to_owned()),
                },
                value: value.to_owned(),
            })
            .collect(),
        namespace_bindings: Vec::new(),
        source_type: None,
        ownership: SourceOwnership::Opaque,
        dependency_confidence: DependencyConfidence::Unknown,
        anchor: None,
        children: SharedVec::default(),
    }
}
pub(crate) fn preserve_misc_node(node: RawNode) -> Result<PreservedNode, FdxError> {
    match node {
        RawNode::Element(element) => preserve_opaque_element(element),
        RawNode::Text(text) => Ok(PreservedNode::Text(text.into())),
        RawNode::CData(text) => Ok(PreservedNode::CData(text.into())),
        RawNode::Comment(text) => Ok(PreservedNode::Comment(text.into())),
        RawNode::ProcessingInstruction { target, data } => {
            Ok(PreservedNode::ProcessingInstruction {
                target: target.into(),
                data: data.into(),
            })
        }
    }
}

pub(crate) fn preserve_opaque_element(element: RawElement) -> Result<PreservedNode, FdxError> {
    let source_type = attribute(&element, "Type").map(str::to_owned);
    let children = element
        .children
        .into_iter()
        .map(preserve_misc_node)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PreservedNode::Element(PreservedElement {
        name: element.name,
        attributes: element
            .attributes
            .into_iter()
            .map(|attribute| PreservedAttribute {
                name: attribute.name,
                value: attribute.value,
            })
            .collect(),
        namespace_bindings: element.namespace_bindings,
        source_type,
        ownership: SourceOwnership::Opaque,
        dependency_confidence: DependencyConfidence::Unknown,
        anchor: None,
        children: children.into(),
    }))
}
