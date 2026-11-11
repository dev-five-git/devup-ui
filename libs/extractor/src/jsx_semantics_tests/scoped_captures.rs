use super::*;
use oxc_ast_visit::{Visit, walk};
use rstest::rstest;
use serial_test::serial;

fn arrow_count(source: &str) -> usize {
    struct Arrows(usize);
    impl<'a> Visit<'a> for Arrows {
        fn visit_arrow_function_expression(
            &mut self,
            expression: &oxc_ast::ast::ArrowFunctionExpression<'a>,
        ) {
            self.0 += 1;
            walk::walk_arrow_function_expression(self, expression);
        }
    }
    let arena = oxc_allocator::Allocator::default();
    let parsed = oxc_parser::Parser::new(&arena, source, oxc_span::SourceType::tsx()).parse();
    assert!(
        parsed.diagnostics.is_empty(),
        "{source}: {:?}",
        parsed.diagnostics
    );
    let mut arrows = Arrows(0);
    arrows.visit_program(&parsed.program);
    arrows.0
}

#[rstest]
#[case("jsx")]
#[case("jsxs")]
#[case("jsxDEV")]
#[case("createElement")]
#[case("element")]
#[serial]
fn all_element_surfaces_keep_ordered_spread_reads_in_the_original_scope(
    #[case] surface: &str,
    #[values(false, true)] concise: bool,
) {
    let element = if surface == "element" {
        "<Box id={(trace.push('id'),'first')} {...read()} className={(trace.push('class'),'authored')} color='red'/>".to_string()
    } else {
        format!(
            "{surface}(Box,{{id:(trace.push('id'),'first'),...read(),className:(trace.push('class'),'authored'),color:'red'}})"
        )
    };
    let runtime = match surface {
        "jsx" | "jsxs" => format!("import {{{surface}}} from 'react/jsx-runtime';"),
        "jsxDEV" => "import {jsxDEV} from 'react/jsx-dev-runtime';".to_string(),
        "createElement" => "import {createElement} from 'react';".to_string(),
        _ => String::new(),
    };
    let source = if concise {
        format!("{BOX}{runtime}export const a=read=>{element};")
    } else {
        format!("{BOX}{runtime}export function a(read){{return {element};}}")
    };
    let compiled = code(&source);
    let actual = whole::execute(
        &format!("const jsxs=jsx,jsxDEV=jsx;{compiled}"),
        "a(()=>{trace.push('spread');return {get className(){trace.push('ownClass');return 'spread';},get style(){trace.push('ownStyle');return {opacity:.25};}};})",
    );
    assert_eq!(
        actual.trace,
        serde_json::json!(["id", "spread", "ownClass", "ownStyle", "class"])
    );
    assert_eq!(actual.element["props"]["id"], "first");
    assert_eq!(actual.element["props"]["style"]["opacity"], 0.25);
    assert!(
        actual.element["props"]["className"]
            .as_str()
            .is_some_and(|class| class.contains("authored"))
    );
}

#[rstest]
#[case("export const a=", "a")]
#[case("globalThis.view=", "globalThis.view")]
#[serial]
fn module_initializers_keep_their_spread_effects_at_the_original_position(
    #[case] prefix: &str,
    #[case] probe: &str,
) {
    let source = format!(
        "{BOX}trace.push('before');{prefix}<Box {{...(trace.push('spread'),{{id:'one'}})}} color='red'/>;trace.push('after');"
    );
    let compiled = code(&source);
    let actual = whole::execute(&compiled, probe);
    assert_eq!(
        actual.trace,
        serde_json::json!(["before", "spread", "after"])
    );
    assert_eq!(actual.element["props"]["id"], "one");
}

#[rstest]
#[case("<Box margin={read() ? `${size()}px` : undefined}/>")]
#[case("<Box margin={read() ? undefined : `${size()}px`}/>")]
#[case("<Box margin={read() ? `${size()}` : `${size()}`}/>")]
#[case("<Box m={{true:size(),false:size()}[read()]}/>")]
#[serial]
fn conditional_and_selected_style_packets_need_no_function_wrapper(#[case] element: &str) {
    let source = format!("{BOX}export const a=(read,size)=>{element};");
    let compiled = code(&source);
    let actual = whole::execute(
        &compiled,
        "[a(()=>{trace.push('yes');return true;},()=>{trace.push('size');return 2;}),a(()=>{trace.push('no');return false;},()=>{trace.push('size');return 3;})]",
    );
    let expected = if element.contains("true:") {
        serde_json::json!(["size", "size", "yes", "size", "size", "no"])
    } else if element.contains("undefined :") {
        serde_json::json!(["yes", "no", "size"])
    } else if element.contains(": undefined") {
        serde_json::json!(["yes", "size", "no"])
    } else {
        serde_json::json!(["yes", "size", "no", "size"])
    };
    assert_eq!(actual.trace, expected);
}

