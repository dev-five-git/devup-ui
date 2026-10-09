use super::*;
use exact_test_support::{
    authority, compile, configure_fresh_plan, exact, fixture, mutate_authority,
};
use rstest::rstest;
use serial_test::serial;
use std::panic::{AssertUnwindSafe, catch_unwind};

#[rstest]
#[case(
    "broken.tsx",
    "import {Box} from '@devup-ui/react'; const x=<Box",
    "Parser panicked"
)]
#[case(
    "visitor.tsx",
    "import {css} from '@devup-ui/react';export const x=css(runtime());",
    "build time"
)]
#[serial]
fn failure_restores_authority_before_and_after_visitor_attempt(
    #[case] file: &str,
    #[case] source: &str,
    #[case] expected: &str,
) {
    // Given
    fixture();
    let before = authority();
    // When
    let error = compile(file, source)
        .err()
        .unwrap_or_else(|| panic!("failure fixture unexpectedly compiled"));
    // Then
    assert!(error.contains(expected), "{error}");
    assert_eq!(authority(), before);
    assert_eq!(cache_names::check(), Ok(()));
    assert_eq!(css::file_map::original_id(file), None);
    reset_build_state_internal();
}

#[test]
#[serial]
fn imported_stylesheet_commit_is_subordinate_to_failed_root_extraction() {
    // Given
    fixture();
    let before = authority();
    let resolver = |specifier: &str, _: &str| {
        (specifier == "./styles.css").then(|| ResolvedModule {
        path: "foreign.css.ts".into(),
        code: "import {style,keyframes} from '@devup-ui/react';export const k=keyframes({from:{opacity:0},to:{opacity:1}});export const c=style({animationName:k});".into(),
    })
    };
    // When
    let result = code_extract_with_modules_internal(
        "root.tsx",
        "import {css} from '@devup-ui/react';import {c} from './styles.css';export const x=css(c,runtime());",
        "@devup-ui/react",
        "df".into(),
        false,
        false,
        false,
        HashMap::new(),
        &resolver,
    );
    // Then
    assert!(result.is_err());
    assert_eq!(authority(), before);
    reset_build_state_internal();
}

#[test]
#[serial]
fn output_collision_restores_committed_inner_and_keeps_sticky_error() {
    // Given
    fixture();
    seed_file_map(vec!["raw.tsx".into()]);
    let red_order_zero = css::content_name::AtomContent {
        property: "color",
        value: Some("red"),
        naming: css::Naming::Own,
        level: 0,
        order: 0,
        selector: None,
        layer: None,
        dynamic: false,
    }
    .content()
    .name("");
    with_style_sheet_mut(|sheet| {
        let previous = sheet.names["OLcolor-vblue"].clone();
        sheet.names.insert(red_order_zero, previous);
    });
    let before = authority();
    let source = "import {Box,css,keyframes} from '@devup-ui/react';export const k=keyframes({from:{opacity:0},to:{opacity:1}});export const d=<Box w='12px'/>;export const x=css({color:'red',styleOrder:0});";
    let classes = css::class_map::get_class_map();
    let original = css::file_map::original_id("raw.tsx")
        .unwrap_or_else(|| panic!("seeded raw.tsx original missing"));
    // The real extractor commits a private reservation before Output preflight.
    let control: Result<(), String> = exact(|| {
        extract_without_source_map(
            "raw.tsx",
            source,
            ExtractOption {
                package: "@devup-ui/react".into(),
                css_dir: "df".into(),
                single_css: false,
                import_main_css: false,
                import_aliases: HashMap::new(),
            },
        )
        .unwrap_or_else(|error| panic!("private allocation control failed: {error}"));
        let mut allocated = css::class_map::get_class_map();
        let private = allocated
            .remove(&format!("D9-{original}"))
            .unwrap_or_else(|| panic!("private allocation control namespace missing"));
        assert_eq!(private.values().copied().collect::<Vec<_>>(), vec![0]);
        assert_eq!(allocated, classes);
        Err("control abort".into())
    });
    assert_eq!(control, Err("control abort".into()));
    assert_eq!(authority(), before);
    // When
    let error = code_extract_internal(
        "raw.tsx",
        source,
        "@devup-ui/react",
        "df".into(),
        false,
        false,
        false,
        HashMap::new(),
    )
    .err()
    .unwrap_or_else(|| panic!("Output collision fixture unexpectedly compiled"));
    // Then
    assert_eq!(authority(), before);
    assert_eq!(cache_names::check(), Err(error.clone()));
    assert_eq!(get_css_internal(None, false), Err(error.clone()));
    assert_eq!(compile("later.tsx", "const x=1;").err(), Some(error));
    reset_build_state_internal();
}

