use super::*;
use rstest::rstest;

#[rstest]
#[case::numeric_block("p=>{return 42;}", "(p) => { return 42; }")]
#[case::async_block(
    "async function(p){return {color:'red'}}",
    "(async function(p) { return { color: \"red\" }; })"
)]
#[case::generator(
    "function*(p){return {color:'red'}}",
    "(function* (p) { return { color: \"red\" }; })"
)]
#[case::async_arrow("async p=>({color:p.color})", "async (p) => ({ color: p.color })")]
#[serial]
fn w38r_u4_callback_when_return_or_function_kind_is_unreadable_reports_original_call(
    #[case] callback: &str,
    #[case] readable: &str,
) {
    // Given: source callbacks are neither finite producers nor readable sync rules.
    let source =
        format!("import {{styled}} from '@devup-ui/react';\nconst Card=styled.div({callback});");
    // When: block/finite callback dispatch fails through public styled extraction.
    let actual = error(&source);
    // Then: the error is located at the original call, not its transformed return.
    assert_eq!(
        actual,
        format!(
            "a.tsx:2:12: Cannot compose `{readable}` at build time: each style must be a rule object, a class, or a condition choosing between them"
        )
    );
}

#[rstest]
#[case::runtime_tag(
    "css`color:${runtimeColor};`",
    "a.tsx:2:9: `css()` cannot use `runtimeColor` at build time: its values must be literals, theme tokens or constants, or be computed from them"
)]
#[case::malformed_spread(
    "css({...{_hover:'red'}},{color:'blue'})",
    "a.tsx:2:9: Cannot compose `{ ...{ _hover: \"red\" } }, { color: \"blue\" }` at build time: each style must be a rule object, a class, or a condition choosing between them"
)]
#[serial]
fn w38r_u4_rules_when_lowering_or_shape_rejects_original_source_reports_exact_location(
    #[case] expression: &str,
    #[case] expected: &str,
) {
    // Given: invalid inputs distinguish unplaced text, dynamic values and selectors.
    let source = format!("import {{css}} from '@devup-ui/react';\nconst a={expression};");
    // When: extract through the public API without evaluating user runtime values.
    let actual = error(&source);
    // Then: retain the authored syntax, location and specific build requirement.
    assert_eq!(actual, expected);
}

#[rstest]
#[case::invalid_css("getCss()", "getCss()")]
#[serial]
fn w38r_u4_call_props_when_css_capture_fails_preserves_located_consumer_error(
    #[case] css: &str,
    #[case] readable: &str,
) {
    // Given: dynamic later props require a prefix capture before style extraction.
    let prefix = "const render=props=>jsx(Box,{...props,css:";
    let source = format!(
        "import {{Box}} from '@devup-ui/react';import {{jsx}} from '@emotion/react';\n{prefix}{css},color:mark('color','red')}});"
    );
    // When: call-props ordering asks the css preflight to capture unsupported input.
    let actual = compile_emotion(&source)
        .err()
        .required("invalid css capture fails");
    // Then: failure remains attached to the written css prop on the source element.
    assert_eq!(
        actual,
        format!(
            "a.tsx:2:{}: `css` on `<Box>` cannot use `{readable}` at build time: it must be a style object, CSS text, a class `css()` gives, or a function of the theme giving one, or an array or condition of them",
            prefix.len() + 1
        )
    );
}

#[rstest]
#[case::property("co", "lor")]
#[serial]
fn w38r_u4_unprepared_styled_when_nested_text_has_unreadable_hole_reports_that_hole(
    #[case] before: &str,
    #[case] after: &str,
) {
    // Given: literal spread bypasses capture, preserving the nested source hole.
    let prefix = format!("const a=styled.div(...{{_hover:`style-order:2;{before}${{");
    let source =
        format!("import {{styled}} from '@devup-ui/react';\n{prefix}getKey()}}{after}:red;`}});");
    // When: public ordered literal extraction reaches its unplaced payload.
    let actual = error(&source);
    // Then: reject the interpolation at its exact authored source position.
    assert_eq!(
        actual,
        format!(
            "a.tsx:2:{}: `styled()` cannot use `getKey()` at build time: its styles must be an object literal or a constant object, or be computed from constants",
            prefix.len() + 1
        )
    );
}
