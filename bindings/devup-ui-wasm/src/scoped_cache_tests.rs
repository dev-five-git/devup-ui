use super::*;
use rstest::rstest;
use serial_test::serial;

#[test]
#[serial]
fn previous_per_file_content_names_cannot_restore_cross_sheet_aliases() {
    reset_build_state_internal();
    let source = "import {Box} from '@devup-ui/react';export const x=<Box color='red'/>;";
    let cold = code_extract_internal(
        "x.tsx",
        source,
        "@devup-ui/react",
        "df".into(),
        false,
        false,
        false,
        HashMap::new(),
    )
    .unwrap_or_else(|error| panic!("{error}"));
    let cold_css = with_style_sheet(|sheet| sheet.create_css(Some("x.tsx"), false));
    let exported = export_sheet_internal().unwrap_or_else(|error| panic!("{error}"));
    let mut previous: StyleSheet =
        serde_json::from_str(&exported).unwrap_or_else(|error| panic!("{error}"));
    let prefix = "FLx_ptsx-";
    for levels in previous
        .properties
        .get_mut("x.tsx")
        .unwrap_or_else(|| panic!("per-file fixture"))
        .values_mut()
    {
        for properties in levels.values_mut() {
            *properties = properties
                .iter()
                .cloned()
                .map(|mut property| {
                    property.class_name = property
                        .class_name
                        .strip_prefix(prefix)
                        .unwrap_or_else(|| panic!("scoped fixture"))
                        .to_string();
                    property
                })
                .collect();
        }
    }
    previous.names = previous
        .names
        .into_iter()
        .filter(|(name, _)| name != "FLx_ptsx")
        .map(|(name, claim)| {
            (
                name.strip_prefix(prefix).unwrap_or(&name).to_string(),
                claim,
            )
        })
        .collect();
    reset_build_state_internal();

    let imported = import_sheet_internal(previous);

    assert_eq!(imported, Ok(()));
    assert_eq!(with_style_sheet(|sheet| sheet.names.len()), 0);
    let fresh = code_extract_internal(
        "x.tsx",
        source,
        "@devup-ui/react",
        "df".into(),
        false,
        false,
        false,
        HashMap::new(),
    )
    .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(fresh.code(), cold.code());
    assert_eq!(
        with_style_sheet(|sheet| sheet.create_css(Some("x.tsx"), false)),
        cold_css
    );
    reset_build_state_internal();
}

#[rstest]
#[case("", 255)]
#[case("du-FLa-", 1)]
#[case("FHprefix-", 255)]
#[serial]
fn previous_hashed_per_file_names_are_rejected_even_with_exact_atom_claims(
    #[case] prefix: &str,
    #[case] order: u8,
) {
    // Given: real exported long-content atoms, stripped of only their sheet namespace.
    reset_build_state_internal();
    set_prefix(Some(prefix.into()));
    let source = format!(
        "import {{Box}} from '@devup-ui/react';export const x=<Box fontFamily='abcdefghijklmnopqrstuvwx' styleOrder={{{order}}}/>;"
    );
    let cold = code_extract_internal(
        "x.tsx",
        &source,
        "@devup-ui/react",
        "df".into(),
        false,
        false,
        false,
        HashMap::new(),
    )
    .unwrap_or_else(|error| panic!("{error}"));
    let cold_css = with_style_sheet(|sheet| sheet.create_css(Some("x.tsx"), false));
    let mut incoming: StyleSheet =
        serde_json::from_str(&export_sheet_internal().unwrap_or_else(|error| panic!("{error}")))
            .unwrap_or_else(|error| panic!("{error}"));
    let scope = format!("{prefix}FLx_ptsx");
    let scoped_prefix = format!("{scope}-");
    for properties in incoming
        .properties
        .get_mut("x.tsx")
        .unwrap_or_else(|| panic!("missing per-file bucket fixture"))
        .get_mut(&order)
        .unwrap_or_else(|| panic!("missing style order fixture"))
        .values_mut()
    {
        *properties = properties
            .iter()
            .cloned()
            .map(|mut property| {
                property.class_name = format!(
                    "{prefix}{}",
                    property
                        .class_name
                        .strip_prefix(&scoped_prefix)
                        .unwrap_or_else(|| panic!("missing scoped name fixture"))
                );
                property
            })
            .collect();
    }
    incoming.names = incoming
        .names
        .into_iter()
        .filter(|(name, _)| name != &scope)
        .map(|(name, claim)| match name.strip_prefix(&scoped_prefix) {
            Some(atom) => (format!("{prefix}{atom}"), claim),
            None => (name, claim),
        })
        .collect();
    reset_build_state_internal();
    set_prefix(Some(prefix.into()));
    // When
    assert_eq!(import_sheet_internal(incoming), Ok(()));
    // Then: an exact atom claim cannot admit an old unscoped namespace.
    assert_eq!(with_style_sheet(|sheet| sheet.names.len()), 0);
    assert_eq!(
        code_extract_internal(
            "x.tsx",
            &source,
            "@devup-ui/react",
            "df".into(),
            false,
            false,
            false,
            HashMap::new(),
        )
        .unwrap_or_else(|error| panic!("{error}"))
        .code(),
        cold.code()
    );
    assert_eq!(
        with_style_sheet(|sheet| sheet.create_css(Some("x.tsx"), false)),
        cold_css
    );
    reset_build_state_internal();
}

