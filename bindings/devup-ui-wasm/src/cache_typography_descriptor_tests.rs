use super::*;
use crate::cache_descriptor::Cursor;
use css::{Naming, content_name::AtomContent};

#[rstest]
#[case(None)]
#[case(Some("first.tsx"))]
#[case(Some("second.tsx"))]
#[serial]
fn get_css_selects_global_or_numbered_file_with_main_import_control(
    #[case] file: Option<&str>,
    #[values(false, true)] import_main_css: bool,
) {
    // Given: distinct file declarations and real global typography CSS.
    configure(false);
    let first = output(
        "first.tsx",
        "import {Box}from '@devup-ui/react';export const x=<Box color='red'/>;",
        false,
    );
    let second = output(
        "second.tsx",
        "import {Box}from '@devup-ui/react';export const x=<Box color='blue'/>;",
        false,
    );
    assert!(first.2.contains("color:red"), "{}", first.2);
    assert!(second.2.contains("color:blue"), "{}", second.2);
    assert_ne!(first.2, second.2);
    assert!(second.1.contains("font-size:14px"), "{}", second.1);
    let file_num = file.map(|file| {
        with_file_map(|map| {
            map.get_by_left(file)
                .copied()
                .unwrap_or_else(|| panic!("registered file: {file}"))
        })
    });
    let expected = match file {
        None => second.1,
        Some("first.tsx") => first.2,
        Some("second.tsx") => second.2,
        Some(other) => panic!("unexpected fixture: {other}"),
    };
    // When
    let actual = get_css_internal(file_num, import_main_css)
        .unwrap_or_else(|error| panic!("getCss: {error}"));
    // Then: file selection is exact; only numbered files receive the requested main import.
    let main_import = "@import \"./devup-ui.css\";";
    assert_eq!(
        actual.matches(main_import).count(),
        usize::from(file.is_some() && import_main_css)
    );
    assert_eq!(actual.replace(main_import, ""), expected);
    fresh();
}

#[rstest]
#[serial]
fn get_css_uses_global_output_when_file_number_is_unmapped(
    #[values(false, true)] import_main_css: bool,
) {
    // Given
    configure(false);
    let expected = output("first.tsx", source(false), false).1;
    // When
    let actual = get_css_internal(Some(usize::MAX), import_main_css);
    // Then: the existing failed lookup passes None to CSS generation.
    assert_eq!(actual, Ok(expected));
    fresh();
}

enum Damage {
    Header,
    Naming,
    Property,
    ValueTag,
    Order,
    Trailing,
}

#[rstest]
#[case(Damage::Header)]
#[case(Damage::Naming)]
#[case(Damage::Property)]
#[case(Damage::ValueTag)]
#[case(Damage::Order)]
#[case(Damage::Trailing)]
#[serial]
fn malformed_typography_descriptor_is_cold_before_claim_preflight(#[case] damage: Damage) {
    // Given: an actual content-named typography atom and its versioned proof.
    configure(false);
    let cold = output("fresh.tsx", source(false), false);
    let mut value = snapshot();
    let claim = value["names"]
        .as_object_mut()
        .unwrap_or_else(|| panic!("names object"))
        .values_mut()
        .find(|claim| {
            claim["descriptor"]
                .as_array()
                .is_some_and(|bytes| bytes.get(1) == Some(&1.into()))
        })
        .unwrap_or_else(|| panic!("typography claim"));
    let mut descriptor: Vec<u8> = serde_json::from_value(claim["descriptor"].clone())
        .unwrap_or_else(|error| panic!("descriptor: {error}"));
    match damage {
        Damage::Header => descriptor[1] = 3,
        Damage::Naming => descriptor[2] = 2,
        Damage::Property => descriptor[11] = b'x',
        Damage::ValueTag => descriptor[21] = 0,
        Damage::Order => {
            let mut cursor = Cursor(&descriptor[3..]);
            assert_eq!(cursor.text(), Some("typography"));
            assert_eq!(cursor.byte(), Some(1));
            assert!(cursor.text().is_some());
            assert_eq!(cursor.byte(), Some(0));
            let index = descriptor.len() - cursor.0.len();
            descriptor[index] = 1;
        }
        Damage::Trailing => descriptor.push(99),
    }
    claim["descriptor"] =
        serde_json::to_value(descriptor).unwrap_or_else(|error| panic!("descriptor JSON: {error}"));
    configure(false);
    let before = snapshot();
    // When: cache admission encounters the corrupt exact typography proof.
    assert_eq!(import(value), Ok(()));
    // Then: the failure is a cold miss, not a real-name collision or partial adoption.
    assert_eq!(snapshot(), before);
    assert_eq!(output("fresh.tsx", source(false), false), cold);
    assert_eq!(cache_names::check(), Ok(()));
    fresh();
}

