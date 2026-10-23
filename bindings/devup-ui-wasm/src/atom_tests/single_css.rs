use super::*;

const RED: &str = "import {Box} from '@devup-ui/react'; export const x=<Box color=\"red\"/>;";

fn build(atom: bool, debug: bool) -> (String, String, String) {
    setup();
    css::atom_hoist::set_atom_hoist(atom.then_some(2));
    css::debug::set_debug(debug);
    set_prefix(Some("du-".into()));
    let option = ExtractOption {
        single_css: true,
        ..ExtractOption::default()
    };
    reserve_class_slots_internal(
        &[ReservationInput {
            filename: "a.tsx".into(),
            code: SOURCE_A.into(),
            css_dir: "df".into(),
            source_type: None,
        }],
        option,
        None,
    );
    let output = code_extract_internal(
        "a.tsx",
        SOURCE_A,
        "@devup-ui/react",
        "df".into(),
        true,
        false,
        false,
        HashMap::new(),
        None,
    )
    .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(get_unreserved_class_slot_count(), 0);
    (
        output.code,
        with_style_sheet(|sheet| sheet.create_css(None, false)),
        export_sheet_internal().unwrap_or_else(|error| panic!("{error}")),
    )
}

#[test]
#[serial]
fn w66_single_css_output_is_exactly_invariant_to_requested_atom_mode() {
    let _scope = TestStateGuard::new();
    // Given / When / Then
    for debug in [false, true] {
        assert_eq!(build(false, debug), build(true, debug));
        assert_eq!(css::atom_hoist::atom_plan(), None);
    }
    reset_build_state_internal();
    css::debug::set_debug(false);
}

#[test]
#[serial]
fn w66_single_css_does_not_freeze_empty_reach_before_per_file_reservation() {
    let _scope = TestStateGuard::new();
    // Given
    setup();
    css::file_routes::reset_file_routes();
    let inputs = [ReservationInput {
        filename: "a.tsx".into(),
        code: RED.into(),
        css_dir: "df".into(),
        source_type: None,
    }];
    // When
    reserve_class_slots_internal(
        &inputs,
        ExtractOption {
            single_css: true,
            ..ExtractOption::default()
        },
        None,
    );
    // Then
    assert_eq!(css::atom_hoist::atom_plan(), None);
    css::file_routes::set_file_routes(HashMap::from([("a.tsx".into(), HashSet::from([0, 1]))]));
    reserve_class_slots_internal(&inputs, ExtractOption::default(), None);
    let output = compile("a.tsx", RED);
    assert!(output.code.contains("className=\""));
    assert_eq!(get_unreserved_class_slot_count(), 0);
    assert!(with_style_sheet(|sheet| sheet.create_css(None, false)).contains("color:red"));
    assert!(
        !with_style_sheet(|sheet| sheet.create_css(Some("a.tsx"), false)).contains("color:red")
    );
    reset_build_state_internal();
}

#[test]
#[serial]
fn single_css_error_and_resolver_unwind_restore_effective_atom_mode() {
    let _scope = TestStateGuard::new();
    // Given
    setup();
    let option = ExtractOption {
        single_css: true,
        ..ExtractOption::default()
    };
    let bad = concat!(
        "import {css} from '@devup-ui/react'; const value=props.value; css({",
        "color:value});"
    );
    // When
    let error = match extractor::extract_without_source_map("bad.tsx", bad, option.clone()) {
        Err(error) => error,
        Ok(_) => panic!("located extraction error"),
    };
    // Then
    assert!(error.to_string().contains("bad.tsx:"));
    assert!(css::atom_hoist::is_atom_hoist());
    assert_eq!(css::atom_hoist::atom_plan(), None);
    let resolver =
        |_: &str, _: &str| -> Option<extractor::ModuleResolution> { panic!("resolver unwind") };
    let unwind = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        reserve_class_slots_internal(&[ReservationInput {
            filename: "root.css.ts".into(),
            code: "import {style} from '@devup-ui/react'; import {base} from './supplier.css'; export const own=style(base);".into(),
            css_dir: "df".into(),
            source_type: None,
        }], option, Some(&resolver));
    }));
    assert!(unwind.is_err());
    assert!(css::atom_hoist::is_atom_hoist());
    assert_eq!(css::atom_hoist::atom_plan(), None);
    reset_build_state_internal();
}

#[test]
#[serial]
fn canonical_global_placement_keeps_atom_mode_in_a_per_file_build() {
    let _scope = TestStateGuard::new();
    // Given
    setup();
    import_canonical_map_internal(HashMap::from([("global.tsx".into(), "@global".into())]));
    // When
    let output = compile("global.tsx", RED);
    // Then
    assert!(output.code.contains("className=\"a\""), "{}", output.code);
    let css = with_style_sheet(|sheet| sheet.create_css(None, false));
    assert_references_resolve(&output.code, &css);
    assert!(with_style_sheet(|sheet| sheet.atom_plan.is_some()));
    assert!(css::atom_hoist::is_atom_hoist());
    reset_build_state_internal();
}
