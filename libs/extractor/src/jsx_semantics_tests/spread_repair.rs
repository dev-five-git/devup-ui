use super::*;
use crate::extract_style::ExtractStyleProperty;
use rstest::rstest;
use serial_test::serial;

#[path = "spread_boundary.rs"]
mod boundary;
#[path = "spread_grouping.rs"]
mod grouping;

fn selected(source: &str, probe: &str) -> serde_json::Value {
    let compiled = output(source);
    let actual = whole::evaluate(source, probe).element;
    let classes: Vec<_> = actual["props"]["className"]
        .as_str()
        .unwrap_or("")
        .split_whitespace()
        .collect();
    let mut declarations = Vec::new();
    css::debug::set_debug(true);
    for style in &compiled.styles {
        let (level, value) = match style {
            ExtractStyleValue::Static(style) => (style.level(), Some(style.value().to_string())),
            ExtractStyleValue::Dynamic(style) => {
                let StyleProperty::Variable { variable_name, .. } = style.extract(None) else {
                    panic!("dynamic variable");
                };
                let supplied = &actual["props"]["style"][&variable_name];
                let value = match supplied {
                    serde_json::Value::String(value)
                        if !value.is_empty() || style.property().starts_with("--") =>
                    {
                        Some(value.clone())
                    }
                    serde_json::Value::Number(value) => Some(value.to_string()),
                    _ => None,
                };
                (style.level(), value)
            }
            _ => continue,
        };
        let name = match style.extract(Some("a.tsx")) {
            Some(StyleProperty::ClassName(name)) => name,
            Some(StyleProperty::Variable { class_name, .. }) => class_name,
            None => continue,
        };
        if classes.contains(&name.as_str()) {
            declarations.push((level, value));
        }
    }
    css::debug::set_debug(false);
    declarations.sort_by_key(|(level, _)| *level);
    serde_json::Value::Array(
        [0, 1, 2, 4]
            .into_iter()
            .map(|viewport| {
                declarations
                    .iter()
                    .rfind(|(level, _)| *level <= viewport)
                    .and_then(|(_, value)| value.as_deref())
                    .map(|value| if value == "0px" { "0" } else { value })
                    .map_or(serde_json::Value::Null, Into::into)
            })
            .collect(),
    )
}

#[rstest]
#[case("color=\"red\"", "color", "red")]
#[case("p={1}", "p", "4px")]
#[case("opacity={0.3}", "opacity", ".3")]
#[serial]
fn authored_styles_when_raw_values_are_absent_remain_unchanged(
    #[case] attribute: &str,
    #[case] key: &str,
    #[case] fallback: &str,
) {
    let source = format!("{BOX}export const a=(rest)=><Box {attribute} {{...rest}}/>;");
    for input in [
        "undefined",
        "null",
        "false",
        "true",
        "NaN",
        "Infinity",
        "-Infinity",
        "''",
    ] {
        let actual = selected(&source, &format!("a({{{key}:{input}}})"));
        assert_eq!(
            actual,
            serde_json::json!([fallback, fallback, fallback, fallback]),
            "{input}"
        );
    }
    assert_eq!(
        selected(&source, "a({})"),
        serde_json::json!([fallback, fallback, fallback, fallback])
    );
}

#[rstest]
#[case("p", "1", "2")]
#[case("p", "1", "'2'")]
#[case("p", "1", "'12px'")]
#[case("p", "1", "0")]
#[case("p", "1", "-2")]
#[case("p", "1", "0.5")]
#[case("p", "1", "'.5'")]
#[case("p", "1", "'1.'")]
#[case("p", "1", "'1.e2'")]
#[case("p", "1", "'1e-2'")]
#[case("p", "1", "'+4'")]
#[case("p", "1", "'-0'")]
#[case("p", "1", "'0x10'")]
#[case("p", "1", "'1_000'")]
#[case("p", "1", "'4\\n'")]
#[case("animationDuration", "1", "2")]
#[case("animationDuration", "1", "'0.5'")]
#[case("opacity", "0.3", "2")]
#[serial]
fn raw_scalars_when_passed_through_leave_authored_units_unchanged(
    #[case] key: &str,
    #[case] fallback: &str,
    #[case] input: &str,
) {
    let source = format!("{BOX}export const a=(rest)=><Box {key}={{{fallback}}} {{...rest}}/>;");
    let actual = selected(&source, &format!("a({{{key}:{input}}})"));
    let expected = match key {
        "p" => "4px",
        "animationDuration" => "1ms",
        "opacity" => ".3",
        _ => panic!("unexpected property"),
    };
    assert_eq!(actual, serde_json::json!(vec![expected; 4]));
    assert_eq!(
        whole::evaluate(
            &source,
            &format!(
                "(()=>{{const raw={input};return Object.is(a({{{key}:raw}}).props.{key},raw);}})()"
            )
        )
        .element,
        true
    );
}

