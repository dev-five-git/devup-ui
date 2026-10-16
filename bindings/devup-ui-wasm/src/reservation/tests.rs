use std::collections::{BTreeMap, HashMap, HashSet};
use std::error::Error;

use super::*;
use crate::{
    code_extract_internal, code_extract_with_modules_internal, export_class_map_internal,
    export_file_map_internal, export_sheet_internal, register_theme_internal,
    reset_build_state_internal, seed_file_map, with_style_sheet,
};
use serial_test::serial;

pub(super) type TestResult = Result<(), Box<dyn Error>>;

pub(super) struct Reset;
impl Drop for Reset {
    fn drop(&mut self) {
        reset_build_state_internal();
        register_theme_internal(sheet::theme::Theme::default());
        css::set_custom_shorthands(BTreeMap::new());
        css::debug::set_debug(false);
    }
}

pub(super) fn setup(single_css: bool) -> (Reset, ExtractOption) {
    reset_build_state_internal();
    css::debug::set_debug(false);
    register_theme_internal(sheet::theme::Theme::default());
    (
        Reset,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    )
}

pub(super) fn input(filename: &str, code: &str) -> ReservationInput {
    ReservationInput {
        filename: filename.into(),
        code: code.into(),
        css_dir: "./css".into(),
    }
}

pub(super) fn transform(
    input: &ReservationInput,
    option: &ExtractOption,
) -> Result<crate::Output, String> {
    code_extract_internal(
        &input.filename,
        &input.code,
        &option.package,
        input.css_dir.clone(),
        option.single_css,
        option.import_main_css,
        true,
        option.import_aliases.clone(),
    )
}

const A: &str = "import {Box,keyframes,globalCss} from '@devup-ui/react'; const spin=keyframes({from:{opacity:0},to:{opacity:1}}); globalCss({body:{margin:0}}); export const A=(value:string)=><Box color='red' gap={value} styleOrder={0} animation={`${spin} 1s`} />;";
const B: &str = "import {Box,keyframes} from '@devup-ui/react'; const fade=keyframes({from:{opacity:1},to:{opacity:0}}); export const B=(value:string)=><Box bg='blue' w={value} animation={`${fade} 2s`} />;";

#[derive(Debug, PartialEq, Eq)]
struct Built {
    sources: BTreeMap<String, (String, Option<String>)>,
    css: Vec<String>,
    slots: String,
    files: String,
}

fn build(order: [usize; 2], single: bool, hoist: bool) -> Result<Built, Box<dyn Error>> {
    let (_reset, option) = setup(single);
    file_map::set_canonical_map(HashMap::from([
        ("a.tsx".into(), "bucket.tsx".into()),
        ("b.tsx".into(), "bucket.tsx".into()),
    ]));
    if hoist {
        css::atom_hoist::set_atom_hoist(Some(2));
        css::file_routes::set_file_routes(HashMap::from([(
            "bucket.tsx".into(),
            HashSet::from([0, 1]),
        )]));
    }
    seed_file_map(vec!["a.tsx".into(), "b.tsx".into()]);
    let inputs = [input("a.tsx", A), input("b.tsx", B)];
    reserve_class_slots_internal(&inputs, option.clone(), None);
    assert_eq!(get_unreserved_class_slot_count(), 0);
    let mut sources = BTreeMap::new();
    for index in order {
        let output = transform(&inputs[index], &option)?;
        assert!(!output.code.contains("<Box"));
        sources.insert(inputs[index].filename.clone(), (output.code, output.map));
    }
    assert_eq!(get_unreserved_class_slot_count(), 0);
    let css = with_style_sheet(|sheet| {
        vec![
            sheet.create_css(None, false),
            sheet.create_css(Some("bucket.tsx"), false),
        ]
    });
    assert!(css.iter().any(|css| css.contains("@keyframes")));
    assert!(css.iter().any(|css| css.contains("var(--")));
    Ok(Built {
        sources,
        css,
        slots: export_class_map_internal()?,
        files: export_file_map_internal()?,
    })
}

