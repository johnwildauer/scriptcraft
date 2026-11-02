use scriptcraft_core::exports::export_text;
use scriptcraft_core::fdx::{read_v6, write_v6};
use scriptcraft_core::layout::{LayoutPurpose, ScreenplayLayout};
use scriptcraft_core::{LayoutInputs, Project};

fn imported(xml: &[u8]) -> Project {
    read_v6(xml)
        .unwrap_or_else(|_| unreachable!("FDX export fixture must import"))
        .project
}

fn exported_body(text: &str) -> Vec<String> {
    text.split_once("\n\n")
        .unwrap_or_else(|| unreachable!("text export header must end with a blank line"))
        .1
        .lines()
        .map(str::to_owned)
        .collect()
}

#[test]
fn plain_text_uses_effective_all_caps_display_and_keeps_native_case() {
    let project = imported(
        b"<FinalDraft DocumentType='Script' Version='6'><ElementSettings Type='Scene Heading'><FontSpec Style='Bold'/></ElementSettings><Content><Paragraph Type='Scene Heading' id='heading'><Text>Int. Kitchen</Text></Paragraph><Paragraph Type='Character' id='character'><Text>jOhN</Text></Paragraph><Paragraph Type='Action' id='action'><Text>Mixed action</Text></Paragraph></Content></FinalDraft>",
    );
    let before = project.clone();

    let text = export_text(&project).unwrap_or_else(|_| unreachable!("text export must succeed"));

    assert!(text.text.contains("Int. Kitchen"));
    assert!(text.text.contains("JOHN"));
    assert!(text.text.contains("Mixed action"));
    assert!(!text.text.contains("jOhN"));
    assert_eq!(project, before, "export must not mutate the source project");

    let native = write_v6(&project).unwrap_or_else(|_| unreachable!("native save must succeed"));
    let reopened = read_v6(&native)
        .unwrap_or_else(|_| unreachable!("native save output must reopen"))
        .project;
    assert_eq!(reopened.elements[0].text, "Int. Kitchen");
    assert_eq!(reopened.elements[1].text, "jOhN");
    assert_eq!(reopened.elements[2].text, "Mixed action");
}

#[test]
fn plain_text_uses_existing_unicode_expanding_display_text() {
    let project = imported(
        "<FinalDraft DocumentType='Script' Version='6'><Content><Paragraph Type='Character' id='character'><Text>straße</Text></Paragraph></Content></FinalDraft>".as_bytes(),
    );
    let before = project.clone();

    let text = export_text(&project).unwrap_or_else(|_| unreachable!("text export must succeed"));

    assert!(text.text.contains("STRASSE"));
    assert!(!text.text.contains("straße"));
    assert_eq!(project.elements[0].text, "straße");
    assert_eq!(project, before, "uppercase expansion is presentation-only");

    let native = write_v6(&project).unwrap_or_else(|_| unreachable!("native save must succeed"));
    let reopened = read_v6(&native)
        .unwrap_or_else(|_| unreachable!("native save output must reopen"))
        .project;
    assert_eq!(reopened.elements[0].text, "straße");
}

#[test]
fn print_wrapping_hidden_omission_and_loss_report_remain_intact() {
    let long_action = "mixed-case screenplay text ".repeat(12);
    let xml = format!(
        "<FinalDraft DocumentType='Script' Version='6'><ElementSettings Type='Note'><FontSpec Style='HiddenText'/></ElementSettings><Content><Paragraph Type='Action' id='action'><Text>{long_action}</Text></Paragraph><Paragraph Type='Note' id='hidden'><Text>private note</Text></Paragraph></Content></FinalDraft>"
    );
    let project = imported(xml.as_bytes());
    let before = project.clone();
    let layout = ScreenplayLayout::build_with_inputs_for_purpose(
        &project,
        &LayoutInputs::default(),
        LayoutPurpose::Print,
    )
    .unwrap_or_else(|_| unreachable!("print layout must succeed"));
    let expected_lines: Vec<String> = layout
        .pages
        .iter()
        .flat_map(|page| &page.lines)
        .map(|line| line.display_text.clone())
        .collect();
    assert!(expected_lines.len() > 1, "fixture must exercise print wrapping");
    assert!(layout
        .pages
        .iter()
        .flat_map(|page| &page.lines)
        .all(|line| line.role != scriptcraft_core::Role::Note));

    let text = export_text(&project).unwrap_or_else(|_| unreachable!("text export must succeed"));

    assert_eq!(exported_body(&text.text), expected_lines);
    assert!(!text.text.contains("private note"));
    assert_eq!(text.loss.roles_lost, 2);
    assert_eq!(text.loss.ids_lost, 2);
    assert!(text.loss.page_layout_lost);
    assert!(text.loss.message.contains("roles, stable IDs, and page layout"));
    assert_eq!(project, before, "layout and export must leave source data unchanged");
}