#[rstest]
#[case("p={value()}")]
#[case("p={on ? '4px' : '8px'}")]
#[case("p={on ? '4px' : null}")]
#[case("p={on ? null : '8px'}")]
#[case("p={on && '4px'}")]
#[case("p={[1,2]}")]
#[case("p='4px' {...first}")]
#[case("padding='4px'")]
#[serial]
fn authored_selection_when_raw_spread_values_vary_keeps_original_branches(#[case] attribute: &str) {
    let source = format!(
        "{BOX}export const a=(rest,on)=>{{const first={{p:'20px'}};const value=()=> '5px';return <Box {attribute} {{...rest}}/>;}};"
    );
    let key = if attribute.starts_with("padding") {
        "padding"
    } else {
        "p"
    };
    for on in [true, false] {
        for input in ["2", "null", "undefined", "false", "''"] {
            let actual = selected(&source, &format!("a({{{key}:{input}}},{on})"));
            assert_eq!(
                actual,
                selected(&source, &format!("a({{}},{on})")),
                "{attribute}/{on}/{input}"
            );
        }
    }
}

#[test]
#[serial]
fn spread_repair_effects_when_spread_getters_run_preserves_native_order() {
    let source = format!(
        "{BOX}export const a=(rest)=> <Box p={{(trace.push('value'),'5px')}} {{...rest}} id={{(trace.push('id'),'x')}}/>;"
    );
    let actual = whole::evaluate(
        &source,
        "(()=>{const s=Symbol('x');const rest=Object.create({ p: 9 });rest[s]=7;Object.defineProperty(rest,'p',{enumerable:true,get(){trace.push('get');return 2;}});const e=a(rest);return [Object.values(e.props.style),e.props[s],e.props.id];})()",
    );
    assert_eq!(actual.element, serde_json::json!([["5px"], 7, "x"]));
    assert_eq!(actual.trace, serde_json::json!(["value", "get", "id"]));
    assert_eq!(
        selected(&source, "a({ p:2 })"),
        serde_json::json!(["5px", "5px", "5px", "5px"])
    );
}

#[rstest]
#[case("p={value()}", ["5px"; 4])]
#[case("p={on ? '4px' : null}", ["4px"; 4])]
#[case("p={on ? null : '8px'}", [""; 4])]
#[case("p={[1,2]}", ["4px", "8px", "8px", "8px"])]
#[case("p='4px' {...first}", ["20px"; 4])]
#[serial]
fn authored_values_when_spread_keys_are_missing_preserve_original_levels(
    #[case] attribute: &str,
    #[case] expected: [&str; 4],
) {
    let source = format!(
        "{BOX}export const a=(rest,on)=>{{const first={{p:'20px'}};const value=()=> '5px';return <Box {attribute} {{...rest}}/>;}};"
    );
    let actual = selected(&source, "a({},true)");
    let expected = expected.map(|value| if value.is_empty() { None } else { Some(value) });
    assert_eq!(actual, serde_json::json!(expected));
}

