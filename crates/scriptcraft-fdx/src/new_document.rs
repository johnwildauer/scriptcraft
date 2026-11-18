//! Preservation-free construction of a new FDX document source tree.

use crate::constants::FDX_VERSION;
use crate::content::{
    add_wrapper_direct_attributes, new_paragraph, set_unqualified_attribute,
};
use crate::context::{SaveContext, SaveOperation};
use crate::diagnostics::{map_paragraph_id_error, FdxError, SaveDiagnosticCode};
use crate::settings::new_element_settings;
use crate::text_state::text_state_element;
use crate::title_page::{new_title_page_header_and_footer, new_title_page_paragraph};
use crate::tree::preserved_element;
use crate::window_state::window_state_element;
use scriptcraft_model::formatting::formatting_in_native_order;
use scriptcraft_model::paragraph_id::ParagraphIdAllocator;
use scriptcraft_model::preservation::{ForeignIdDomain, PreservedNode};
use scriptcraft_model::{Project, Role};
use std::collections::BTreeSet;

/// Construct a complete new-document tree without source-preservation input.
///
/// The declaration and newline convention are handled by the top-level writer.
pub(crate) fn make_new_document_nodes(
    project: &Project,
    context: &mut SaveContext,
) -> Result<Vec<PreservedNode>, FdxError> {
    let mut root = preserved_element(
        "FinalDraft",
        vec![("DocumentType", "Script"), ("Version", FDX_VERSION)],
    );
    let mut lineage_ids = BTreeSet::new();
    let document_ref_id = context.allocate_lineage_id(&mut lineage_ids)?;
    let xref_id = if matches!(context.operation(), SaveOperation::Save | SaveOperation::SaveAs) {
        Some(context.allocate_lineage_id(&mut lineage_ids)?)
    } else {
        None
    };
    let mut document_ref = preserved_element(
        "DocumentRef",
        vec![
            ("DateTime", context.timestamp()),
            ("id", &document_ref_id),
        ],
    );
    if let Some(xref_id) = xref_id {
        let xref_datetime = match context.operation() {
            SaveOperation::DirectSerialization => context.timestamp(),
            SaveOperation::Save | SaveOperation::SaveAs => context
                .session_open_timestamp()
                .ok_or(FdxError::InvalidValue)?,
        };
        document_ref
            .children
            .push(PreservedNode::Element(preserved_element(
                "XRef",
                vec![("DateTime", xref_datetime), ("id", &xref_id)],
            )));
    }
    root.children.push(PreservedNode::Element(document_ref));

    let mut content = preserved_element("Content", Vec::new());
    let mut reserved_paragraph_ids = lineage_ids;
    reserved_paragraph_ids.extend(project_source_paragraph_ids(project));
    let mut paragraph_ids = ParagraphIdAllocator::new(reserved_paragraph_ids);
    let mut index = 0usize;
    while let Some(element) = project.elements.get(index) {
        if let Some(group) = project
            .dual_dialogue_groups
            .iter()
            .find(|group| group.first_member() == &element.id)
        {
            let mut outer = preserved_element("Paragraph", vec![("Type", "General"), ("id", "")]);
            let outer_id = paragraph_ids.allocate().map_err(map_paragraph_id_error)?;
            set_unqualified_attribute(&mut outer, "id", &outer_id)?;
            if let Some(overrides) = project.dual_dialogue_paragraph_overrides.get(&group.id) {
                add_wrapper_direct_attributes(&mut outer, *overrides)?;
            }
            let mut dual = preserved_element("DualDialogue", Vec::new());
            let mut spell = preserved_element("SpellCheckIgnoreLists", Vec::new());
            spell.children.push(PreservedNode::Element(preserved_element(
                "IgnoredRanges",
                Vec::new(),
            )));
            spell.children.push(PreservedNode::Element(preserved_element(
                "IgnoredWords",
                Vec::new(),
            )));
            dual.children.push(PreservedNode::Element(spell));
            let members = group.member_ids().cloned().collect::<Vec<_>>();
            if members.is_empty() {
                return Err(SaveDiagnosticCode::NewDocumentHeadingNodeMismatch.error());
            }
            for (offset, member_id) in members.iter().enumerate() {
                let member = project
                    .elements
                    .get(index.saturating_add(offset))
                    .filter(|candidate| &candidate.id == member_id)
                    .ok_or_else(|| SaveDiagnosticCode::NewDocumentHeadingNodeMismatch.error())?;
                let overrides = project.direct_paragraph_overrides.get(member_id).copied();
                dual.children
                    .push(new_paragraph(member, overrides, &mut paragraph_ids)?);
            }
            outer.children.push(PreservedNode::Element(dual));
            content.children.push(PreservedNode::Element(outer));
            index = index
                .checked_add(members.len())
                .ok_or(FdxError::ResourceLimit)?;
            continue;
        }

        let direct_overrides = project.direct_paragraph_overrides.get(&element.id).copied();
        let mut paragraph = new_paragraph(element, direct_overrides, &mut paragraph_ids)?;
        if element.role == Role::SceneHeading {
            let scene_end = project.elements[index + 1..]
                .iter()
                .position(|candidate| candidate.role == Role::SceneHeading)
                .map_or(project.elements.len(), |offset| index + 1 + offset);
            let names = scriptcraft_model::scene_metadata::derived_character_projection(
                &project.elements[index + 1..scene_end],
            )
            .into_iter()
            .map(|(_, name)| name)
            .collect::<Vec<_>>();
            let PreservedNode::Element(heading) = &mut paragraph else {
                return Err(SaveDiagnosticCode::NewDocumentHeadingNodeMismatch.error());
            };
            let properties = scriptcraft_model::scene_metadata::new_scene_properties(
                &names,
                &mut paragraph_ids,
            )
            .map_err(|error| match error {
                scriptcraft_model::scene_metadata::SceneMetadataError::ParagraphIdentityEntropyUnavailable => {
                    FdxError::ParagraphIdentityEntropyUnavailable
                }
                scriptcraft_model::scene_metadata::SceneMetadataError::ParagraphIdentityCollisionLimit => {
                    FdxError::ParagraphIdentityCollisionLimit
                }
                _ => SaveDiagnosticCode::NewDocumentScenePropertiesUnsupported.error(),
            })?;
            let text_index = heading
                .children
                .iter()
                .position(|node| matches!(node, PreservedNode::Element(child) if child.name.local_name == "Text"))
                .unwrap_or(heading.children.len());
            heading
                .children
                .insert(text_index, PreservedNode::Element(properties));
        }
        content.children.push(paragraph);
        index = index.checked_add(1).ok_or(FdxError::ResourceLimit)?;
    }
    root.children.push(PreservedNode::Element(content));
    root.children
        .push(PreservedNode::Element(window_state_element(
            &root.name,
            context.window_state(),
        )));
    root.children
        .push(PreservedNode::Element(text_state_element(
            &root.name,
            project,
            context.text_state(),
        )?));
    for formatting in formatting_in_native_order() {
        root.children
            .push(PreservedNode::Element(new_element_settings(formatting.role)));
    }

    let mut title_page = preserved_element("TitlePage", Vec::new());
    title_page.children.push(PreservedNode::Element(
        new_title_page_header_and_footer(&mut paragraph_ids)?,
    ));
    let mut title_content = preserved_element("Content", Vec::new());
    for element in &project.title_page {
        title_content
            .children
            .push(new_title_page_paragraph(element, &mut paragraph_ids)?);
    }
    title_page
        .children
        .push(PreservedNode::Element(title_content));
    title_page.children.push(PreservedNode::Element(preserved_element(
        "TextState",
        vec![("Scaling", "100"), ("Selection", "0,0"), ("ShowInvisibles", "No")],
    )));
    root.children.push(PreservedNode::Element(title_page));
    Ok(vec![PreservedNode::Element(root)])
}

