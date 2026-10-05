use super::*;
use serial_test::serial;

fn compile(file: &str, source: &str) -> Result<Output, String> {
    code_extract_internal(
        file,
        source,
        "@devup-ui/react",
        "df".into(),
        true,
        false,
        false,
        HashMap::new(),
    )
}

#[test]
#[serial]
fn public_extraction_uses_fixed_short_and_first80_descriptor_goldens() {
    // Given
    reset_build_state_internal();
    css::debug::set_debug(false);
    let source = "import {css} from '@devup-ui/react'; export const x=css({color:'red',fontFamily:'abcdefghijklmnopqrstuvwx'});";
    let mut bytes = vec![1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 11];
    bytes.extend_from_slice(b"font-family");
    bytes.extend_from_slice(&[1, 0, 0, 0, 0, 0, 0, 0, 24]);
    bytes.extend_from_slice(b"abcdefghijklmnopqrstuvwx");
    bytes.extend_from_slice(&[0, 255, 0, 0]);
    // When
    let output = compile("golden.tsx", source).unwrap_or_else(|error| panic!("{error}"));
    // Then
    let expected = format!("OH{}", css::content_hash::fingerprint(&bytes));
    assert!(output.code().contains("OLcolor-vred"), "{}", output.code());
    assert!(output.code().contains(&expected), "{}", output.code());
    with_style_sheet(|sheet| assert_eq!(sheet.names[&expected].descriptor, bytes));
    reset_build_state_internal();
}

#[test]
#[serial]
fn restored_forced_collision_reports_both_actual_sites_before_global_css_removal() {
    // Given
    reset_build_state_internal();
    css::debug::set_debug(false);
    compile(
        "a.tsx",
        "import {css} from '@devup-ui/react';\nexport const x=css({color:'blue'});",
    )
    .unwrap_or_else(|error| panic!("{error}"));
    compile(
        "b.tsx",
        "import {globalCss} from '@devup-ui/react'; globalCss({body:{padding:'11px'}});",
    )
    .unwrap_or_else(|error| panic!("{error}"));
    let serialized = export_sheet_internal().unwrap_or_else(|error| panic!("{error}"));
    let mut forced: StyleSheet =
        serde_json::from_str(&serialized).unwrap_or_else(|error| panic!("{error}"));
    let original = forced.names["OLcolor-vblue"].clone();
    forced.names.insert("OLcolor-vred".into(), original);
    import_sheet_internal(forced).unwrap_or_else(|error| panic!("{error}"));
    let before = export_sheet_internal().unwrap_or_else(|error| panic!("{error}"));
    let css_before = with_style_sheet(|sheet| sheet.create_css(None, false));
    // When
    let error = compile(
        "b.tsx",
        "import {css} from '@devup-ui/react';\nexport const x=css({color:'red'});",
    )
    .err()
    .unwrap_or_else(|| panic!("forced collision was accepted"));
    // Then
    assert!(error.contains("a.tsx:2:27:"), "{error}");
    assert!(error.contains("b.tsx:2:27:"), "{error}");
    assert!(error.contains("`'blue'`"), "{error}");
    assert!(error.contains("`'red'`"), "{error}");
    assert_eq!(
        export_sheet_internal().unwrap_or_else(|error| panic!("{error}")),
        before
    );
    assert_eq!(
        with_style_sheet(|sheet| sheet.create_css(None, false)),
        css_before
    );
    reset_build_state_internal();
}

#[test]
#[serial]
fn sheet_import_merges_exact_claims_and_rejects_unequal_claims_atomically() {
    // Given
    reset_build_state_internal();
    css::debug::set_debug(false);
    compile(
        "b.tsx",
        "import {css} from '@devup-ui/react'; export const x=css({color:'red'});",
    )
    .unwrap_or_else(|error| panic!("{error}"));
    let before = export_sheet_internal().unwrap_or_else(|error| panic!("{error}"));
    let original: StyleSheet =
        serde_json::from_str(&before).unwrap_or_else(|error| panic!("{error}"));
    let mut incoming = StyleSheet {
        names: original.names,
        ..StyleSheet::default()
    };
    incoming
        .names
        .get_mut("OLcolor-vred")
        .unwrap_or_else(|| panic!("missing claim"))
        .descriptor
        .push(99);
    // When
    let rejected = import_sheet_internal(incoming);
    // Then
    assert!(rejected.is_err());
    assert_eq!(
        export_sheet_internal().unwrap_or_else(|error| panic!("{error}")),
        before
    );
    reset_build_state_internal();
}