#[test]
#[serial]
fn spread_frames_when_emitted_have_no_style_key_selection() {
    for attribute in [
        "color='red'",
        "p={1}",
        "p={value()}",
        "p={[1,2]}",
        "p={on && '4px'}",
        "p='4px' {...first}",
    ] {
        let source = format!(
            "{BOX}export const a=(rest,on)=>{{const first={{p:'20px'}};const value=()=> '5px';return <Box {attribute} {{...rest}}/>;}};"
        );
        let compiled = output(&source);
        assert!(!compiled.code.contains("__devupNumber"));
        assert!(!compiled.code.contains("-spread-"));
        for key in ["color", "p"] {
            assert!(!compiled.code.contains(&format!("\"{key}\" in ")));
        }
        assert_eq!(
            selected(&source, "a({p:2,color:'raw'},true)"),
            selected(&source, "a({},true)")
        );
    }
}

#[rstest]
#[case::native_exact(
    "export const App = ({ rest, cls }) => <><div {...rest} css={{ color: 'red' }} /><div className={cls} {...rest} css={{ color: 'red' }} style={{ opacity: 1 }} /></>;",
    false,
    false
)]
#[case::native_array_control(
    "export const App = ({ rest, cls }) => [<div {...rest} css={{ color: 'red' }} />,<div className={cls} {...rest} css={{ color: 'red' }} style={{ opacity: 1 }} />];",
    false,
    false
)]
#[case::styled_array_control(
    "import styled from '@emotion/styled';const Button=styled.button({color:'blue',margin:1});export const App=({rest,cls})=>[<Button {...rest} css={{padding:2}}/>,<Button {...rest} css={[cls,{top:1}]}/>];",
    true,
    false
)]
#[case::native_interleaved_control(
    "export const App=({rest,cls,mark})=>[<div data-before={mark('before1')} {...rest} css={{color:'red'}} data-after={mark('after1')}/>,<div data-before={mark('before2')} className={cls} {...rest} css={{color:'red'}} data-after={mark('after2')} style={{opacity:1}}/>];",
    false,
    true
)]
#[serial]
fn merged_classes_read_the_written_spread_snapshot_once(
    #[case] source: &str,
    #[case] styled: bool,
    #[case] interleaved: bool,
    #[values(false, true)] single_css: bool,
    #[values("ordinary", "mutation", "throw-second", "poison-extra")] phase: &str,
) {
    reset_class_map();
    reset_file_map();
    css::debug::set_debug(false);
    let options = ExtractOption {
        single_css,
        import_aliases: HashMap::from([
            ("@emotion/react".into(), ImportAlias::NamedToNamed),
            (
                "@emotion/styled".into(),
                ImportAlias::DefaultToNamed("styled".into()),
            ),
        ]),
        ..ExtractOption::default()
    };
    let bucket = crate::css_bucket("test.tsx", &options);
    let output = extract("test.tsx", source, options).unwrap_or_else(|error| panic!("{error:?}"));
    let finish = if styled {
        "elements.map(element=>render(element))"
    } else {
        "elements"
    };
    let probe = format!(
        r"(()=>{{
let reads=0;const style={{opacity:0.4}};const callback=()=>0;
const rest={{get className(){{trace.push('class'+(++reads));if(('{phase}'==='throw-second'&&reads===2)||('{phase}'==='poison-extra'&&reads===3))throw new Error('stop');return '{phase}'==='mutation'?'next'+reads:'first';}},get style(){{trace.push('style');return style;}},get onClick(){{trace.push('callback');return callback;}}}};
try{{const result=App({{rest,cls:'authored',mark:label=>trace.push(label)}});const elements=Array.isArray(result)?result:result.children;const finished={finish};return {{outcome:'ok',reads,classes:finished.map(e=>e.props.className),targets:finished.map(e=>e.target??e.type),callbackIdentity:finished.map(e=>e.props.onClick===callback),styleIdentity:finished.map(e=>e.props.style===style),opacity:finished.map(e=>e.props.style.opacity)}};}}catch(error){{return {{outcome:error.message,reads}};}}
}})()"
    );
    let (value, trace) = if styled {
        let observed = crate::styled_source_tests::captured_render::execute(
            ("test.tsx", bucket.as_deref()),
            &output,
            &format!("const trace=[];JSON.stringify([{probe},trace]);"),
        );
        (observed[0].clone(), observed[1].clone())
    } else {
        let observed = whole::execute(&output.code, &probe);
        (observed.element, observed.trace)
    };
    if !styled {
        // Native props reference compares authored getter/marker ordering, not CSS styling.
        let reference = whole::execute(source, &probe);
        assert_eq!(trace, reference.trace);
        assert_eq!(value["outcome"], reference.element["outcome"]);
        assert_eq!(value["reads"], reference.element["reads"]);
    }
    if phase == "throw-second" {
        assert_eq!(value, serde_json::json!({"outcome":"stop","reads":2}));
        assert_eq!(
            trace,
            if interleaved {
                serde_json::json!([
                    "before1", "class1", "style", "callback", "after1", "before2", "class2"
                ])
            } else {
                serde_json::json!(["class1", "style", "callback", "class2"])
            }
        );
        return;
    }
    assert_eq!(value["outcome"], "ok", "{value}");
    assert_eq!(value["reads"], 2, "{value}");
    assert_eq!(
        trace,
        if interleaved {
            serde_json::json!([
                "before1", "class1", "style", "callback", "after1", "before2", "class2", "style",
                "callback", "after2"
            ])
        } else {
            serde_json::json!(["class1", "style", "callback", "class2", "style", "callback"])
        }
    );
    assert_eq!(
        value["targets"],
        if styled {
            serde_json::json!(["button", "button"])
        } else {
            serde_json::json!(["div", "div"])
        }
    );
    assert_eq!(value["callbackIdentity"], serde_json::json!([true, true]));
    assert_eq!(
        value["styleIdentity"],
        if styled {
            serde_json::json!([true, true])
        } else {
            serde_json::json!([true, false])
        }
    );
    assert_eq!(
        value["opacity"],
        if styled {
            serde_json::json!([0.4, 0.4])
        } else {
            serde_json::json!([0.4, 1])
        }
    );
    for (index, class) in value["classes"]
        .as_array()
        .unwrap_or_else(|| panic!("expected rendered class list"))
        .iter()
        .enumerate()
    {
        let expected = if phase == "mutation" {
            format!("next{}", index + 1)
        } else {
            "first".into()
        };
        let names: Vec<_> = class
            .as_str()
            .unwrap_or_else(|| panic!("expected rendered class name"))
            .split_whitespace()
            .collect();
        assert!(names.contains(&expected.as_str()), "{value}");
        assert_eq!(names.contains(&"authored"), styled && index == 1, "{value}");
    }
}