#[test]
#[serial]
fn sibling_captures_and_loop_callbacks_do_not_share_evaluated_values() {
    let source = format!(
        "{BOX}export function a(read){{const result=[];for(let index=0;index<3;index++){{result.push(<Box {{...read(index)}} color='red' onClick={{()=>index}}/>);}}return result;}}"
    );
    let compiled = code(&source);
    let actual = whole::execute(
        &compiled,
        "a(index=>{trace.push(index);return {id:index,className:'class'+index};}).map(element=>[element.props.id,element.props.onClick(),element.props.className])",
    );
    assert_eq!(actual.trace, serde_json::json!([0, 1, 2]));
    for index in 0..3 {
        assert_eq!(actual.element[index][0], index);
        assert_eq!(actual.element[index][1], index);
        assert!(
            actual.element[index][2]
                .as_str()
                .is_some_and(|class| class.contains(&format!("class{index}")))
        );
    }
}

#[test]
#[serial]
fn nested_reentrant_renders_have_distinct_capture_bindings() {
    let source = format!("{BOX}export const a=read=><Box {{...read()}} color='red'/>;");
    let compiled = code(&source);
    let actual = whole::execute(
        &compiled,
        "(()=>{const outer=a(()=>{const inner=a(()=>({id:'inner',className:'inside'}));trace.push(inner.props.id);return {id:'outer',className:'outside'};});return [outer.props.id,outer.props.className];})()",
    );
    assert_eq!(actual.trace, serde_json::json!(["inner"]));
    assert_eq!(actual.element[0], "outer");
    assert!(
        actual.element[1]
            .as_str()
            .is_some_and(|class| class.contains("outside") && !class.contains("inside"))
    );
}

#[rstest]
#[case("()=>42")]
#[case("async()=>await 42")]
#[case("function(){return 42;}")]
#[case("class { static value=42; }")]
#[serial]
fn scoped_argument_assignment_does_not_infer_a_temporary_function_name(#[case] value: &str) {
    let source =
        format!("{BOX}export const a=read=><Box onClick={{{value}}} {{...read()}} color='red'/>;");
    let compiled = code(&source);
    let actual = whole::execute(&compiled, "a(()=>({})).props.onClick.name");
    assert_eq!(
        actual.element,
        if value.starts_with("class ") {
            ""
        } else {
            "onClick"
        }
    );
}

#[rstest]
#[case(
    "export function a(read){eval('void 0');return <Box {...read()} color='red'/>;}",
    "a(()=>({id:'kept'}))"
)]
#[case(
    "const read=()=>({id:'kept'});export const a=(view=<Box {...read()} color='red'/>)=>view;",
    "a()"
)]
#[case(
    "const read=()=>({id:'kept'});class A{view=<Box {...read()} color='red'/>;}export const a=()=>new A().view;",
    "a()"
)]
#[serial]
fn expression_only_and_eval_visible_contexts_keep_a_private_capture_scope(
    #[case] body: &str,
    #[case] probe: &str,
) {
    let source = format!("{BOX}{body}");
    let compiled = code(&source);
    assert_eq!(
        arrow_count(&compiled),
        arrow_count(&source) + 1,
        "{compiled}"
    );
    let actual = whole::execute(&compiled, probe);
    assert_eq!(actual.element["props"]["id"], "kept");
}

#[test]
#[serial]
fn unrelated_async_callback_creation_does_not_require_a_capture_wrapper() {
    let source = format!(
        "{BOX}export const a=read=><Box {{...read()}} color='red' onClick={{async()=>await 42}}/>;"
    );
    let compiled = code(&source);
    let actual = whole::execute(&compiled, "a(()=>({id:'kept'})).props.id");
    assert_eq!(actual.element, "kept");
}

#[rstest]
#[case("<div css={{width:read()}}/>")]
#[case("jsx('div',{css:{width:read()}})")]
#[case("<div css={{width:read()}} style={{opacity:1}} className='authored'/>")]
#[serial]
fn numeric_css_conversion_uses_its_existing_capture(#[case] element: &str) {
    let source = format!("{EMOTION}{JSX_RUNTIME}export const a=read=>{element};");
    let compiled = compile_emotion(&source)
        .unwrap_or_else(|error| panic!("{error}"))
        .code;
    let actual = whole::execute(
        &compiled,
        "[3,0,'4rem',null,undefined,false].map(value=>Object.values(a(()=>{trace.push(String(value));return value;}).props.style)[0])",
    );
    assert_eq!(
        actual.trace,
        serde_json::json!(["3", "0", "4rem", "null", "undefined", "false"])
    );
    assert_eq!(
        actual.element,
        serde_json::json!(["3px", 0, "4rem", null, null, false])
    );
}

