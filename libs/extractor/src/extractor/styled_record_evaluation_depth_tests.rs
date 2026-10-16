use boa_engine::{Context, Source};
use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::{ast::Statement, builder::AstBuilder};
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;

use super::{Evaluations, bind_render};
use crate::utils::{call_with_values, readable_code};

#[rstest]
fn ordered_captures_when_many_guarded_reads_depend_on_prior_bindings(
    #[values(false, true)] outer: bool,
    #[values(false, true)] inner: bool,
) {
    // Given: shallow authored expressions, reserved names and dependent guarded reads.
    let allocator = Allocator::default();
    let builder = AstBuilder::new(&allocator);
    let mut reads = Evaluations::new(&builder, "__devupRecord0 __devupRecord1".to_string());
    let parse = |source: &str| {
        let parsed = Parser::new(&allocator, source, SourceType::mjs()).parse();
        assert_eq!(parsed.diagnostics.len(), 0);
        let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
            panic!("capture fixture must be an expression");
        };
        statement.expression.clone_in(&allocator)
    };
    let outer_read = reads.capture(
        &parse("(trace.push('outer'), outer);"),
        &reads.boolean(true),
    );
    let inner_read = reads.capture(&parse("(trace.push('inner'), inner);"), &outer_read);
    let guard = reads.choose(&outer_read, &inner_read, &reads.boolean(false));
    let mut previous = "-1".to_string();
    let mut values = Vec::new();
    for index in 0..32 {
        let value = reads.capture(
            &parse(&format!("(trace.push('leaf:{index}'), {previous} + 1);")),
            &guard,
        );
        previous = readable_code(&value);
        values.push(previous.clone());
    }
    let body = parse(&format!(
        "JSON.stringify([__devupRecord0,__devupRecord1,[{}].filter(v=>v!==undefined),trace]);",
        values.join(",")
    ));

    // When: parse and execute production binding emission at the default test stack.
    let render = bind_render(&builder, reads.bindings, body);
    let script = format!(
        "const trace=[];const outer={outer},inner={inner},__devupRecord0='authored',__devupRecord1='collision';{};",
        readable_code(&render)
    );
    let mut context = Context::default();
    let result = context
        .eval(Source::from_bytes(script.as_bytes()))
        .unwrap_or_else(|error| panic!("dependent capture execution failed: {error}"));
    let result = result
        .as_string()
        .unwrap_or_else(|| panic!("dependent capture observation must be JSON"))
        .to_std_string_escaped();

    // Then: every selected read runs once in order; absent branches never read leaves.
    let mut trace = vec!["outer".to_string()];
    let mut expected_values = Vec::new();
    if outer {
        trace.push("inner".to_string());
        if inner {
            for index in 0..32 {
                trace.push(format!("leaf:{index}"));
                expected_values.push(index);
            }
        }
    }
    let expected = serde_json::to_string(&("authored", "collision", expected_values, trace))
        .unwrap_or_else(|error| panic!("capture observation serialization failed: {error}"));
    assert_eq!(result, expected);
}

#[rstest]
#[case::class("class {static {trace.push(this.name)}}", "")]
#[case::function("function(){trace.push(arguments.callee.name)}", "controller();")]
fn anonymous_names_when_prepared_controller_is_bound(
    #[case] initializer: &str,
    #[case] observe: &str,
    #[values(false, true)] bounded: bool,
) {
    // Given: the authored styled conditional spread reaches typed preparation.
    let allocator = Allocator::default();
    let builder = AstBuilder::new(&allocator);
    let source = format!("styled.div({{color:'orange',...({initializer}?{{color:'red'}}:{{}})}});");
    let parsed = Parser::new(&allocator, &source, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
        panic!("styled fixture must be an expression");
    };
    let oxc_ast::ast::Expression::CallExpression(call) = &statement.expression else {
        panic!("styled fixture must be a call");
    };
    let rules = call.arguments[0].to_expression();
    let prepared = super::super::styled_record::prepare(&builder, rules, &statement.expression)
        .unwrap_or_else(|| panic!("anonymous controller must require preparation"));
    assert_eq!(prepared.bindings.len(), 1);
    let body_source = format!(
        "((record,controller)=>{{{observe}return JSON.stringify([record.color,controller.name,trace]);}})({},{});",
        readable_code(&prepared.rules),
        prepared.bindings[0].0
    );
    let body = Parser::new(&allocator, &body_source, SourceType::tsx()).parse();
    assert_eq!(body.diagnostics.len(), 0);
    let Statement::ExpressionStatement(body) = &body.program.body[0] else {
        panic!("observation must be an expression");
    };

    // When: execute either the old argument seam or the production bounded binder.
    let body = body.expression.clone_in(&allocator);
    let render = if bounded {
        bind_render(&builder, prepared.bindings, body)
    } else {
        call_with_values(&builder, prepared.bindings, body)
    };
    let script = format!("const trace=[];{};", readable_code(&render));
    let mut context = Context::default();
    let result = context
        .eval(Source::from_bytes(script.as_bytes()))
        .unwrap_or_else(|error| panic!("prepared anonymous controller execution failed: {error}"));
    let result = result
        .as_string()
        .unwrap_or_else(|| panic!("anonymous controller observation must be JSON"))
        .to_std_string_escaped();

    // Then: later color wins and static/self-name observers see the original empty name.
    assert_eq!(result, r#"["red","",[""]]"#);
}

#[rstest]
#[case::class("class {static {trace.push(this.name)}}", "")]
#[case::function("function(){trace.push(arguments.callee.name)}", "value();")]
fn anonymous_names_when_capture_initializer_is_bound(
    #[case] initializer: &str,
    #[case] observe: &str,
    #[values(false, true)] bounded: bool,
) {
    // Given: an anonymous expression captured under the production literal-true guard.
    let allocator = Allocator::default();
    let builder = AstBuilder::new(&allocator);
    let source = format!("({initializer});");
    let parsed = Parser::new(&allocator, &source, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
        panic!("anonymous fixture must be an expression");
    };
    let mut reads = Evaluations::new(&builder, source.clone());
    let value = reads.capture(&statement.expression, &reads.boolean(true));
    let body_source = format!(
        "(value=>{{{observe}return JSON.stringify([value.name,trace]);}})({});",
        readable_code(&value)
    );
    let body = Parser::new(&allocator, &body_source, SourceType::tsx()).parse();
    assert_eq!(body.diagnostics.len(), 0);
    let Statement::ExpressionStatement(body) = &body.program.body[0] else {
        panic!("observation must be an expression");
    };

    // When: execute either the old argument seam or the production bounded binder.
    let body = body.expression.clone_in(&allocator);
    let render = if bounded {
        bind_render(&builder, reads.bindings, body)
    } else {
        call_with_values(&builder, reads.bindings, body)
    };
    let script = format!("const trace=[];{};", readable_code(&render));
    let mut context = Context::default();
    let result = context
        .eval(Source::from_bytes(script.as_bytes()))
        .unwrap_or_else(|error| panic!("anonymous capture execution failed: {error}"));
    let result = result
        .as_string()
        .unwrap_or_else(|| panic!("anonymous capture observation must be JSON"))
        .to_std_string_escaped();

    // Then: both the value's name and its authored observer retain argument semantics.
    assert_eq!(result, r#"["",[""]]"#);
}
