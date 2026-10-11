use super::*;
use serial_test::serial;

fn emotion_error(source: &str) -> String {
    match compile_emotion(source) {
        Err(message) => message,
        Ok(compiled) => panic!("the file compiles: {}", compiled.code),
    }
}

fn emotion_output(source: &str) -> ExtractOutput {
    compile_emotion(source).unwrap_or_else(|error| panic!("the file does not compile: {error}"))
}

#[test]
#[serial]
fn the_css_prop_before_a_spread_is_a_located_error() {
    let message = emotion_error(&format!(
        "{EMOTION}export const a = (rest) => <div css={{{{ color: 'red' }}}} {{...rest}} />;"
    ));

    assert!(
        message.contains("`css` on `<div>` cannot use `...rest`"),
        "{message}"
    );
    assert!(message.starts_with("a.tsx:2:"), "{message}");
    assert!(
        message.contains("write the spread before `css`"),
        "{message}"
    );
}

#[test]
#[serial]
fn the_css_prop_after_a_spread_compiles() {
    let output = emotion_output(&format!(
        "{EMOTION}export const a = (rest) => <div {{...rest}} css={{{{ color: 'red' }}}} />;"
    ));

    assert_eq!(
        static_styles(&output),
        vec![("color".to_string(), "red".to_string())]
    );
}

#[test]
#[serial]
fn the_css_prop_of_a_call_before_a_spread_is_a_located_error() {
    let message = emotion_error(
        "import { jsx } from '@emotion/react';\nexport const a = (rest) => jsx('div', { css: { color: 'red' }, ...rest });",
    );

    assert!(
        message.contains("`css` on `<div>` cannot use `...rest`"),
        "{message}"
    );
}

#[test]
#[serial]
fn the_css_prop_of_a_call_after_a_spread_compiles() {
    let output = emotion_output(
        "import { jsx } from '@emotion/react';\nexport const a = (rest) => jsx('div', { ...rest, css: { color: 'red' } });",
    );

    assert_eq!(
        static_styles(&output),
        vec![("color".to_string(), "red".to_string())]
    );
}
