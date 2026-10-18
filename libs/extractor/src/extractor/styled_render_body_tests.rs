use boa_engine::{Context, Source};
use oxc_allocator::Allocator;
use oxc_ast::ast::{ArrowFunctionBody, Statement, VariableDeclarationKind};
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;

use super::*;

fn expression<'a>(builder: &AstBuilder<'a>, source: &str) -> Expression<'a> {
    let parsed = Parser::new(builder.allocator(), source, SourceType::mjs()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
        panic!("render fixture must be an expression");
    };
    unwrap_syntax_only(&statement.expression).clone_in(builder.allocator())
}

fn component<'a>(
    builder: &AstBuilder<'a>,
    render: StyledRender<'_, 'a>,
    jsx: bool,
) -> Expression<'a> {
    let source_type = if jsx {
        SourceType::tsx()
    } else {
        SourceType::mjs()
    };
    let parsed = Parser::new(builder.allocator(), "", source_type).parse();
    let mut helper = StyledJsx::reserve(&parsed.program);
    create_styled_component(
        builder,
        &Base::named("div".to_string()),
        render,
        &mut helper,
    )
}

#[rstest]
fn direct_body_when_generated_component_has_captures(#[values(false, true)] jsx: bool) {
    // Given: ordered captures with a dependency on the preceding binding.
    let allocator = Allocator::default();
    let builder = AstBuilder::new(&allocator);
    let bindings = vec![
        ("first".to_string(), expression(&builder, "readFirst();")),
        (
            "second".to_string(),
            expression(&builder, "readSecond(first);"),
        ),
    ];

    // When: build the actual props-taking component arrow in either output mode.
    let render = component(
        &builder,
        StyledRender {
            class_name: &None,
            style_vars: &None,
            bindings,
            withheld: &[],
            css_slot: None,
            markers: None,
        },
        jsx,
    );

    // Then: the arrow directly owns sibling const declarations and the element return.
    let Expression::ArrowFunctionExpression(arrow) = render else {
        panic!("generated component must be an arrow");
    };
    assert_eq!(arrow.params.items.len(), 1);
    let ArrowFunctionBody::FunctionBody(body) = &arrow.body else {
        panic!("captured component must directly own a block body");
    };
    let [
        Statement::VariableDeclaration(first),
        Statement::VariableDeclaration(second),
        Statement::ReturnStatement(returned),
    ] = body.statements.as_slice()
    else {
        panic!("captures must be sibling declarations followed by the render return");
    };
    for (declaration, name, initializer) in [
        (first, "first", "readFirst()"),
        (second, "second", "readSecond(first)"),
    ] {
        assert_eq!(declaration.kind, VariableDeclarationKind::Const);
        let [binding] = declaration.declarations.as_slice() else {
            panic!("each capture must have its own declaration");
        };
        let identifier = binding
            .id
            .get_binding_identifier()
            .unwrap_or_else(|| panic!("capture needs an identifier"));
        assert_eq!(identifier.name, name);
        let Some(Expression::SequenceExpression(sequence)) = &binding.init else {
            panic!("capture must retain its sequence name barrier");
        };
        let [Expression::NumericLiteral(zero), value] = sequence.expressions.as_slice() else {
            panic!("capture name barrier must be exactly (0, initializer)");
        };
        assert_eq!(zero.value.to_bits(), 0.0_f64.to_bits());
        assert_eq!(readable_code(value), initializer);
    }
    match returned
        .argument
        .as_ref()
        .unwrap_or_else(|| panic!("render needs a return value"))
    {
        Expression::JSXElement(_) => assert!(jsx),
        Expression::CallExpression(call) => {
            assert!(!jsx);
            assert!(matches!(&call.callee, Expression::Identifier(name)
                if name.name == "__devupJsx0"));
        }
        _ => panic!("component must return its element, not a capture IIFE"),
    }
}

#[rstest]
fn concise_body_when_generated_component_has_no_captures(#[values(false, true)] jsx: bool) {
    // Given: no per-render captures.
    let allocator = Allocator::default();
    let builder = AstBuilder::new(&allocator);

    // When: build the generated component without bindings.
    let render = component(
        &builder,
        StyledRender {
            class_name: &None,
            style_vars: &None,
            bindings: Vec::new(),
            withheld: &[],
            css_slot: None,
            markers: None,
        },
        jsx,
    );

    // Then: its element remains a concise expression body.
    let Expression::ArrowFunctionExpression(arrow) = render else {
        panic!("generated component must be an arrow");
    };
    assert_eq!(
        matches!(
            arrow
                .body
                .as_expression()
                .unwrap_or_else(|| panic!("empty captures must stay concise")),
            Expression::JSXElement(_)
        ),
        jsx
    );
}

#[rstest]
#[case::class("class { static { trace.push('class:' + this.name); } }", true)]
#[case::function("function() {}", false)]
fn names_and_evaluation_when_captures_move_into_generated_render(
    #[case] initializer: &str,
    #[case] class_observer: bool,
) {
    // Given: an anonymous definition and a dependent read of destructured rest props.
    let allocator = Allocator::default();
    let builder = AstBuilder::new(&allocator);
    let class_name = Some(expression(&builder, "__devupRecord1;"));
    let bindings = vec![
        (
            "__devupRecord0".to_string(),
            expression(&builder, &format!("({initializer});")),
        ),
        (
            "__devupRecord1".to_string(),
            expression(
                &builder,
                "(trace.push('capture:' + __devupRecord0.name + ':' + rest.value), rest.value);",
            ),
        ),
    ];
    let render = component(
        &builder,
        StyledRender {
            class_name: &class_name,
            style_vars: &None,
            bindings,
            withheld: &[],
            css_slot: None,
            markers: None,
        },
        false,
    );
    let script = format!(
        r"
        const trace = [];
        const __devupJsx0 = (tag, props) => props.className;
        let current = 'one';
        const props = {{
            get style() {{ trace.push('style'); return {{}}; }},
            get className() {{ trace.push('className'); }},
            get as() {{ trace.push('as'); }},
            get forwardedAs() {{ trace.push('forwardedAs'); }},
            get value() {{ trace.push('rest'); return current; }}
        }};
        const render = {};
        trace.push('defined');
        JSON.stringify([render(props), (current = 'two', render(props)), trace]);
    ",
        readable_code(&render)
    );

    // When: execute the emitted component through two renders.
    let mut context = Context::default();
    let result = context
        .eval(Source::from_bytes(script.as_bytes()))
        .unwrap_or_else(|error| panic!("generated render evaluation failed: {error}"));
    let result = result
        .as_string()
        .unwrap_or_else(|| panic!("observation must be JSON text"))
        .to_std_string_escaped();

    // Then: destructuring precedes ordered captures each time, with no inferred name.
    let mut trace = vec!["defined".to_string()];
    for value in ["one", "two"] {
        trace.extend(["style", "className", "as", "forwardedAs", "rest"].map(str::to_string));
        if class_observer {
            trace.push("class:".to_string());
        }
        trace.push(format!("capture::{value}"));
    }
    let expected = serde_json::to_string(&("one", "two", trace))
        .unwrap_or_else(|error| panic!("expected observation must serialize: {error}"));
    assert_eq!(result, expected);
}
