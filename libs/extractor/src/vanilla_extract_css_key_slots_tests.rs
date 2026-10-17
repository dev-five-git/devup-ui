use super::super::{Collector, StyleCollector, StyleEntry, register_vanilla_extract_apis};
use crate::{ExtractOption, ExtractOutput, extract_without_source_map, own_css_plan::Node};
use boa_engine::{Context, Source};
use oxc_allocator::Allocator;
use oxc_ast::ast::Expression;
use oxc_parser::Parser;
use oxc_span::SourceType;
use std::{cell::RefCell, rc::Rc};

fn native_call(
    collector: &StyleCollector,
    context: &mut Context,
    source: &str,
) -> Result<StyleEntry, String> {
    let id = context
        .eval(Source::from_bytes(source))
        .map_err(|error| error.to_string())?;
    let id = id
        .as_string()
        .ok_or("css must return a class")?
        .to_std_string_escaped();
    collector
        .borrow()
        .styles
        .styles
        .get(&id)
        .cloned()
        .ok_or_else(|| "missing captured call".into())
}

fn generated_cardinality(entry: &StyleEntry) -> Result<(usize, usize), String> {
    let allocator = Allocator::default();
    let Ok(expression) = Parser::new(
        &allocator,
        entry.css.as_deref().ok_or("missing generated call")?,
        SourceType::tsx(),
    )
    .parse_expression() else {
        panic!("generated bounded call must parse");
    };
    let Expression::CallExpression(call) = expression else {
        panic!("expected generated call");
    };
    let Some(Expression::ArrayExpression(array)) = call.arguments[0].as_expression() else {
        return Err("expected bounded-call array".into());
    };
    let wrappers = array
        .elements
        .iter()
        .filter_map(|item| item.as_expression())
        .filter(|item| {
            matches!(
                crate::utils::unwrap_syntax_only(item),
                Expression::ObjectExpression(_)
            )
        })
        .count();
    Ok((array.elements.len(), wrappers))
}

#[rstest::rstest]
#[case(16)]
#[case(256)]
#[case(4096)]
fn key_slots_bound_generated_ast_when_keys_repeat(#[case] depth: usize) -> Result<(), String> {
    // Given: the native API captures every object in a heap-built chain.
    let collector = Rc::new(RefCell::new(Collector::default()));
    let mut context = Context::default();
    register_vanilla_extract_apis(&mut context, &collector)?;
    let source = format!(
        "let rules={{p:2}};for(let i=0;i<{depth};i++){{rules={{_hover:rules}}}}__vanilla_extract__.css(rules);"
    );
    // When
    let entry = native_call(&collector, &mut context, &source)?;
    // Then: graph occurrences survive; only generated key expressions share slots.
    let plan = entry.own_plan.as_ref().ok_or("missing deep plan")?;
    assert_eq!(plan.nodes.len(), depth + 2);
    let mut id = plan.roots[0];
    for _ in 0..depth {
        let Node::Object(properties) = &plan.nodes[id] else {
            return Err("lost object occurrence".into());
        };
        assert_eq!(properties.len(), 1);
        assert_eq!(properties[0].name, "_hover");
        id = properties[0].value;
    }
    let Node::Object(properties) = &plan.nodes[id] else {
        return Err("lost leaf object".into());
    };
    assert_eq!(properties.len(), 1);
    assert_eq!(properties[0].name, "p");
    assert!(matches!(plan.nodes[properties[0].value], Node::Scalar(_)));
    assert_eq!(generated_cardinality(&entry)?, (3, 2));
    Ok(())
}

#[test]
fn key_slots_use_each_calls_known_expression_snapshot() -> Result<(), String> {
    // Given: identical authored keys become known identifiers only in the second call.
    let collector = Rc::new(RefCell::new(Collector::default()));
    let mut context = Context::default();
    register_vanilla_extract_apis(&mut context, &collector)?;
    let source = concat!(
        "(()=>{let rules={",
        "p:2};for(let i=0;i<16;i++){rules={_hover:rules}}return __vanilla_extract__.css(rules)})()"
    );
    let first = native_call(&collector, &mut context, source)?;
    collector
        .borrow_mut()
        .styles
        .styles
        .insert("_hover".into(), StyleEntry::default());
    // When
    let second = native_call(&collector, &mut context, source)?;
    // Then: no cross-call slot indices or raw-name-to-expression cache.
    assert_eq!(
        first.css.as_deref(),
        Some("css([({[\"_hover\"]:0}),({[\"p\"]:0}),2])")
    );
    assert_eq!(
        second.css.as_deref(),
        Some("css([({[_hover]:0}),({[\"p\"]:0}),2])")
    );
    assert_eq!(generated_cardinality(&first)?, (3, 2));
    assert_eq!(generated_cardinality(&second)?, (3, 2));
    Ok(())
}

fn compile(source: &str) -> Result<ExtractOutput, String> {
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    extract_without_source_map("/key-slots.css.ts", source, ExtractOption::default())
        .map_err(|error| error.to_string())
}

#[test]
#[serial_test::serial]
fn key_slots_keep_identical_scalar_expression_separate() -> Result<(), String> {
    // Given: the scalar is read before an identical key expression is captured.
    let selector = format!("&{}", ":hover".repeat(16));
    let expected = compile(&format!(
        "import{{css}}from'@devup-ui/react';export const card=css({{fontFamily:'_hover',selectors:{{'{selector}':{{fontFamily:'_hover'}}}}}});"
    ))?;
    let source = concat!(
        "import{css}from'@devup-ui/react';let rules={fontFamily:'_hover'};",
        "for(let i=0;i<16;i++){rules={_hover:rules}}",
        "export const card=css({fontFamily:'_hover',...rules});"
    );
    // When
    let output = compile(source)?;
    // Then
    assert_eq!(output.code, expected.code);
    assert_eq!(output.styles, expected.styles);
    Ok(())
}

#[test]
#[serial_test::serial]
fn key_slots_keep_separate_calls_and_known_class_selectors() -> Result<(), String> {
    // Given: two bounded calls have different slot order and registered references.
    let selector = format!("&{}", ":hover".repeat(16));
    let expected = compile(&format!(
        "import{{css}}from'@devup-ui/react';const base=css({{color:'red'}});export const first=css({{selectors:{{[`${{base}} &`]:{{selectors:{{'{selector}':{{p:2}}}}}}}}}});export const second=css(base,{{selectors:{{'{selector}':{{fontFamily:'_hover'}}}}}});"
    ))?;
    let source = concat!(
        "import{css}from'@devup-ui/react';const base=css({color:'red'});",
        "let a={p:2},b={fontFamily:'_hover'};for(let i=0;i<16;i++){a={_hover:a};b={_hover:b}}",
        "export const first=css({selectors:{[`${base} &`]:a}});export const second=css(base,b);"
    );
    // When
    let output = compile(source)?;
    // Then
    assert_eq!(output.code, expected.code);
    assert_eq!(output.styles, expected.styles);
    Ok(())
}
