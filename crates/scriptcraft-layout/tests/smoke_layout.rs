#![forbid(unsafe_code)]

use scriptcraft_layout::{Caret, LayoutComposition, ScreenplayLayout};
use scriptcraft_model::dual_dialogue::{active_dual_dialogue_projection, DualDialogueGroup};
use scriptcraft_model::formatting::SourceLayoutSettings;
use scriptcraft_model::{
    BodyOwner, ElementTypeKey, LayoutInputs, Project, Role, ScreenplayElement, StableId,
};

fn valid<T, E>(result: Result<T, E>) -> T {
    result.unwrap_or_else(|_| unreachable!("layout smoke scenario is valid"))
}

fn add_dialogue_paginate_alias(project: &mut Project) -> ElementTypeKey {
    let mut definitions = project
        .element_definitions()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    let mut alias = definitions
        .iter()
        .find(|definition| definition.key.as_str() == "Action")
        .cloned()
        .unwrap_or_else(|| unreachable!("Project::new supplies the base Action definition"));
    let key = valid(ElementTypeKey::new("Dialogue-like"));
    alias.key = key.clone();
    alias.behavior.paginate_as = Some("Dialogue".to_owned());
    definitions.push(alias);
    valid(project.replace_element_definitions(definitions));
    key
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
    // This legacy public-API smoke pins source/display behavior only; its fixed
    // default profile is not evidence of resolved-metric wrapping acceptance.
    assert_eq!(lines.len(), 2);
    assert_eq!((lines[0].source.start, lines[0].source.end), (0, 62));
    assert_eq!(
        (lines[1].source.start, lines[1].source.end),
        (62, text.len())
    );
    assert_eq!(lines[0].source_for_display_char(60), 60);
    assert_eq!(lines[0].source_for_display_char(61), 62);
    for span in lines.iter().flat_map(|line| line.spans.iter()) {
        assert_eq!(
            span.display_boundary_offsets.first(),
            Some(&Default::default())
        );
        assert_eq!(
            span.display_boundary_offsets.len(),
            span.display_text.chars().count() + 1
        );
        assert_eq!(
            span.display_boundary_offsets
                .last()
                .map(|offset| offset.raw()),
            Some(span.geometry.width.raw())
        );
    }

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

#[test]
fn dual_source_members_switch_between_sequential_and_active_columns() {
    let mut project = valid(Project::new());
    let members = [
        valid(ScreenplayElement::new(
            "inactive-left-cue",
            Role::Character,
            "LEFT",
        )),
        valid(ScreenplayElement::new(
            "inactive-left-line",
            Role::Action,
            "left line",
        )),
        valid(ScreenplayElement::new(
            "inactive-right-cue",
            Role::Character,
            "RIGHT",
        )),
        valid(ScreenplayElement::new(
            "inactive-right-line",
            Role::Dialogue,
            "right line",
        )),
    ];
    let member_ids = members.iter().map(|member| member.id.clone()).collect();
    project.elements = members.into_iter().map(BodyOwner::shared).collect();
    project.dual_dialogue_groups.push(DualDialogueGroup {
        id: valid(StableId::new("inactive-dual-group")),
        members: member_ids,
        wrapper: Default::default(),
    });
    assert!(active_dual_dialogue_projection(&project, &project.dual_dialogue_groups[0]).is_none());
    assert_eq!(project.validate(), Ok(()));

    let layout = valid(ScreenplayLayout::build_with_composition(
        &project,
        &LayoutInputs::default(),
        &SourceLayoutSettings::default(),
        LayoutComposition::BodyEditor,
    ));
    let lines = layout
        .pages
        .iter()
        .flat_map(|page| &page.lines)
        .filter(|line| line.element < 4)
        .collect::<Vec<_>>();
    assert_eq!(
        lines.iter().map(|line| line.element).collect::<Vec<_>>(),
        vec![0, 1, 2, 3]
    );
    assert!(lines
        .windows(2)
        .all(|pair| { pair[0].paragraph_bounds.y < pair[1].paragraph_bounds.y }));

    let dialogue_alias = add_dialogue_paginate_alias(&mut project);
    BodyOwner::update(&mut project.elements[1], |element| {
        element.set_element_type(dialogue_alias);
    });
    assert!(active_dual_dialogue_projection(&project, &project.dual_dialogue_groups[0]).is_none());
    let alias_layout = valid(ScreenplayLayout::build_with_composition(
        &project,
        &LayoutInputs::default(),
        &SourceLayoutSettings::default(),
        LayoutComposition::BodyEditor,
    ));
    let alias_lines = alias_layout
        .pages
        .iter()
        .flat_map(|page| &page.lines)
        .filter(|line| line.element < 4)
        .collect::<Vec<_>>();
    assert_eq!(
        alias_lines
            .iter()
            .map(|line| line.element)
            .collect::<Vec<_>>(),
        vec![0, 1, 2, 3]
    );

    BodyOwner::update(&mut project.elements[1], |element| {
        element.set_transitional_role(Role::Dialogue);
    });
    assert!(active_dual_dialogue_projection(&project, &project.dual_dialogue_groups[0]).is_some());
    let active_layout = valid(ScreenplayLayout::build_with_composition(
        &project,
        &LayoutInputs::default(),
        &SourceLayoutSettings::default(),
        LayoutComposition::BodyEditor,
    ));
    let left_cue_y = active_layout
        .pages
        .iter()
        .flat_map(|page| &page.lines)
        .find(|line| line.element == 0)
        .map(|line| line.paragraph_bounds.y);
    let right_cue_y = active_layout
        .pages
        .iter()
        .flat_map(|page| &page.lines)
        .find(|line| line.element == 2)
        .map(|line| line.paragraph_bounds.y);
    assert_eq!(left_cue_y, right_cue_y);
}

#[test]
fn vertical_page_fit_ignores_legacy_occupied_line_capacity() {
    let mut project = valid(Project::new());
    project.elements = vec![
        BodyOwner::shared(valid(ScreenplayElement::new(
            "fit-one",
            Role::Action,
            "one",
        ))),
        BodyOwner::shared(valid(ScreenplayElement::new(
            "fit-two",
            Role::Action,
            "two",
        ))),
    ];
    let mut inputs = LayoutInputs::default();
    inputs.lines_per_page = 1;
    let layout = valid(ScreenplayLayout::build_with_composition(
        &project,
        &inputs,
        &SourceLayoutSettings::default(),
        LayoutComposition::BodyEditor,
    ));

    let action_lines = layout
        .pages
        .iter()
        .flat_map(|page| page.lines.iter().map(move |line| (page.number, line)))
        .filter(|(_, line)| line.element < 2)
        .collect::<Vec<_>>();
    assert_eq!(action_lines.len(), 2);
    assert_eq!(action_lines[0].0, action_lines[1].0);
    assert!(action_lines[1].1.paragraph_bounds.y > action_lines[0].1.paragraph_bounds.y);
}
