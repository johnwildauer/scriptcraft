//! Semantic, identity, budget, and retained-source verification for FDX output.

use crate::diagnostics::{FdxError, SaveDiagnosticCode};
use crate::reader::validate_xml_chars;
use crate::writer::validate_qname;
use crate::{
    MAX_ATTRIBUTES_PER_ELEMENT, MAX_SOURCE_EVENTS, MAX_SOURCE_NAME_OR_VALUE_BYTES,
    MAX_SOURCE_TEXT_BYTES,
};
use scriptcraft_model::dual_dialogue::DualDialogueGroup;
use scriptcraft_model::preservation::{
    ForeignIdDomain, ForeignIdentity, PreservedNode, PreservedSection, SourceAnchorId, SourceName,
    SourceOwnership,
};
use scriptcraft_model::{BodyOwner, Project, TitlePageParagraph};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IdentityDomainSummary {
    Document,
    Paragraph,
    Scene,
    Beat,
    CastMember,
    Character,
    ListItem,
    Board,
    Image,
    Revision,
    Tag,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct IdentitySetDifference {
    domain: IdentityDomainSummary,
    missing: bool,
    unexpected: bool,
}

pub(crate) fn same_non_paragraph_identities(
    left: &[ForeignIdentity],
    right: &[ForeignIdentity],
) -> bool {
    left.iter()
        .filter(|identity| identity.domain != ForeignIdDomain::Paragraph)
        .eq(right
            .iter()
            .filter(|identity| identity.domain != ForeignIdDomain::Paragraph))
}

pub(crate) fn identity_set(identities: &[ForeignIdentity]) -> BTreeSet<ForeignIdentity> {
    identities.iter().cloned().collect()
}

pub(crate) fn first_identity_set_difference(
    expected: &BTreeSet<ForeignIdentity>,
    actual: &BTreeSet<ForeignIdentity>,
) -> Option<IdentitySetDifference> {
    if let Some(identity) = expected.difference(actual).next() {
        return Some(IdentitySetDifference {
            domain: identity_domain_summary(&identity.domain),
            missing: true,
            unexpected: false,
        });
    }
    actual
        .difference(expected)
        .next()
        .map(|identity| IdentitySetDifference {
            domain: identity_domain_summary(&identity.domain),
            missing: false,
            unexpected: true,
        })
}

fn identity_domain_summary(domain: &ForeignIdDomain) -> IdentityDomainSummary {
    match domain {
        ForeignIdDomain::Document => IdentityDomainSummary::Document,
        ForeignIdDomain::Paragraph => IdentityDomainSummary::Paragraph,
        ForeignIdDomain::Scene => IdentityDomainSummary::Scene,
        ForeignIdDomain::Beat => IdentityDomainSummary::Beat,
        ForeignIdDomain::CastMember => IdentityDomainSummary::CastMember,
        ForeignIdDomain::Character => IdentityDomainSummary::Character,
        ForeignIdDomain::ListItem => IdentityDomainSummary::ListItem,
        ForeignIdDomain::Board => IdentityDomainSummary::Board,
        ForeignIdDomain::Image => IdentityDomainSummary::Image,
        ForeignIdDomain::Revision => IdentityDomainSummary::Revision,
        ForeignIdDomain::Tag => IdentityDomainSummary::Tag,
        ForeignIdDomain::Other(_) => IdentityDomainSummary::Other,
    }
}

pub(crate) fn same_semantic_elements(
    left: &[std::sync::Arc<BodyOwner>],
    right: &[std::sync::Arc<BodyOwner>],
) -> bool {
    left.len() == right.len()
        && left.iter().zip(right).all(|(left, right)| {
            left.element_type() == right.element_type()
                && left.text == right.text
                && if left.runs.is_empty() {
                    right.runs.iter().all(|run| run.attributes.is_empty())
                } else {
                    left.runs == right.runs
                }
        })
}

pub(crate) fn same_semantic_element_definitions(left: &Project, right: &Project) -> bool {
    left.element_definitions() == right.element_definitions()
}

pub fn same_dual_dialogue_projection(left: &Project, right: &Project) -> bool {
    fn member_positions(project: &Project, group: &DualDialogueGroup) -> Option<Vec<usize>> {
        group
            .member_ids()
            .map(|id| {
                project
                    .elements
                    .iter()
                    .position(|element| &element.id == id)
            })
            .collect()
    }

    fn active_side_positions(
        project: &Project,
        group: &DualDialogueGroup,
    ) -> Option<(Vec<usize>, Vec<usize>)> {
        let projection =
            scriptcraft_model::dual_dialogue::active_dual_dialogue_projection(project, group)?;
        let positions = |side: &scriptcraft_model::dual_dialogue::DualDialogueSide| {
            side.member_ids()
                .map(|id| {
                    project
                        .elements
                        .iter()
                        .position(|element| &element.id == id)
                })
                .collect::<Option<Vec<_>>>()
        };
        Some((positions(&projection.left)?, positions(&projection.right)?))
    }

    if left.dual_dialogue_groups.len() != right.dual_dialogue_groups.len() {
        return false;
    }
    for (left_group, right_group) in left
        .dual_dialogue_groups
        .iter()
        .zip(&right.dual_dialogue_groups)
    {
        if left_group.members.len() != right_group.members.len()
            || active_side_positions(left, left_group) != active_side_positions(right, right_group)
            || member_positions(left, left_group) != member_positions(right, right_group)
            || (left.preservation.is_some() && left_group.members != right_group.members)
        {
            return false;
        }
        if left.preservation.is_some()
            && left_group.wrapper.outer_foreign_id != right_group.wrapper.outer_foreign_id
        {
            return false;
        }
        if let Some(expected) = left_group.wrapper.source.as_ref() {
            if !right_group.wrapper.source.as_ref().is_some_and(|actual| {
                expected.format == actual.format
                    && expected.section == actual.section
                    && expected.ordinal == actual.ordinal
                    && expected.foreign_id == actual.foreign_id
                    && expected.foreign_role == actual.foreign_role
            }) {
                return false;
            }
        }
        let left_overrides = left.dual_dialogue_paragraph_overrides.get(&left_group.id);
        let right_overrides = right.dual_dialogue_paragraph_overrides.get(&right_group.id);
        if left_overrides != right_overrides {
            return false;
        }
        if let Some(positions) = member_positions(left, left_group) {
            for position in positions {
                let Some(left_element) = left.elements.get(position) else {
                    return false;
                };
                let Some(right_element) = right.elements.get(position) else {
                    return false;
                };
                let expected_source = left_element.source_provenance.as_ref();
                let actual_source = right_element.source_provenance.as_ref();
                if let Some(expected_source) = expected_source {
                    if let Some(expected_foreign_id) = expected_source.foreign_id.as_deref() {
                        if !actual_source.is_some_and(|actual| {
                            actual.foreign_id.as_deref() == Some(expected_foreign_id)
                                && actual.section == expected_source.section
                        }) {
                            return false;
                        }
                    }
                }
                if left.direct_paragraph_overrides.get(&left_element.id)
                    != right.direct_paragraph_overrides.get(&right_element.id)
                {
                    return false;
                }
            }
        }
    }
    left.elements.len() == right.elements.len()
}

pub(crate) fn same_semantic_title_paragraphs(
    left: &[TitlePageParagraph],
    right: &[TitlePageParagraph],
) -> bool {
    left.len() == right.len()
        && left.iter().zip(right).all(|(left, right)| {
            left.element_type() == right.element_type()
                && left.text == right.text
                && left.formatting == right.formatting
                && if left.runs.is_empty() {
                    right.runs.iter().all(|run| run.attributes.is_empty())
                } else {
                    left.runs == right.runs
                }
        })
}

pub(crate) fn same_writer_semantic_projection(left: &Project, right: &Project) -> bool {
    left.page_layout() == right.page_layout()
        && same_semantic_element_definitions(left, right)
        && same_semantic_title_paragraphs(&left.title_page, &right.title_page)
        && same_semantic_elements(&left.elements, &right.elements)
        && same_dual_dialogue_projection(left, right)
}

pub(crate) fn verify_writer_semantic_projection(
    expected: &Project,
    actual: &Project,
) -> Result<(), FdxError> {
    if same_writer_semantic_projection(expected, actual) {
        Ok(())
    } else {
        Err(SaveDiagnosticCode::WriteSemanticRereadMismatch.error())
    }
}


pub(crate) fn validate_project_output_budgets(project: &Project) -> Result<(), FdxError> {
    let mut text_bytes = 0usize;
    let mut event_count = 3usize;
    for (text, runs) in project
        .title_page
        .iter()
        .map(|element| (element.text.as_str(), element.runs.as_slice()))
        .chain(
            project
                .elements
                .iter()
                .map(|element| (element.text.as_str(), element.runs.as_slice())),
        )
    {
        event_count = event_count.saturating_add(1);
        text_bytes = text_bytes.saturating_add(text.len());
        let run_count = runs.len().max(usize::from(runs.is_empty()));
        event_count = event_count.saturating_add(run_count);
        if runs.is_empty() && !text.is_empty() {
            event_count = event_count.saturating_add(1);
        }
        for run in runs {
            if !run.text.is_empty() {
                event_count = event_count.saturating_add(1);
            }
            if run.attributes.len() > MAX_ATTRIBUTES_PER_ELEMENT {
                return Err(FdxError::ResourceLimit);
            }
            event_count = event_count.saturating_add(run.attributes.len());
            for attribute in &run.attributes {
                if attribute.name.len() > MAX_SOURCE_NAME_OR_VALUE_BYTES
                    || attribute.value.len() > MAX_SOURCE_NAME_OR_VALUE_BYTES
                {
                    return Err(FdxError::ResourceLimit);
                }
                validate_qname(&attribute.name)?;
                validate_xml_chars(&attribute.value)?;
            }
        }
        if text_bytes > MAX_SOURCE_TEXT_BYTES || event_count > MAX_SOURCE_EVENTS {
            return Err(FdxError::ResourceLimit);
        }
    }
    Ok(())
}
pub(crate) fn normalize_sections(sections: &[PreservedSection]) -> Vec<PreservedSection> {
    sections
        .iter()
        .cloned()
        .map(|mut section| {
            section.children = normalize_source_tree(&section.children).into();
            section
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StructuralNodeKind {
    Element,
    Text,
    CData,
    Comment,
    ProcessingInstruction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StructuralNodeSummary {
    kind: StructuralNodeKind,
    namespace_uri: Option<String>,
    local_name: Option<String>,
    child_count: usize,
    attribute_count: usize,
}

/// Private, value-free first-difference metadata for unit tests. The path is a
/// sequence of ordered child indexes prefixed by the source-section index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceTreeDifference {
    node_path: Vec<usize>,
    field: &'static str,
    expected_detail: Option<String>,
    actual_detail: Option<String>,
    expected: Option<StructuralNodeSummary>,
    actual: Option<StructuralNodeSummary>,
}

impl SourceTreeDifference {
    pub(crate) fn public_detail(&self) -> String {
        const MAX_DETAIL_BYTES: usize = 240;
        const MAX_PATH_DEPTH: usize = 12;
        let mut path = String::from("section[");
        path.push_str(&self.node_path.first().copied().unwrap_or(0).to_string());
        path.push(']');
        for (depth, index) in self.node_path.iter().skip(1).enumerate() {
            if depth >= MAX_PATH_DEPTH {
                path.push_str("/…");
                break;
            }
            path.push_str("/node[");
            path.push_str(&index.to_string());
            path.push(']');
        }
        let expected = self.expected_detail.as_deref().unwrap_or("absent");
        let actual = self.actual_detail.as_deref().unwrap_or("absent");
        let detail = format!(
            "path={path}; field={}; expected={expected}; actual={actual}",
            self.field
        );
        if detail.len() <= MAX_DETAIL_BYTES {
            detail
        } else {
            let mut bounded = detail;
            bounded.truncate(MAX_DETAIL_BYTES.saturating_sub(3));
            bounded.push_str("...");
            bounded
        }
    }
}

fn difference(
    node_path: Vec<usize>,
    field: &'static str,
    expected_detail: Option<String>,
    actual_detail: Option<String>,
    expected: Option<StructuralNodeSummary>,
    actual: Option<StructuralNodeSummary>,
) -> SourceTreeDifference {
    SourceTreeDifference {
        node_path,
        field,
        expected_detail,
        actual_detail,
        expected,
        actual,
    }
}

fn safe_xml_name(name: &str) -> String {
    const MAX_NAME_BYTES: usize = 36;
    let mut safe = String::new();
    for character in name.chars() {
        if safe.len() >= MAX_NAME_BYTES {
            safe.push_str("...");
            break;
        }
        safe.push(
            if character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.') {
                character
            } else {
                '_'
            },
        );
    }
    safe
}

fn safe_name_detail(value: Option<&str>) -> Option<String> {
    value.map(safe_xml_name)
}

fn source_name_difference(
    expected: &SourceName,
    actual: &SourceName,
) -> Option<(&'static str, Option<String>, Option<String>)> {
    if expected.local_name != actual.local_name {
        return Some((
            "element.local-name",
            safe_name_detail(Some(&expected.local_name)),
            safe_name_detail(Some(&actual.local_name)),
        ));
    }
    if expected.namespace_uri != actual.namespace_uri {
        return Some((
            "element.namespace",
            Some(
                if expected.namespace_uri.is_some() {
                    "present"
                } else {
                    "absent"
                }
                .into(),
            ),
            Some(
                if actual.namespace_uri.is_some() {
                    "present"
                } else {
                    "absent"
                }
                .into(),
            ),
        ));
    }
    if expected.qualified_name != actual.qualified_name {
        return Some((
            "element.qualified-name",
            Some(
                if expected.qualified_name.is_some() {
                    "present"
                } else {
                    "absent"
                }
                .into(),
            ),
            Some(
                if actual.qualified_name.is_some() {
                    "present"
                } else {
                    "absent"
                }
                .into(),
            ),
        ));
    }
    None
}

fn ownership_category(ownership: &SourceOwnership) -> &'static str {
    match ownership {
        SourceOwnership::Opaque => "opaque",
        SourceOwnership::Managed(_) => "managed",
        SourceOwnership::DualDialogueWrapper(_) => "dual_dialogue_wrapper",
        SourceOwnership::Attached(_) => "attached",
    }
}

fn anchor_category(anchor: &Option<SourceAnchorId>) -> &'static str {
    if anchor.is_some() {
        "present"
    } else {
        "absent"
    }
}

fn text_classification(text: &str) -> String {
    let class = if text.is_empty() {
        "empty"
    } else if text.chars().all(char::is_whitespace) {
        "whitespace-only"
    } else {
        "contains-non-whitespace"
    };
    format!("length={},{}", text.chars().count(), class)
}

fn node_description(node: &PreservedNode) -> String {
    match node {
        PreservedNode::Element(element) => format!(
            "element:{};children={};attributes={}",
            safe_xml_name(&element.name.local_name),
            element.children.len(),
            element.attributes.len()
        ),
        PreservedNode::Text(text) => format!("text:{}", text_classification(text)),
        PreservedNode::CData(text) => format!("cdata:{}", text_classification(text)),
        PreservedNode::Comment(text) => format!("comment:{}", text_classification(text)),
        PreservedNode::ProcessingInstruction { .. } => "processing-instruction".into(),
    }
}

fn structural_node_summary(node: &PreservedNode) -> StructuralNodeSummary {
    match node {
        PreservedNode::Element(element) => StructuralNodeSummary {
            kind: StructuralNodeKind::Element,
            namespace_uri: element.name.namespace_uri.clone(),
            local_name: Some(element.name.local_name.clone()),
            child_count: element.children.len(),
            attribute_count: element.attributes.len(),
        },
        PreservedNode::Text(_) => StructuralNodeSummary {
            kind: StructuralNodeKind::Text,
            namespace_uri: None,
            local_name: None,
            child_count: 0,
            attribute_count: 0,
        },
        PreservedNode::CData(_) => StructuralNodeSummary {
            kind: StructuralNodeKind::CData,
            namespace_uri: None,
            local_name: None,
            child_count: 0,
            attribute_count: 0,
        },
        PreservedNode::Comment(_) => StructuralNodeSummary {
            kind: StructuralNodeKind::Comment,
            namespace_uri: None,
            local_name: None,
            child_count: 0,
            attribute_count: 0,
        },
        PreservedNode::ProcessingInstruction { .. } => StructuralNodeSummary {
            kind: StructuralNodeKind::ProcessingInstruction,
            namespace_uri: None,
            local_name: None,
            child_count: 0,
            attribute_count: 0,
        },
    }
}

pub(crate) fn first_source_tree_difference(
    expected: &[PreservedSection],
    actual: &[PreservedSection],
) -> Option<SourceTreeDifference> {
    let section_count = expected.len().max(actual.len());
    for section_index in 0..section_count {
        let expected_section = expected.get(section_index);
        let actual_section = actual.get(section_index);
        let Some(expected_section) = expected_section else {
            return Some(difference(
                vec![section_index],
                "section.presence",
                None,
                actual_section
                    .map(|section| format!("kind={:?},ordinal={}", section.kind, section.ordinal)),
                None,
                actual_section
                    .and_then(|section| section.children.first())
                    .map(structural_node_summary),
            ));
        };
        let Some(actual_section) = actual_section else {
            return Some(difference(
                vec![section_index],
                "section.presence",
                Some(format!(
                    "kind={:?},ordinal={}",
                    expected_section.kind, expected_section.ordinal
                )),
                None,
                expected_section
                    .children
                    .first()
                    .map(structural_node_summary),
                None,
            ));
        };
        let section_path = vec![section_index];
        if let Some((field, left, right)) =
            source_name_difference(&expected_section.name, &actual_section.name)
        {
            let field = match field {
                "element.local-name" => "section.local-name",
                "element.namespace" => "section.namespace",
                _ => "section.qualified-name",
            };
            return Some(difference(
                section_path,
                field,
                left,
                right,
                expected_section
                    .children
                    .first()
                    .map(structural_node_summary),
                actual_section.children.first().map(structural_node_summary),
            ));
        }
        if expected_section.kind != actual_section.kind {
            return Some(difference(
                section_path,
                "section.kind",
                Some(format!("{:?}", expected_section.kind)),
                Some(format!("{:?}", actual_section.kind)),
                expected_section
                    .children
                    .first()
                    .map(structural_node_summary),
                actual_section.children.first().map(structural_node_summary),
            ));
        }
        if expected_section.ordinal != actual_section.ordinal {
            return Some(difference(
                section_path,
                "section.ordinal",
                Some(expected_section.ordinal.to_string()),
                Some(actual_section.ordinal.to_string()),
                expected_section
                    .children
                    .first()
                    .map(structural_node_summary),
                actual_section.children.first().map(structural_node_summary),
            ));
        }
        if let Some(difference) = first_node_sequence_difference(
            &expected_section.children,
            &actual_section.children,
            vec![section_index],
        ) {
            return Some(difference);
        }
    }
    None
}

fn first_node_sequence_difference(
    expected: &[PreservedNode],
    actual: &[PreservedNode],
    path: Vec<usize>,
) -> Option<SourceTreeDifference> {
    let node_count = expected.len().max(actual.len());
    for child_index in 0..node_count {
        let mut child_path = path.clone();
        child_path.push(child_index);
        let expected_node = expected.get(child_index);
        let actual_node = actual.get(child_index);
        let (Some(expected_node), Some(actual_node)) = (expected_node, actual_node) else {
            let expected_detail = expected_node.map_or_else(
                || format!("absent;sibling-count={}", expected.len()),
                |node| {
                    format!(
                        "{};sibling-count={}",
                        node_description(node),
                        expected.len()
                    )
                },
            );
            let actual_detail = actual_node.map_or_else(
                || format!("absent;sibling-count={}", actual.len()),
                |node| format!("{};sibling-count={}", node_description(node), actual.len()),
            );
            return Some(difference(
                child_path,
                "child.presence",
                Some(expected_detail),
                Some(actual_detail),
                expected_node.map(structural_node_summary),
                actual_node.map(structural_node_summary),
            ));
        };
        if expected_node != actual_node
            && (actual[child_index + 1..].contains(expected_node)
                || expected[child_index + 1..].contains(actual_node))
        {
            return Some(difference(
                child_path,
                "child.order",
                Some(node_description(expected_node)),
                Some(node_description(actual_node)),
                Some(structural_node_summary(expected_node)),
                Some(structural_node_summary(actual_node)),
            ));
        }
        match (expected_node, actual_node) {
            (PreservedNode::Element(expected_element), PreservedNode::Element(actual_element)) => {
                let expected_summary = Some(structural_node_summary(expected_node));
                let actual_summary = Some(structural_node_summary(actual_node));
                if let Some((field, left, right)) =
                    source_name_difference(&expected_element.name, &actual_element.name)
                {
                    return Some(difference(
                        child_path,
                        field,
                        left,
                        right,
                        expected_summary,
                        actual_summary,
                    ));
                }
                let common_attributes = expected_element
                    .attributes
                    .len()
                    .min(actual_element.attributes.len());
                for attribute_index in 0..common_attributes {
                    let left = &expected_element.attributes[attribute_index];
                    let right = &actual_element.attributes[attribute_index];
                    if left.name != right.name {
                        return Some(difference(
                            child_path,
                            "attribute.name-or-order",
                            safe_name_detail(Some(&left.name.local_name)),
                            safe_name_detail(Some(&right.name.local_name)),
                            expected_summary,
                            actual_summary,
                        ));
                    }
                    if left.value != right.value {
                        return Some(difference(
                            child_path,
                            "attribute.value",
                            safe_name_detail(Some(&left.name.local_name)),
                            safe_name_detail(Some(&right.name.local_name)),
                            expected_summary,
                            actual_summary,
                        ));
                    }
                }
                if expected_element.attributes.len() != actual_element.attributes.len() {
                    let left = expected_element
                        .attributes
                        .get(common_attributes)
                        .map(|attribute| safe_xml_name(&attribute.name.local_name));
                    let right = actual_element
                        .attributes
                        .get(common_attributes)
                        .map(|attribute| safe_xml_name(&attribute.name.local_name));
                    return Some(difference(
                        child_path,
                        "attribute.count-or-presence",
                        left,
                        right,
                        expected_summary,
                        actual_summary,
                    ));
                }
                let common_bindings = expected_element
                    .namespace_bindings
                    .len()
                    .min(actual_element.namespace_bindings.len());
                for binding_index in 0..common_bindings {
                    let left = &expected_element.namespace_bindings[binding_index];
                    let right = &actual_element.namespace_bindings[binding_index];
                    if left.prefix != right.prefix {
                        return Some(difference(
                            child_path,
                            "namespace-binding.prefix-or-order",
                            Some(if left.prefix.is_empty() {
                                "default".into()
                            } else {
                                safe_xml_name(&left.prefix)
                            }),
                            Some(if right.prefix.is_empty() {
                                "default".into()
                            } else {
                                safe_xml_name(&right.prefix)
                            }),
                            expected_summary,
                            actual_summary,
                        ));
                    }
                    if left.namespace_uri != right.namespace_uri {
                        return Some(difference(
                            child_path,
                            "namespace-binding.namespace",
                            Some("present".into()),
                            Some("present".into()),
                            expected_summary,
                            actual_summary,
                        ));
                    }
                }
                if expected_element.namespace_bindings.len()
                    != actual_element.namespace_bindings.len()
                {
                    return Some(difference(
                        child_path,
                        "namespace-binding.count-or-presence",
                        expected_element
                            .namespace_bindings
                            .get(common_bindings)
                            .map(|binding| safe_xml_name(&binding.prefix)),
                        actual_element
                            .namespace_bindings
                            .get(common_bindings)
                            .map(|binding| safe_xml_name(&binding.prefix)),
                        expected_summary,
                        actual_summary,
                    ));
                }
                if expected_element.source_type != actual_element.source_type {
                    return Some(difference(
                        child_path,
                        "source-type",
                        Some(if expected_element.source_type.is_some() {
                            "present".into()
                        } else {
                            "absent".into()
                        }),
                        Some(if actual_element.source_type.is_some() {
                            "present".into()
                        } else {
                            "absent".into()
                        }),
                        expected_summary,
                        actual_summary,
                    ));
                }
                if expected_element.ownership != actual_element.ownership {
                    return Some(difference(
                        child_path,
                        "ownership",
                        Some(ownership_category(&expected_element.ownership).into()),
                        Some(ownership_category(&actual_element.ownership).into()),
                        expected_summary,
                        actual_summary,
                    ));
                }
                if expected_element.dependency_confidence != actual_element.dependency_confidence {
                    return Some(difference(
                        child_path,
                        "dependency-confidence",
                        Some(format!("{:?}", expected_element.dependency_confidence)),
                        Some(format!("{:?}", actual_element.dependency_confidence)),
                        expected_summary,
                        actual_summary,
                    ));
                }
                // Anchor identifiers are in-memory keys and are allocated
                // again when the emitted XML is re-read. Retain the semantic
                // source projection check: a winning settings node must stay
                // anchored and an inert duplicate must stay unanchored.
                if expected_element.anchor.is_some() != actual_element.anchor.is_some() {
                    return Some(difference(
                        child_path,
                        "anchor",
                        Some(anchor_category(&expected_element.anchor).into()),
                        Some(anchor_category(&actual_element.anchor).into()),
                        expected_summary,
                        actual_summary,
                    ));
                }
                if let Some(difference) = first_node_sequence_difference(
                    &expected_element.children,
                    &actual_element.children,
                    child_path,
                ) {
                    return Some(difference);
                }
            }
            (PreservedNode::Text(expected_text), PreservedNode::Text(actual_text))
                if expected_text == actual_text => {}
            (PreservedNode::Text(expected_text), PreservedNode::Text(actual_text)) => {
                return Some(difference(
                    child_path,
                    "text.content",
                    Some(text_classification(expected_text)),
                    Some(text_classification(actual_text)),
                    Some(structural_node_summary(expected_node)),
                    Some(structural_node_summary(actual_node)),
                ));
            }
            (PreservedNode::CData(expected_text), PreservedNode::CData(actual_text))
                if expected_text == actual_text => {}
            (PreservedNode::CData(expected_text), PreservedNode::CData(actual_text)) => {
                return Some(difference(
                    child_path,
                    "cdata.content",
                    Some(text_classification(expected_text)),
                    Some(text_classification(actual_text)),
                    Some(structural_node_summary(expected_node)),
                    Some(structural_node_summary(actual_node)),
                ));
            }
            (PreservedNode::Comment(expected_text), PreservedNode::Comment(actual_text))
                if expected_text == actual_text => {}
            (PreservedNode::Comment(expected_text), PreservedNode::Comment(actual_text)) => {
                return Some(difference(
                    child_path,
                    "comment.content",
                    Some(text_classification(expected_text)),
                    Some(text_classification(actual_text)),
                    Some(structural_node_summary(expected_node)),
                    Some(structural_node_summary(actual_node)),
                ));
            }
            (
                PreservedNode::ProcessingInstruction {
                    target: expected_target,
                    data: expected_data,
                },
                PreservedNode::ProcessingInstruction {
                    target: actual_target,
                    data: actual_data,
                },
            ) if expected_target == actual_target && expected_data == actual_data => {}
            (
                PreservedNode::ProcessingInstruction {
                    target: expected_target,
                    data: expected_data,
                },
                PreservedNode::ProcessingInstruction {
                    target: actual_target,
                    data: actual_data,
                },
            ) => {
                let (field, left, right) = if expected_target != actual_target {
                    (
                        "processing-instruction.target",
                        Some("present".into()),
                        Some("present".into()),
                    )
                } else {
                    (
                        "processing-instruction.data",
                        Some(text_classification(expected_data)),
                        Some(text_classification(actual_data)),
                    )
                };
                return Some(difference(
                    child_path,
                    field,
                    left,
                    right,
                    Some(structural_node_summary(expected_node)),
                    Some(structural_node_summary(actual_node)),
                ));
            }
            _ => {
                return Some(difference(
                    child_path,
                    "node.kind-or-order",
                    Some(node_description(expected_node)),
                    Some(node_description(actual_node)),
                    Some(structural_node_summary(expected_node)),
                    Some(structural_node_summary(actual_node)),
                ));
            }
        }
    }
    None
}

fn normalize_source_tree(nodes: &[PreservedNode]) -> Vec<PreservedNode> {
    let mut normalized = normalize_text_nodes(nodes);
    let normalized_definition_anchor = SourceAnchorId::for_element_definition(0).ok();
    let mut stack = normalized.iter_mut().collect::<Vec<_>>();
    while let Some(node) = stack.pop() {
        if let PreservedNode::Element(element) = node {
            if matches!(element.ownership, SourceOwnership::Managed(_)) {
                // Semantic element IDs and source-anchor IDs are session-local;
                // they are not serialized FDX fields and may be reallocated on
                // reread. Compare the source XML structure, not these keys.
                element.ownership = SourceOwnership::Opaque;
                element.anchor = None;
            } else if element.name.local_name == "ElementSettings" {
                // Definition anchors are not serialized. Normalize only their
                // allocation identity while preserving presence, so winners
                // remain distinguishable from inert, unanchored duplicates.
                if element.anchor.is_some() {
                    element.anchor = normalized_definition_anchor
                        .clone()
                        .or_else(|| element.anchor.clone());
                }
            }
            stack.extend(element.children.iter_mut());
        }
    }
    normalized
}

fn normalize_text_nodes(nodes: &[PreservedNode]) -> Vec<PreservedNode> {
    let mut normalized = Vec::<PreservedNode>::new();
    for node in nodes {
        let normalized_node = match node {
            PreservedNode::Element(element) => {
                let mut element = element.clone();
                element.children = normalize_text_nodes(&element.children).into();
                PreservedNode::Element(element)
            }
            other => other.clone(),
        };
        if matches!(&normalized_node, PreservedNode::Text(text) if text.is_empty()) {
            // The XML writer emits no bytes for an empty text node, so a reader
            // cannot reconstruct it. Omit it only at this comparison boundary.
            continue;
        }
        if let PreservedNode::Text(text) = normalized_node {
            if let Some(PreservedNode::Text(previous)) = normalized.last_mut() {
                previous.push_str(&text);
            } else {
                normalized.push(PreservedNode::Text(text));
            }
        } else {
            normalized.push(normalized_node);
        }
    }
    normalized
}