#[rstest]
#[case(false, false, 0)]
#[case(true, false, 255)]
#[case(false, true, 255)]
#[case(false, false, 255)]
#[serial]
fn actual_exported_shared_and_scoped_names_round_trip(
    #[case] single: bool,
    #[case] hoist: bool,
    #[case] order: u8,
) {
    // Given: actual global base, single CSS, hoisted, or current per-file output.
    reset_build_state_internal();
    set_prefix(Some("du-FLa-".into()));
    if hoist {
        set_atom_hoist(Some(2));
        import_file_routes_internal(HashMap::from([(
            "x.tsx".into(),
            std::collections::HashSet::from([0, 1]),
        )]));
    }
    let source = format!(
        "import {{Box,keyframes}} from '@devup-ui/react';export const k=keyframes({{from:{{opacity:0}},to:{{opacity:1}}}});export const x=<Box color='red' styleOrder={{{order}}}/>;"
    );
    code_extract_internal(
        "x.tsx",
        &source,
        "@devup-ui/react",
        "df".into(),
        single,
        false,
        false,
        HashMap::new(),
    )
    .unwrap_or_else(|error| panic!("{error}"));
    let exported = export_sheet_internal().unwrap_or_else(|error| panic!("{error}"));
    let incoming: StyleSheet =
        serde_json::from_str(&exported).unwrap_or_else(|error| panic!("{error}"));
    let property = incoming
        .properties
        .values()
        .flat_map(|orders| orders.values())
        .flat_map(|levels| levels.values())
        .flatten()
        .next()
        .unwrap_or_else(|| panic!("missing exported property fixture"));
    let expected = if order == 0 {
        "du-FLa-OLcolor-vred-o0"
    } else if single || hoist {
        "du-FLa-OLcolor-vred"
    } else {
        "du-FLa-FLx_ptsx-OLcolor-vred"
    };
    assert_eq!(property.class_name, expected);
    assert_eq!(property.hoisted, hoist);
    assert_eq!(
        incoming
            .keyframes
            .values()
            .map(BTreeMap::len)
            .sum::<usize>(),
        1
    );
    assert!(
        incoming
            .keyframes
            .values()
            .flat_map(|frames| frames.keys())
            .all(|name| name.starts_with("du-FLa-K"))
    );
    let before = incoming.create_css(None, false);
    reset_build_state_internal();
    set_prefix(Some("du-FLa-".into()));
    // When: import runs without relying on the producer's active hoist configuration.
    let result = import_sheet_internal(incoming);
    // Then
    assert_eq!(result, Ok(()));
    assert_eq!(
        with_style_sheet(|sheet| sheet.create_css(None, false)),
        before
    );
    reset_build_state_internal();
}

#[rstest]
#[case("a-a")]
#[case("Rcolor-vred")]
#[case("manual-card")]
#[serial]
fn generationless_serialized_counter_and_manual_names_are_cold(#[case] name: &str) {
    // Given: legacy JSON with a real per-file declaration, no generation marker or names registry.
    reset_build_state_internal();
    let json = serde_json::json!({
        "properties": {"x.tsx": {"255": {"0": [{"c": name, "p": "color", "v": "red", "s": null}]}}}
    });
    let incoming: StyleSheet =
        serde_json::from_value(json).unwrap_or_else(|error| panic!("{error}"));
    // When
    let result = import_sheet_internal(incoming);
    // Then: serialized legacy names are never silently promoted to current state.
    assert_eq!(result, Ok(()));
    assert_eq!(with_style_sheet(|sheet| sheet.properties.len()), 0);
    reset_build_state_internal();
}
