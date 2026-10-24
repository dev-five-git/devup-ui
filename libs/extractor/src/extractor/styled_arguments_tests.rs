use boa_engine::{Context as BoaContext, Source};
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;

use super::*;
use crate::extractor::styled_record_evaluation::bind_render;

#[test]
fn callback_preparation_when_a_responsive_spread_refuses_capture_keeps_the_authored_schedule() {
    // Given: the callback can be lowered, but its enclosing record cannot be captured.
    let allocator = Allocator::default();
    let builder = AstBuilder::new(&allocator);
    let source = "styled({color:p=>p.color,margin:[...values]})";
    let mut scope = Parser::new(&allocator, source, SourceType::mjs())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));
    let mut expression = scope.clone_in(&allocator);
    let Expression::CallExpression(call) = &mut expression else {
        panic!("expected styled call")
    };
    let mut reads = Reads::default();
    // When: argument preparation attempts and refuses the complete callback record.
    let prepared = prepare(
        &builder,
        call,
        Context {
            filename: "arguments.tsx",
            scope: &mut scope,
            reads: &mut reads,
            refused: &[],
        },
    );
    // Then: no partial binding escapes; the callback runs once and the responsive spread remains intact.
    assert_eq!(prepared.errors, vec![]);
    assert_eq!(prepared.bindings.len(), 0);
    let code = readable_code(call.arguments[0].to_expression());
    let script = format!(
        "const trace=[];const values=[1,2];const rest={{get color(){{trace.push('color');return 'red'}}}};const result={code};JSON.stringify([result.color,result.margin,trace]);"
    );
    let actual = BoaContext::default()
        .eval(Source::from_bytes(&script))
        .unwrap_or_else(|error| panic!("{error}"))
        .as_string()
        .unwrap_or_else(|| panic!("expected JSON observation"))
        .to_std_string_escaped();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&actual)
            .unwrap_or_else(|error| panic!("{error}")),
        serde_json::json!(["red", [1, 2], ["color"]])
    );
}

#[rstest]
#[case::callback_spread(
    "styled(...[{color:read('spread','red')}],p=>({margin:p.m}))",
    r#"[{"color":"red","margin":"2px"},["spread","props"]]"#
)]
#[case::array_spread(
    "styled(...[{color:read('spread','red')}],on?[{margin:read('margin',2)}]:[])",
    r#"[{"color":"red","margin":2},["spread","margin"]]"#
)]
fn repair_745_styled_arguments_keep_spread_capture_order(
    #[case] source: &str,
    #[case] expected: &str,
) {
    // Given: spread rules preceding a callback or a conditional rule array.
    let allocator = Allocator::default();
    let builder = AstBuilder::new(&allocator);
    let mut scope = Parser::new(&allocator, source, SourceType::mjs())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));
    let mut expression = scope.clone_in(&allocator);
    assert!(expression.is_call_expression());
    if let Expression::CallExpression(call) = &mut expression {
        let mut reads = Reads::default();

        // When: prepare real styled arguments and execute their ordered bindings.
        let prepared = prepare(
            &builder,
            call,
            Context {
                filename: "arguments.tsx",
                scope: &mut scope,
                reads: &mut reads,
                refused: &[],
            },
        );
        assert_eq!(prepared.errors, vec![]);
        let rules = call.arguments[0].to_expression().clone_in(&allocator);
        let render = bind_render(&builder, prepared.bindings, rules);
        let script = format!(
            "const trace=[];const on=true;const rest={{get m(){{trace.push('props');return 2}}}};const read=(name,value)=>(trace.push(name),value);JSON.stringify([{},trace]);",
            readable_code(&render)
        );
        let observed = BoaContext::default()
            .eval(Source::from_bytes(script.as_bytes()))
            .unwrap_or_else(|error| panic!("{error}"))
            .as_string()
            .unwrap_or_else(|| panic!("JSON observation"))
            .to_std_string_escaped();

        // Then: spread reads precede later argument reads, each once, with the same rules.
        assert_eq!(observed, expected);
    }
}

#[rstest]
#[case::callback("styled({color:await tone},p=>({margin:p.m}))", 1)]
#[case::array("styled(on?[{color:await tone}]:[],{margin:1})", 0)]
fn repair_745_styled_arguments_keep_suspending_fallback(
    #[case] source: &str,
    #[case] captures: usize,
) {
    // Given: a suspending sibling that preparation must leave at its original seam.
    let allocator = Allocator::default();
    let builder = AstBuilder::new(&allocator);
    let mut scope = Parser::new(&allocator, source, SourceType::mjs())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));
    let mut expression = scope.clone_in(&allocator);
    assert!(expression.is_call_expression());
    if let Expression::CallExpression(call) = &mut expression {
        let mut reads = Reads::default();

        // When: prepare the admitted call without hoisting the suspension.
        let prepared = prepare(
            &builder,
            call,
            Context {
                filename: "arguments.tsx",
                scope: &mut scope,
                reads: &mut reads,
                refused: &[],
            },
        );

        // Then: successful callback captures survive, but the awaiting value stays authored.
        assert_eq!(prepared.errors, vec![]);
        assert_eq!(prepared.bindings.len(), captures);
        assert!(readable_code(call.arguments[0].to_expression()).contains("await tone"));
    }
}

#[test]
fn repair_745_nested_callback_keeps_unmapped_theme_error() {
    // Given: a nested property callback with its own, different props root.
    let allocator = Allocator::default();
    let builder = AstBuilder::new(&allocator);
    let source = "styled(p=>({color:q=>q.theme.colors[key]}))";
    let mut scope = Parser::new(&allocator, source, SourceType::mjs())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));
    let mut expression = scope.clone_in(&allocator);
    assert!(expression.is_call_expression());
    if let Expression::CallExpression(call) = &mut expression {
        let mut reads = Reads::default();

        // When: lower the whole-rule callback through the ordinary argument seam.
        let prepared = prepare(
            &builder,
            call,
            Context {
                filename: "arguments.tsx",
                scope: &mut scope,
                reads: &mut reads,
                refused: &[],
            },
        );

        // Then: the inner callback's unknown theme access retains its located guard.
        assert_eq!(prepared.errors.len(), 1);
        assert!(prepared.errors[0].1.contains("a theme read must be a path"));
        assert_eq!(
            prepared.errors[0].0,
            u32::try_from(
                source
                    .find("q=>")
                    .unwrap_or_else(|| panic!("inner callback fixture"))
            )
            .unwrap_or_else(|error| panic!("{error}"))
        );
    }
}
