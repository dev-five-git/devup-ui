use super::*;
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;

#[rstest::rstest]
#[case::factory_fallback(true)]
#[case::authored_leaf(false)]
#[serial_test::serial]
fn factory_params_use_the_call_location_when_the_leaf_has_no_offset(#[case] clear_offset: bool) {
    // Given a factory call with either a generated zero-span or an authored empty params list.
    let _state = crate::test_state::TestStateGuard::new();
    let arena = Allocator::default();
    let source = "import {Box} from '@devup-ui/react';import {jsx} from 'react/jsx-runtime';\njsx(Box,{_nthChild:{params:[],color:'red'}});";
    let mut parsed = Parser::new(&arena, source, SourceType::tsx()).parse();
    let Some(Statement::ExpressionStatement(statement)) = parsed.program.body.last_mut() else {
        panic!("expected factory statement");
    };
    let Expression::CallExpression(call) = &mut statement.expression else {
        panic!("expected factory call");
    };
    let call_offset = call.span.start;
    let Expression::ObjectExpression(props) = call.arguments[1].to_expression_mut() else {
        panic!("expected props");
    };
    let ObjectPropertyKind::ObjectProperty(selector) = &mut props.properties[0] else {
        panic!("expected selector");
    };
    let Expression::ObjectExpression(rules) = &mut selector.value else {
        panic!("expected selector rules");
    };
    let ObjectPropertyKind::ObjectProperty(params) = &mut rules.properties[0] else {
        panic!("expected params");
    };
    let Expression::ArrayExpression(array) = &mut params.value else {
        panic!("expected params array");
    };
    let leaf_offset = array.span.start;
    if clear_offset {
        array.span = oxc_span::SPAN;
    }
    let mut visitor = DevupVisitor::new(&arena, "factory.tsx", "@devup-ui/react", Vec::new(), None);
    // When the actual factory visitor extracts its structural params.
    visitor.visit_program(&mut parsed.program);
    // Then zero-span errors use the call, while authored leaves keep their own location.
    assert_eq!(
        visitor.errors,
        vec![(
            if clear_offset {
                call_offset
            } else {
                leaf_offset
            },
            element_error(
                "Box",
                "[]",
                crate::extractor::selector_params::SELECTOR_PARAMS
            ),
        )]
    );
}
