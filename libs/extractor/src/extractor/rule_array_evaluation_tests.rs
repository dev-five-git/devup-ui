use boa_engine::{Context, Source};
use oxc_allocator::Allocator;
use oxc_ast::builder::AstBuilder;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;

use super::super::styled_record_evaluation::{Evaluations, bind_render};
use crate::utils::readable_code;

#[rstest]
#[case::object_and(
    "({x:read('left',1)}) && [{x:read('right',2)}]",
    r#"[[{"x":2}],["left","right"]]"#
)]
#[case::array_or(
    "[{x:read('left',1)}] || [{x:read('right',2)}]",
    r#"[[{"x":1}],["left"]]"#
)]
#[case::object_coalesce(
    "({x:read('left',1)}) ?? [{x:read('right',2)}]",
    r#"[{"x":1},["left"]]"#
)]
#[case::null_coalesce(
    "read('left',null) ?? [{x:read('right',2)}]",
    r#"[[{"x":2}],["left","right"]]"#
)]
#[case::zero_coalesce("read('left',0) ?? [{x:read('right',2)}]", r#"[0,["left"]]"#)]
#[case::false_coalesce("read('left',false) ?? [{x:read('right',2)}]", r#"[false,["left"]]"#)]
#[case::spread(
    "[...[{x:read('spread',3)}],{x:read('tail',4)}]",
    r#"[[{"x":3},{"x":4}],["spread","tail"]]"#
)]
fn repair_745_rule_array_captures_preserve_evaluation(
    #[case] source: &str,
    #[case] expected: &str,
) {
    // Given: authored evaluations with observable order and short circuiting.
    let allocator = Allocator::default();
    let builder = AstBuilder::new(&allocator);
    let expression = Parser::new(&allocator, source, SourceType::mjs())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));
    let mut reads = Evaluations::new(&builder, source.to_string());

    // When: execute the prepared expression through the production binder.
    let prepared = super::prepare(&mut reads, &expression, |_| false)
        .unwrap_or_else(|| panic!("capturable fixture"));
    let render = bind_render(&builder, reads.bindings, prepared);
    let script = format!(
        "const trace=[];const read=(name,value)=>(trace.push(name),value);JSON.stringify([{},trace]);",
        readable_code(&render)
    );
    let observed = Context::default()
        .eval(Source::from_bytes(script.as_bytes()))
        .unwrap_or_else(|error| panic!("{error}"))
        .as_string()
        .unwrap_or_else(|| panic!("JSON observation"))
        .to_std_string_escaped();

    // Then: the chosen value and each once-only evaluation retain authored order.
    assert_eq!(observed, expected);
}

#[rstest]
#[case::await_value("[{color:await tone}]")]
#[case::yield_value("[{color:yield tone}]")]
#[case::getter("[{get color(){return 'red'}}]")]
#[case::setter("[{set color(value){}}]")]
#[case::method("[{color(){return 'red'}}]")]
fn repair_745_rule_array_refuses_unpreparable_values(#[case] source: &str) {
    // Given: a parsed suspension or non-data property that cannot be captured.
    let allocator = Allocator::default();
    let builder = AstBuilder::new(&allocator);
    let source = if source.contains("yield") {
        format!("function*(){{return {source};}}")
    } else {
        source.to_string()
    };
    let expression = Parser::new(&allocator, &source, SourceType::mjs())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));
    let expression = match &expression {
        oxc_ast::ast::Expression::FunctionExpression(function) => crate::css_prop::returned(
            function
                .body
                .as_deref()
                .unwrap_or_else(|| panic!("generator body")),
        )
        .unwrap_or_else(|| panic!("returned fixture")),
        expression => expression,
    };
    let mut reads = Evaluations::new(&builder, source.clone());

    // When: attempt preparation at the capture boundary.
    let prepared = super::prepare(&mut reads, expression, |_| false);

    // Then: the original refusal remains explicit, rather than capturing invalid syntax.
    assert!(prepared.is_none());
}

#[rstest]
#[case("read() ? [] : []")]
#[case("(read() ? [] : []) || (other && [])")]
fn repair_745_refused_controllers_keep_authored_spans(#[case] source: &str) {
    // Given: impure controllers nested on either side of a logical composition.
    let allocator = Allocator::default();
    let expression = Parser::new(&allocator, source, SourceType::mjs())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));
    let mut refused = Vec::new();

    // When: collect controller refusals before any rewriting.
    super::super::styled_rule_arrays::refused_controllers(&expression, &mut refused);

    // Then: the call-controlled choice remains refused at its authored start.
    let start = u32::try_from(
        source
            .find("read()")
            .unwrap_or_else(|| panic!("controller fixture")),
    )
    .unwrap_or_else(|error| panic!("{error}"));
    assert!(refused.contains(&start), "{refused:?}");
}