#[test]
#[serial]
fn reserved_real_outputs_match_when_callbacks_reverse() -> TestResult {
    // Given / When / Then: same physical reservation, opposite real extraction order.
    for (single, hoist) in [(true, false), (false, false), (false, true)] {
        assert_eq!(build([0, 1], single, hoist)?, build([1, 0], single, hoist)?);
    }
    Ok(())
}

#[test]
#[serial]
fn reservation_preserves_nonempty_sheet_theme_and_unchanged_registries() -> TestResult {
    // Given
    let (_reset, option) = setup(true);
    let mut theme: sheet::theme::Theme = serde_json::from_value(
        serde_json::json!({"colors":{"default":{"brand":"red"}},"length":{"default":{"size":["8px",null,"12px"]}},"shadow":{"default":{"card":["0 0 1px red",null,"0 0 2px red"]}},"typography":{"heading":{"fontSize":"20px"}}}),
    )?;
    theme.tailwind_css = "@theme { --color-custom: #123; }".into();
    register_theme_internal(theme);
    css::set_custom_shorthands(BTreeMap::from([("customGap".into(), vec!["gap".into()])]));
    css::set_prefix(Some("du-".into()));
    transform(&input("live.tsx", A), &option)?;
    with_style_sheet_mut(|sheet| {
        sheet.add_import("live.tsx", "url(font.css)");
        sheet.add_font_face(
            "live.tsx",
            &BTreeMap::from([("font-family".into(), "Existing".into())]),
        );
        sheet.add_css("live.tsx", "body { color: red; }");
    });
    let before = export_sheet_internal()?;
    let theme = with_style_sheet(|sheet| serde_json::to_string(&sheet.theme))?;
    let mut files = file_map::get_file_map();
    files.insert("unused.tsx".into(), files.len());
    let canonical = file_map::get_canonical_map();
    let routes = css::file_routes::get_file_routes();
    let registry = css::theme_tokens::get_responsive_theme_token("$size");
    // When
    reserve_class_slots_internal(
        &[input(
            "unused.tsx",
            "import {Box} from '@devup-ui/react'; <Box w='$size' boxShadow='$card' typography='heading' customGap={3} />;",
        )],
        option,
        None,
    );
    // Then
    assert_eq!(export_sheet_internal()?, before);
    assert_eq!(
        with_style_sheet(|sheet| serde_json::to_string(&sheet.theme))?,
        theme
    );
    assert_eq!(file_map::get_file_map(), files);
    assert_eq!(file_map::get_canonical_map(), canonical);
    assert_eq!(css::file_routes::get_file_routes(), routes);
    assert_eq!(
        css::theme_tokens::get_responsive_theme_token("$size"),
        registry
    );
    assert_eq!(css::theme_tokens::get_typography_keys(), vec!["heading"]);
    assert!(css::theme_tokens::is_color_token("brand"));
    assert_eq!(
        css::tailwind_definitions::theme_value("color", "custom"),
        Some("#123".into())
    );
    assert_eq!(css::get_custom_shorthand_names(), vec!["customGap"]);
    assert_eq!(css::get_prefix(), Some("du-".into()));
    assert!(!css::debug::is_debug());
    assert_eq!(css::atom_hoist::atom_hoist_threshold(), None);
    assert_eq!(get_unreserved_class_slot_count(), 0);
    let token_input = input(
        "unused.tsx",
        "import {Box} from '@devup-ui/react'; <Box w='$size' boxShadow='$card' typography='heading' customGap={3} />;",
    );
    transform(
        &token_input,
        &ExtractOption {
            single_css: true,
            ..ExtractOption::default()
        },
    )?;
    assert_eq!(get_unreserved_class_slot_count(), 0);
    Ok(())
}

