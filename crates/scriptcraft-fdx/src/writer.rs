//! Bounded FDX serialization and retained-tree XML emission.

use crate::constants::{XML_DECLARATION, XML_DECLARATION_WITHOUT_TRAILING_LF};
use crate::context::{valid_document_ref_datetime, SaveContext, SaveOperation};
use crate::diagnostics::{
    map_model_error, map_preservation_error, save_diagnostic_at, FdxError, SaveDiagnosticCode,
};
use crate::document::WindowState;
use crate::lineage::reconcile_document_lineage;
use crate::reader::{read_v6, validate_xml_chars};
use crate::reconcile::reconcile_project_elements;
use crate::split_state::reconcile_root_display_mode;
use crate::text_state::{reconcile_root_text_state, reconcile_title_page_invisibles};
use crate::verification::{
    first_identity_set_difference, first_source_tree_difference, identity_set, normalize_sections,
    same_non_paragraph_identities, validate_project_output_budgets,
    verify_writer_semantic_projection, IdentitySetDifference, SourceTreeDifference,
};
use crate::version::prepare_preserved_root_output;
use crate::whitespace::{
    collect_inserted_managed_paragraph_ids, format_generated_structure,
    format_inserted_paragraph_whitespace,
};
use crate::window_state::reconcile_root_window_state;
use crate::{MAX_SOURCE_BYTES, MAX_SOURCE_DEPTH, MAX_SOURCE_NAME_OR_VALUE_BYTES};
use scriptcraft_model::preservation::{
    ForeignIdDomain, PreservedElement, PreservedNode, SourceEncoding,
};
use scriptcraft_model::formatted_text::{normalize_project_runs, FormattedTextError};
use scriptcraft_model::{Project, SourceFormat};
use std::collections::BTreeSet;

pub(crate) fn write_node(
    node: &PreservedNode,
    output: &mut String,
    depth: usize,
) -> Result<(), FdxError> {
    if depth > MAX_SOURCE_DEPTH || output.len() > MAX_SOURCE_BYTES {
        return Err(FdxError::ResourceLimit);
    }
    match node {
        PreservedNode::Element(element) => write_element(element, output, depth),
        PreservedNode::Text(text) => escape_text(text, output),
        PreservedNode::CData(text) => {
            validate_xml_chars(text)?;
            if text.contains("]]>") {
                return Err(FdxError::InvalidValue);
            }
            append_output(output, "<![CDATA[")?;
            append_output(output, text)?;
            append_output(output, "]]>")?;
            Ok(())
        }
        PreservedNode::Comment(comment) => {
            validate_xml_chars(comment)?;
            if comment.contains("--") || comment.ends_with('-') {
                return Err(FdxError::InvalidValue);
            }
            append_output(output, "<!--")?;
            append_output(output, comment)?;
            append_output(output, "-->")?;
            Ok(())
        }
        PreservedNode::ProcessingInstruction { target, data } => {
            validate_qname(target)?;
            validate_xml_chars(data)?;
            if data.contains("?>") {
                return Err(FdxError::InvalidValue);
            }
            append_output(output, "<?")?;
            append_output(output, target)?;
            if !data.is_empty() {
                append_output(output, " ")?;
                append_output(output, data)?;
            }
            append_output(output, "?>")?;
            Ok(())
        }
    }
}