#[rstest]
#[case("prepare error")]
#[case("output error")]
#[serial]
fn simulated_failure_restores_full_live_records_after_inner_commit(#[case] failure: &str) {
    // Given
    fixture();
    configure_fresh_plan();
    assert_eq!(css::atom_hoist::atom_plan(), None);
    let before = authority();
    // When
    let result: Result<(), String> = exact(|| {
        mutate_authority();
        assert_eq!(
            css::atom_hoist::atom_plan(),
            Some(std::collections::BTreeSet::from(["tentative.tsx".into()]))
        );
        Err(failure.into())
    });
    // Then
    assert_eq!(result, Err(failure.into()));
    assert_eq!(authority(), before);
    reset_build_state_internal();
}

#[test]
#[serial]
fn unwind_restores_full_live_records_after_sheet_cleanup_and_update() {
    // Given
    fixture();
    configure_fresh_plan();
    assert_eq!(css::atom_hoist::atom_plan(), None);
    let before = authority();
    // When
    let panic = catch_unwind(AssertUnwindSafe(|| {
        exact::<()>(|| {
            mutate_authority();
            assert_eq!(
                css::atom_hoist::atom_plan(),
                Some(std::collections::BTreeSet::from(["tentative.tsx".into()]))
            );
            panic!("emission unwind");
        })
    }));
    // Then
    assert!(panic.is_err());
    assert_eq!(authority(), before);
    assert_eq!(cache_names::check(), Ok(()));
    reset_build_state_internal();
}

#[test]
#[serial]
fn resolver_unwind_restores_real_common_extraction_boundary() {
    // Given
    fixture();
    let before = authority();
    let resolver = |_: &str, _: &str| -> Option<ResolvedModule> { panic!("resolver unwind") };
    // When
    let panic = catch_unwind(AssertUnwindSafe(|| {
        code_extract_with_modules_internal(
            "root.tsx",
            "import {Box} from '@devup-ui/react';import {COLOR} from './tokens';export const x=<Box color={COLOR}/>;",
            "@devup-ui/react",
            "df".into(),
            false,
            false,
            false,
            HashMap::new(),
            &resolver,
        )
    }));
    // Then
    assert!(panic.is_err());
    assert_eq!(authority(), before);
    reset_build_state_internal();
}

#[test]
#[serial]
fn evaluated_visitor_retry_commits_output_without_registering_unseeded_original() {
    // Given
    reset_build_state_internal();
    set_debug(false);
    // When
    let output = compile("retry.tsx", "import {css} from '@devup-ui/react';function a(){return 'red'}const b=()=>a();export const c=css({color:b(),padding:'16px'});")
        .unwrap_or_else(|error| panic!("evaluated visitor retry failed: {error}"));
    // Then
    assert!(
        output
            .css()
            .unwrap_or_else(|| panic!("retry CSS missing"))
            .contains("color:red")
    );
    assert!(with_style_sheet(|sheet| sheet
        .names
        .values()
        .any(|claim| claim.content.contains("red"))));
    assert_eq!(css::file_map::original_id("retry.tsx"), None);
    reset_build_state_internal();
}
