use super::*;
use scriptcraft_model::formatting::{FontFallback, ParagraphAlignment};
use scriptcraft_model::RunAttribute;

fn paragraph() -> EffectiveParagraph {
    EffectiveParagraph {
        left: FixedPoint::default(),
        right: FixedPoint::from_points(500),
        first_indent: FixedPoint::default(),
        alignment: ParagraphAlignment::Left,
        space_before: FixedPoint::default(),
        line_advance: FixedPoint::from_points(12),
        char_width: FixedPoint::from_points(7),
        starts_new_page: false,
        pagination: Default::default(),
    }
}

fn style(color: Option<&str>) -> EffectiveTextStyle {
    EffectiveTextStyle {
        declared_family: "Courier Prime".to_owned(),
        family: FontFallback::CourierPrime,
        size: 12,
        bold: false,
        italic: false,
        underline: false,
        all_caps: false,
        hidden_text: false,
        source_color: color.map(str::to_owned),
    }
}

#[test]
fn open_key_and_effective_color_remain_full_equality_after_fingerprint_lookup(
) -> Result<(), scriptcraft_model::ModelError> {
    let element_id = StableId::new("owner")?;
    let action = ElementTypeKey::new("Action")?;
    let custom = ElementTypeKey::new("Custom Action")?;
    let base_style = style(None);
    let runs: [TextRun; 0] = [];
    let atomic: [std::ops::Range<usize>; 0] = [];
    let original_identity = ElementWrapIdentity {
        element_id: &element_id,
        element_key: Some(&action),
        element_type: Some(action.clone()),
        purpose: LayoutPurpose::Editor,
        title_page: false,
        geometry_mode: EffectiveGeometryMode::Sequential,
        paragraph: paragraph(),
        text_style: &base_style,
        resolved_text_style: None,
        char_width: FixedPoint::from_points(7),
        runs: &runs,
        atomic_ranges: &atomic,
    };
    let retained = ElementWrapContext::from_identity(original_identity.clone());
    let different_key = ElementWrapIdentity {
        element_key: Some(&custom),
        ..original_identity.clone()
    };
    assert!(!retained.matches(&different_key));
    let explicit_color = style(Some("unsupported-explicit-color"));
    let different_color = ElementWrapIdentity {
        text_style: &explicit_color,
        ..original_identity.clone()
    };
    assert!(!retained.matches(&different_color));
    let custom_color = style(Some("#87878F8FE0E0"));
    let different_explicit_rgb = ElementWrapIdentity {
        text_style: &custom_color,
        ..original_identity.clone()
    };
    assert!(!retained.matches(&different_explicit_rgb));
    let expanded_runs = [TextRun {
        text: "x".to_owned(),
        attributes: Vec::new(),
    }];
    let different_run_repertoire = ElementWrapIdentity {
        runs: &expanded_runs,
        ..original_identity.clone()
    };
    assert!(!retained.matches(&different_run_repertoire));
    let attributed_runs = [TextRun {
        text: "x".to_owned(),
        attributes: vec![RunAttribute {
            name: "Bold".to_owned(),
            value: "Yes".to_owned(),
        }],
    }];
    let different_run_attributes = ElementWrapIdentity {
        runs: &attributed_runs,
        ..original_identity.clone()
    };
    assert!(!retained.matches(&different_run_attributes));

    // Force two unequal identities into the same candidate bucket so the
    // lookup path proves that a fingerprint is not accepted as equality.
    let mut cache = ElementWrapCache::default();
    let mut diagnostics = LayoutBuildDiagnostics::default();
    let original_fingerprint = cache.fingerprint(&original_identity, "x");
    let first_result = cache.wrap(
        &element_id,
        Some(&action),
        "x",
        0,
        Some(action.clone()),
        original_identity.paragraph,
        base_style.clone(),
        original_identity.char_width,
        &runs,
        &atomic,
        &atomic,
        LayoutPurpose::Editor,
        false,
        EffectiveGeometryMode::Sequential,
        Some(&mut diagnostics),
    );
    assert_eq!(cache.entry_count, 1);
    let reused_result = cache.wrap(
        &element_id,
        Some(&action),
        "x",
        1,
        Some(action.clone()),
        original_identity.paragraph,
        base_style.clone(),
        original_identity.char_width,
        &runs,
        &atomic,
        &atomic,
        LayoutPurpose::Editor,
        false,
        EffectiveGeometryMode::Sequential,
        Some(&mut diagnostics),
    );
    assert_eq!(diagnostics.element_wraps_reused, 1);
    assert_eq!(
        first_result
            .iter()
            .map(|line| &line.text)
            .collect::<Vec<_>>(),
        reused_result
            .iter()
            .map(|line| &line.text)
            .collect::<Vec<_>>()
    );

    let different_fingerprint = cache.fingerprint(&different_key, "x");
    let mut retained_bucket = cache
        .buckets
        .remove(&original_fingerprint)
        .expect("the original wrap should be retained");
    let retained_entry = retained_bucket
        .pop()
        .expect("the original wrap bucket should contain an entry");
    cache
        .buckets
        .entry(different_fingerprint)
        .or_default()
        .push(retained_entry);
    diagnostics = LayoutBuildDiagnostics::default();
    let _ = cache.wrap(
        &element_id,
        Some(&custom),
        "x",
        0,
        Some(action.clone()),
        different_key.paragraph,
        base_style.clone(),
        different_key.char_width,
        &runs,
        &atomic,
        &atomic,
        LayoutPurpose::Editor,
        false,
        EffectiveGeometryMode::Sequential,
        Some(&mut diagnostics),
    );
    assert_eq!(diagnostics.element_wraps_reused, 0);
    Ok(())
}
