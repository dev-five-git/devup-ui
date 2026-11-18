use super::*;

/// A `const` choosing among styles that only `css` props read renders, each
/// time, the styles Emotion would read from the choice it makes then
#[rstest]
#[case::conditional(
    "const rules=on?{color:'red'}:{color:'blue'};",
    serde_json::json!([[["red"], [], "string"], [["blue"], [], "string"]])
)]
#[case::and(
    "const rules=on&&{color:'red',margin:1};",
    serde_json::json!([[["red"], ["1px"], "string"], [[], [], "string"]])
)]
#[case::or(
    "const rules=(on?null:css({color:'blue'}))||{color:'red'};",
    serde_json::json!([[["red"], [], "string"], [["blue"], [], "string"]])
)]
#[case::nullish(
    "const rules=(on?undefined:css({color:'blue'}))??[{color:'red'},{margin:2}];",
    serde_json::json!([[["red"], ["2px"], "string"], [["blue"], [], "string"]])
)]
#[case::nested(
    "const rules=on?(other?{color:'red'}:{margin:3}):null;",
    serde_json::json!([[["red"], [], "string"], [[], [], "string"]])
)]
#[serial]
fn bound_choices_render_the_chosen_styles(
    #[case] declaration: &str,
    #[case] expected: serde_json::Value,
    #[values(false, true)] single_css: bool,
) {
    let _state = crate::test_state::TestStateGuard::new();
    reset_class_map();
    reset_file_map();
    let options = ExtractOption {
        single_css,
        import_aliases: std::collections::HashMap::from([(
            "@emotion/react".to_string(),
            crate::ImportAlias::NamedToNamed,
        )]),
        ..ExtractOption::default()
    };
    let source = format!(
        "import {{css}} from '@emotion/react';\nexport function f(on,other=true){{{declaration}return <div css={{rules}}/>;}}"
    );
    let bucket = crate::css_bucket("bound-choice.tsx", &options);
    let output = extract("bound-choice.tsx", &source, options)
        .unwrap_or_else(|error| panic!("{source}\n{error}"));
    let observed = super::execute_extracted(
        ("bound-choice.tsx", bucket.as_deref()),
        &output,
        "const look=element=>[values(element,'color'),values(element,'margin'),typeof element.props.className];JSON.stringify([look(f(true)),look(f(false))]);",
    );
    assert_eq!(observed, expected, "{}", output.code);
}