#[test]
#[serial]
fn current_typography_claim_collision_keeps_both_locations_and_live_state() {
    // Given: two genuinely different preset expansions with real source locations.
    let source =
        "import {Box} from '@devup-ui/react';\nexport const x=<Box _hover={{typography:'body'}}/>;";
    let configure = |size: &str| {
        fresh();
        register_theme_internal(
            serde_json::from_value(serde_json::json!({"typography":{"body":{"fontSize":size}}}))
                .unwrap_or_else(|error| panic!("theme fixture: {error}")),
        );
    };
    configure("18px");
    output("cached.tsx", source, true);
    let mut incoming = snapshot();
    let cached_claim = incoming["names"]
        .as_object()
        .unwrap_or_else(|| panic!("cached names object"))
        .values()
        .find(|claim| {
            claim["descriptor"]
                .as_array()
                .is_some_and(|bytes| bytes.get(1) == Some(&1.into()))
        })
        .unwrap_or_else(|| panic!("cached typography claim"))
        .clone();
    configure("14px");
    output("live.tsx", source, true);
    let before = snapshot();
    let live_name = before["names"]
        .as_object()
        .unwrap_or_else(|| panic!("live names object"))
        .iter()
        .find(|(_, claim)| {
            claim["descriptor"]
                .as_array()
                .is_some_and(|bytes| bytes.get(1) == Some(&1.into()))
        })
        .unwrap_or_else(|| panic!("live typography claim"))
        .0
        .clone();
    incoming["properties"] = serde_json::json!({});
    incoming["names"]
        .as_object_mut()
        .unwrap_or_else(|| panic!("incoming names object"))
        .insert(live_name.clone(), cached_claim);
    let column = source
        .split('\n')
        .nth(1)
        .unwrap_or_else(|| panic!("authored second line"))
        .find("'body'")
        .unwrap_or_else(|| panic!("authored typography operand"))
        + 1;
    // When: a complete current snapshot presents conflicting exact bytes under a live name.
    let error = import(incoming)
        .err()
        .unwrap_or_else(|| panic!("conflicting claims must fail"));
    // Then: both source sites survive, no live state changes, and the genuine error stays sticky.
    assert!(
        error.contains(&format!("cached.tsx:2:{column}:")),
        "{error}"
    );
    assert!(error.contains(&format!("live.tsx:2:{column}:")), "{error}");
    assert!(error.contains(&format!("`{live_name}`")), "{error}");
    assert_eq!(snapshot(), before);
    let extraction_error = code_extract_internal(
        "live.tsx",
        source,
        "@devup-ui/react",
        "df".into(),
        true,
        false,
        false,
        HashMap::new(),
    )
    .err()
    .unwrap_or_else(|| panic!("sticky collision rejects extraction"));
    assert_eq!(extraction_error, error);
    assert_eq!(get_css_internal(None, false), Err(error.clone()));
    assert_eq!(snapshot(), before);
    assert_eq!(cache_names::check(), Err(error));
    fresh();
}

#[rstest]
#[case(None)]
#[case(Some("t0"))]
#[case(Some("tgg"))]
#[case(Some("t0000000000000001"))]
#[serial]
fn risky_typography_proof_accepts_only_exact_complete_declarations(
    #[case] malformed: Option<&str>,
) {
    // Given: real expanded declarations, named with Risky provenance at the pure proof boundary.
    configure(false);
    output("fresh.tsx", source(false), false);
    let (mut property, mut levels, descriptor) = with_style_sheet(|sheet| {
        let levels = &sheet.properties["fresh.tsx"][&255];
        let property = levels
            .values()
            .flatten()
            .find(|property| property.typography)
            .unwrap_or_else(|| panic!("expanded typography property"));
        (
            property.clone(),
            levels.clone(),
            sheet.names[&property.class_name].descriptor.clone(),
        )
    });
    let mut cursor = Cursor(&descriptor[3..]);
    assert_eq!(cursor.text(), Some("typography"));
    assert_eq!(cursor.byte(), Some(1));
    let original = cursor
        .text()
        .unwrap_or_else(|| panic!("typography declaration encoding"));
    let content = AtomContent {
        property: "typography",
        value: Some(malformed.unwrap_or(original)),
        naming: Naming::Risky,
        level: 0,
        order: 255,
        selector: property.selector.as_ref(),
        layer: None,
        dynamic: false,
    }
    .content();
    let old_name = property.class_name.clone();
    property.class_name = content.name("FLfresh_ptsx-");
    for properties in levels.values_mut() {
        *properties = properties
            .iter()
            .cloned()
            .map(|mut other| {
                if other.class_name == old_name {
                    other.class_name.clone_from(&property.class_name);
                }
                other
            })
            .collect();
    }
    // When: provenance and declaration decoding are checked against the actual expanded group.
    let result =
        crate::cache_special_proof::typography(&property, (&levels, 255), &content.descriptor);
    // Then: Risky is valid, but odd/non-hex/truncated declaration encodings cannot prove it.
    assert_eq!(result, malformed.is_none().then_some(()));
    fresh();
}
