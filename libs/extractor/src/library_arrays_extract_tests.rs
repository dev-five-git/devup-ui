use crate::{
    ExtractOption, ImportAlias, extract, extract_style::extract_style_value::ExtractStyleValue,
};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use rstest::rstest;
use serial_test::serial;
use std::collections::HashMap;

fn options() -> ExtractOption {
    ExtractOption {
        import_aliases: HashMap::from([
            ("@emotion/react".to_string(), ImportAlias::NamedToNamed),
            (
                "@emotion/styled".to_string(),
                ImportAlias::DefaultToNamed("styled".to_string()),
            ),
            (
                "styled-components".to_string(),
                ImportAlias::DefaultToNamed("styled".to_string()),
            ),
            (
                "@vanilla-extract/css".to_string(),
                ImportAlias::NamedToNamed,
            ),
        ]),
        ..ExtractOption::default()
    }
}

/// The declarations extracting `source` as `filename` gives, sorted
fn declarations(filename: &str, source: &str) -> Vec<(String, String, u8)> {
    let _state = crate::test_state::TestStateGuard::new();
    reset_class_map();
    reset_file_map();
    let output =
        extract(filename, source, options()).unwrap_or_else(|error| panic!("{source}\n{error}"));
    let mut declarations: Vec<_> = output
        .styles
        .iter()
        .flat_map(|style| match style {
            ExtractStyleValue::Static(style) => vec![style.clone()],
            ExtractStyleValue::Keyframes(keyframes) => {
                keyframes.keyframes.values().flatten().cloned().collect()
            }
            other => panic!("unexpected style: {other:?}"),
        })
        .map(|style| (style.property, style.value, style.level))
        .collect();
    declarations.sort_unstable();
    declarations
}

fn owned(expected: &[(&str, &str, u8)]) -> Vec<(String, String, u8)> {
    expected
        .iter()
        .map(|(property, value, level)| (property.to_string(), value.to_string(), *level))
        .collect()
}

// Each library's arrays render as its runtime renders them; Devup UI's stay breakpoints
#[rstest]
#[case::emotion_fallbacks(
    "import { css } from '@emotion/react'\nexport const a = css({ height: ['100vh', '100dvh'], color: ['red', null, 'blue'] })",
    &[("color", "red;color:blue", 0), ("height", "100vh;height:100dvh", 0)]
)]
#[case::emotion_number_first(
    "import { css } from '@emotion/react'\nexport const a = css({ margin: [4], padding: [4, '8px'] })",
    &[]
)]
#[case::emotion_styled(
    "import styled from '@emotion/styled'\nexport const A = styled.div({ margin: ['1px', 3] })",
    &[("margin", "1px;margin:3px", 0)]
)]
#[case::emotion_global(
    "import { Global } from '@emotion/react'\nexport const A = () => <Global styles={{ body: { margin: ['1px', 3] } }} />",
    &[("margin", "1px;margin:3px", 0)]
)]
#[case::styled_components_css(
    "import { css } from 'styled-components'\nexport const a = css({ fontFamily: ['Arial', 'sans-serif'], margin: [4] })",
    &[("font-family", "Arial,sans-serif", 0)]
)]
#[case::styled_components_factory(
    "import styled from 'styled-components'\nexport const A = styled.div.attrs({})({ margin: ['1px', '3px'] })\nexport const B = styled['span']({ color: ['red', null] })",
    &[("color", "red,", 0), ("margin", "1px,3px", 0)]
)]
#[case::styled_components_constant(
    "import { css } from 'styled-components'\nconst base = { fontFamily: ['Arial', 'sans-serif'] }\nexport const a = css(base)",
    &[("font-family", "Arial,sans-serif", 0)]
)]
#[case::vanilla_extract_outside_stylesheets(
    "import { style } from '@vanilla-extract/css'\nexport const a = style({ margin: [4, '8px'], color: ['red', null] })",
    &[("color", "red;color:null", 0), ("margin", "4;margin:8px", 0)]
)]
#[case::devup(
    "import { css } from '@emotion/react'\nimport { css as devup } from '@devup-ui/react'\nexport const a = devup({ color: ['red', 'blue'] })\nexport const b = (() => css({ color: ['red'] }))()",
    &[("color", "blue", 1), ("color", "red", 0)]
)]
#[serial]
fn library_arrays_render_as_their_runtime(
    #[case] source: &str,
    #[case] expected: &[(&str, &str, u8)],
) {
    assert_eq!(declarations("a.tsx", source), owned(expected), "{source}");
}

// A stylesheet importing vanilla-extract runs its arrays as fallbacks; one
// importing Devup UI keeps its responsive arrays
#[rstest]
#[case::vanilla_extract(
    "import { style } from '@vanilla-extract/css'\nexport const a = style({ margin: [4, '8px'], color: ['red', null, 'blue'] })",
    &[("color", "red;color:null;color:blue", 0), ("margin", "4;margin:8px", 0)]
)]
#[case::vanilla_extract_global(
    "import { globalStyle, keyframes } from '@vanilla-extract/css'\nglobalStyle('body', { margin: ['1px', '2px'] })\nexport const k = keyframes({ from: { color: ['red', 'blue'] } })",
    &[("color", "red;color:blue", 0), ("margin", "1px;margin:2px", 0)]
)]
#[case::vanilla_extract_rules(
    "import { style } from '@vanilla-extract/css'\nexport const a = style({ _hover: [{ color: 'red' }], p: [1, 2] })",
    &[("color", "red", 0), ("padding", "4px", 0), ("padding", "8px", 1)]
)]
#[case::devup(
    "import { style } from '@devup-ui/react'\nexport const a = style({ margin: [4, '8px'] })",
    &[("margin", "16px", 0), ("margin", "8px", 1)]
)]
#[serial]
fn stylesheet_arrays_follow_their_import(
    #[case] source: &str,
    #[case] expected: &[(&str, &str, u8)],
) {
    assert_eq!(
        declarations("a.css.ts", source),
        owned(expected),
        "{source}"
    );
}
