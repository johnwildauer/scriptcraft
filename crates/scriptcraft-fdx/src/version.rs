//! Supported-version decisions and bounded source preparation for migration.

use crate::constants::{FDX_ROOT, FDX_VERSION};
use crate::diagnostics::FdxError;
use crate::reader::{attribute, parse_document, RawAttribute, RawElement, RawNode};
use crate::tree::preserve_misc_node;
use crate::writer::write_node;
use crate::{FdxVersionDirection, FdxVersionStatus, MAX_SOURCE_BYTES};
use scriptcraft_model::preservation::{PreservedNode, PreservedSection, SourceName};

/// Securely parse and classify the root version without building an editing
/// candidate. Call this before presenting a warning for an explicit non-6
/// declaration. Input is bounded and never written or resolved externally.
pub fn preflight_version(input: &[u8]) -> Result<FdxVersionStatus, FdxError> {
    if input.len() > MAX_SOURCE_BYTES {
        return Err(FdxError::SourceTooLarge);
    }
    std::str::from_utf8(input).map_err(|_| FdxError::InvalidEncoding)?;
    let parsed = parse_document(input)?;
    let root = parsed
        .iter()
        .find_map(|node| match node {
            RawNode::Element(element) => Some(element),
            _ => None,
        })
        .ok_or(FdxError::InvalidRoot)?;
    if root.name.local_name != FDX_ROOT || attribute(root, "DocumentType") != Some("Script") {
        return Err(FdxError::InvalidRoot);
    }
    version_status(root)
}

pub(crate) fn prepare_non_v6_source(
    input: &[u8],
    direction: FdxVersionDirection,
) -> Result<Vec<u8>, FdxError> {
    let mut parsed = parse_document(input)?;
    let root_index = parsed
        .iter()
        .position(|node| matches!(node, RawNode::Element(_)))
        .ok_or(FdxError::InvalidRoot)?;
    let root = match &mut parsed[root_index] {
        RawNode::Element(root) => root,
        _ => return Err(FdxError::InvalidRoot),
    };
    if root.name.local_name != FDX_ROOT || attribute(root, "DocumentType") != Some("Script") {
        return Err(FdxError::InvalidRoot);
    }
    let FdxVersionStatus::ExplicitNon6 {
        direction: actual_direction,
        ..
    } = version_status(root)?
    else {
        return Err(FdxError::UnsupportedVersion);
    };
    if actual_direction != direction {
        return Err(FdxError::UnsupportedVersion);
    }
    let version_attribute = root.attributes.iter_mut().find(|attribute| {
        attribute.name.namespace_uri.is_none() && attribute.name.local_name == "Version"
    });
    if let Some(attribute) = version_attribute {
        attribute.value = FDX_VERSION.to_owned();
    } else {
        return Err(FdxError::MissingVersion);
    }

    if direction == FdxVersionDirection::Older {
        let root_namespace = root.name.namespace_uri.clone();
        root.children.retain(|node| match node {
            RawNode::Element(element) => {
                is_fdx_element(element, "Content", root_namespace.as_deref())
                    || is_fdx_element(element, "TitlePage", root_namespace.as_deref())
            }
            _ => true,
        });
        for child in &mut root.children {
            let RawNode::Element(section) = child else {
                continue;
            };
            if !is_fdx_element(section, "Content", root_namespace.as_deref())
                && !is_fdx_element(section, "TitlePage", root_namespace.as_deref())
            {
                continue;
            }
            let is_content = is_fdx_element(section, "Content", root_namespace.as_deref());
            for item in &mut section.children {
                let RawNode::Element(paragraph) = item else {
                    continue;
                };
                if paragraph.name.local_name != "Paragraph" {
                    if is_content {
                        return Err(FdxError::UnsupportedConstruct);
                    }
                    continue;
                }
                if paragraph.name.namespace_uri != root_namespace {
                    return Err(FdxError::UnsupportedConstruct);
                }
                let paragraph_type = attribute(paragraph, "Type");
                if paragraph_type.is_some_and(|value| {
                    crate::settings::role_from_fdx(value).is_some()
                }) {
                    continue;
                }
                if let Some(type_attribute) = paragraph.attributes.iter_mut().find(|attribute| {
                    attribute.name.namespace_uri.is_none() && attribute.name.local_name == "Type"
                }) {
                    type_attribute.value = "General".to_owned();
                } else {
                    paragraph.attributes.push(RawAttribute {
                        name: SourceName {
                            namespace_uri: None,
                            local_name: "Type".to_owned(),
                            qualified_name: Some("Type".to_owned()),
                        },
                        value: "General".to_owned(),
                    });
                }
            }
        }
    }

    let mut output = String::new();
    for node in parsed {
        let preserved = preserve_misc_node(node)?;
        write_node(&preserved, &mut output, 0)?;
        if output.len() > MAX_SOURCE_BYTES {
            return Err(FdxError::SourceTooLarge);
        }
    }
    Ok(output.into_bytes())
}

pub(crate) fn set_preserved_root_version(sections: &mut [PreservedSection]) -> Result<(), FdxError> {
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
    let mut versions = root.attributes.iter_mut().filter(|attribute| {
        attribute.name.namespace_uri.is_none() && attribute.name.local_name == "Version"
    });
    let version = versions.next().ok_or(FdxError::MissingVersion)?;
    if versions.next().is_some() {
        return Err(FdxError::DuplicateVersion);
    }
    version.value = FDX_VERSION.to_owned();
    Ok(())
}

pub(crate) fn version_status(root: &RawElement) -> Result<FdxVersionStatus, FdxError> {
    let mut declarations = root.attributes.iter().filter(|attribute| {
        attribute.name.namespace_uri.is_none() && attribute.name.local_name == "Version"
    });
    let Some(declaration) = declarations.next() else {
        return Ok(FdxVersionStatus::Missing);
    };
    if declarations.next().is_some() {
        return Ok(FdxVersionStatus::Duplicate);
    }
    let declared = declaration.value.as_str();
    if declared.is_empty() || !declared.bytes().all(|byte| byte.is_ascii_digit()) {
        return Ok(FdxVersionStatus::Malformed);
    }
    let normalized = declared.trim_start_matches('0');
    let normalized = if normalized.is_empty() {
        "0"
    } else {
        normalized
    };
    if normalized == FDX_VERSION {
        return Ok(FdxVersionStatus::Version6);
    }
    let direction = if normalized.len() < FDX_VERSION.len()
        || (normalized.len() == FDX_VERSION.len() && normalized < FDX_VERSION)
    {
        FdxVersionDirection::Older
    } else {
        FdxVersionDirection::Newer
    };
    Ok(FdxVersionStatus::ExplicitNon6 {
        declared: declared.to_owned(),
        direction,
    })
}

fn is_fdx_element(element: &RawElement, name: &str, root_ns: Option<&str>) -> bool {
    element.name.local_name == name && element.name.namespace_uri.as_deref() == root_ns
}
