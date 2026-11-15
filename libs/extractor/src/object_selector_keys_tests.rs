//! Style object keys that hold an object and name no style property are nested
//! selectors, as Emotion and styled-components read them.
use crate::{
    ExtractOption, ImportAlias, extract, extract_style::extract_style_value::ExtractStyleValue,
};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use insta::assert_debug_snapshot;
use rstest::rstest;
use serial_test::serial;
use std::collections::{BTreeSet, HashMap};

fn option() -> ExtractOption {
    ExtractOption {
        import_aliases: HashMap::from([
            (
                "@emotion/styled".into(),
                ImportAlias::DefaultToNamed("styled".into()),
            ),
            (
                "styled-components".into(),
                ImportAlias::DefaultToNamed("styled".into()),
            ),
            ("@emotion/css".into(), ImportAlias::NamedToNamed),
        ]),
        ..ExtractOption::default()
    }
}

/// Each extracted style as `selector level property: value`
fn rules(source: &str) -> BTreeSet<String> {
    reset_class_map();
    reset_file_map();
    extract("test.tsx", source, option())
        .unwrap_or_else(|error| panic!("{error}"))
        .styles
        .into_iter()
        .map(|style| match style {
            ExtractStyleValue::Static(style) => format!(
                "{:?} {} {}: {}",
                style.selector(),
                style.level(),
                style.property(),
                style.value()
            ),
            ExtractStyleValue::Dynamic(style) => format!(
                "{:?} {} {}: <dynamic>",
                style.selector(),
                style.level(),
                style.property()
            ),
            style => format!("{style:?}"),
        })
        .collect()
}

#[rstest]
#[case::css_descendants(
    "css_descendants",
    "import { css } from '@devup-ui/react';\nconst c = css({ color: 'red', body: { color: 'blue' }, 'h1, h2': { m: 0 } });"
)]
#[case::styled_member(
    "styled_member",
    "import { styled } from '@devup-ui/react';\nconst S = styled.div({ color: 'red', span: { color: 'blue' } });"
)]
#[case::styled_component(
    "styled_component",
    "import { styled, Box } from '@devup-ui/react';\nconst S = styled(Box)({ span: { color: 'blue' } });"
)]
#[case::css_prop(
    "css_prop",
    "import { Box } from '@devup-ui/react';\nconst e = <Box css={{ span: { color: 'blue' }, label: { color: 'red' } }} />;"
)]
#[case::pseudo_names(
    "pseudo_names",
    "import { css } from '@devup-ui/react';\nconst c = css({ hover: { color: 'blue' }, before: { content: '\"\"' }, print: { color: 'gray' }, themeDark: { color: 'white' } });"
)]
#[case::selector_list(
    "selector_list",
    "import { css } from '@devup-ui/react';\nconst c = css({ 'h1, :hover, &.x, [data-a], ::before, .a > b, Button, _focus, @media print': { color: 'blue' } });"
)]
#[case::nested_rules(
    "nested_rules",
    "import { css } from '@devup-ui/react';\nconst c = css({ span: { color: ['red', null, 'blue'], _hover: { color: 'green' }, em: { color: 'pink' }, '@media (min-width: 1px)': { color: 'gray' } } });"
)]
#[case::properties_stay(
    "properties_stay",
    "import { css } from '@devup-ui/react';\nconst c = css({ p: { color: 'blue' }, colr: 'red', vars: { '--a': '1px' } });"
)]
#[case::under_pseudo_prop(
    "under_pseudo_prop",
    "import { Box } from '@devup-ui/react';\nconst e = <Box _hover={{ span: { color: 'blue' } }} />;"
)]
#[case::under_selectors_prop(
    "under_selectors_prop",
    "import { Box } from '@devup-ui/react';\nconst e = <Box selectors={{ '.a': { span: { color: 'red' } } }} />;"
)]
#[case::global_css(
    "global_css",
    "import { globalCss } from '@devup-ui/react';\nglobalCss({ body: { color: 'red', span: { color: 'blue' } }, 'h1, h2': { m: 0 } });"
)]
#[case::emotion_styled(
    "emotion_styled",
    "import styled from '@emotion/styled';\nconst S = styled.div({ span: { color: 'blue' }, '&:hover': { color: 'red' } });"
)]
#[case::emotion_css(
    "emotion_css",
    "import { css } from '@emotion/css';\nconst c = css({ span: { color: 'blue' } });"
)]
#[case::styled_components(
    "styled_components",
    "import styled from 'styled-components';\nconst S = styled.div({ 'h1, h2': { margin: 0 } });"
)]
#[case::dynamic_value(
    "dynamic_value",
    "import { styled } from '@devup-ui/react';\nconst S = styled.div({ span: { color: (p) => p.c } });"
)]
#[case::jsx_props_keep_their_meaning(
    "jsx_props_keep_their_meaning",
    "import { Box } from '@devup-ui/react';\nconst e = <Box foo={{ color: 'blue' }} {...{ bar: { color: 'red' } }} />;"
)]
#[serial]
fn object_keys_nest_selectors(#[case] name: &str, #[case] source: &str) {
    insta::with_settings!({ snapshot_suffix => name }, {
        assert_debug_snapshot!(rules(source));
    });
}

#[rstest]
#[case(
    "import { css } from '@devup-ui/react';\nconst c = css({ 'h1, _nope': { color: 'red' } });",
    "_nope"
)]
#[case(
    "import { Box } from '@devup-ui/react';\nconst e = <Box selectors={{ div: { color: 'red' } }} />;",
    "div"
)]
#[serial]
fn names_that_select_nothing_stay_errors(#[case] source: &str, #[case] key: &str) {
    reset_class_map();
    reset_file_map();
    let error = extract("test.tsx", source, option())
        .err()
        .map_or_default(|error| error.to_string());
    assert!(
        error.contains(&format!("cannot use `{key}` at build time")),
        "{error}"
    );
}