fn write_element(
    element: &PreservedElement,
    output: &mut String,
    depth: usize,
) -> Result<(), FdxError> {
    if output.len() > MAX_SOURCE_BYTES {
        return Err(FdxError::SourceTooLarge);
    }
    let name = element
        .name
        .qualified_name
        .as_deref()
        .unwrap_or(&element.name.local_name);
    validate_qname(name)?;
    if element.name.namespace_uri.is_some() && element.name.qualified_name.is_none() {
        return Err(SaveDiagnosticCode::WriteElementNamespacePrefixMissing.error());
    }
    let mut attribute_names = BTreeSet::new();
    let mut expanded_attribute_names = BTreeSet::new();
    let mut bindings = BTreeSet::new();
    append_output(output, "<")?;
    append_output(output, name)?;
    for binding in &element.namespace_bindings {
        if !bindings.insert(binding.prefix.as_str()) {
            return Err(FdxError::InvalidValue);
        }
        let name = if binding.prefix.is_empty() {
            "xmlns".to_owned()
        } else {
            format!("xmlns:{}", binding.prefix)
        };
        validate_qname(&name)?;
        append_output(output, " ")?;
        append_output(output, &name)?;
        append_output(output, "=\"")?;
        escape_attribute(&binding.namespace_uri, output)?;
        append_output(output, "\"")?;
    }
    for attribute in &element.attributes {
        let name = attribute
            .name
            .qualified_name
            .as_deref()
            .unwrap_or(&attribute.name.local_name);
        validate_qname(name)?;
        if name.len() > MAX_SOURCE_NAME_OR_VALUE_BYTES
            || attribute.value.len() > MAX_SOURCE_NAME_OR_VALUE_BYTES
            || !attribute_names.insert(name)
            || !expanded_attribute_names.insert((
                attribute.name.namespace_uri.clone(),
                attribute.name.local_name.clone(),
            ))
        {
            return Err(FdxError::InvalidValue);
        }
        append_output(output, " ")?;
        append_output(output, name)?;
        append_output(output, "=\"")?;
        escape_attribute(&attribute.value, output)?;
        append_output(output, "\"")?;
    }
    if element.children.is_empty() {
        append_output(output, "/>")?;
        return Ok(());
    }
    append_output(output, ">")?;
    for child in &element.children {
        write_node(child, output, depth.saturating_add(1))?;
    }
    append_output(output, "</")?;
    append_output(output, name)?;
    append_output(output, ">")?;
    Ok(())
}

fn escape_text(value: &str, output: &mut String) -> Result<(), FdxError> {
    validate_xml_chars(value)?;
    for character in value.chars() {
        match character {
            '&' => append_output(output, "&amp;")?,
            '<' => append_output(output, "&lt;")?,
            '>' => append_output(output, "&gt;")?,
            '\r' => append_output(output, "&#xD;")?,
            _ => append_character(output, character)?,
        }
    }
    Ok(())
}

fn escape_attribute(value: &str, output: &mut String) -> Result<(), FdxError> {
    validate_xml_chars(value)?;
    for character in value.chars() {
        match character {
            '&' => append_output(output, "&amp;")?,
            '<' => append_output(output, "&lt;")?,
            '"' => append_output(output, "&quot;")?,
            '\r' => append_output(output, "&#xD;")?,
            '\n' => append_output(output, "&#xA;")?,
            '\t' => append_output(output, "&#x9;")?,
            _ => append_character(output, character)?,
        }
    }
    Ok(())
}

fn append_output(output: &mut String, value: &str) -> Result<(), FdxError> {
    if output.len().saturating_add(value.len()) > MAX_SOURCE_BYTES {
        return Err(FdxError::SourceTooLarge);
    }
    output.push_str(value);
    Ok(())
}

fn append_character(output: &mut String, value: char) -> Result<(), FdxError> {
    if output.len().saturating_add(value.len_utf8()) > MAX_SOURCE_BYTES {
        return Err(FdxError::SourceTooLarge);
    }
    output.push(value);
    Ok(())
}

pub(crate) fn validate_qname(name: &str) -> Result<(), FdxError> {
    let mut parts = name.split(':');
    let first = parts.next().ok_or(FdxError::InvalidValue)?;
    let second = parts.next();
    if first.is_empty() || parts.next().is_some() || second.is_some_and(str::is_empty) {
        return Err(FdxError::InvalidValue);
    }
    for part in std::iter::once(first).chain(second) {
        let mut characters = part.chars();
        let Some(initial) = characters.next() else {
            return Err(FdxError::InvalidValue);
        };
        if !is_xml_name_start(initial) || !characters.all(is_xml_name_char) {
            return Err(FdxError::InvalidValue);
        }
    }
    Ok(())
}

