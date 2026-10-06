use crate::{ExtractOutput, ExtractStyleValue};

pub(super) struct TypographyTheme;

impl TypographyTheme {
    pub(super) fn register() -> Self {
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

pub(super) fn fixture(expression: &str, runtime: bool, order: &str) -> String {
    let mut fields = vec![("typography", expression)];
    match order {
        "before" => fields.insert(0, ("className", "state.c")),
        "after" => fields.push(("className", "state.c")),
        "alone" => {}
        _ => panic!("fixture order"),
    }
    let element = if runtime {
        format!(
            "jsx(Box,{{{}}})",
            fields
                .iter()
                .map(|(key, value)| format!("{key}:{value}"))
                .collect::<Vec<_>>()
                .join(",")
        )
    } else {
        format!(
            "<Box {}/>",
            fields
                .iter()
                .map(|(key, value)| format!("{key}={{{value}}}"))
                .collect::<Vec<_>>()
                .join(" ")
        )
    };
    let source = format!(
        "import {{Box}}from '@devup-ui/react';import {{jsx}}from 'react/jsx-runtime';function render(t,state){{return {element}}}let trace=[];let value='body';let state={{get seed(){{trace.push('seed');return value}},get t(){{trace.push('t');return value}},get key(){{trace.push('key');return 't'}},get flag(){{trace.push('flag');return true}},get bad(){{throw Error('not lazy')}},get empty(){{trace.push('empty');return value}},get arg(){{trace.push('arg');return 7}},pick(arg){{trace.push(this===state&&arg===7?'call':'broken');return value}},get c(){{trace.push('c');return 'external'}}}};const jsx=(tag,props)=>({{tag,...props}});const node=render(state.seed,state);"
    );
    if expression == "typography" {
        source.replace("const node=", "let reads=0;Object.defineProperty(globalThis,'typography',{get(){if(++reads>1)throw Error('second read');return state.t}});const node=")
    } else {
        source
    }
}

pub(super) fn observe(script: &str) -> serde_json::Value {
    let script = format!("const Box='div';{script}");
    let boa = crate::assignment_test_support::evaluate(&script);
    let node = crate::named_capture_support::evaluate(&script);
    assert_eq!(boa, node, "native engines disagree\n{script}");
    serde_json::from_str(&node).unwrap_or_else(|error| panic!("{error}\n{node}"))
}

pub(super) fn class_only(output: &ExtractOutput) {
    assert_eq!(
        output
            .styles
            .iter()
            .filter(|value| matches!(value, ExtractStyleValue::Dynamic(_)))
            .count(),
        0,
        "typography must not create CSS variables"
    );
    assert_eq!(
        output.styles.iter().filter(|value| matches!(value, ExtractStyleValue::Static(style) if style.property == "typography")).count(),
        0,
        "root typography must not create CSS declarations"
    );
}

pub(super) fn declarations(output: &ExtractOutput) -> String {
    let records = output
        .styles
        .iter()
        .filter_map(|value| match (value, value.extract(None)?) {
            (
                ExtractStyleValue::Static(style),
                crate::extract_style::style_property::StyleProperty::ClassName(class),
            ) => Some((
                class,
                format!("{}:{}:{}", style.property, style.value, style.level),
            )),
            _ => None,
        })
        .collect::<Vec<_>>();
    serde_json::to_string(&records).unwrap_or_else(|error| panic!("{error}"))
}
