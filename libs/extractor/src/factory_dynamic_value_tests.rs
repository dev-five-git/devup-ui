use super::*;

const FILE: &str = "/src/factory-dynamic.tsx";

#[rstest]
#[case::before("...raw, bg:'red', other:OWN")]
#[case::after("bg:'red', other:OWN, ...raw")]
#[case::opaque_literal("bg:'red', other:OWN, ...{bg:'green', ...raw}")]
#[case::computed("bg:'red', other:OWN, ...raw, [key]:'blue'")]
#[serial]
fn compiled_factory_values_when_spreads_are_opaque_keep_explicit_styles_independent(
    #[case] props: &str,
    #[values(false, true)] namespace: bool,
    #[values(false, true)] single_css: bool,
) {
    // Given
    let _state = crate::test_state::TestStateGuard::new();
    let call = if namespace { "ui.css" } else { "i" };
    let props = props.replace("OWN", &format!("held({call}({{overflow:'hidden'}}))"));
    let source = format!(
        "{} import {{jsx as o}} from 'react/jsx-runtime'; function render(raw,key,trace){{ const held=x=>(trace.push('held'),x); return o(t, (({{{props}}}) as const)); }}",
        imports(namespace)
    );
    // When
    let output = extract_factory(&source, single_css);
    // Then: the opaque keys are DOM data, not styling inputs; the utility runs at build time.
    let projection = "(() => {const result=render({bg:'blue',other:'raw',_hover:'opaque'},'bg',trace);trace.push(result.target,result.props.bg,result.props.other,result.props._hover);return result;})()";
    assert_eq!(
        held_observation(&output, projection, single_css),
        "[[\"held\",\"div\",\"blue\",\"raw\",\"opaque\"],[\"hidden\"]]"
    );
    let backgrounds: Vec<_> = output.styles.iter().filter_map(|style| match style {
        ExtractStyleValue::Static(value) if value.property() == "background" => Some(value.value()),
        _ => None,
    }).collect();
    assert_eq!(backgrounds, ["red"]);
}

fn imports(namespace: bool) -> &'static str {
    if namespace {
        "import {Box as t} from '@devup-ui/react'; import * as ui from '@devup-ui/react';"
    } else {
        "import {Box as t,css as i} from '@devup-ui/react';"
    }
}

fn extract_factory(source: &str, single_css: bool) -> ExtractOutput {
    extract(
        FILE,
        source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("{source}\n{error}"))
}

fn render_observation(output: &ExtractOutput, invocation: &str, filename: Option<&str>) -> String {
    use oxc_allocator::TakeIn;
    use oxc_ast_visit::{VisitMut, walk_mut::walk_expression};

    struct Syntax<'a>(AstBuilder<'a>);
    impl<'a> VisitMut<'a> for Syntax<'a> {
        fn visit_expression(&mut self, expression: &mut Expression<'a>) {
            let inner = crate::utils::unwrap_syntax_only_mut(expression);
            let value = inner.take_in(&self.0);
            *expression = value;
            walk_expression(self, expression);
        }
    }

    let arena = Allocator::default();
    let mut parsed = Parser::new(&arena, &output.code, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{}", output.code);
    Syntax(AstBuilder::new(&arena)).visit_program(&mut parsed.program);
    let javascript = ExtractOutput {
        styles: output.styles.clone(),
        code: Codegen::new().build(&parsed.program).code,
        map: None,
        css_file: output.css_file.clone(),
        dependencies: output.dependencies.clone(),
    };
    super::render_observation(&javascript, invocation, filename)
}

fn held_observation(output: &ExtractOutput, invocation: &str, single_css: bool) -> String {
    let filename = (!single_css).then_some(FILE);
    let variables: Vec<(String, String)> = output
        .styles
        .iter()
        .filter_map(|style| match style.extract(filename) {
            Some(StyleProperty::Variable {
                class_name,
                variable_name,
                ..
            }) => Some((class_name, variable_name)),
            _ => None,
        })
        .collect();
    let variables = serde_json::to_string(&variables).unwrap();
    let projection = format!(
        "(() => {{ const result=({invocation}); const active=new Set((result.props.className || '').split(/\\s+/)); return {{props:{{className:{variables}.filter(([name])=>active.has(name)).map(([,key])=>(result.props.style || {{}})[key]).join(' ')}}}}; }})()"
    );
    render_observation(output, &projection, filename)
}

