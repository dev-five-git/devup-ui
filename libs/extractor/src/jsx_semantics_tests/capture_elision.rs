use super::*;
use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::{
    ast::{Expression, Statement},
    builder::AstBuilder,
};
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;
use serial_test::serial;

fn lowered(values: &[(&str, &str)], body: &str) -> (String, usize) {
    let arena = Allocator::default();
    let ast = AstBuilder::new(&arena);
    let parse = |source: &str| {
        let wrapped = arena.alloc_str(&format!("function* parse(){{return ({source});}}"));
        let parsed = Parser::new(&arena, wrapped, SourceType::tsx()).parse();
        assert!(
            parsed.diagnostics.is_empty(),
            "{:?}: {source}",
            parsed.diagnostics
        );
        let Statement::FunctionDeclaration(function) = &parsed.program.body[0] else {
            panic!("expected function");
        };
        let Some(body) = &function.body else {
            panic!("expected body");
        };
        let Statement::ReturnStatement(returned) = &body.statements[0] else {
            panic!("expected return");
        };
        returned
            .argument
            .as_ref()
            .unwrap_or_else(|| panic!("expected value"))
            .clone_in(&arena)
    };
    let expression = crate::utils::call_with_values(
        &ast,
        values
            .iter()
            .map(|(name, value)| ((*name).to_string(), parse(value)))
            .collect(),
        parse(body),
    );
    let parameters = match &expression {
        Expression::CallExpression(call) => match crate::utils::unwrap_syntax_only(&call.callee) {
            Expression::ArrowFunctionExpression(arrow) => arrow.params.items.len(),
            _ => 0,
        },
        _ => 0,
    };
    (crate::utils::readable_code(&expression), parameters)
}

fn baseline(values: &[(&str, &str)], body: &str) -> String {
    format!(
        "(({})=>({body}))({})",
        values
            .iter()
            .map(|(name, _)| *name)
            .collect::<Vec<_>>()
            .join(","),
        values
            .iter()
            .map(|(_, value)| format!("({value})"))
            .collect::<Vec<_>>()
            .join(",")
    )
}

#[rstest]
#[case("<div style={{'--v':slot}}>{trace.push('child')}</div>", 0)]
#[case("<div id={trace.push('prefix')} style={{'--v':slot}}/>", 1)]
#[case("<div style={{'--a':slot,'--b':slot}}/>", 1)]
#[case("<div a={<span/>} b={slot}/>", 1)]
#[case("<div a={<></>} b={slot}/>", 1)]
#[case("<div a=<span/> b={slot}/>", 1)]
#[case("<div>text{slot}</div>", 0)]
#[case("<div>{trace.push('prefix')}{slot}</div>", 1)]
#[case("<div><span/>{slot}</div>", 1)]
#[case("<div {...{}} style={{'--v':slot}}/>", 1)]
#[case("<div {...slot}/>", 0)]
#[case("<div enabled title='text' value={slot}/>", 0)]
#[case("<Component value={slot}/>", 1)]
#[case("slot ? 'yes' : 'no'", 0)]
#[case("false ? slot : 'no'", 1)]
#[case("slot || ''", 0)]
#[case("false && slot", 1)]
#[case("slot + 'px'", 0)]
#[case("'prefix' + slot", 0)]
#[case("missing + slot", 1)]
#[case("!slot", 0)]
#[case("+slot", 0)]
#[case("void slot", 0)]
#[case("typeof slot", 1)]
#[case("delete slot.x", 1)]
#[case("slot.length", 0)]
#[case("({red:'class'})[slot]", 0)]
#[case("missing[slot]", 1)]
#[case("null?.[slot]", 1)]
#[case("null?.[slot].length", 1)]
#[case("slot?.length", 0)]
#[case("slot?.['length']", 0)]
#[case("slot?.()", 1)]
#[case("`prefix-${slot}`", 0)]
#[case("`${trace.push('prefix')}-${slot}`", 1)]
#[case("(1, 2, slot)", 0)]
#[case("((1, 2), slot)", 0)]
#[case("(trace.push('prefix'), slot)", 1)]
#[case("[,,slot]", 0)]
#[case("[[1], slot]", 0)]
#[case("[...slot]", 0)]
#[case("[...[], slot]", 1)]
#[case("[{x: 1},slot]", 0)]
#[case("[<div/>,slot]", 1)]
#[case("[`${'a'}`,slot]", 0)]
#[case("[missing && 1,slot]", 1)]
#[case("[(missing ? 1 : 2),slot]", 1)]
#[case("[false && 1, slot]", 0)]
#[case("[true ? 1 : 2, slot]", 0)]
#[case("[null, true, 1n, slot].length", 0)]
#[case("({x:1, value:slot})", 0)]
#[case("({slot})", 0)]
#[case("({...slot})", 0)]
#[case("({[slot]:1})", 1)]
#[case("({get x(){return slot;}})", 1)]
#[case("({method(){return slot;}})", 1)]
#[case("()=>slot", 1)]
#[case("function(){return slot;}", 1)]
#[case("class {static value=slot;}", 1)]
#[case("slot=1", 1)]
#[case("slot++", 1)]
#[case("eval('slot')", 1)]
#[case("[slot, (eval)('slot')]", 1)]
#[case("'unused'", 1)]
fn capture_elision_preserves_native_values_effects_and_throw_order(
    #[case] body: &str,
    #[case] retained: usize,
) {
    let values = [("slot", "(trace.push('value'),'red')")];
    let (compiled, parameters) = lowered(&values, body);
    assert_eq!(parameters, retained, "{body}: {compiled}");
    let probe = "(()=>{try{return ['value',a()];}catch(error){return ['error',String(error)];}})()";
    let expected = whole::execute(
        &format!("export const a=()=>({});", baseline(&values, body)),
        probe,
    );
    let actual = whole::execute(&format!("export const a=()=>({compiled});"), probe);
    assert_eq!(actual.element, expected.element, "{body}: {compiled}");
    assert_eq!(actual.trace, expected.trace, "{body}: {compiled}");
}