fn is_xml_name_start(character: char) -> bool {
    matches!(character, 'A'..='Z' | '_' | 'a'..='z')
        || matches!(character as u32,
            0xC0..=0xD6 | 0xD8..=0xF6 | 0xF8..=0x2FF | 0x370..=0x37D
            | 0x37F..=0x1FFF | 0x200C..=0x200D | 0x2070..=0x218F
            | 0x2C00..=0x2FEF | 0x3001..=0xD7FF | 0xF900..=0xFDCF
            | 0xFDF0..=0xFFFD | 0x10000..=0xEFFFF)
}

fn is_xml_name_char(character: char) -> bool {
    is_xml_name_start(character)
        || matches!(character, '-' | '.' | '0'..='9' | '\u{B7}' | '\u{0300}'..='\u{036F}' | '\u{203F}'..='\u{2040}')
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WriterStage {
    ProjectValidation,
    PreservationValidation,
    OriginalSourceGraphComparison,
    ManagedSourceReconciliation,
    NewDocumentGeneration,
    XmlWriting,
    OutputReread,
    SemanticComparison,
    IdentityComparison,
    SourceTreeComparison,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WriterFailure {
    error: FdxError,
    stage: WriterStage,
    structural_difference: Option<SourceTreeDifference>,
    identity_difference: Option<IdentitySetDifference>,
}

/// Serialize the supplied Project's ordered definitions and content to UTF-8
/// Version 6 XML.
///
/// For imported documents, managed paragraphs are reconciled into the retained
/// ordered source tree; unknown and source-owned nodes are re-emitted in place.
/// Without preservation state, the supplied current body/title values and
/// complete ordered definitions are emitted in a minimal supported FDX
/// structure. This is generic serialization, not a new-document constructor or
/// template source. ScriptCraft identity, runtime geometry inputs, and editing
/// policies are not serialized. Output is structurally re-read through the
/// bounded reader before it is returned. This does not establish desktop
/// save/reopen guarantees or Final Draft application interoperability. The
/// reference-compatible UTF-8 standalone declaration is emitted once for every
/// serialization; imported XML is otherwise retained semantically, not
/// lexically: entity spelling, quote style and exact attribute interleaving with
/// namespace declarations are not guaranteed to be byte-identical. UTF-8 is
/// emitted; supported tree order, expanded names, attributes, text, comments,
/// processing instructions and opaque children are checked against the re-read
/// output.
pub fn write_v6(project: &Project) -> Result<Vec<u8>, FdxError> {
    write_v6_with_diagnostics(project).map_err(|failure| failure.error)
}

pub fn write_v6_for_save(
    project: &Project,
    context: &mut SaveContext,
) -> Result<Vec<u8>, FdxError> {
    write_v6_with_context(project, context).map_err(|failure| failure.error)
}

// WriterFailure intentionally carries bounded structural/identity differences
// to the save-diagnostics boundary; boxing it would churn the internal protocol.
#[allow(clippy::result_large_err)]
fn write_v6_with_diagnostics(project: &Project) -> Result<Vec<u8>, WriterFailure> {
    let mut context = SaveContext::new(SaveOperation::DirectSerialization);
    write_v6_with_context(project, &mut context)
}

#[allow(clippy::result_large_err)]
fn write_v6_with_context(
    project: &Project,
    context: &mut SaveContext,
) -> Result<Vec<u8>, WriterFailure> {
    let mut stage = WriterStage::ProjectValidation;
    let mut structural_difference = None;
    let mut identity_difference = None;
    write_v6_inner(
        project,
        context,
        &mut stage,
        &mut structural_difference,
        &mut identity_difference,
    )
    .map_err(|error| WriterFailure {
        error,
        stage,
        structural_difference,
        identity_difference,
    })
}

fn write_v6_inner(
    project: &Project,
    context: &mut SaveContext,
    stage: &mut WriterStage,
    structural_difference: &mut Option<SourceTreeDifference>,
    identity_difference: &mut Option<IdentitySetDifference>,
) -> Result<Vec<u8>, FdxError> {
    // Serialization owns a defensive detached normalization boundary. Every
    // subsequent validation, reconciliation, budget, and reread comparison
    // consumes this same candidate rather than mutating the caller's Project.
    let normalized_project =
        normalize_project_runs(project).map_err(map_inline_normalization_error)?;
    let project = &normalized_project;

    // Project validation requires every retained definition anchor to resolve
    // against the current registry. Reconcile a detached validation copy first
    // so removing a definition (including its inert duplicate family) can be
    // verified instead of being rejected before the writer gets to project it.
    // The output path below starts from the normalized detached preservation
    // snapshot and performs the corresponding projection for comparison.
    let mut validation_project = project.clone();
    if validation_project.preservation.is_some() {
        crate::element_settings::reconcile_project_element_settings(&mut validation_project)?;
    }
    validation_project.validate().map_err(map_model_error)?;
    if !valid_document_ref_datetime(context.timestamp()) {
        return Err(FdxError::InvalidValue);
    }
    if context.operation() != SaveOperation::DirectSerialization
        && context
            .session_open_timestamp()
            .is_none_or(|timestamp| !valid_document_ref_datetime(timestamp))
    {
        return Err(FdxError::InvalidValue);
    }
    let window_state = context.window_state();
    if WindowState::new(
        window_state.left,
        window_state.top,
        window_state.width,
        window_state.height,
    ) != Some(window_state)
    {
        return Err(FdxError::InvalidValue);
    }
    validate_project_output_budgets(project)?;

    let (nodes, expected_preserved_sections) = if let Some(state) = &project.preservation {
        *stage = WriterStage::PreservationValidation;
        state.validate().map_err(map_preservation_error)?;
        if state.source.format != SourceFormat::Fdx || state.source.encoding != SourceEncoding::Utf8
        {
            return Err(SaveDiagnosticCode::WriteSourceFormat.error());
        }
        *stage = WriterStage::OriginalSourceGraphComparison;
        let original = read_v6(&state.source.original_bytes).map_err(|error| {
            save_diagnostic_at(error, SaveDiagnosticCode::WriteOriginalRereadPropagation)
        })?;
        let original_state = original
            .project
            .preservation
            .as_ref()
            .ok_or_else(|| SaveDiagnosticCode::WriteOriginalPreservationMissing.error())?;
        if !same_non_paragraph_identities(&state.identities, &original_state.identities)
            || state.references != original_state.references
            || state.board_graph != original_state.board_graph
        {
            return Err(SaveDiagnosticCode::WriteOriginalIdentityGraphMismatch.error());
        }

        let mut sections = state.sections.clone();
        let mut effective_identities = state.identities.clone();
        prepare_preserved_root_output(&mut sections)?;
        crate::settings::reconcile_root_page_layout(&mut sections, project.page_layout())?;
        if context.operation() != SaveOperation::DirectSerialization {
            effective_identities.extend(reconcile_document_lineage(
                &mut sections,
                &state.identities,
                context,
            )?);
            reconcile_root_window_state(&mut sections, context.window_state())?;
            reconcile_root_display_mode(&mut sections, context.display_mode())?;
            reconcile_root_text_state(
                &mut sections,
                project,
                context.text_state(),
                context.invisibles_state().show_body,
            )?;
            reconcile_title_page_invisibles(&mut sections, context.invisibles_state())?;
        }
        *stage = WriterStage::ManagedSourceReconciliation;
        let mut definition_projection = state.clone();
        definition_projection.sections = sections;
        crate::element_settings::reconcile_retained_project_element_settings(
            &mut definition_projection,
            project.element_definitions(),
        )?;
        let mut sections = definition_projection.sections;
        reconcile_project_elements(&mut sections, project)?;
        let original_paragraph_ids = original_state
            .identities
            .iter()
            .filter(|identity| identity.domain == ForeignIdDomain::Paragraph)
            .map(|identity| identity.value.clone())
            .collect::<BTreeSet<_>>();
        let inserted_paragraph_ids =
            collect_inserted_managed_paragraph_ids(&sections, &original_paragraph_ids);
        format_inserted_paragraph_whitespace(&mut sections, &inserted_paragraph_ids);
        let expected_sections = sections.clone();
        let nodes = sections
            .into_iter()
            .flat_map(|section| section.children)
            .collect::<Vec<_>>();
        (nodes, Some((expected_sections, effective_identities)))
    } else {
        *stage = WriterStage::NewDocumentGeneration;
        let mut nodes = crate::new_document::make_new_document_nodes(project, context)?;
        format_generated_structure(&mut nodes);
        (nodes, None)
    };

    *stage = WriterStage::XmlWriting;
    let declaration = if project.preservation.is_some() {
        // Source-backed output assigns declaration-to-root separation solely to
        // the retained top-level nodes emitted below.
        XML_DECLARATION_WITHOUT_TRAILING_LF
    } else {
        // Preserve the canonical declaration-plus-LF convention for new files.
        XML_DECLARATION
    };
    let mut output = declaration.to_owned();
    for node in &nodes {
        write_node(node, &mut output, 0)?;
        if output.len() > MAX_SOURCE_BYTES {
            return Err(FdxError::SourceTooLarge);
        }
    }
    if output.len() > MAX_SOURCE_BYTES {
        return Err(FdxError::SourceTooLarge);
    }
    let bytes = output.into_bytes();
    *stage = WriterStage::OutputReread;
    let written = read_v6(&bytes).map_err(|error| {
        save_diagnostic_at(error, SaveDiagnosticCode::WriteOutputRereadPropagation)
    })?;
    *stage = WriterStage::SemanticComparison;
    verify_writer_semantic_projection(project, &written.project)?;
    if let Some((expected_sections, expected_identities)) = expected_preserved_sections {
        *stage = WriterStage::IdentityComparison;
        let actual_state = written
            .project
            .preservation
            .as_ref()
            .ok_or_else(|| SaveDiagnosticCode::WriteOutputPreservationMissing.error())?;
        let actual_identity_set = identity_set(&actual_state.identities);
        let expected_identity_set = identity_set(&expected_identities);
        if actual_identity_set != expected_identity_set {
            *identity_difference =
                first_identity_set_difference(&expected_identity_set, &actual_identity_set);
            return Err(SaveDiagnosticCode::WriteIdentitySetMismatch.error());
        }
        let actual_sections = &actual_state.sections;
        if normalize_sections(actual_sections) != normalize_sections(&expected_sections) {
            *stage = WriterStage::SourceTreeComparison;
            let expected = normalize_sections(&expected_sections);
            let actual = normalize_sections(actual_sections);
            let Some(difference) = first_source_tree_difference(&expected, &actual) else {
                return Err(SaveDiagnosticCode::WriteNormalizedSourceTreeMismatch.error());
            };
            let public_detail = difference.public_detail();
            *structural_difference = Some(difference);
            return Err(FdxError::SaveDiagnosticSourceTreeMismatch(public_detail));
        }
    }
    Ok(bytes)
}

fn map_inline_normalization_error(error: FormattedTextError) -> FdxError {
    match error {
        FormattedTextError::Model(error) => map_model_error(error),
        FormattedTextError::Preservation(error) => map_preservation_error(error),
        FormattedTextError::TextTooLong
        | FormattedTextError::TooManyRuns
        | FormattedTextError::TooManyRunAttributes
        | FormattedTextError::RunAttributeTooLarge => FdxError::ResourceLimit,
        _ => FdxError::InvalidValue,
    }
}
