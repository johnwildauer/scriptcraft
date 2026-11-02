use scriptcraft_core::fdx::write_v6;
use scriptcraft_core::{Project, Role, ScreenplayElement};

fn generated_document(character_cues: &[&str]) -> String {
    let mut project = Project::new().unwrap_or_else(|_| unreachable!());
    project.elements = vec![
        ScreenplayElement::new("heading", Role::SceneHeading, "INT. ROOM")
            .unwrap_or_else(|_| unreachable!()),
    ];
    for (index, cue) in character_cues.iter().enumerate() {
        project.elements.push(
            ScreenplayElement::new(format!("character-{index}"), Role::Character, *cue)
                .unwrap_or_else(|_| unreachable!()),
        );
    }
    let bytes = write_v6(&project).unwrap_or_else(|_| unreachable!());
    String::from_utf8(bytes).unwrap_or_else(|_| unreachable!())
}

fn arc_count(xml: &str, name: &str) -> usize {
    let arc = format!("<CharacterArcBeat Name=\"{name}\">");
    xml.matches(&arc).count()
}

#[test]
fn mixed_case_and_delivery_extensions_project_to_unique_uppercase_names() {
    let cues = ["John", "JOHN", "HELEN MIRREN", "helen mirren (V.o.)"];
    let xml = generated_document(&cues);

    assert_eq!(arc_count(&xml, "JOHN"), 1);
    assert_eq!(arc_count(&xml, "HELEN MIRREN"), 1);
    assert!(xml.find("Name=\"JOHN\"").unwrap_or(usize::MAX)
        < xml.find("Name=\"HELEN MIRREN\"").unwrap_or(0));
    for cue in cues {
        assert!(xml.contains(&format!("<Text>{cue}</Text>")));
    }
}

#[test]
fn supported_suffixes_whitespace_empty_cues_and_arbitrary_parentheticals_are_bounded() {
    let cues = [
        "First (V.O.)",
        "Second (o.s.)",
        "Third (O.C.)",
        "Fourth (cont'd)",
        "Fifth (subtitle)",
        "Sixth (TeXt)",
        "Seventh (v.o)",
        "\u{2003}Mixed\u{00a0}(TEXT)\u{2003}",
        "",
        "\u{2003}\u{00a0}",
        "Jane (YOUNG)",
    ];
    let xml = generated_document(&cues);
    let expected = [
        "FIRST",
        "SECOND",
        "THIRD",
        "FOURTH",
        "FIFTH",
        "SIXTH",
        "SEVENTH",
        "MIXED",
        "JANE (YOUNG)",
    ];

    let positions = expected
        .iter()
        .map(|name| {
            assert_eq!(arc_count(&xml, name), 1, "missing or duplicate {name}");
            xml.find(&format!("Name=\"{name}\"")).unwrap_or(usize::MAX)
        })
        .collect::<Vec<_>>();
    assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(xml.contains("<Text>Jane (YOUNG)</Text>"));
}

#[test]
fn unicode_uppercase_expansion_does_not_rewrite_canonical_character_text() {
    let xml = generated_document(&["straße"]);

    assert_eq!(arc_count(&xml, "STRASSE"), 1);
    assert!(xml.contains("<Text>straße</Text>"));
}
