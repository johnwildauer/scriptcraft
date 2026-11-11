#![forbid(unsafe_code)]

use scriptcraft_core::fdx::{read_v6, write_v6, FdxError};
use scriptcraft_core::Project;

const BASIC: &[u8] = include_bytes!("fixtures/dual_dialogue_basic.fdx");
const EMPTY_SIDES: &[u8] = include_bytes!("fixtures/dual_dialogue_empty_sides.fdx");
const ALTS: &[u8] = include_bytes!("fixtures/dialogue_alts.fdx");

fn without_source(mut project: Project) -> Project {
    project.preservation = None;
    for group in &mut project.dual_dialogue_groups {
        group.wrapper.outer_foreign_id = None;
        group.wrapper.source = None;
        group.wrapper.preservation_anchor = None;
    }
    project
}

#[test]
fn source_backed_dual_roundtrip_keeps_flat_ids_wrapper_and_opaque_source() {
    let imported = read_v6(BASIC).expect("checked-in basic dual fixture imports");
    let before_ids = imported.project.elements.iter().map(|item| item.id.clone()).collect::<Vec<_>>();
    let saved = write_v6(&imported.project).expect("source-backed project serializes");
    let xml = String::from_utf8(saved.clone()).expect("FDX output is UTF-8");
    assert!(xml.contains("outer-dual-001"));
    assert!(xml.contains("vendor-outer=\"retained\""));
    assert!(xml.matches("SpellCheckIgnoreLists").count() >= 2);
    let reopened = read_v6(&saved).expect("source-backed output reimports");
    assert_eq!(reopened.project.dual_dialogue_groups.len(), 1);
    assert_eq!(reopened.project.elements.iter().map(|item| item.id.clone()).collect::<Vec<_>>(), before_ids);
    assert_eq!(reopened.project.elements[2].text, "First half—then the rest.");
}

#[test]
fn preservation_free_writer_emits_one_complete_wrapper_and_reimports_members() {
    let imported = read_v6(BASIC).expect("fixture imports");
    let project = without_source(imported.project);
    let bytes = write_v6(&project).expect("preservation-free semantic group serializes");
    let xml = String::from_utf8(bytes.clone()).expect("FDX output is UTF-8");
    assert_eq!(xml.matches("<DualDialogue>").count(), 1);
    assert_eq!(xml.matches("<SpellCheckIgnoreLists>").count(), 1);
    let reopened = read_v6(&bytes).expect("generated dual wrapper reimports");
    assert_eq!(reopened.project.elements.len(), project.elements.len());
    assert_eq!(reopened.project.dual_dialogue_groups.len(), 1);
}

#[test]
fn empty_side_wrappers_and_both_alts_scopes_survive_serialization() {
    let empty = read_v6(EMPTY_SIDES).expect("empty-side fixture imports");
    assert!(!empty.project.dual_dialogue_groups.is_empty());
    let empty_saved = write_v6(&empty.project).expect("empty sides serialize");
    let empty_reopened = read_v6(&empty_saved).expect("empty-side output reimports");
    assert_eq!(empty_reopened.project.elements.len(), empty.project.elements.len());
    assert_eq!(empty_reopened.project.dual_dialogue_groups.len(), empty.project.dual_dialogue_groups.len());

    let alts = read_v6(ALTS).expect("Alts fixture imports");
    let saved = write_v6(&alts.project).expect("inline and collection Alts serialize");
    let xml = String::from_utf8(saved.clone()).expect("FDX output is UTF-8");
    for sentinel in ["INLINE_ALT_SENTINEL", "MEMBER_ALT_SENTINEL", "COLLECTION_MEMBER_SENTINEL", "ORPHAN_SENTINEL"] {
        assert!(xml.contains(sentinel));
    }
    let reopened = read_v6(&saved).expect("Alts output reimports");
    assert_eq!(reopened.project.elements, alts.project.elements);
}

#[test]
fn unsupported_namespace_and_malformed_group_shapes_stop_import_safely() {
    let source = std::str::from_utf8(BASIC).expect("fixture is UTF-8");
    let unsupported = source.replace("<DualDialogue>", "<DualDialogue xmlns=\"urn:unsupported\">");
    assert_eq!(read_v6(unsupported.as_bytes()).err(), Some(FdxError::UnsupportedConstruct));

    let malformed = br#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="General" id="outer"><DualDialogue><Paragraph Type="Character" id="only-cue"><Text>ALONE</Text></Paragraph></DualDialogue></Paragraph></Content></FinalDraft>"#;
    assert_eq!(read_v6(malformed).err(), Some(FdxError::UnsupportedConstruct));
}

#[test]
fn repeated_title_source_ids_get_distinct_transient_ids_without_body_shadowing() {
    let source = br#"<FinalDraft DocumentType="Script" Version="6"><Content><Paragraph Type="Action" id="body"><Text>body</Text></Paragraph></Content><TitlePage><Content><Paragraph id="same-title"><Text>first</Text></Paragraph><Paragraph id="same-title"><Text>second</Text></Paragraph></Content></TitlePage></FinalDraft>"#;
    let imported = read_v6(source).expect("repeated title source occurrences remain importable");
    assert_eq!(imported.project.elements[0].id.as_str(), "body");
    assert_eq!(imported.project.title_page.len(), 2);
    assert_ne!(imported.project.title_page[0].id, imported.project.title_page[1].id);
    assert!(imported.project.title_page.iter().all(|paragraph| {
        paragraph.source_provenance.as_ref().and_then(|source| source.foreign_id.as_deref()) == Some("same-title")
    }));
    let saved = write_v6(&imported.project).expect("title/body identity projection serializes");
    assert_eq!(read_v6(&saved).expect("title/body output reimports").project.title_page.len(), 2);
}
