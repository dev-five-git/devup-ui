use std::collections::BTreeMap;

use oxc_allocator::{Allocator, FromIn, GetAllocator};
use oxc_ast::{ast::Expression, builder::AstBuilder};
use oxc_span::SPAN;
use rstest::rstest;

use super::visit_style_expressions;
use crate::{
    ExtractStyleProp, ExtractStyleValue,
    extract_style::{
        extract_dynamic_style::ExtractDynamicStyle, extract_static_style::ExtractStaticStyle,
    },
    utils::readable_code,
};

#[derive(Clone, Copy)]
enum Container {
    Conditional,
    StaticArray,
    Enum,
    Expression,
    MemberExpression,
}

fn identifier<'a>(builder: &AstBuilder<'a>, name: &str) -> Expression<'a> {
    Expression::new_identifier(
        SPAN,
        oxc_ast::ast::Ident::from_in(name, builder.allocator()),
        builder,
    )
}

fn payload<'a>(builder: &AstBuilder<'a>, name: &str) -> ExtractStyleProp<'a> {
    ExtractStyleProp::Expression {
        expression: identifier(builder, name),
        styles: vec![
            ExtractStyleValue::Dynamic(ExtractDynamicStyle::new("color", 2, name, None)),
            ExtractStyleValue::Dynamic(ExtractDynamicStyle::new(
                "background",
                4,
                &format!("{name} + {name}.color !important"),
                None,
            )),
            ExtractStyleValue::Static(ExtractStaticStyle::new(
                "--authored",
                "__devupRecord0",
                3,
                None,
            )),
        ],
    }
}

fn assert_identifier(expression: &Expression<'_>, expected: &str) {
    let Expression::Identifier(identifier) = expression else {
        panic!("visitor must preserve the identifier expression");
    };
    assert_eq!(identifier.name.as_str(), expected);
    assert_eq!(readable_code(expression), expected);
}

fn assert_payload(style: &ExtractStyleProp<'_>, expected: &str) {
    let ExtractStyleProp::Expression { expression, styles } = style else {
        panic!("fixture payload must remain an Expression");
    };
    assert_identifier(expression, expected);
    assert_eq!(styles.len(), 3);
    let ExtractStyleValue::Dynamic(simple) = &styles[0] else {
        panic!("first payload must remain dynamic");
    };
    assert_eq!(simple.identifier(), expected);
    assert_eq!(
        simple,
        &ExtractDynamicStyle::new("color", 2, expected, None)
    );
    let ExtractStyleValue::Dynamic(code) = &styles[1] else {
        panic!("second payload must remain dynamic code");
    };
    assert_eq!(code.identifier(), format!("{expected} + {expected}.color"));
    assert_eq!(
        code,
        &ExtractDynamicStyle::new(
            "background",
            4,
            &format!("{expected} + {expected}.color !important"),
            None,
        )
    );
    assert_eq!(
        styles[2],
        ExtractStyleValue::Static(ExtractStaticStyle::new(
            "--authored",
            "__devupRecord0",
            3,
            None,
        ))
    );
}

