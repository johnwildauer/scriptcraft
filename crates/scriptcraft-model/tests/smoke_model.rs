#![forbid(unsafe_code)]

use scriptcraft_model::{
    BodyOwner, ModelError, Project, Role, RunAttribute, ScreenplayElement, TextRun,
    TitlePageParagraph,
};
use scriptcraft_model::formatted_text::{parse_style_attribute, RunStyleFlags};

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("model smoke uses valid bounded input"))
}

#[test]
fn model_public_api_smoke() {
    const BODY_TEXT: &str = "Café — opening";

    let mut project = valid(Project::new());
    let element = valid(ScreenplayElement::with_runs(
        "smoke-body",
        Role::Action,
        vec![
            TextRun {
                text: "Café".into(),
                attributes: vec![RunAttribute {
                    name: "Style".into(),
                    value: "Bold+VendorToken".into(),
                }],
            },
            TextRun {
                text: " — opening".into(),
                attributes: Vec::new(),
            },
        ],
        None,
    ));
    project.elements = vec![BodyOwner::shared(element)];
    project.title_page = vec![valid(TitlePageParagraph::new(
        "smoke-title",
        "Smoke Story",
        Default::default(),
    ))];

    assert_eq!(project.elements.len(), 1);
    assert_eq!(project.title_page.len(), 1);
    let body = project.elements[0].as_element();
    let title = &project.title_page[0];
    assert_eq!(body.id.as_str(), "smoke-body");
    assert_eq!(body.role, Role::Action);
    assert_eq!(body.text, BODY_TEXT);
    assert_eq!(title.id.as_str(), "smoke-title");
    assert_eq!(title.role, None);
    assert_eq!(title.text, "Smoke Story");

    let run_text = body
        .runs
        .iter()
        .map(|run| run.text.as_str())
        .collect::<String>();
    assert_eq!(body.text, run_text);
    assert_eq!(body.runs[0].attributes.len(), 1);
    assert_eq!(body.runs[0].attributes[0].name, "Style");
    assert_eq!(body.runs[0].attributes[0].value, "Bold+VendorToken");
    let style = valid(parse_style_attribute(&body.runs[0].attributes));
    assert_eq!(
        style.flags,
        RunStyleFlags {
            bold: true,
            ..RunStyleFlags::default()
        }
    );
    assert_eq!(style.unknown_tokens, vec!["VendorToken".to_owned()]);

    let e_acute = match BODY_TEXT.find('é') {
        Some(offset) => offset,
        None => unreachable!("body literal contains é"),
    };
    for offset in BODY_TEXT
        .char_indices()
        .map(|(offset, _)| offset)
        .chain([BODY_TEXT.len()])
    {
        assert_eq!(body.validate_boundary(offset), Ok(()));
    }
    assert_eq!(
        body.validate_boundary(e_acute + 1),
        Err(ModelError::InvalidUtf8Boundary)
    );

    assert_eq!(project.validate(), Ok(()));
}