#[test]
#[serial]
fn invalid_inputs_do_not_emit_or_stop_valid_suffix_and_real_error_stays() -> TestResult {
    // Given
    let (_reset, option) = setup(true);
    let bad = input(
        "bad.tsx",
        "import {css} from '@devup-ui/react'; const value=props.value; css({ color: value });",
    );
    let error = transform(&bad, &option)
        .err()
        .ok_or("expected located build error")?;
    let before = export_sheet_internal()?;
    let valid = input(
        "valid.tsx",
        "import {Box} from '@devup-ui/react'; <Box bg='blue' />;",
    );
    // When
    reserve_class_slots_internal(&[input("invalid.ext", A), bad, valid], option.clone(), None);
    // Then
    assert_eq!(export_sheet_internal()?, before);
    let output = transform(
        &input(
            "valid.tsx",
            "import {Box} from '@devup-ui/react'; <Box bg='blue' />;",
        ),
        &option,
    )?;
    assert!(output.css.is_some());
    assert_eq!(get_unreserved_class_slot_count(), 0);
    assert_eq!(
        transform(
            &input(
                "bad.tsx",
                "import {css} from '@devup-ui/react'; const value=props.value; css({ color: value });"
            ),
            &option
        )
        .err(),
        Some(error)
    );
    Ok(())
}

#[test]
#[serial]
fn residual_names_repeat_and_watch_reservation_keeps_cumulative_count() -> TestResult {
    // Given
    let (_reset, option) = setup(true);
    reserve_class_slots_internal(&[], option.clone(), None);
    let generated = input(
        "virtual.tsx",
        "import {Box} from '@devup-ui/react'; <Box gap={value} />;",
    );
    // When
    let first = transform(&generated, &option)?;
    let count = get_unreserved_class_slot_count();
    let repeat = transform(&generated, &option)?;
    reserve_class_slots_internal(
        &[input(
            "watch.tsx",
            "import {Box} from '@devup-ui/react'; <Box color='red' />;",
        )],
        option.clone(),
        None,
    );
    // Then
    assert!(count > 0);
    assert_eq!(get_unreserved_class_slot_count(), count);
    assert_eq!(first.code, repeat.code);
    assert!(first.code.contains("className=\"a\"") && first.code.contains("--b"));
    reset_build_state_internal();
    let legacy = transform(&generated, &option)?;
    assert_eq!(legacy.code, first.code);
    assert_eq!(get_unreserved_class_slot_count(), 0);
    Ok(())
}

#[test]
#[serial]
fn unwind_restores_sheet_files_and_allocator_after_nested_resolver() -> TestResult {
    // Given
    let (_reset, option) = setup(false);
    transform(&input("live.tsx", A), &option)?;
    let before = (
        export_sheet_internal()?,
        export_class_map_internal()?,
        export_file_map_internal()?,
    );
    let resolver =
        |_: &str, _: &str| -> Option<extractor::ResolvedModule> { panic!("resolver unwind") };
    // When
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        reserve_class_slots_internal(
            &[
                input("first.tsx", B),
                input(
                    "root.css.ts",
                    "import {style} from '@devup-ui/react'; import {base} from './supplier.css'; export const own=style(base);",
                ),
            ],
            option,
            Some(&resolver),
        );
    }));
    // Then
    assert!(result.is_err());
    assert_eq!(
        (
            export_sheet_internal()?,
            export_class_map_internal()?,
            export_file_map_internal()?
        ),
        before
    );
    assert_eq!(get_unreserved_class_slot_count(), 0);
    Ok(())
}

