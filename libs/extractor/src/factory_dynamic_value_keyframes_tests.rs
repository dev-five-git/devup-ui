use super::*;

#[rstest]
#[case::direct("OWN")]
#[case::template("`${OWN}`")]
#[case::member("lookup(OWN)['a']")]
#[serial]
fn held_animation_names_link_keyframes_when_factory_prepares_dynamic_values(
    #[case] expression: &str,
    #[values(false, true)] namespace: bool,
    #[values(false, true)] single_css: bool,
) {
    // Given: the ordinary class observer intentionally does not accept keyframes.
    let _state = crate::test_state::TestStateGuard::new();
    let (import, call) = if namespace {
        (
            "import {Box as t} from '@devup-ui/react'; import * as ui from '@devup-ui/react';",
            "ui.keyframes",
        )
    } else {
        (
            "import {Box as t,keyframes as k} from '@devup-ui/react';",
            "k",
        )
    };
    let expression = expression.replace(
        "OWN",
        &format!("{call}({{from:{{opacity:0}},to:{{opacity:1}}}})"),
    );
    let source = format!(
        "{import} import {{jsx as o}} from 'react/jsx-runtime'; function render(){{const lookup=x=>({{a:x}}); return o(t,{{animationName:{expression}}});}}"
    );
    // When
    let output = extract_factory(&source, single_css);
    // Then: execute the emitted value without installing any keyframes callable.
    let arena = Allocator::default();
    let parsed = Parser::new(&arena, &output.code, SourceType::mjs()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let mut program = parsed.program;
    program
        .body
        .retain(|statement| !matches!(statement, Statement::ImportDeclaration(_)));
    let script = format!(
        "const o=(target,props)=>({{target,props}}); {} JSON.stringify(Object.values(render().props.style));",
        Codegen::new().build(&program).code
    );
    let observed = Context::default()
        .eval(Source::from_bytes(&script))
        .unwrap_or_else(|error| panic!("{error}\n{script}"));
    let values: Vec<String> =
        serde_json::from_str(&observed.as_string().unwrap().to_std_string_escaped()).unwrap();
    assert_eq!(values.len(), 1);
    let keyframes: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Keyframes(frames) => Some((style, frames)),
            _ => None,
        })
        .collect();
    assert_eq!(keyframes.len(), 1);
    let Some(StyleProperty::ClassName(name)) =
        keyframes[0].0.extract((!single_css).then_some(FILE))
    else {
        panic!("missing keyframes name")
    };
    assert_eq!(values, [name]);
    let steps: Vec<_> = keyframes[0]
        .1
        .keyframes
        .iter()
        .map(|(step, styles)| {
            (
                step.as_str(),
                styles
                    .iter()
                    .map(|style| (style.property(), style.value()))
                    .collect::<Vec<_>>(),
            )
        })
        .collect();
    assert_eq!(
        steps,
        [
            ("from", vec![("opacity", "0")]),
            ("to", vec![("opacity", "1")])
        ]
    );
    assert_eq!(
        output
            .styles
            .iter()
            .filter(|style| matches!(style, ExtractStyleValue::Dynamic(_)))
            .count(),
        1
    );
}
