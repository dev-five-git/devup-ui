use super::*;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("OL")]
#[case("RL")]
#[case("KL")]
#[case("OH")]
#[serial]
fn seeded_counter_cache_round_trips_when_prefix_resembles_content(#[case] prefix: &str) {
    // Given: actual safe-own counter output under an unrestricted configured prefix.
    reset_build_state_internal();
    set_prefix(Some(prefix.into()));
    seed_file_map(vec!["x.tsx".into()]);
    let output = code_extract_internal(
        "x.tsx",
        "import {Box} from '@devup-ui/react';export const x=<Box color='red'/>;",
        "@devup-ui/react",
        "df".into(),
        false,
        false,
        false,
        HashMap::new(),
    )
    .unwrap_or_else(|error| panic!("{error}"));
    let incoming: StyleSheet =
        serde_json::from_str(&export_sheet_internal().unwrap_or_else(|error| panic!("{error}")))
            .unwrap_or_else(|error| panic!("{error}"));
    let class = format!("{prefix}a-a");
    assert!(output.code().contains(&class));
    assert!(!incoming.names.contains_key(&class));
    let before = incoming.create_css(Some("x.tsx"), false);
    reset_build_state_internal();
    set_prefix(Some(prefix.into()));
    // When
    let imported = import_sheet_internal(incoming);
    // Then: cached CSS remains usable instead of being mistaken for an unclaimed atom.
    assert_eq!(imported, Ok(()));
    assert_eq!(
        with_style_sheet(|sheet| sheet.create_css(Some("x.tsx"), false)),
        before
    );
    reset_build_state_internal();
}

fn compiled_content(single: bool) -> Output {
    code_extract_internal(
        "x.tsx",
        "import {Box,keyframes} from '@devup-ui/react';export const k=keyframes({from:{opacity:0},to:{opacity:1}});export const x=<Box color='red' fontFamily='abcdefghijklmnopqrstuvwxyz'/>;",
        "@devup-ui/react",
        "df".into(),
        single,
        false,
        false,
        HashMap::new(),
    ).unwrap_or_else(|error| panic!("{error}"))
}

fn exported_content(prefix: &str, single: bool) -> StyleSheet {
    reset_build_state_internal();
    set_prefix(Some(prefix.into()));
    compiled_content(single);
    serde_json::from_str(&export_sheet_internal().unwrap_or_else(|error| panic!("{error}")))
        .unwrap_or_else(|error| panic!("{error}"))
}

#[rstest]
#[case("OL")]
#[case("RL")]
#[case("KL")]
#[case("OH")]
#[serial]
fn generated_cache_round_trips_when_prefix_resembles_content(
    #[case] prefix: &str,
    #[values(false, true)] single: bool,
) {
    // Given: genuine short/hashed content atoms and global keyframes, not legacy counters.
    let incoming = exported_content(prefix, single);
    let namespace = if single {
        prefix.into()
    } else {
        format!("{prefix}FLx_ptsx-")
    };
    assert!(
        incoming
            .names
            .contains_key(&format!("{namespace}OLcolor-vred"))
    );
    assert!(
        incoming
            .names
            .keys()
            .any(|name| name.starts_with(&format!("{namespace}OH")))
    );
    assert_eq!(
        incoming
            .keyframes
            .values()
            .map(BTreeMap::len)
            .sum::<usize>(),
        1
    );
    let before = incoming.create_css(None, false);
    let claims = incoming.names.clone();
    reset_build_state_internal();
    set_prefix(Some(prefix.into()));
    // When
    let imported = import_sheet_internal(incoming);
    // Then: the real imported CSS and exact claims survive the prefix boundary.
    assert_eq!(imported, Ok(()));
    with_style_sheet(|sheet| {
        assert_eq!(sheet.create_css(None, false), before);
        assert_eq!(sheet.names, claims);
    });
    reset_build_state_internal();
}

enum ClaimDamage {
    MissingAtom,
    MissingScope,
    CorruptScope,
}

#[rstest]
#[case("OL")]
#[case("RL")]
#[case("KL")]
#[case("OH")]
#[serial]
fn genuine_generated_cache_rejects_damaged_claims_when_prefix_resembles_content(
    #[case] prefix: &str,
    #[values(
        ClaimDamage::MissingAtom,
        ClaimDamage::MissingScope,
        ClaimDamage::CorruptScope
    )]
    damage: ClaimDamage,
) {
    // Given: only one required exact claim is removed or corrupted in real scoped output.
    let mut incoming = exported_content(prefix, false);
    let cold_css = incoming.create_css(Some("x.tsx"), false);
    let cold_code = compiled_content(false).code();
    let scope = format!("{prefix}FLx_ptsx");
    match damage {
        ClaimDamage::MissingAtom => {
            incoming
                .names
                .remove(&format!("{scope}-OLcolor-vred"))
                .unwrap_or_else(|| panic!("missing atom claim fixture"));
        }
        ClaimDamage::MissingScope => {
            incoming
                .names
                .remove(&scope)
                .unwrap_or_else(|| panic!("missing scope claim fixture"));
        }
        ClaimDamage::CorruptScope => {
            incoming
                .names
                .get_mut(&scope)
                .unwrap_or_else(|| panic!("missing scope claim fixture"))
                .descriptor
                .push(1);
        }
    }
    reset_build_state_internal();
    set_prefix(Some(prefix.into()));
    // When
    let imported = import_sheet_internal(incoming);
    // Then: recognizing a user prefix must not adopt any damaged generated namespace.
    assert_eq!(imported, Ok(()));
    assert_eq!(with_style_sheet(|sheet| sheet.names.len()), 0);
    assert_eq!(compiled_content(false).code(), cold_code);
    assert_eq!(
        with_style_sheet(|sheet| sheet.create_css(Some("x.tsx"), false)),
        cold_css
    );
    reset_build_state_internal();
}

#[rstest]
#[case("OL")]
#[case("RL")]
#[case("KL")]
#[case("OH")]
#[serial]
fn previous_global_namespace_is_rejected_in_per_file_cache_with_content_shaped_prefix(
    #[case] prefix: &str,
) {
    // Given: valid global atoms and claims placed in an old nonzero-order per-file bucket.
    let mut incoming = exported_content(prefix, true);
    let cold = exported_content(prefix, false);
    let cold_css = cold.create_css(Some("x.tsx"), false);
    let cold_code = compiled_content(false).code();
    let orders = incoming
        .properties
        .remove("")
        .unwrap_or_else(|| panic!("missing single CSS bucket fixture"));
    incoming.properties.insert("x.tsx".into(), orders);
    reset_build_state_internal();
    set_prefix(Some(prefix.into()));
    // When
    let imported = import_sheet_internal(incoming);
    // Then: stripping the configured prefix cannot allow previous cross-sheet aliases back in.
    assert_eq!(imported, Ok(()));
    assert_eq!(with_style_sheet(|sheet| sheet.names.len()), 0);
    assert_eq!(with_style_sheet(|sheet| sheet.properties.len()), 0);
    assert_eq!(compiled_content(false).code(), cold_code);
    assert_eq!(
        with_style_sheet(|sheet| sheet.create_css(Some("x.tsx"), false)),
        cold_css
    );
    reset_build_state_internal();
}