#[test]
#[serial]
fn rejected_cache_import_cannot_be_hidden_by_a_plugin_catch() {
    // Given
    reset_build_state_internal();
    css::debug::set_debug(false);
    compile(
        "source.tsx",
        "import {css} from '@devup-ui/react';export const x=css({color:'red'});",
    )
    .unwrap_or_else(|error| panic!("{error}"));
    let snapshot = export_sheet_internal().unwrap_or_else(|error| panic!("{error}"));
    let mut incomplete: StyleSheet =
        serde_json::from_str(&snapshot).unwrap_or_else(|error| panic!("{error}"));
    incomplete.names.clear();
    let rejected = import_sheet_internal(incomplete)
        .err()
        .unwrap_or_else(|| panic!("unprotected cache was imported"));
    // When
    let next = compile(
        "other.tsx",
        "import {css} from '@devup-ui/react';export const x=css({padding:'4px'});",
    );
    // Then
    assert_eq!(next.err(), Some(rejected));
    assert_eq!(
        export_sheet_internal().unwrap_or_else(|error| panic!("{error}")),
        snapshot
    );
    let valid = serde_json::from_str(&snapshot).unwrap_or_else(|error| panic!("{error}"));
    import_sheet_internal(valid).unwrap_or_else(|error| panic!("{error}"));
    assert!(
        compile(
            "other.tsx",
            "import {css} from '@devup-ui/react';export const x=css({padding:'4px'});"
        )
        .is_ok()
    );
    reset_build_state_internal();
}

#[test]
#[serial]
fn evaluated_helper_composition_and_keyframes_keep_factory_call_sites() {
    // Given
    reset_build_state_internal();
    css::debug::set_debug(false);
    let source = "import {style,keyframes} from '@devup-ui/react';\nconst make=()=>style({color:'red'});\nconst base=make();\nexport const combined=style([base,{padding:'4px'}]);\nexport const fade=keyframes({from:{opacity:0},to:{opacity:1}});";
    // When
    let output = compile("actual.css.ts", source).unwrap_or_else(|error| panic!("{error}"));
    // Then
    assert!(
        !output.code().contains("__devup_style_origin"),
        "{}",
        output.code()
    );
    assert!(!output.code().contains("__origin"), "{}", output.code());
    with_style_sheet(|sheet| {
        assert!(
            sheet
                .names
                .values()
                .any(|claim| claim
                    .origin
                    .as_ref()
                    .is_some_and(|origin| origin.file == "actual.css.ts"
                        && origin.line == 2
                        && origin.expression == "style({color:'red'})"))
        );
        assert!(
            sheet
                .names
                .values()
                .any(|claim| claim
                    .origin
                    .as_ref()
                    .is_some_and(|origin| origin.file == "actual.css.ts"
                        && origin.line == 5
                        && origin.expression == "keyframes({from:{opacity:0},to:{opacity:1}})"))
        );
        assert!(sheet.names.values().all(|claim| claim.origin.is_some()));
    });
    reset_build_state_internal();
}

#[test]
#[serial]
fn imported_stylesheet_helpers_keep_the_actual_foreign_factory_site() {
    // Given
    reset_build_state_internal();
    css::debug::set_debug(false);
    let source = "import {style} from '@devup-ui/react';import {make} from './helper';export const result=make();";
    let resolver = |specifier: &str, _: &str| {
        (specifier == "./helper").then(|| ResolvedModule {
        path: "helper.ts".into(),
        code: "import {style} from '@devup-ui/react';\nexport const make=()=>style({color:'red'});".into(),
    })
    };
    // When
    let output = code_extract_with_modules_internal(
        "root.css.ts",
        source,
        "@devup-ui/react",
        "df".into(),
        true,
        false,
        false,
        HashMap::new(),
        &resolver,
    )
    .unwrap_or_else(|error| panic!("{error}"));
    // Then
    assert!(!output.code().contains("__origin"), "{}", output.code());
    with_style_sheet(|sheet| {
        assert!(
            sheet
                .names
                .values()
                .any(|claim| claim
                    .origin
                    .as_ref()
                    .is_some_and(|origin| origin.file == "helper.ts"
                        && origin.line == 2
                        && origin.expression == "style({color:'red'})"))
        );
    });
    reset_build_state_internal();
}

#[test]
#[serial]
fn fingerprint_claim_has_a_real_producer_when_factory_flows_through_a_function() {
    // Given
    reset_build_state_internal();
    css::debug::set_debug(false);
    let source = "import {style} from '@devup-ui/react';const identity=(factory)=>factory;export const cls=identity(style)({fontFamily:'abcdefghijklmnopqrstuvwx'});";
    // When
    compile("higher-order.css.ts", source).unwrap_or_else(|error| panic!("{error}"));
    // Then
    with_style_sheet(|sheet| {
        assert!(sheet.names.values().all(|claim| matches!(&claim.location,
        css::style_origin::RealLocation::ProducedByCall(origin) if origin.file == "higher-order.css.ts"
            && origin.expression == "identity(style)({fontFamily:'abcdefghijklmnopqrstuvwx'})")), "{:?}", sheet.names);
    });
    reset_build_state_internal();
}
