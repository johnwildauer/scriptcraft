#![forbid(unsafe_code)]

use scriptcraft_core::formatting::SourceLayoutSettings;
use scriptcraft_core::layout::{Caret, LayoutComposition, ScreenplayLayout};
use scriptcraft_core::{BodyOwner, LayoutInputs, Project, Role, ScreenplayElement};

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("layout smoke scenario is valid"))
}

#[test]
fn layout_public_api_smoke() {
    let text = format!("{}éZ", "a".repeat(60));
    let mut project = valid(Project::new());
    project.elements = vec![BodyOwner::shared(valid(ScreenplayElement::new(
        "layout-action",
        Role::Action,
        text.clone(),
    )))];
    let original = project.clone();
    let inputs = LayoutInputs::default();
    let source = SourceLayoutSettings::default();

    let first = valid(ScreenplayLayout::build_with_composition(
        &project,
        &inputs,
        &source,
        LayoutComposition::BodyEditor,
    ));
    let second = valid(ScreenplayLayout::build_with_composition(
        &project,
        &inputs,
        &source,
        LayoutComposition::BodyEditor,
    ));
    assert_eq!(first, second);
    assert!(!first.pages.is_empty());

    let lines = first
        .pages
        .iter()
        .flat_map(|page| &page.lines)
        .filter(|line| line.element == 0)
        .collect::<Vec<_>>();
    assert_eq!(lines.len(), 2);
    assert_eq!((lines[0].source.start, lines[0].source.end), (0, 62));
    assert_eq!(
        (lines[1].source.start, lines[1].source.end),
        (62, text.len())
    );
    assert_eq!(lines[0].source_for_display_char(60), 60);
    assert_eq!(lines[0].source_for_display_char(61), 62);

    let caret = Caret {
        element: 0,
        offset: 60,
    };
    let caret_geometry = valid(first.caret_geometry(caret));
    assert_eq!(lines[0].page, 0);
    assert_eq!(first.pages[0].lines[0], *lines[0]);
    assert_eq!(caret_geometry.y, lines[0].geometry.y);
    assert_eq!(caret_geometry.height, lines[0].geometry.height);
    assert_eq!(project, original);
}