#[rstest]
#[case("export const App=()=> <div css={{color:'red'}}/>;")]
#[case("export const App=({title})=> <div title={title} css={{color:'red'}}/>;")]
#[case("import {Box} from '@devup-ui/react';export const App=()=> <Box color='red'/>;")]
#[serial]
fn spread_owner_fix_keeps_static_and_single_reads_without_extra_work(
    #[case] source: &str,
    #[values(false, true)] single_css: bool,
) {
    use oxc_ast_visit::{Visit, walk};
    #[derive(Default)]
    struct Work {
        calls: usize,
        arrays: usize,
    }
    impl<'a> Visit<'a> for Work {
        fn visit_call_expression(&mut self, value: &oxc_ast::ast::CallExpression<'a>) {
            self.calls += 1;
            walk::walk_call_expression(self, value);
        }
        fn visit_array_expression(&mut self, value: &oxc_ast::ast::ArrayExpression<'a>) {
            self.arrays += 1;
            walk::walk_array_expression(self, value);
        }
    }
    reset_class_map();
    reset_file_map();
    css::debug::set_debug(false);
    let output = extract(
        "test.tsx",
        source,
        ExtractOption {
            single_css,
            import_aliases: HashMap::from([("@emotion/react".into(), ImportAlias::NamedToNamed)]),
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("{error:?}"));
    let arena = oxc_allocator::Allocator::default();
    let parsed = oxc_parser::Parser::new(&arena, &output.code, oxc_span::SourceType::tsx()).parse();
    assert!(!parsed.fatal_error && parsed.diagnostics.is_empty());
    let mut work = Work::default();
    work.visit_program(&parsed.program);
    assert_eq!((work.calls, work.arrays), (0, 0), "{}", output.code);
    assert!(output.styles.iter().any(|style| matches!(style,ExtractStyleValue::Static(value) if value.property()=="color" && value.value()=="red")));
}