#[test]
#[serial]
fn nested_supplier_reservation_is_reused_by_two_roots() -> TestResult {
    // Given
    let (_reset, option) = setup(false);
    seed_file_map(vec![
        "supplier.css.ts".into(),
        "a.css.ts".into(),
        "b.css.ts".into(),
    ]);
    let resolver = |specifier: &str, _: &str| {
        (specifier == "./supplier.css").then(|| extractor::ResolvedModule {
            path: "supplier.css.ts".into(),
            code: "import {style} from '@devup-ui/react'; export const base=style({color:'red'});"
                .into(),
        })
    };
    let source = "import {style} from '@devup-ui/react'; import {base} from './supplier.css'; export const own=style([base,{width:'12px'}]);";
    let inputs = [input("a.css.ts", source), input("b.css.ts", source)];
    // When
    reserve_class_slots_internal(&inputs, option.clone(), Some(&resolver));
    for input in inputs.iter().rev() {
        let output = code_extract_with_modules_internal(
            &input.filename,
            &input.code,
            &option.package,
            input.css_dir.clone(),
            false,
            false,
            true,
            HashMap::new(),
            &resolver,
        )?;
        assert!(output.code.contains("export const own"));
    }
    // Then
    assert_eq!(get_unreserved_class_slot_count(), 0);
    Ok(())
}

#[test]
#[serial]
fn resolved_token_keys_are_reserved_before_real_output() -> TestResult {
    // Given
    let (_reset, option) = setup(true);
    register_theme_internal(serde_json::from_value(
        serde_json::json!({"length":{"default":{"size":["8px",null,"12px"]}},"shadow":{"default":{"card":["0 0 1px red",null,"0 0 2px red"]}}}),
    )?);
    let token = input(
        "tokens.tsx",
        "import {Box} from '@devup-ui/react'; <Box w={['$size']} boxShadow={['$card']} />;",
    );
    // When
    reserve_class_slots_internal(&[token], option.clone(), None);
    // Then
    let reserved_slots: usize = css::class_map::get_class_map()
        .values()
        .map(HashMap::len)
        .sum();
    assert_eq!(reserved_slots, 2);
    let output = transform(
        &input(
            "tokens.tsx",
            "import {Box} from '@devup-ui/react'; <Box w={['$size']} boxShadow={['$card']} />;",
        ),
        &option,
    )?;
    let css = output.css.ok_or("expected token CSS")?;
    assert!(css.contains("width:8px"));
    assert!(css.contains("box-shadow:0 0 1px red"));
    assert!(!css.contains("$size"));
    assert!(!css.contains("$card"));
    assert_eq!(get_unreserved_class_slot_count(), 0);
    Ok(())
}

#[test]
#[serial]
fn debug_configuration_stays_unchanged_when_reserving() -> TestResult {
    // Given
    let (_reset, option) = setup(true);
    css::debug::set_debug(true);
    let before = css::class_map::get_class_map();
    // When
    reserve_class_slots_internal(
        &[input("z.tsx", B), input("a.tsx", A)],
        option.clone(),
        None,
    );
    // Then
    assert!(css::debug::is_debug());
    assert_eq!(css::class_map::get_class_map(), before);
    assert_eq!(get_unreserved_class_slot_count(), 0);
    assert!(
        transform(&input("a.tsx", A), &option)?
            .code
            .contains("color-0-red")
    );
    Ok(())
}

#[test]
#[serial]
fn supplied_physical_order_is_not_resorted_by_engine_id() -> TestResult {
    // Given
    let (_reset, option) = setup(true);
    let red = input(
        "z.tsx",
        "import {Box} from '@devup-ui/react'; <Box color='red' />;",
    );
    let blue = input(
        "a.tsx",
        "import {Box} from '@devup-ui/react'; <Box color='blue' />;",
    );
    // When
    reserve_class_slots_internal(&[red, blue], option.clone(), None);
    // Then
    assert!(
        transform(
            &input(
                "z.tsx",
                "import {Box} from '@devup-ui/react'; <Box color='red' />;"
            ),
            &option
        )?
        .code
        .contains("className=\"a\"")
    );
    assert!(
        transform(
            &input(
                "a.tsx",
                "import {Box} from '@devup-ui/react'; <Box color='blue' />;"
            ),
            &option
        )?
        .code
        .contains("className=\"b\"")
    );
    assert_eq!(get_unreserved_class_slot_count(), 0);
    Ok(())
}
