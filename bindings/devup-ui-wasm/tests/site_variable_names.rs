#![cfg(not(target_arch = "wasm32"))]

use std::collections::{BTreeMap, BTreeSet};

use extractor::extract_style::extract_style_value::ExtractStyleValue;
use extractor::{ExtractOption, ResolvedModule, extract, extract_with_modules};
use rstest::rstest;
use serial_test::serial;
use sheet::StyleSheet;

const FILE: &str = "/src/shared.tsx";
const IMPORTED_SELECTOR: &str = "import {Box} from '@devup-ui/react';import {SEL} from 'conditional';export const View=({pad})=><Box selectors={{[SEL]:{p:pad}}}/>;";

fn reset(debug: bool, atom: bool) {
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    css::file_map::reset_canonical_map();
    css::file_map::seed_file_numbers(&[FILE.to_string()]);
    css::debug::set_debug(debug);
    css::atom_hoist::set_atom_hoist(atom.then_some(1));
    css::atom_hoist::restore_atom_plan(None);
    css::set_prefix(Some("test-".to_string()));
}

fn imported_order(
    order: [&'static str; 2],
    mode: (bool, bool),
) -> (BTreeMap<String, String>, String) {
    reset(mode.0, mode.1);
    let mut sheet = StyleSheet::default();
    let mut code = BTreeMap::new();
    let mut variables = BTreeSet::new();
    for selector in order {
        let resolver = move |_: &str, _: &str| {
            Some(ResolvedModule {
                path: format!("/pkg/{}.ts", selector.trim_start_matches("&:")),
                code: format!("export const SEL='{selector}';"),
            })
        };
        let output = extract_with_modules(
            FILE,
            IMPORTED_SELECTOR,
            ExtractOption::default(),
            false,
            &resolver,
        )
        .unwrap_or_else(|error| panic!("imported selector extraction failed: {error}"));
        for style in &output.styles {
            if let ExtractStyleValue::Dynamic(style) = style {
                variables.insert(style.variable_name());
            }
        }
        sheet
            .update_styles(&output.styles, FILE, false)
            .unwrap_or_else(|error| panic!("{error}"));
        code.insert(selector.to_string(), output.code);
    }
    assert_eq!(
        variables.len(),
        1,
        "one original assignment must share one variable"
    );
    let css = sheet.create_css(Some(FILE), false);
    assert!(css.contains(":hover"), "missing hover rule: {css}");
    assert!(css.contains(":focus"), "missing focus rule: {css}");
    (code, css)
}

#[rstest]
#[case(false, false)]
#[case(true, false)]
#[case(false, true)]
#[case(true, true)]
#[serial]
fn imported_selector_bytes_match_when_environment_order_reverses(
    #[case] debug: bool,
    #[case] atom: bool,
) {
    // Given: the same source assignment, with an environment-dependent import.
    let mode = (debug, atom);
    // When: both environments extract into the real sheet in opposite orders.
    let ab = imported_order(["&:hover", "&:focus"], mode);
    let ba = imported_order(["&:focus", "&:hover"], mode);
    // Then: every byte of both emitted JS modules and CSS agrees.
    assert_eq!(ab, ba);
}

#[rstest]
#[case(
    "import {Box} from '@devup-ui/react';export const View=({pad,gap,left,right})=><Box px={pad} py={gap} m={[left,right]} _hover={{p:pad}} _focus={{p:gap}}/>;",
    6
)]
#[case(
    "import {Box} from '@devup-ui/react';export const View=({pad})=><Box p={pad} m={pad}/>;",
    2
)]
#[case(
    "import * as stylex from '@stylexjs/stylex';const styles=stylex.create({dynamic:(width,height)=>({width,height,paddingLeft:width,paddingRight:height})});export const View=({width,height})=><div {...stylex.props(styles.dynamic(width,height))}/>;",
    4
)]
#[case(
    "import styled from '@emotion/styled';export const View=styled.div`padding: ${p=>p.pad}; margin: ${p=>p.gap};`;",
    2
)]
#[case(
    "import styled from '@emotion/styled';export const View=styled.div`padding: ${p=>p.pad}px; margin: ${p=>p.gap}em;`;",
    2
)]
#[serial]
fn different_assignments_remain_distinct_when_only_site_names_variables(
    #[case] source: &str,
    #[case] minimum_styles: usize,
) {
    for mode in [(false, false), (true, false), (false, true), (true, true)] {
        // Given: real shorthand, responsive, multi-parameter or template syntax.
        reset(mode.0, mode.1);
        // When: the native extractor and sheet compile it.
        let mut option = ExtractOption::default();
        option.import_aliases.insert(
            "@emotion/styled".to_string(),
            extractor::ImportAlias::DefaultToNamed("styled".to_string()),
        );
        let output = extract(FILE, source, option)
            .unwrap_or_else(|error| panic!("dynamic fixture extraction failed: {error}"));
        let mut sheet = StyleSheet::default();
        sheet
            .update_styles(&output.styles, FILE, false)
            .unwrap_or_else(|error| panic!("{error}"));
        let css = sheet.create_css(Some(FILE), false);
        let mut assignments = BTreeMap::new();
        let mut count = 0;
        for style in &output.styles {
            if let ExtractStyleValue::Dynamic(style) = style {
                count += 1;
                let variable = style.variable_name();
                assert!(variable.starts_with("---test-S"));
                assert!(
                    output.code.contains(&variable),
                    "missing JS assignment: {}",
                    output.code
                );
                assert!(
                    css.contains(&format!("var({variable})")),
                    "missing CSS use: {css}"
                );
                // Then: sharing is legal only for identical assignment values.
                if let Some(previous) = assignments.insert(variable.clone(), style.identifier()) {
                    assert_eq!(
                        previous,
                        style.identifier(),
                        "different values alias {variable}: {}",
                        output.code
                    );
                }
            }
        }
        assert!(
            count >= minimum_styles,
            "fixture did not exercise its dynamic styles: {}",
            output.code
        );
        assert!(
            assignments.len() >= 2,
            "fixture must preserve distinct values"
        );
    }
}

#[path = "site_variable_names/sparse.rs"]
mod sparse;
