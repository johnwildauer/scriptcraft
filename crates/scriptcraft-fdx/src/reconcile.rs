//! Source-backed projection of canonical model state into retained FDX nodes.
//! The FDX writer owns this reconciliation between model state and retained source nodes.

use crate::constants::FDX_ROOT;
use crate::diagnostics::{save_diagnostic_at, FdxError, SaveDiagnosticCode};
use crate::reader::validate_xml_chars;
use crate::writer::validate_qname;
use scriptcraft_model::dual_dialogue::DualDialogueGroup;
use scriptcraft_model::element_type::ElementTypeKey;
use scriptcraft_model::preservation::{
    is_user_paragraph_data_name, DependencyConfidence, PreservedAttribute, PreservedElement,
    PreservedNode, PreservedSection, SectionKind, SharedVec, SourceName, SourceOwnership,
};
use scriptcraft_model::title_page::TitlePageParagraph;
use scriptcraft_model::{Project, RunAttribute, ScreenplayElement, StableId, TextRun};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn reconcile_project_elements(
    sections: &mut [PreservedSection],
    project: &Project,
) -> Result<(), FdxError> {
    let body = project
        .elements
        .iter()
        .map(|element| (element.id.clone(), element.as_element()))
        .collect::<BTreeMap<_, _>>();
    let title = project
        .title_page
        .iter()
        .map(|element| (element.id.clone(), element))
        .collect::<BTreeMap<_, _>>();
    let groups = project
        .dual_dialogue_groups
        .iter()
        .map(|group| (group.id.clone(), group))
        .collect::<BTreeMap<_, _>>();
    let mut body_order = Vec::new();
    let mut title_order = Vec::new();
    let mut group_order = Vec::new();
    let mut seen = BTreeSet::new();
    let mut roots = 0usize;
    let root_namespace = sections
        .iter()
        .flat_map(|section| &section.children)
        .find_map(|node| match node {
            PreservedNode::Element(element) if element.name.local_name == FDX_ROOT => {
                element.name.namespace_uri.clone()
            }
            _ => None,
        });
    for section in sections.iter_mut() {
        for node in &mut section.children {
            reconcile_node(
                node,
                SectionKind::Other,
                0,
                root_namespace.as_deref(),
                &body,
                &title,
                &groups,
                false,
                &mut body_order,
                &mut title_order,
                &mut group_order,
                &mut seen,
                &mut roots,
            )
            .map_err(|error| {
                save_diagnostic_at(error, SaveDiagnosticCode::ReconcileProjectNodePropagation)
            })?;
        }
    }
    if roots != 1
        || body_order
            != project
                .elements
                .iter()
                .map(|element| element.id.clone())
                .collect::<Vec<_>>()
        || title_order
            != project
                .title_page
                .iter()
                .map(|element| element.id.clone())
                .collect::<Vec<_>>()
        || group_order
            != project
                .dual_dialogue_groups
                .iter()
                .map(|group| group.id.clone())
                .collect::<Vec<_>>()
    {
        return Err(SaveDiagnosticCode::ReconcileProjectManagedOrderMismatch.error());
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn reconcile_node(
    node: &mut PreservedNode,
    section: SectionKind,
    depth: usize,
    root_namespace: Option<&str>,
    body: &BTreeMap<StableId, &ScreenplayElement>,
    title: &BTreeMap<StableId, &TitlePageParagraph>,
    groups: &BTreeMap<StableId, &DualDialogueGroup>,
    in_title_content: bool,
    body_order: &mut Vec<StableId>,
    title_order: &mut Vec<StableId>,
    group_order: &mut Vec<StableId>,
    seen: &mut BTreeSet<StableId>,
    roots: &mut usize,
) -> Result<(), FdxError> {
    let PreservedNode::Element(element) = node else {
        return Ok(());
    };
    if depth == 0 && element.name.local_name == FDX_ROOT {
        *roots = roots.checked_add(1).ok_or(FdxError::ResourceLimit)?;
    }
    let is_fdx_element = element.name.namespace_uri.as_deref() == root_namespace;
    let current_section = if depth == 1 && is_fdx_element && element.name.local_name == "Content" {
        SectionKind::Content
    } else if depth == 1 && is_fdx_element && element.name.local_name == "TitlePage" {
        SectionKind::TitlePage
    } else {
        section
    };
    let current_title_content = current_section == SectionKind::TitlePage
        && (in_title_content
            || (depth == 2 && is_fdx_element && element.name.local_name == "Content"));
    if let SourceOwnership::DualDialogueWrapper(id) = element.ownership.clone() {
        if current_section != SectionKind::Content
            || depth != 2
            || !is_fdx_element
            || element.name.local_name != "Paragraph"
            || !seen.insert(id.clone())
        {
            return Err(SaveDiagnosticCode::ReconcileNodeManagedOutsideSection.error());
        }
        let group = groups
            .get(&id)
            .copied()
            .ok_or_else(|| SaveDiagnosticCode::ReconcileNodeSemanticElementMissing.error())?;
        reconcile_dual_dialogue_wrapper(element, group, root_namespace, body, body_order, seen)?;
        group_order.push(id);
        return Ok(());
    }
    if let SourceOwnership::Managed(id) = element.ownership.clone() {
        if !seen.insert(id.clone()) {
            return Err(SaveDiagnosticCode::ReconcileNodeDuplicateManagedIdentity.error());
        }
        match current_section {
            SectionKind::Content => {
                if depth != 2 || !is_fdx_element || element.name.local_name != "Paragraph" {
                    return Err(SaveDiagnosticCode::ReconcileNodeManagedOutsideSection.error());
                }
                body_order.push(id.clone());
                let semantic = body.get(&id).copied().ok_or_else(|| {
                    SaveDiagnosticCode::ReconcileNodeSemanticElementMissing.error()
                })?;
                reconcile_managed_paragraph(element, semantic).map_err(|error| {
                    save_diagnostic_at(
                        error,
                        SaveDiagnosticCode::ReconcileManagedParagraphPropagation,
                    )
                })?;
            }
            SectionKind::TitlePage => {
                let exact_title_paragraph = is_fdx_element
                    && element.name.local_name == "Paragraph"
                    && depth == 3
                    && in_title_content;
                if !exact_title_paragraph {
                    return Err(SaveDiagnosticCode::ReconcileNodeManagedOutsideSection.error());
                }
                title_order.push(id.clone());
                let semantic = title.get(&id).copied().ok_or_else(|| {
                    SaveDiagnosticCode::ReconcileNodeSemanticElementMissing.error()
                })?;
                reconcile_managed_title_paragraph(element, semantic).map_err(|error| {
                    save_diagnostic_at(
                        error,
                        SaveDiagnosticCode::ReconcileManagedParagraphPropagation,
                    )
                })?;
            }
            SectionKind::Other => {
                return Err(SaveDiagnosticCode::ReconcileNodeManagedOutsideSection.error())
            }
        }
        return Ok(());
    }
    for child in &mut element.children {
        reconcile_node(
            child,
            current_section,
            depth.saturating_add(1),
            root_namespace,
            body,
            title,
            groups,
            current_title_content,
            body_order,
            title_order,
            group_order,
            seen,
            roots,
        )
        .map_err(|error| {
            save_diagnostic_at(error, SaveDiagnosticCode::ReconcileProjectChildPropagation)
        })?;
    }
    Ok(())
}

fn reconcile_dual_dialogue_wrapper(
    outer: &mut PreservedElement,
    group: &DualDialogueGroup,
    root_namespace: Option<&str>,
    body: &BTreeMap<StableId, &ScreenplayElement>,
    body_order: &mut Vec<StableId>,
    seen: &mut BTreeSet<StableId>,
) -> Result<(), FdxError> {
    if outer.source_type.as_deref() != Some("General")
        || outer
            .attributes
            .iter()
            .filter(|attribute| {
                attribute.name.namespace_uri.is_none() && attribute.name.local_name == "Type"
            })
            .count()
            != 1
        || !outer.attributes.iter().any(|attribute| {
            attribute.name.namespace_uri.is_none()
                && attribute.name.local_name == "Type"
                && attribute.value == "General"
        })
    {
        return Err(SaveDiagnosticCode::ReconcileParagraphTypeAttributeMismatch.error());
    }
    let outer_ids = outer
        .attributes
        .iter()
        .filter(|attribute| {
            attribute.name.namespace_uri.is_none()
                && attribute.name.local_name.eq_ignore_ascii_case("id")
        })
        .collect::<Vec<_>>();
    if outer_ids.len() != 1
        || group.wrapper.outer_foreign_id.as_deref() != Some(outer_ids[0].value.as_str())
    {
        return Err(SaveDiagnosticCode::ReconcileNodeSemanticElementMissing.error());
    }
    let mut dual_index = None;
    let mut outer_attachment = false;
    for (index, child) in outer.children.iter().enumerate() {
        match child {
            PreservedNode::Element(element)
                if element.name.local_name == "DualDialogue"
                    && element.name.namespace_uri.as_deref() == root_namespace =>
            {
                if dual_index.replace(index).is_some() {
                    return Err(SaveDiagnosticCode::ReconcileNodeManagedOutsideSection.error());
                }
            }
            PreservedNode::Element(element)
                if is_user_paragraph_data_name(&element.name, &outer.name) && !outer_attachment =>
            {
                outer_attachment = true;
            }
            PreservedNode::Text(text) if text.chars().all(char::is_whitespace) => {}
            _ => return Err(SaveDiagnosticCode::ReconcileNodeManagedOutsideSection.error()),
        }
    }
    let dual = match dual_index.and_then(|index| outer.children.get_mut(index)) {
        Some(PreservedNode::Element(element)) => element,
        _ => return Err(SaveDiagnosticCode::ReconcileNodeManagedOutsideSection.error()),
    };
    let expected_members = group.member_ids().cloned().collect::<Vec<_>>();
    let mut actual_members = Vec::with_capacity(expected_members.len());
    let mut saw_spell_block = false;
    for child in &mut dual.children {
        match child {
            PreservedNode::Text(text) if text.chars().all(char::is_whitespace) => {}
            PreservedNode::Element(element)
                if element.name.local_name == "SpellCheckIgnoreLists"
                    && element.name.namespace_uri.as_deref() == root_namespace
                    && !saw_spell_block
                    && actual_members.is_empty() =>
            {
                if !is_supported_preserved_spell_block(element, root_namespace) {
                    return Err(SaveDiagnosticCode::ReconcileNodeManagedOutsideSection.error());
                }
                saw_spell_block = true;
            }
            PreservedNode::Element(element)
                if saw_spell_block
                    && element.name.local_name == "Paragraph"
                    && element.name.namespace_uri.as_deref() == root_namespace =>
            {
                let SourceOwnership::Managed(id) = element.ownership.clone() else {
                    return Err(SaveDiagnosticCode::ReconcileNodeManagedOutsideSection.error());
                };
                if expected_members.get(actual_members.len()) != Some(&id)
                    || !seen.insert(id.clone())
                {
                    return Err(SaveDiagnosticCode::ReconcileNodeDuplicateManagedIdentity.error());
                }
                let semantic = body.get(&id).copied().ok_or_else(|| {
                    SaveDiagnosticCode::ReconcileNodeSemanticElementMissing.error()
                })?;
                reconcile_managed_paragraph(element, semantic).map_err(|error| {
                    save_diagnostic_at(
                        error,
                        SaveDiagnosticCode::ReconcileManagedParagraphPropagation,
                    )
                })?;
                actual_members.push(id.clone());
                body_order.push(id.clone());
            }
            _ => return Err(SaveDiagnosticCode::ReconcileNodeManagedOutsideSection.error()),
        }
    }
    if !saw_spell_block || actual_members != expected_members {
        return Err(SaveDiagnosticCode::ReconcileProjectManagedOrderMismatch.error());
    }
    Ok(())
}

fn is_supported_preserved_spell_block(
    block: &PreservedElement,
    root_namespace: Option<&str>,
) -> bool {
    let mut lists = Vec::new();
    for child in &block.children {
        match child {
            PreservedNode::Text(text) if text.chars().all(char::is_whitespace) => {}
            PreservedNode::Element(element)
                if element.name.namespace_uri.as_deref() == root_namespace
                    && element.ownership == SourceOwnership::Opaque
                    && element.name.local_name == "IgnoredRanges" =>
            {
                lists.push("IgnoredRanges")
            }
            PreservedNode::Element(element)
                if element.name.namespace_uri.as_deref() == root_namespace
                    && element.ownership == SourceOwnership::Opaque
                    && element.name.local_name == "IgnoredWords" =>
            {
                lists.push("IgnoredWords")
            }
            _ => return false,
        }
    }
    lists == ["IgnoredRanges", "IgnoredWords"]
}

fn reconcile_managed_paragraph(
    paragraph: &mut PreservedElement,
    semantic: &ScreenplayElement,
) -> Result<(), FdxError> {
    let element_type = semantic.element_type();
    project_required_paragraph_type(paragraph, element_type.as_str())?;

    reconcile_managed_text_runs(paragraph, &semantic.text, &semantic.runs)
}

fn reconcile_managed_title_paragraph(
    paragraph: &mut PreservedElement,
    semantic: &TitlePageParagraph,
) -> Result<(), FdxError> {
    let element_type = semantic.element_type();
    project_optional_paragraph_type(paragraph, element_type.as_ref().map(ElementTypeKey::as_str))?;

    reconcile_managed_text_runs(paragraph, &semantic.text, &semantic.runs)
}

fn source_type_attribute_index(paragraph: &PreservedElement) -> Result<Option<usize>, FdxError> {
    let mut found = None;
    for (index, attribute) in paragraph.attributes.iter().enumerate() {
        if attribute.name.namespace_uri.is_none() && attribute.name.local_name == "Type" {
            if found.replace(index).is_some() {
                return Err(SaveDiagnosticCode::ReconcileParagraphTypeAttributeMismatch.error());
            }
        }
    }
    if let Some(index) = found {
        let value = &paragraph.attributes[index].value;
        if value.is_empty()
            || value.len() > scriptcraft_model::element_type::MAX_ELEMENT_TYPE_KEY_BYTES
            || ElementTypeKey::new(value.clone()).is_err()
            || paragraph.source_type.as_deref() != Some(value)
        {
            return Err(SaveDiagnosticCode::ReconcileParagraphTypeAttributeMismatch.error());
        }
    } else if paragraph.source_type.is_some() {
        return Err(SaveDiagnosticCode::ReconcileParagraphTypeAttributeMismatch.error());
    }
    Ok(found)
}

fn project_required_paragraph_type(
    paragraph: &mut PreservedElement,
    target: &str,
) -> Result<(), FdxError> {
    let Some(index) = source_type_attribute_index(paragraph)? else {
        return Err(SaveDiagnosticCode::ReconcileParagraphTypeAttributeMismatch.error());
    };
    paragraph.attributes[index].value = target.to_owned();
    paragraph.source_type = Some(target.to_owned());
    Ok(())
}

fn project_optional_paragraph_type(
    paragraph: &mut PreservedElement,
    target: Option<&str>,
) -> Result<(), FdxError> {
    let index = source_type_attribute_index(paragraph)?;
    match (index, target) {
        (Some(index), Some(target)) => paragraph.attributes[index].value = target.to_owned(),
        (Some(index), None) => {
            paragraph.attributes.remove(index);
        }
        (None, Some(target)) => {
            let insertion_index = paragraph
                .attributes
                .iter()
                .position(|attribute| {
                    attribute.name.namespace_uri.is_none()
                        && attribute.name.local_name.eq_ignore_ascii_case("id")
                })
                .unwrap_or(paragraph.attributes.len());
            paragraph.attributes.insert(
                insertion_index,
                PreservedAttribute {
                    name: SourceName {
                        namespace_uri: None,
                        local_name: "Type".to_owned(),
                        qualified_name: Some("Type".to_owned()),
                    },
                    value: target.to_owned(),
                },
            );
        }
        (None, None) => {}
    }
    paragraph.source_type = target.map(str::to_owned);
    Ok(())
}

fn reconcile_managed_text_runs(
    paragraph: &mut PreservedElement,
    canonical_text: &str,
    semantic_runs: &[TextRun],
) -> Result<(), FdxError> {
    if canonical_text.is_empty() {
        if !semantic_runs.is_empty() {
            return Err(SaveDiagnosticCode::ReconcileParagraphEmptyRunsMismatch.error());
        }
        let source_has_managed_text = paragraph.children.iter().any(|child| {
            matches!(
                child,
                PreservedNode::Element(element)
                    if element.name.local_name == "Text"
                        && element.name.namespace_uri == paragraph.name.namespace_uri
                        && has_managed_text_content(element)
            )
        });
        if !source_has_managed_text {
            // An already-empty source Text is part of a no-op source tree. Do
            // not interpret the lack of semantic runs as an instruction to
            // erase that representation or any inline/attribute payload.
            return Ok(());
        }
        let mut reconciled_children = Vec::with_capacity(paragraph.children.len());
        for child in std::mem::take(&mut paragraph.children) {
            let PreservedNode::Element(mut element) = child else {
                reconciled_children.push(child);
                continue;
            };
            if element.name.local_name != "Text"
                || element.name.namespace_uri != paragraph.name.namespace_uri
                || !has_managed_text_content(&element)
            {
                reconciled_children.push(PreservedNode::Element(element));
                continue;
            }
            if is_fully_managed_text(&element) {
                continue;
            }
            let mut retained_children = Vec::with_capacity(element.children.len());
            for inline in std::mem::take(&mut element.children) {
                retained_children.push(match inline {
                    PreservedNode::Text(_) => PreservedNode::Text("".into()),
                    PreservedNode::CData(_) => PreservedNode::CData("".into()),
                    opaque => opaque,
                });
            }
            element.children = retained_children.into();
            reconciled_children.push(PreservedNode::Element(element));
        }
        paragraph.children = reconciled_children.into();
        return Ok(());
    }

    let mut runs = semantic_runs
        .iter()
        .filter(|run| !run.text.is_empty())
        .cloned()
        .collect::<Vec<_>>();
    if runs.is_empty() {
        runs.push(TextRun {
            text: canonical_text.to_owned(),
            attributes: Vec::new(),
        });
    }
    if runs.iter().map(|run| run.text.as_str()).collect::<String>() != canonical_text {
        return Err(SaveDiagnosticCode::ReconcileParagraphRunProjectionMismatch.error());
    }

    let source_indexes = paragraph
        .children
        .iter()
        .enumerate()
        .filter_map(|(index, child)| match child {
            PreservedNode::Element(element)
                if element.name.local_name == "Text"
                    && element.name.namespace_uri == paragraph.name.namespace_uri
                    && has_managed_text_content(element) =>
            {
                Some(index)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let mut templates = Vec::with_capacity(source_indexes.len());
    for index in &source_indexes {
        let PreservedNode::Element(element) = &paragraph.children[*index] else {
            return Err(SaveDiagnosticCode::ReconcileParagraphRunProjectionMismatch.error());
        };
        templates.push(element.clone());
    }
    if templates.len() > runs.len()
        && templates.iter().skip(runs.len()).any(|element| {
            !element.attributes.is_empty()
                || !element.namespace_bindings.is_empty()
                || element
                    .children
                    .iter()
                    .any(|child| !matches!(child, PreservedNode::Text(_) | PreservedNode::CData(_)))
        })
    {
        // A removed semantic run cannot authorize dropping an opaque Text
        // attribute or inline/mixed-content slot with it.
        return Err(SaveDiagnosticCode::ReconcileParagraphRunProjectionMismatch.error());
    }

    let mut reconciled = Vec::with_capacity(runs.len());
    for (index, run) in runs.iter().enumerate() {
        validate_xml_chars(&run.text)?;
        let template = templates
            .get(index)
            .or_else(|| templates.last())
            .cloned()
            .unwrap_or_else(|| new_text_element_for_paragraph(paragraph));
        reconciled.push(PreservedNode::Element(reconcile_text_element(
            template, run,
        )?));
    }

    if source_indexes.is_empty() {
        paragraph.children.extend(reconciled);
        return Ok(());
    }

    let insertion_after = source_indexes.last().copied().unwrap_or_default() + 1;
    let mut replacements = reconciled.into_iter();
    let mut next_source = source_indexes.iter().copied().peekable();
    let mut children = Vec::with_capacity(
        paragraph
            .children
            .len()
            .saturating_add(runs.len().saturating_sub(source_indexes.len())),
    );
    for (index, child) in std::mem::take(&mut paragraph.children)
        .into_iter()
        .enumerate()
    {
        if next_source.peek().copied() == Some(index) {
            next_source.next();
            if let Some(replacement) = replacements.next() {
                children.push(replacement);
            }
        } else {
            children.push(child);
        }
        if index + 1 == insertion_after {
            children.extend(replacements.by_ref());
        }
    }
    paragraph.children = children.into();
    Ok(())
}

fn reconcile_text_element(
    mut element: PreservedElement,
    run: &TextRun,
) -> Result<PreservedElement, FdxError> {
    if element
        .children
        .iter()
        .any(|child| !matches!(child, PreservedNode::Text(_) | PreservedNode::CData(_)))
    {
        // Inline markup and XML miscellany are source-owned topology. The
        // semantic run cannot identify character spans on either side of those
        // nodes, so unchanged runs pass through verbatim while edits remain a
        // bounded safe-stop for the later edit-lifecycle work order.
        if preserved_text_projection(&element) == run.text
            && source_run_attributes_match(&element, &run.attributes)
        {
            return Ok(element);
        }
        return Err(SaveDiagnosticCode::ReconcileParagraphRunProjectionMismatch.error());
    }

    let mut attributes = Vec::with_capacity(run.attributes.len());
    for attribute in &run.attributes {
        validate_qname(&attribute.name)?;
        validate_xml_chars(&attribute.value)?;
        let name = if let Some(existing) = element.attributes.iter().find(|candidate| {
            candidate
                .name
                .qualified_name
                .as_deref()
                .unwrap_or(candidate.name.local_name.as_str())
                == attribute.name
        }) {
            existing.name.clone()
        } else if attribute.name.contains(':') {
            return Err(SaveDiagnosticCode::NewParagraphNamespacedRunAttribute.error());
        } else {
            SourceName {
                namespace_uri: None,
                local_name: attribute.name.clone(),
                qualified_name: Some(attribute.name.clone()),
            }
        };
        attributes.push(PreservedAttribute {
            name,
            value: attribute.value.clone(),
        });
    }
    element.attributes = attributes;

    let mut text_children = Vec::new();
    if let Some(first) = element.children.first() {
        match first {
            PreservedNode::CData(_) if run.text.contains("]]> ") => {
                return Err(FdxError::UnsupportedConstruct)
            }
            PreservedNode::CData(_) if run.text.contains("]]>") => {
                return Err(FdxError::UnsupportedConstruct)
            }
            PreservedNode::CData(_) => {
                text_children.push(PreservedNode::CData(run.text.clone().into()))
            }
            PreservedNode::Text(_) => {
                text_children.push(PreservedNode::Text(run.text.clone().into()))
            }
            _ => return Err(SaveDiagnosticCode::ReconcileParagraphUnexpectedFirstChild.error()),
        }
        for child in element.children.iter().skip(1) {
            match child {
                PreservedNode::CData(_) => text_children.push(PreservedNode::CData("".into())),
                PreservedNode::Text(_) => text_children.push(PreservedNode::Text("".into())),
                _ => return Err(SaveDiagnosticCode::ReconcileParagraphUnexpectedLaterChild.error()),
            }
        }
    } else if !run.text.is_empty() {
        text_children.push(PreservedNode::Text(run.text.clone().into()));
    }
    element.children = text_children.into();
    Ok(element)
}

fn preserved_text_projection(element: &PreservedElement) -> String {
    let mut projection = String::new();
    for child in &element.children {
        match child {
            PreservedNode::Text(text) | PreservedNode::CData(text) => {
                projection.push_str(text.as_str());
            }
            PreservedNode::Element(_)
            | PreservedNode::Comment(_)
            | PreservedNode::ProcessingInstruction { .. } => {}
        }
    }
    projection
}

fn source_run_attributes_match(element: &PreservedElement, attributes: &[RunAttribute]) -> bool {
    element.attributes.len() == attributes.len()
        && element
            .attributes
            .iter()
            .zip(attributes)
            .all(|(source, semantic)| {
                source
                    .name
                    .qualified_name
                    .as_deref()
                    .unwrap_or(source.name.local_name.as_str())
                    == semantic.name
                    && source.value == semantic.value
            })
}

fn has_managed_text_content(element: &PreservedElement) -> bool {
    element.children.iter().any(|child| match child {
        PreservedNode::Text(text) | PreservedNode::CData(text) => !text.is_empty(),
        PreservedNode::Element(_)
        | PreservedNode::Comment(_)
        | PreservedNode::ProcessingInstruction { .. } => false,
    })
}

fn is_fully_managed_text(element: &PreservedElement) -> bool {
    element.attributes.is_empty()
        && element.namespace_bindings.is_empty()
        && element.children.iter().all(|child| match child {
            PreservedNode::Text(_) | PreservedNode::CData(_) => true,
            PreservedNode::Element(_)
            | PreservedNode::Comment(_)
            | PreservedNode::ProcessingInstruction { .. } => false,
        })
}

fn new_text_element_for_paragraph(paragraph: &PreservedElement) -> PreservedElement {
    let prefix = paragraph
        .name
        .qualified_name
        .as_deref()
        .and_then(|name| name.split_once(':').map(|(prefix, _)| prefix));
    let qualified_name =
        prefix.map_or_else(|| "Text".to_owned(), |prefix| format!("{prefix}:Text"));
    PreservedElement {
        name: SourceName {
            namespace_uri: paragraph.name.namespace_uri.clone(),
            local_name: "Text".to_owned(),
            qualified_name: Some(qualified_name),
        },
        attributes: Vec::new(),
        namespace_bindings: Vec::new(),
        source_type: None,
        ownership: SourceOwnership::Opaque,
        dependency_confidence: DependencyConfidence::Unknown,
        anchor: None,
        children: SharedVec::default(),
    }
}