fn project_source_paragraph_ids(project: &Project) -> BTreeSet<String> {
    fn collect_source_attributes(nodes: &[PreservedNode], reserved: &mut BTreeSet<String>) {
        for node in nodes {
            let PreservedNode::Element(element) = node else {
                continue;
            };
            if element.name.local_name == "Paragraph" {
                if let Some(id) = element
                    .attributes
                    .iter()
                    .find(|attribute| attribute.name.local_name.eq_ignore_ascii_case("id"))
                {
                    reserved.insert(id.value.clone());
                }
            }
            collect_source_attributes(&element.children, reserved);
        }
    }

    let mut reserved = BTreeSet::new();
    for provenance in project
        .elements
        .iter()
        .filter_map(|element| element.source_provenance.as_ref())
        .chain(
            project
                .title_page
                .iter()
                .filter_map(|element| element.source_provenance.as_ref()),
        )
    {
        if let Some(id) = &provenance.foreign_id {
            reserved.insert(id.clone());
        }
    }
    reserved.extend(
        project
            .dual_dialogue_groups
            .iter()
            .filter_map(|group| group.wrapper.outer_foreign_id.clone()),
    );
    if let Some(preservation) = &project.preservation {
        reserved.extend(
            preservation
                .identities
                .iter()
                .filter(|identity| identity.domain == ForeignIdDomain::Paragraph)
                .map(|identity| identity.value.clone()),
        );
        for section in preservation.sections.iter() {
            collect_source_attributes(&section.children, &mut reserved);
        }
    }
    reserved
}