#[rstest]
#[case("<div css={{width:value}} style={{opacity:1}} className='authored'/>")]
#[case("jsx('div',{css:{width:value},style:{opacity:1},className:'authored'})")]
#[serial]
fn numeric_parameter_reads_need_no_wrapper_when_css_merges(#[case] element: &str) {
    let source = format!("{EMOTION}{JSX_RUNTIME}export const a=value=>{element};");
    let compiled = compile_emotion(&source)
        .unwrap_or_else(|error| panic!("{error}"))
        .code;
    let actual = whole::execute(&compiled, "[3,0,'4rem'].map(value=>a(value).props)");
    for (index, value) in [
        serde_json::json!("3px"),
        serde_json::json!(0),
        serde_json::json!("4rem"),
    ]
    .into_iter()
    .enumerate()
    {
        let style = actual.element[index]["style"]
            .as_object()
            .unwrap_or_else(|| panic!("style object"));
        assert!(
            style
                .iter()
                .any(|(name, slot)| name.starts_with("--") && slot == &value)
        );
        assert_eq!(style["opacity"], 1);
    }
}

#[test]
#[serial]
fn ordinary_callbacks_keep_their_original_identity_and_name() {
    let source = format!(
        "{BOX}export const a=(read,callback)=><Box onClick={{callback}} {{...read()}} color='red'/>;"
    );
    let compiled = code(&source);
    let actual = whole::execute(
        &compiled,
        "(()=>{function original(){}const element=a(()=>({}),original);return [element.props.onClick===original,element.props.onClick.name];})()",
    );
    assert_eq!(actual.element, serde_json::json!([true, "original"]));
}

#[rstest]
#[serial]
fn suspended_generators_keep_each_invocations_captures(#[values(false, true)] throws: bool) {
    let source = format!(
        "{BOX}export function* a(read){{try{{return <Box {{...read()}} title={{yield 'pause'}} color='red'/>;}}catch(error){{return error;}}}}"
    );
    let compiled = code(&source);
    let actual = whole::execute(
        &compiled,
        &format!(
            "(()=>{{const first=a(()=>({{id:'first',className:'one'}}));const second=a(()=>({{id:'second',className:'two'}}));const started=[first.next().value,second.next().value];return [started,second.next('second-title').value,{}];}})()",
            if throws {
                "first.throw('boom').value"
            } else {
                "first.next('first-title').value"
            }
        ),
    );
    assert_eq!(actual.element[0], serde_json::json!(["pause", "pause"]));
    assert_eq!(actual.element[1]["props"]["id"], "second");
    assert_eq!(actual.element[1]["props"]["title"], "second-title");
    if throws {
        assert_eq!(actual.element[2], "boom");
    } else {
        assert_eq!(actual.element[2]["props"]["id"], "first");
        assert_eq!(actual.element[2]["props"]["title"], "first-title");
    }
}

#[rstest]
#[serial]
fn concurrent_async_renders_keep_values_across_suspension(#[values(false, true)] rejects: bool) {
    let source = format!(
        "{BOX}export async function a(read,wait){{try{{return <Box {{...read()}} title={{await wait}} color='red'/>;}}catch(error){{return error;}}}}"
    );
    let compiled = code(&source);
    let script = format!(
        "const h=(type,props)=>({{type,props}});{}\n(async()=>{{let resume;const wait=new Promise(resolve=>resume=resolve);const first=a(()=>({{id:'first',className:'one'}}),wait);const second=await a(()=>({{id:'second',className:'two'}}),{});resume('first-title');process.stdout.write(JSON.stringify([await first,second]));}})().catch(error=>{{console.error(error);process.exitCode=1;}});",
        whole::plain_script(&compiled),
        if rejects {
            "Promise.reject('boom')"
        } else {
            "Promise.resolve('second-title')"
        }
    );
    let actual = std::process::Command::new("node")
        .args(["-e", &script])
        .output()
        .unwrap_or_else(|error| panic!("Node is required for async capture checks: {error}"));
    assert!(
        actual.status.success(),
        "{}",
        String::from_utf8_lossy(&actual.stderr)
    );
    let actual: serde_json::Value = serde_json::from_slice(&actual.stdout)
        .unwrap_or_else(|error| panic!("{error}: {}", String::from_utf8_lossy(&actual.stdout)));
    assert_eq!(actual[0]["props"]["id"], "first");
    assert_eq!(actual[0]["props"]["title"], "first-title");
    if rejects {
        assert_eq!(actual[1], "boom");
    } else {
        assert_eq!(actual[1]["props"]["id"], "second");
        assert_eq!(actual[1]["props"]["title"], "second-title");
    }
}