#[rstest]
#[case::direct(("other: OWN", "[]", 1))]
#[case::spread(("...{ className:'external',other:OWN }", "[]", 1))]
#[case::nested_spread(("...{ ...{ other:OWN } }", "[]", 1))]
#[case::responsive(("other:[OWN,null,OWN]", "[]", 2))]
#[case::hover(("_hover:{ other:OWN }", "[]", 1))]
#[case::selector(("selectors:{ '&:hover,&:focus':{ other:OWN } }", "[]", 2))]
#[case::at_rule(("'@media':{ 'screen':{ other:OWN } }", "[]", 1))]
#[case::layer(("'@layer':{ 'test':{ other:OWN } }", "[]", 1))]
#[case::template(("other:`${OWN}`", "[]", 1))]
#[case::sequence(("other:(trace.push('sequence'),OWN)", "[\"sequence\"]", 1))]
#[case::binary(("other:OWN+''", "[]", 1))]
#[case::helper(("other:held(OWN)", "[\"held\"]", 1))]
#[case::member_fallback(("other:lookup(OWN)[key]", "[\"lookup\"]", 1))]
#[case::member_object(("other:({ a:OWN,b:OWN })[key]", "[]", 1))]
#[case::member_array(("other:([OWN,OWN])[0]", "[]", 1))]
#[case::member_dynamic_array(("other:([OWN,OWN])[index]", "[]", 1))]
#[case::member_spread(("other:({ ...p,a:OWN })[key]", "[]", 1))]
#[case::array_spread(("other:([OWN,...p])[index]", "[]", 1))]
#[case::wrapper(("other:(({ ...p,a:OWN }) as const)[key]", "[]", 1))]
#[case::static_object(("other:({ a:OWN })['a']", "[]", 1))]
#[case::shorthand(("mx:OWN", "[]", 2))]
#[serial]
fn held_css_links_real_styles_when_factory_serializes_dynamic_values(
    #[case] sample: (&str, &str, usize),
    #[values(false, true)] namespace: bool,
    #[values(false, true)] single_css: bool,
) {
    // Given
    let _state = crate::test_state::TestStateGuard::new();
    let (props, trace, count) = sample;
    let call = if namespace {
        "ui.css({overflow:'hidden'})"
    } else {
        "i({overflow:'hidden'})"
    };
    let props = props.replace("OWN", call);
    let source = format!(
        "{} import {{jsx as o}} from 'react/jsx-runtime'; function render(h,p,key,trace,index) {{ const held=x=>(trace.push('held'),x); const lookup=x=>(trace.push('lookup'),{{a:x}}); return o(t,{{{props}}}); }}",
        imports(namespace)
    );
    // When
    let mut output = extract_factory(&source, single_css);
    // Then: a prepared literal still occupies the original dynamic variable slot.
    let dynamic = output
        .styles
        .iter()
        .filter(|style| matches!(style, ExtractStyleValue::Dynamic(_)))
        .count();
    assert_eq!(dynamic, count, "{}", output.code);
    if props.starts_with("'@layer'") {
        let layers: Vec<_> = output
            .styles
            .iter()
            .filter_map(|style| match style {
                ExtractStyleValue::LayerOrder(order) => Some(order.layers.as_slice()),
                _ => None,
            })
            .collect();
        assert_eq!(layers, [vec!["test".to_string()].as_slice()]);
        output
            .styles
            .retain(|style| !matches!(style, ExtractStyleValue::LayerOrder(_)));
    }
    assert_eq!(
        held_observation(&output, "render(false, [], 'a', trace, 0)", single_css),
        format!("[{trace},[\"hidden\"]]")
    );
}

#[rstest]
#[serial]
fn exact_spread_factory_holds_compiled_css_when_original_reference_rows_execute(
    #[values(false, true)] namespace: bool,
    #[values(false, true)] single_css: bool,
) {
    // Given: the exact original source, filename and invocation, not a relaxed substitute.
    let _state = crate::test_state::TestStateGuard::new();
    let (import, call, filename) = if namespace {
        (
            "import { Box as t } from '@devup-ui/react'; import * as ui from '@devup-ui/react';",
            "ui.css",
            "/src/W70h-seam-spread-with-call-namespace.js",
        )
    } else {
        (
            "import { Box as t, css as i } from '@devup-ui/react';",
            "i",
            "/src/W70h-seam-spread-with-call-alias.js",
        )
    };
    let source = format!(
        "{import} import {{ jsx as o }} from 'react/jsx-runtime'; function render(h, p, getProps, clsx) {{ return o(t, {{ ...{{ className: 'external', other: {call}({{ overflow: 'hidden' }}) }} }}); }}"
    );
    // When
    let output = extract(
        filename,
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    )
    .unwrap();
    // Then
    let filename = (!single_css).then_some(filename);
    let dynamic: Vec<_> = output
        .styles
        .iter()
        .filter(|style| matches!(style, ExtractStyleValue::Dynamic(_)))
        .collect();
    assert_eq!(dynamic.len(), 1);
    let Some(StyleProperty::Variable { variable_name, .. }) = dynamic[0].extract(filename) else {
        panic!("missing held variable")
    };
    let projection = format!(
        "(() => {{const result=render(false, {{}}, () => ({{}}), undefined); trace.push(result.target, result.props.className.split(/\\s+/).includes('external')); return {{props:{{className:result.props.style[{variable_name:?}]}}}};}})()"
    );
    assert_eq!(
        render_observation(&output, &projection, filename),
        "[[\"div\",true],[\"hidden\"]]"
    );
}
