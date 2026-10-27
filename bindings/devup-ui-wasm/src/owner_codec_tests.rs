use std::collections::{BTreeSet, HashMap};

use rstest::rstest;
use serial_test::serial;
use sheet::StyleSheet;

use crate::{
    TestStateGuard, export_class_map_internal, export_sheet_internal,
    import_class_map_json_internal, import_sheet_internal, import_sheet_json_internal,
    reset_build_state_internal, with_style_sheet,
};

fn rich_sheet() -> Result<StyleSheet, serde_json::Error> {
    let selectors = [
        serde_json::Value::Null,
        serde_json::json!({"Selector": ":hover"}),
        serde_json::json!({"Global": [".owner", "01\0𐀀.tsx"]}),
        serde_json::json!({"At": {
            "kind": "Container", "query": "(width > 1px)", "selector": ":focus",
            "outer": [{"kind": "Supports", "query": "(display:grid)"}],
            "file": "01\0𐀀.tsx"
        }}),
    ];
    let properties: Vec<_> = selectors
        .into_iter()
        .enumerate()
        .map(|(index, selector)| {
            serde_json::json!({
                "c": format!("owner{index}"), "p": "content", "v": "01 1e3 \"\\\n\0𐀀",
                "s": selector, "l": "owner", "t": true, "h": true, "o": u32::MAX
            })
        })
        .collect();
    serde_json::from_value(serde_json::json!({
        "properties": {"01\0𐀀.tsx": {"0": {"255": properties}, "255": {"0": []}}},
        "css": {"01\0𐀀.tsx": [{"css": ".raw{content:'01 1e3'}"}]},
        "keyframes": {"01\0𐀀.tsx": {"motion": {"from": [["opacity", "0"]]}}},
        "global_css_files": ["01\0𐀀.tsx"],
        "imports": {"01\0𐀀.tsx": ["url('01\\𐀀.css')"]},
        "font_faces": {"01\0𐀀.tsx": [{"font-family": "owner\0𐀀", "src": "url('01')"}]},
        "layer_orders": {"01\0𐀀.tsx": ["owner", "01", "1e3"]},
        "atomNamingVersion": 2
    }))
}

#[rstest]
#[case(None)]
#[case(Some(BTreeSet::new()))]
#[case(Some(BTreeSet::from(["01\0𐀀.tsx".to_string()])))]
#[serial]
fn owner_sheet_restores_every_field_when_exported(
    #[case] plan: Option<BTreeSet<String>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given a real export with all persisted sheet fields and a different live plan.
    let _state = TestStateGuard::new();
    reset_build_state_internal();
    let mut expected = rich_sheet()?;
    expected.atom_plan.clone_from(&plan);
    let json = serde_json::to_string(&expected.export_snapshot())?;
    let expected_css = expected.create_css(Some("01\0𐀀.tsx"), false);
    css::atom_hoist::restore_atom_plan(Some(BTreeSet::from(["other".into()])));
    // When the direct codec imports the serializer's bytes.
    import_sheet_json_internal(&json)?;
    // Then no persisted field or global plan is lost, including numeric-key boundaries.
    with_style_sheet(|actual| {
        assert_eq!(actual.atom_plan, expected.atom_plan);
        assert_eq!(actual.properties, expected.properties);
        assert_eq!(actual.css, expected.css);
        assert_eq!(actual.keyframes, expected.keyframes);
        assert_eq!(actual.global_css_files, expected.global_css_files);
        assert_eq!(actual.imports, expected.imports);
        assert_eq!(actual.font_faces, expected.font_faces);
        assert_eq!(actual.layer_orders, expected.layer_orders);
        assert_eq!(actual.create_css(Some("01\0𐀀.tsx"), false), expected_css);
    });
    assert_eq!(css::atom_hoist::atom_plan(), plan);
    Ok(())
}

#[test]
#[serial]
fn owner_classes_preserve_allocator_keys_when_exported() -> Result<(), Box<dyn std::error::Error>> {
    // Given shared counters, an empty bucket, numeric-looking and escaped keys.
    let _state = TestStateGuard::new();
    let expected = HashMap::from([
        (
            String::new(),
            HashMap::from([("01\0𐀀\"\\".into(), usize::try_from(u32::MAX)?)]),
        ),
        ("empty".into(), HashMap::new()),
        (
            "1e3".into(),
            HashMap::from([("01".into(), 0), ("255".into(), 255)]),
        ),
    ]);
    css::class_map::set_class_map(expected.clone());
    let json = export_class_map_internal()?;
    css::class_map::set_class_map(HashMap::new());
    // When decoding matching Rust bytes.
    import_class_map_json_internal(&json)?;
    // Then every key and the wasm32 maximum are preserved exactly.
    assert_eq!(css::class_map::get_class_map(), expected);
    Ok(())
}

#[test]
#[serial]
fn owner_sheet_replaces_a_sibling_when_aba_restored() -> Result<(), Box<dyn std::error::Error>> {
    // Given an exported rich A and a different installed B.
    let _state = TestStateGuard::new();
    import_sheet_internal(rich_sheet()?);
    let a = export_sheet_internal()?;
    let expected_css = with_style_sheet(|sheet| sheet.create_css(None, false));
    reset_build_state_internal();
    // When A resumes after the sibling reset.
    import_sheet_json_internal(&a)?;
    // Then the restored CSS agrees with the original sheet.
    assert_eq!(
        with_style_sheet(|sheet| sheet.create_css(None, false)),
        expected_css
    );
    Ok(())
}