#[rstest]
#[case("function(){}")]
#[case("()=>{}")]
#[case("class {}")]
#[case("eval(\"trace.push('eval');'red'\")")]
#[case("(eval)(\"trace.push('eval');'red'\")")]
fn capture_elision_keeps_name_inference_and_direct_eval_scope(#[case] value: &str) {
    let values = [("slot", value)];
    let body = "({fn: slot}).fn.name";
    let (compiled, parameters) = lowered(&values, body);
    assert_eq!(parameters, 1, "{compiled}");
    let expected = whole::execute(
        &format!("export const a=()=>({});", baseline(&values, body)),
        "a()",
    );
    let actual = whole::execute(&format!("export const a=()=>({compiled});"), "a()");
    assert_eq!(actual.element, expected.element);
    assert_eq!(actual.trace, expected.trace);
}

#[rstest]
#[case("<div id={first} style={{'--v':last}}/>", 0)]
#[case("<div id={last} style={{'--v':first}}/>", 1)]
#[case("`${first} ${last}`", 0)]
#[case("first + last", 0)]
fn multiple_capture_elision_preserves_argument_order(#[case] body: &str, #[case] retained: usize) {
    let values = [
        ("first", "(trace.push('first'),'a')"),
        ("last", "(trace.push('last'),'b')"),
    ];
    let (compiled, parameters) = lowered(&values, body);
    assert_eq!(parameters, retained, "{compiled}");
    let expected = whole::execute(
        &format!("export const a=()=>({});", baseline(&values, body)),
        "a()",
    );
    let actual = whole::execute(&format!("export const a=()=>({compiled});"), "a()");
    assert_eq!(actual.element, expected.element);
    assert_eq!(actual.trace, expected.trace);
    assert_eq!(actual.trace, serde_json::json!(["first", "last"]));
}

#[test]
fn template_captures_keep_object_coercion_after_later_arguments() {
    let values = [
        ("first", "({toString(){trace.push('coerce');return 'a';}})"),
        ("last", "(trace.push('last'),'b')"),
    ];
    let (compiled, parameters) = lowered(&values, "`${first} ${last}`");
    assert_eq!(parameters, 2, "{compiled}");
    let actual = whole::execute(&format!("export const a=()=>({compiled});"), "a()");
    assert_eq!(actual.element, "a b");
    assert_eq!(actual.trace, serde_json::json!(["last", "coerce"]));
}

#[rstest]
#[case("null")]
#[case("undefined")]
#[case("false")]
#[case("0")]
#[case("Symbol('class')")]
#[case("({[Symbol.toPrimitive](hint){trace.push(hint);return 'class';}})")]
#[case("({toString(){trace.push('coerce');throw Error('coercion');}})")]
fn inlined_template_capture_keeps_one_string_conversion(#[case] value: &str) {
    let values = [("slot", "`${read()}`")];
    let body = "`prefix-${slot} external`";
    let (compiled, parameters) = lowered(&values, body);
    assert_eq!(parameters, 0, "{compiled}");
    assert_eq!(compiled, "`prefix-${read()} external`");
    let probe = "(()=>{try{return ['value',a()];}catch(error){return ['error',String(error)];}})()";
    let prefix = format!("function read(){{trace.push('read');return {value};}}");
    let expected = whole::execute(
        &format!("{prefix}export const a=()=>({});", baseline(&values, body)),
        probe,
    );
    let actual = whole::execute(&format!("{prefix}export const a=()=>({compiled});"), probe);
    assert_eq!(actual.element, expected.element);
    assert_eq!(actual.trace, expected.trace);
}

