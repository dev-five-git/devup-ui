use super::*;
use crate::extract_style::extract_static_style::ExtractStaticStyle;

pub(super) fn fixture(body: &str) -> String {
    format!(
        "import {{ClassNames}} from '@emotion/react';\nconst a=<ClassNames>{{({{css}})=>css`{body}`}}</ClassNames>;"
    )
}

pub(super) fn rules(colors: &[&str], order: Option<u8>) -> Vec<ExtractStyleValue> {
    let mut rules: Vec<_> = std::iter::once(("background", "blue"))
        .chain(colors.iter().map(|color| ("color", *color)))
        .map(|(property, value)| {
            let mut style = ExtractStaticStyle::new(property, value, 0, None);
            style.style_order = order;
            ExtractStyleValue::Static(style)
        })
        .collect();
    rules.sort();
    rules
}

pub(super) fn tokens(value: &serde_json::Value) -> Vec<&str> {
    let mut tokens: Vec<_> = value
        .as_str()
        .required("ClassNames result must be a string")
        .split_whitespace()
        .collect();
    tokens.sort_unstable();
    tokens
}

pub(super) fn control(source: &str, expected_rules: Vec<ExtractStyleValue>, expected: &[&str]) {
    // When: the real public alias extractor and existing whole-code evaluator run.
    let compiled = compile_emotion(source).required("ClassNames sibling must compile");
    let actual = whole::evaluate_code(&compiled.code, "a");
    // Then: whole enum/value equality rejects extra variants and duplicate declarations.
    let mut actual_styles = compiled.styles.iter().cloned().collect::<Vec<_>>();
    actual_styles.sort_unstable();
    assert_eq!(actual_styles, expected_rules);
    assert_eq!(tokens(&actual.element), expected);
    assert_eq!(actual.trace, serde_json::json!([]));
}

pub(super) fn guarded_control(flag: bool, expected: &[&str]) {
    // Given: each boolean render has its own effectful guard and original object alternatives.
    let source = concat!(
        "import {ClassNames} from '@emotion/react';\n",
        "const render=flag=>{const guard=()=>(trace.push('guard'),flag);",
        "return <ClassNames>{({css})=>css`background:blue;",
        "${guard()?{color:'red'}:{color:'green'}};style-order:2`}</ClassNames>;};"
    );
    // When: public extraction runs, then one generated render receives the external boolean.
    let compiled = compile_emotion(source).required("guarded ClassNames must compile");
    let actual = whole::evaluate_code(&compiled.code, &format!("render({flag})"));
    // Then: both declarations exist, only the chosen classes apply, and the guard runs once.
    let mut actual_styles = compiled.styles.iter().cloned().collect::<Vec<_>>();
    actual_styles.sort_unstable();
    assert_eq!(actual_styles, rules(&["red", "green"], Some(2)));
    assert_eq!(tokens(&actual.element), expected);
    assert_eq!(actual.trace, serde_json::json!(["guard"]));
}
