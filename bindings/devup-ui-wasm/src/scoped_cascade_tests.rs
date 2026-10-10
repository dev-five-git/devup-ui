use super::*;
use rstest::rstest;
use serial_test::serial;

fn compile(file: &str, source: &str, single: bool) -> Output {
    code_extract_internal(
        file,
        source,
        "@devup-ui/react",
        "df".into(),
        single,
        false,
        false,
        HashMap::new(),
    )
    .unwrap_or_else(|error| panic!("{error}"))
}

#[rstest]
#[case(false, false)]
#[case(false, true)]
#[case(true, false)]
#[case(true, true)]
#[serial]
fn independent_sheets_cannot_override_responsive_atoms_when_base_content_matches(
    #[case] seeded: bool,
    #[case] reverse: bool,
) {
    // Given: two delivered sheets share base content, but only A has a desktop override.
    reset_build_state_internal();
    css::debug::set_debug(false);
    if seeded {
        seed_file_map(vec!["a.tsx".into(), "b.tsx".into()]);
    }
    let inputs = [
        (
            "a.tsx",
            "import {Box} from '@devup-ui/react'; export const x=<Box flexDir={['column','row']} display={['none',null,null,'flex']}/>;",
        ),
        (
            "b.tsx",
            "import {Box} from '@devup-ui/react'; export const x=<Box flexDir='column' display='none'/>;",
        ),
    ];
    let order = if reverse { [1, 0] } else { [0, 1] };
    // When: extraction arrives in either order without changing placement.
    let mut outputs = BTreeMap::new();
    for index in order {
        outputs.insert(
            inputs[index].0,
            compile(inputs[index].0, inputs[index].1, false),
        );
    }
    // Then: B's base selectors cannot match A's classes; each rule remains in its exact bucket.
    with_style_sheet(|sheet| {
        let base = |file: &str| {
            sheet.properties[file][&255][&0]
                .iter()
                .find(|property| property.property == "flex-direction")
                .unwrap_or_else(|| panic!("missing base rule"))
                .class_name
                .clone()
        };
        let a = base("a.tsx");
        let b = base("b.tsx");
        assert_ne!(a, b);
        assert!(outputs["a.tsx"].code().contains(&a));
        assert!(outputs["b.tsx"].code().contains(&b));
        assert!(
            !sheet
                .create_css(Some("b.tsx"), false)
                .contains(&format!(".{a}{{"))
        );
        assert_eq!(
            sheet.properties["a.tsx"][&255]
                .keys()
                .copied()
                .collect::<Vec<_>>(),
            vec![0, 1, 3]
        );
        assert_eq!(
            sheet.properties["b.tsx"][&255]
                .keys()
                .copied()
                .collect::<Vec<_>>(),
            vec![0]
        );
        assert_eq!(sheet.properties.get(""), None);
    });
    reset_build_state_internal();
}

#[rstest]
#[case(false, false)]
#[case(false, true)]
#[case(true, false)]
#[case(true, true)]
#[serial]
fn shared_delivery_keeps_existing_rule_placement_when_atoms_are_unprefixed(
    #[case] hoist: bool,
    #[case] reverse: bool,
) {
    // Given: single CSS or a frozen atom-hoist plan, not moved per-file rules.
    reset_build_state_internal();
    css::debug::set_debug(false);
    if hoist {
        set_atom_hoist(Some(2));
        import_file_routes_internal(HashMap::from([
            ("a.tsx".into(), std::collections::HashSet::from([0, 1])),
            ("b.tsx".into(), std::collections::HashSet::from([0, 1])),
        ]));
    }
    let inputs = [
        (
            "a.tsx",
            "import {Box} from '@devup-ui/react'; export const x=<Box flexDir={['column','row']}/>;",
        ),
        (
            "b.tsx",
            "import {Box} from '@devup-ui/react'; export const x=<Box flexDir='column'/>;",
        ),
    ];
    // When
    for index in if reverse { [1, 0] } else { [0, 1] } {
        compile(inputs[index].0, inputs[index].1, !hoist);
    }
    // Then: the shared sheet retains base-before-responsive order; raw buckets do not change.
    with_style_sheet(|sheet| {
        let css = sheet.create_css(None, false);
        assert!(
            css.find("flex-direction:column")
                .unwrap_or_else(|| panic!("missing base"))
                < css
                    .find("@media")
                    .unwrap_or_else(|| panic!("missing responsive rule"))
        );
        assert_eq!(css.matches("flex-direction:column").count(), 1);
        assert!(css.contains("flex-direction:row"));
        let expected = if hoist {
            vec!["a.tsx", "b.tsx"]
        } else {
            vec![""]
        };
        assert_eq!(
            sheet
                .properties
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            expected
        );
        for orders in sheet.properties.values() {
            for property in orders[&255].values().flatten() {
                assert!(property.class_name.starts_with('O'));
                assert_eq!(property.hoisted, hoist);
            }
        }
    });
    reset_build_state_internal();
}

#[test]
#[serial]
fn collapsed_bucket_retains_its_identity_prefix_and_cache_claims() {
    // Given: A and its child share a canonical bucket, B remains independently delivered.
    reset_build_state_internal();
    import_canonical_map_internal(HashMap::from([("child.tsx".into(), "a.tsx".into())]));
    compile(
        "child.tsx",
        "import {Box} from '@devup-ui/react'; export const x=<Box flexDir={['column','row']}/>;",
        false,
    );
    compile(
        "b.tsx",
        "import {Box} from '@devup-ui/react'; export const x=<Box flexDir='column'/>;",
        false,
    );
    let before = export_sheet_internal().unwrap_or_else(|error| panic!("{error}"));
    let css_before = with_style_sheet(|sheet| sheet.create_css(Some("a.tsx"), false));
    // When: exact name claims round-trip through the existing cache protocol.
    import_sheet_internal(serde_json::from_str(&before).unwrap_or_else(|error| panic!("{error}")))
        .unwrap_or_else(|error| panic!("{error}"));
    // Then
    assert_eq!(
        with_style_sheet(|sheet| sheet.create_css(Some("a.tsx"), false)),
        css_before
    );
    with_style_sheet(|sheet| {
        assert_eq!(
            sheet
                .properties
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            vec!["a.tsx", "b.tsx"]
        );
        let a = sheet.properties["a.tsx"][&255][&0]
            .iter()
            .next()
            .unwrap_or_else(|| panic!("missing A"));
        let b = sheet.properties["b.tsx"][&255][&0]
            .iter()
            .next()
            .unwrap_or_else(|| panic!("missing B"));
        assert_ne!(a.class_name, b.class_name);
        assert!(a.class_name.starts_with("FLa_ptsx-"));
        assert!(sheet.names.contains_key(&a.class_name));
        assert_eq!(sheet.names["FLa_ptsx"].content, "a.tsx");
    });
    reset_build_state_internal();
}