#[rstest]
#[case("`${slot} external`", "`prefix-${read()}`", 0)]
#[case("tag`${slot} external`", "`${read()}`", 1)]
fn inlined_template_capture_keeps_text_and_tagged_template_values(
    #[case] body: &str,
    #[case] value: &str,
    #[case] retained: usize,
) {
    let values = [("slot", value)];
    let (compiled, parameters) = lowered(&values, body);
    assert_eq!(parameters, retained, "{compiled}");
    let prefix = "function read(){trace.push('read');return {toString(){trace.push('coerce');return 'class';}};}function tag(parts,value){return [parts.raw,value];}";
    let expected = whole::execute(
        &format!("{prefix}export const a=()=>({});", baseline(&values, body)),
        "a()",
    );
    let actual = whole::execute(&format!("{prefix}export const a=()=>({compiled});"), "a()");
    assert_eq!(actual.element, expected.element);
    assert_eq!(actual.trace, expected.trace);
}

#[test]
#[serial]
fn emotion_class_template_keeps_the_original_flat_interpolation() {
    let source = "import {css} from '@emotion/react';const override=css({color:'blue'});export const a=()=> <div css={{color:'red',margin:0}} className={`${override} external`}/>;";
    let compiled = compile_emotion(source)
        .unwrap_or_else(|error| panic!("{error}: {source}"))
        .code;
    assert!(
        compiled.contains("className={`${override} external "),
        "{compiled}"
    );
    assert!(!compiled.contains("__devupValue"), "{compiled}");
    let actual = whole::execute(&compiled, "a()");
    assert_eq!(
        actual.element["props"]["className"],
        "color-0-blue--255-a external margin-0-0--255-a"
    );
}

#[test]
fn capture_elision_keeps_yield_in_the_original_generator_scope() {
    let values = [("slot", "yield (trace.push('yield'),'pause')")];
    let (compiled, parameters) = lowered(&values, "<div>{slot}</div>");
    assert_eq!(parameters, 1, "{compiled}");
    let actual = whole::execute(
        &format!("export function* a(){{return ({compiled});}}"),
        "(()=>{const iterator=a();trace.push(iterator.next().value);return iterator.next('child').value;})()",
    );
    assert_eq!(actual.element["children"], serde_json::json!(["child"]));
    assert_eq!(actual.trace, serde_json::json!(["yield", "pause"]));
}

#[test]
#[serial]
fn typography_table_with_children_has_no_capture_wrapper() {
    let source = "import {Text} from '@devup-ui/react';export const a=(size,child)=><Text typography={({lg:'buttonLg',md:'button'} as const)[size]}>{child()}</Text>;";
    let compiled = code(source);
    assert!(!compiled.contains("__devupChoice"), "{compiled}");
    let actual = whole::evaluate(
        source,
        "a({toString(){trace.push('key');return 'lg';}},()=>{trace.push('child');return 'text';})",
    );
    assert_eq!(actual.element["props"]["className"], "typo-buttonLg");
    assert_eq!(actual.element["children"], serde_json::json!(["text"]));
    assert_eq!(actual.trace, serde_json::json!(["key", "child"]));
}

#[rstest]
#[case(true, serde_json::json!(["read", "child"]))]
#[case(false, serde_json::json!(["child"]))]
#[serial]
fn branch_payload_with_encoded_style_reads_keeps_its_capture(
    #[case] on: bool,
    #[case] trace: serde_json::Value,
) {
    let source = format!(
        "{BOX}export const a=(on,read,child)=><Box w={{on ? `${{read()}}px` : undefined}}>{{child()}}</Box>;"
    );
    let actual = whole::evaluate(
        &source,
        &format!(
            "a({on},()=>{{trace.push('read');return 12;}},()=>{{trace.push('child');return 'text';}})"
        ),
    );
    assert_eq!(actual.trace, trace);
    assert_eq!(actual.element["children"], serde_json::json!(["text"]));
    let style = actual.element["props"]["style"]
        .as_object()
        .unwrap_or_else(|| panic!("missing style: {}", actual.element));
    if on {
        assert_eq!(
            style.values().collect::<Vec<_>>(),
            vec![&serde_json::json!("12px")]
        );
    } else {
        assert!(style.is_empty());
    }
}