#[rstest]
#[case::conditional(Container::Conditional)]
#[case::static_array(Container::StaticArray)]
#[case::enum_map(Container::Enum)]
#[case::expression(Container::Expression)]
#[case::member_map(Container::MemberExpression)]
fn payloads_are_rewritten_when_style_expressions_are_visited(#[case] container: Container) {
    // Given: two distinct nested captures, including serialized dynamic code.
    let allocator = Allocator::default();
    let builder = AstBuilder::new(&allocator);
    let first = payload(&builder, "__devupRecord0");
    let second = payload(&builder, "__devupRecord1");
    let control = identifier(&builder, "__devupRecord2");
    let mut style = match container {
        Container::Conditional => ExtractStyleProp::Conditional {
            condition: control,
            consequent: Some(Box::new(first)),
            alternate: Some(Box::new(second)),
        },
        Container::StaticArray => ExtractStyleProp::StaticArray(vec![first, second]),
        Container::Enum => ExtractStyleProp::Enum {
            condition: control,
            map: BTreeMap::from([
                ("__devupRecord0".to_string(), vec![first, second]),
                ("empty".to_string(), vec![]),
            ]),
        },
        Container::Expression => first,
        Container::MemberExpression => ExtractStyleProp::MemberExpression {
            expression: control,
            map: BTreeMap::from([
                ("__devupRecord0".to_string(), Box::new(first)),
                ("second".to_string(), Box::new(second)),
            ]),
        },
    };

    // When: the callback rewrites actual AST identifier references/code payloads.
    visit_style_expressions(&builder, &mut style, &mut |expression| {
        let Expression::Identifier(value) = expression else {
            panic!("fixture visitor receives identifier expressions");
        };
        let renamed = value.name.replace("__devupRecord", "__devupInherited");
        value.name = oxc_ast::ast::Ident::from_in(renamed.as_str(), builder.allocator());
    });

    // Then: every branch/map/array entry and AST control was rewritten.
    match &style {
        ExtractStyleProp::Conditional {
            condition,
            consequent,
            alternate,
        } => {
            assert_identifier(condition, "__devupInherited2");
            let (Some(consequent), Some(alternate)) = (consequent, alternate) else {
                panic!("both conditional branches must remain present");
            };
            assert_payload(consequent, "__devupInherited0");
            assert_payload(alternate, "__devupInherited1");
        }
        ExtractStyleProp::StaticArray(styles) => {
            assert_eq!(styles.len(), 2);
            assert_payload(&styles[0], "__devupInherited0");
            assert_payload(&styles[1], "__devupInherited1");
        }
        ExtractStyleProp::Enum { condition, map } => {
            assert_identifier(condition, "__devupInherited2");
            assert_eq!(map.len(), 2);
            assert_eq!(map["empty"].len(), 0);
            assert_payload(&map["__devupRecord0"][0], "__devupInherited0");
            assert_payload(&map["__devupRecord0"][1], "__devupInherited1");
        }
        ExtractStyleProp::Expression { .. } => assert_payload(&style, "__devupInherited0"),
        ExtractStyleProp::MemberExpression { expression, map } => {
            assert_identifier(expression, "__devupInherited2");
            assert_eq!(map.len(), 2);
            assert_payload(&map["__devupRecord0"], "__devupInherited0");
            assert_payload(&map["second"], "__devupInherited1");
        }
        ExtractStyleProp::Static(_) | ExtractStyleProp::Unreadable { .. } => {
            panic!("container must remain unchanged");
        }
    }
}

#[test]
fn metadata_is_unchanged_when_style_expressions_are_visited() {
    // Given: static declarations and unreadable diagnostics are not expressions.
    let allocator = Allocator::default();
    let builder = AstBuilder::new(&allocator);
    let static_value = ExtractStyleValue::Static(ExtractStaticStyle::new(
        "--authored",
        "__devupRecord0",
        3,
        None,
    ));
    let mut style = ExtractStyleProp::StaticArray(vec![
        ExtractStyleProp::Static(static_value.clone()),
        ExtractStyleProp::Unreadable {
            offset: 17,
            code: "__devupRecord0".to_string(),
            prop: true,
        },
        ExtractStyleProp::Static(ExtractStyleValue::Dynamic(ExtractDynamicStyle::new(
            "color",
            2,
            "__devupRecord0",
            None,
        ))),
    ]);

    // When: only the dynamic declaration is visited.
    visit_style_expressions(&builder, &mut style, &mut |expression| {
        assert_identifier(expression, "__devupRecord0");
        *expression = identifier(&builder, "__devupInherited0");
    });

    // Then: metadata stays byte-identical and the dynamic declaration changes.
    let ExtractStyleProp::StaticArray(styles) = style else {
        panic!("array fixture");
    };
    let ExtractStyleProp::Static(value) = &styles[0] else {
        panic!("static fixture");
    };
    assert_eq!(value, &static_value);
    let ExtractStyleProp::Unreadable { offset, code, prop } = &styles[1] else {
        panic!("unreadable fixture");
    };
    assert_eq!(
        (*offset, code.as_str(), *prop),
        (17, "__devupRecord0", true)
    );
    let ExtractStyleProp::Static(value) = &styles[2] else {
        panic!("dynamic fixture");
    };
    assert_eq!(
        value,
        &ExtractStyleValue::Dynamic(ExtractDynamicStyle::new(
            "color",
            2,
            "__devupInherited0",
            None,
        ))
    );
}
