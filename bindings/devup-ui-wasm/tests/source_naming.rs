use std::collections::BTreeMap;

use extractor::{ExtractOption, ResolvedModule, extract_with_modules};
use rstest::rstest;
use serial_test::serial;

type EnvironmentOutputs = BTreeMap<usize, (String, Option<String>, String)>;

fn environment_order(
    source: &str,
    values: [&str; 2],
    order: [usize; 2],
) -> (EnvironmentOutputs, String) {
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    css::file_map::reset_canonical_map();
    css::debug::set_debug(false);
    css::atom_hoist::set_atom_hoist(None);
    css::atom_hoist::restore_atom_plan(None);
    let stylesheet = source.contains("import {style}");
    let file = if stylesheet {
        "/src/shared.css.ts"
    } else {
        "/src/shared.tsx"
    };
    css::file_map::seed_file_numbers(&[file.to_string()]);
    let mut outputs = BTreeMap::new();
    let mut final_sheet = sheet::StyleSheet::default();
    for environment in order {
        let value = values[environment].to_string();
        let resolver = move |specifier: &str, _: &str| {
            Some(ResolvedModule {
                path: if specifier == "./barrel" {
                    "/src/barrel.ts".to_string()
                } else if environment == 0 {
                    "/src/data.ts".to_string()
                } else {
                    "/src/data.client.ts".to_string()
                },
                code: if specifier == "./barrel" {
                    concat!("export {", "value", "} from './data';").to_string()
                } else {
                    format!("export const value={value};")
                },
            })
        };
        let output = extract_with_modules(file, source, ExtractOption::default(), true, &resolver)
            .unwrap_or_else(|error| panic!("{error}"));
        final_sheet
            .update_styles(&output.styles, file, false)
            .unwrap_or_else(|error| panic!("{error}"));
        let mut sheet = sheet::StyleSheet::default();
        sheet
            .update_styles(&output.styles, file, false)
            .unwrap_or_else(|error| panic!("{error}"));
        let css = sheet.create_css(Some(file), false);
        assert!(css.contains("padding:16px"), "{css}");
        if stylesheet {
            assert!(!css.contains(".a-a{"), "{css}");
        } else {
            assert!(css.contains(".a-a{padding:16px}"), "{css}");
            assert!(output.code.contains("a-a"), "{}", output.code);
        }
        let expected_colors: &[&str] = match values[environment] {
            "'red'" | "true" | "['red']" => &["red"],
            "'blue'" | "false" => &["blue"],
            "['red','blue']" => &["red", "blue"],
            value => panic!("unexpected fixture value: {value}"),
        };
        for color in expected_colors {
            let declaration = format!("color:{color}");
            let identities: Vec<_> = css
                .split('}')
                .filter_map(|rule| rule.rsplit_once('{'))
                .filter(|(_, body)| *body == declaration)
                .map(|(selector, _)| {
                    let selector = selector
                        .rsplit_once('{')
                        .map_or(selector, |(_, selector)| selector);
                    selector
                        .rsplit_once("*/")
                        .map_or(selector, |(_, selector)| selector)
                })
                .collect();
            assert_ne!(identities.len(), 0, "{css}");
            for identity in &identities {
                assert_ne!(*identity, ".a-a", "{css}");
                assert!(
                    output.code.contains(identity.trim_start_matches('.')),
                    "{} / {css}",
                    output.code
                );
            }
            if source.contains("export const local") {
                assert_eq!(identities.len(), 2, "{css}");
                assert_ne!(identities[0], identities[1], "{css}");
                assert!(identities.contains(&".a-b"), "{css}");
            }
        }
        assert!(
            output
                .map
                .as_ref()
                .unwrap_or_else(|| panic!("missing original-source map"))
                .contains(&source.replace('"', "\\\""))
        );
        outputs.insert(environment, (output.code, output.map, css));
    }
    (outputs, final_sheet.create_css(Some(file), false))
}

#[rstest]
#[case::relative("import {value} from './data';", "value", ["'red'", "'blue'"])]
#[case::bare("import {value} from 'data';", "value", ["'red'", "'blue'"])]
#[case::reexport("import {value} from './barrel';", "value", ["'red'", "'blue'"])]
#[case::local_chain(
    "import {value} from './data';const a=value;const b=a;",
    "b",
    ["'red'", "'blue'"]
)]
#[case::control(
    "import {value} from './data';",
    "value ? 'red' : 'blue'",
    ["true", "false"]
)]
#[case::responsive_shape(
    "import {value} from './data';",
    "value",
    ["['red']", "['red','blue']"]
)]
#[case::equal_content("import {value} from './data';", "value", ["'red'", "'red'"])]
#[case::functions(
    "import {value} from './data';function get(){return value;}const a=()=>get();",
    "a()",
    ["'red'", "'blue'"]
)]
#[serial]
fn names_when_environment_order_changes(
    #[case] declarations: &str,
    #[case] expression: &str,
    #[case] values: [&str; 2],
) {
    // Given: the same source resolves plain on the server and remapped on the client.
    let local = if values == ["'red'", "'red'"] {
        "export const local=css({color:'red'});"
    } else {
        ""
    };
    let source = format!(
        "import {{css}} from '@devup-ui/react';{declarations}export const cls=css({{color:{expression},padding:'16px'}});{local}"
    );
    // When: both environments compile in opposite orders.
    let forward = environment_order(&source, values, [0, 1]);
    let reverse = environment_order(&source, values, [1, 0]);
    // Then: full code/maps and materialized CSS agree, with no imported Own slots.
    assert_eq!(forward, reverse);
}

#[test]
#[serial]
fn stylesheet_names_when_environment_order_changes() {
    // Given: executed styles depend on external module data.
    let source = "import {style} from '@devup-ui/react';import {value} from './data';const choose=()=>value;export const cls=style({color:choose(),padding:'16px'});";
    // When: module loading runs under both environment orders.
    let forward = environment_order(source, ["'red'", "'blue'"], [0, 1]);
    let reverse = environment_order(source, ["'red'", "'blue'"], [1, 0]);
    // Then: evaluated stylesheet code, original-source maps and CSS are identical.
    assert_eq!(forward, reverse);
}
