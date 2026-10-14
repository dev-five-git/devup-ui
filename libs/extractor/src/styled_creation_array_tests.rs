use crate::{
    ExtractStyleValue,
    assignment_test_support::{evaluate, extracted, jsx_js},
    extract_style::style_property::StyleProperty,
};
use rstest::rstest;
use serial_test::serial;

struct TypographyTheme;

impl TypographyTheme {
    fn register() -> Self {
        css::theme_tokens::set_typography_keys(vec!["body".to_string(), "heading".to_string()]);
        css::content_typography::set(
            [("body", "14px"), ("heading", "24px")]
                .into_iter()
                .map(|(name, size)| {
                    (
                        name.to_string(),
                        vec![(0, "font-size".to_string(), size.to_string())],
                    )
                })
                .collect(),
        );
        Self
    }
}

impl Drop for TypographyTheme {
    fn drop(&mut self) {
        css::theme_tokens::set_typography_keys(vec![]);
        css::content_typography::set(Default::default());
    }
}

#[rstest]
#[case(true, false)]
#[case(false, false)]
#[case(true, true)]
#[case(false, true)]
#[serial]
fn selected_typography_array_keeps_base_when_flag_changes_before_both_renders(
    #[case] choice: bool,
    #[case] responsive: bool,
    #[values(false, true)] literal: bool,
) {
    // Given: real different theme content, and an observable creation-only flag.
    let _theme = TypographyTheme::register();
    let rules = if responsive {
        "flag?{typography:['heading','body']}:{typography:['body','heading']}"
    } else {
        "flag?{typography:['heading']}:{typography:['body']}"
    };
    let rules = match (literal, responsive, choice) {
        (false, _, _) => rules,
        (true, false, true) => "{typography:['heading']}",
        (true, false, false) => "{typography:['body']}",
        (true, true, true) => "{typography:['heading','body']}",
        (true, true, false) => "{typography:['body','heading']}",
    };
    let output = extracted(&format!(
        "import {{styled}}from '@devup-ui/react';const A=styled.div({rules});"
    ));
    let records = output
        .styles
        .iter()
        .filter_map(|value| match (value, value.extract(None)?) {
            (ExtractStyleValue::Static(style), StyleProperty::ClassName(class)) => Some((
                class,
                format!("{}:{}:{}", style.property, style.value, style.level),
            )),
            _ => None,
        })
        .collect::<Vec<_>>();
    let records = serde_json::to_string(&records)
        .unwrap_or_else(|error| panic!("records must serialize: {error}"));
    // When: the generated packet is created once, then mutated before each render.
    let actual = evaluate(&format!(
        r"const trace=[];const state={{choice:{choice}}};
Object.defineProperty(globalThis,'flag',{{get(){{trace.push('flag');return state.choice}}}});
{} const created=trace.slice();const records={records};
const project=node=>[String(node.className??'').split(/\s+/).filter(Boolean).map(name=>{{
if(name.startsWith('typo-'))return name;
const record=records.find(([key])=>key===name);if(!record)throw Error('missing class '+name);
return record[1];}}).sort(),Object.entries(node.style??{{}})];
state.choice=!state.choice;const first=project(A({{}}));const rendered=trace.slice();
state.choice=!state.choice;const second=project(A({{}}));
JSON.stringify([created,rendered,trace,first,second]);",
        jsx_js(&output.code)
    ));
    // Then: explicit selected roles survive, without typography variables or rereads.
    let (base, later) = if choice {
        ("typo-heading", "typography:body:1")
    } else {
        ("typo-body", "typography:heading:1")
    };
    let mut roles = vec![base];
    if responsive {
        roles.push(later);
    }
    roles.sort_unstable();
    let expected = serde_json::json!([roles, []]);
    let trace = if literal { vec![] } else { vec!["flag"] };
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&actual)
            .unwrap_or_else(|error| panic!("observation must be JSON: {error}")),
        serde_json::json!([
            trace.clone(),
            trace.clone(),
            trace,
            expected.clone(),
            expected
        ])
    );
    assert!(
        output
            .styles
            .iter()
            .all(|value| !matches!(value, ExtractStyleValue::Dynamic(_)))
    );
    assert!(output.styles.iter().all(|value| !matches!(value,
        ExtractStyleValue::Static(style) if style.property == "typography" && style.level == 0)));
}

#[path = "styled_creation_array_projection_tests.rs"]
mod projections;
