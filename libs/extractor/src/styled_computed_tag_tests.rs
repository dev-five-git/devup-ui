//! `styled['div']` builds what `styled.div` does (EMO-17)
use std::collections::{BTreeSet, HashMap};

use css::{class_map::reset_class_map, file_map::reset_file_map};
use insta::assert_snapshot;
use rstest::rstest;
use serial_test::serial;

use crate::{ExtractOption, ImportAlias, extract};

const EMOTION: &str = "import styled from '@emotion/styled';";
const STYLED_COMPONENTS: &str = "import styled from 'styled-components';";
const DEVUP: &str = "import { styled } from '@devup-ui/react';";
const DEVUP_NAMESPACE: &str = "import * as Devup from '@devup-ui/react';";

fn option() -> ExtractOption {
    ExtractOption {
        import_aliases: HashMap::from([
            (
                "@emotion/styled".to_string(),
                ImportAlias::DefaultToNamed("styled".to_string()),
            ),
            (
                "styled-components".to_string(),
                ImportAlias::DefaultToNamed("styled".to_string()),
            ),
        ]),
        ..ExtractOption::default()
    }
}

/// The code and styles `code` compiles to, or the error it fails with
fn compiled(code: &str) -> Result<(String, BTreeSet<String>), String> {
    reset_class_map();
    reset_file_map();
    extract("test.tsx", code, option())
        .map(|output| {
            let styles = output
                .styles
                .iter()
                .map(|style| format!("{style:?}"))
                .collect();
            (output.code, styles)
        })
        .map_err(|error| error.to_string())
}

#[rstest]
#[case(EMOTION, "styled.div`padding: 4px;`", "styled['div']`padding: 4px;`")]
#[case(
    EMOTION,
    "styled.div({ padding: 4 })",
    "styled[\"div\"]({ padding: 4 })"
)]
#[case(EMOTION, "styled.span`color: red;`", "styled[`span`]`color: red;`")]
#[case(
    EMOTION,
    "styled.button({ padding: 4 }, { margin: 2 })",
    "styled['button']({ padding: 4 }, { margin: 2 })"
)]
#[case(
    STYLED_COMPONENTS,
    "styled.div`padding: 4px;`",
    "styled['div']`padding: 4px;`"
)]
#[case(
    STYLED_COMPONENTS,
    "styled.input.attrs({ type: 'text' })`color: red;`",
    "styled['input'].attrs({ type: 'text' })`color: red;`"
)]
#[case(
    STYLED_COMPONENTS,
    "styled.input.attrs({ type: 'text' })({ padding: 4 })",
    "styled['input'].attrs({ type: 'text' })({ padding: 4 })"
)]
#[case(
    STYLED_COMPONENTS,
    "styled.div.withConfig({ shouldForwardProp: (prop) => prop !== 'tone' })`color: red;`",
    "styled['div'].withConfig({ shouldForwardProp: (prop) => prop !== 'tone' })`color: red;`"
)]
#[case(DEVUP, "styled.div`padding: 4px;`", "styled['div']`padding: 4px;`")]
#[case(DEVUP, "styled.p({ bg: 'red' })", "styled['p']({ bg: 'red' })")]
#[case(
    DEVUP,
    "styled.input.attrs({ type: 'text' })({ bg: 'red' })",
    "styled['input'].attrs({ type: 'text' })({ bg: 'red' })"
)]
#[case(
    DEVUP_NAMESPACE,
    "Devup.styled.div({ bg: 'red' })",
    "Devup.styled['div']({ bg: 'red' })"
)]
#[serial]
fn computed_literal_tag_compiles_as_the_static_tag(
    #[case] import: &str,
    #[case] member: &str,
    #[case] computed: &str,
) {
    // Other computed members, called or not, stay as written
    let source = |styled: &str| {
        format!(
            "{import}\nconst helpers = {{ run: () => 'a' }};\nexport const ran = helpers['run']();\nexport const run = helpers['run'];\nconst S = {styled};\nexport const App = () => <S tone='a' />;"
        )
    };
    let expected = compiled(&source(member));
    assert!(expected.is_ok(), "{member}: {expected:?}");
    assert_eq!(compiled(&source(computed)), expected, "{computed}");
    assert!(expected.is_ok_and(|(code, _)| code.contains("helpers[\"run\"]()")));
}

#[rstest]
#[case(EMOTION, "styled[tag]`color: red;`")]
#[case(EMOTION, "styled['my-element']`color: red;`")]
#[case(EMOTION, "styled?.['div']({ color: 'red' })")]
#[case(EMOTION, "styled[0]`color: red;`")]
#[case(STYLED_COMPONENTS, "styled[tag].attrs({ type: 'text' })`color: red;`")]
#[case(DEVUP, "styled[tag]`color: red;`")]
#[case(DEVUP, "styled['my-element']({ bg: 'red' })")]
#[case(DEVUP, "styled?.['div']({ bg: 'red' })")]
#[serial]
fn computed_tag_the_build_cannot_name_is_a_located_error(
    #[case] import: &str,
    #[case] styled: &str,
) {
    let error = compiled(&format!(
        "{import}\nconst tag = 'div';\nconst S = {styled};\nexport const App = () => <S />;"
    ));
    let message = error.err().unwrap_or_default();
    assert!(
        message.starts_with("test.tsx:3:11: "),
        "{styled}\n{message}"
    );
    assert!(
        message.contains("`styled` is read at runtime"),
        "{styled}\n{message}"
    );
}

#[test]
#[serial]
fn computed_tag_keeps_the_numbers_emotion_reads_as_pixels() {
    let compiled = compiled(&format!(
        "{EMOTION}\nexport const S = styled['div']({{ padding: 4, zIndex: 2 }});"
    ));
    assert_snapshot!(format!("{compiled:#?}"));
}
