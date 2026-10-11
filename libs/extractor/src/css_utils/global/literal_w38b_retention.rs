use super::*;
use oxc_allocator::Allocator;
use oxc_ast::ast::{ObjectPropertyKind, Statement};
use oxc_span::{GetSpan, SourceType};
use rstest::rstest;

#[rstest]
#[case("tag`selectors:${{...rest}};body{style-order:2;color:blue}`;")]
#[case("tag`selectors:${{[key]:'red'}};body{style-order:2;color:blue}`;")]
fn cloned_selector_when_opaque_rules_are_removed_retains_authored_property(#[case] source: &str) {
    // Given: an exact template hole supplies a parser-owned object, not a generated record.
    let allocator = Allocator::default();
    let parsed = oxc_parser::Parser::new(&allocator, source, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let semantic = oxc_semantic::SemanticBuilder::new().build(&parsed.program);
    assert_eq!(semantic.diagnostics.len(), 0);
    let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
        panic!("expression statement required")
    };
    let Expression::TaggedTemplateExpression(tag) = &statement.expression else {
        panic!("tagged template required")
    };
    let Expression::ObjectExpression(original) = &tag.quasi.expressions[0] else {
        panic!("authored object required")
    };
    let ast = AstBuilder::new(&allocator);
    let text = CssText::from_template(&ast, &tag.quasi, Some(source));
    let mut object = text.scoped_object(&ast, 0..text.text.len(), true);
    let mut raw = Vec::new();
    // When: the actual global preparation operation removes opaque rules.
    remove_opaque(&mut object, &mut raw, &[]);
    // Then: the cloned property keeps its source shape, value and complete original spans.
    let Expression::ObjectExpression(object) = object else {
        panic!("generated outer object required")
    };
    let selectors = object
        .properties
        .iter()
        .find_map(|property| match property {
            ObjectPropertyKind::ObjectProperty(property)
                if property
                    .key
                    .static_name()
                    .is_some_and(|key| key == "selectors") =>
            {
                Some(&property.value)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("generated selectors declaration"));
    let Expression::ObjectExpression(retained) = selectors else {
        panic!("cloned selectors object required")
    };
    assert_eq!(retained.properties.len(), 1);
    assert_eq!(raw, Vec::<String>::new());
    match (&original.properties[0], &retained.properties[0]) {
        (
            ObjectPropertyKind::SpreadProperty(original),
            ObjectPropertyKind::SpreadProperty(actual),
        ) => {
            assert_eq!(actual.span, original.span);
            assert_eq!(actual.argument.span(), original.argument.span());
            assert_eq!(crate::utils::readable_code(&actual.argument), "rest");
            let (Expression::Identifier(original), Expression::Identifier(actual)) =
                (&original.argument, &actual.argument)
            else {
                panic!("authored spread identifier required")
            };
            assert!(original.reference_id.get().is_some());
            assert_eq!(actual.reference_id.get(), original.reference_id.get());
        }
        (
            ObjectPropertyKind::ObjectProperty(original),
            ObjectPropertyKind::ObjectProperty(actual),
        ) => {
            assert!(actual.computed);
            assert_eq!(actual.span, original.span);
            assert_eq!(actual.key.span(), original.key.span());
            assert_eq!(actual.value.span(), original.value.span());
            assert_eq!(actual.key.static_name(), None);
            assert_eq!(
                crate::utils::readable_code(
                    actual
                        .key
                        .as_expression()
                        .unwrap_or_else(|| panic!("authored computed key"))
                ),
                "key"
            );
            assert_eq!(crate::utils::readable_code(&actual.value), "\"red\"");
        }
        _ => panic!("cloned property shape changed"),
    }
}
