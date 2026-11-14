//! Compile a tag-only selection directly to its generated class table.

use super::unwrap_syntax_only;
use oxc_allocator::{CloneIn, FromIn, GetAllocator, TakeIn};
use oxc_ast::{
    ast::{ChainElement, ComputedMemberExpression, Expression, ObjectPropertyKind, PropertyKind},
    builder::AstBuilder,
};
use oxc_span::{GetSpan, SPAN};
use rustc_hash::FxHashMap;

pub(super) fn slot<'b, 'a>(value: &'b Expression<'a>) -> Option<&'b ComputedMemberExpression<'a>> {
    match unwrap_syntax_only(value) {
        Expression::ComputedMemberExpression(member) => Some(&**member),
        Expression::ChainExpression(chain) => match &chain.expression {
            ChainElement::ComputedMemberExpression(member) => Some(&**member),
            _ => None,
        },
        _ => None,
    }
    .filter(|member| {
        super::get_key_by_literal_expression(&member.expression).as_deref() == Some("0")
    })
}

/// Only compiler-created `[tag]` packets qualify; captured values must remain eager.
fn tag(value: &Expression<'_>, key: &str) -> bool {
    matches!(value, Expression::ArrayExpression(array) if array.span == SPAN
        && matches!(array.elements.as_slice(), [oxc_ast::ast::ArrayExpressionElement::StringLiteral(tag)] if tag.value == key))
}

pub(super) fn project<'a>(
    builder: &AstBuilder<'a>,
    classes: &Expression<'a>,
    selection: &Expression<'a>,
) -> Option<Expression<'a>> {
    let Expression::ObjectExpression(classes) = classes else {
        return None;
    };
    if classes.span != SPAN {
        return None;
    }
    let mut mapped = FxHashMap::default();
    for property in &classes.properties {
        let ObjectPropertyKind::ObjectProperty(property) = property else {
            return None;
        };
        let key = super::get_str_by_property_key(&property.key)?.into_owned();
        if key == "__proto__"
            || property.kind != PropertyKind::Init
            || property.method
            || !matches!(property.value, Expression::StringLiteral(_))
        {
            return None;
        }
        mapped.insert(key, &property.value);
    }
    let Expression::ComputedMemberExpression(selection) = unwrap_syntax_only(selection) else {
        return None;
    };
    if selection.optional {
        return None;
    }
    let mut table = selection.object.clone_in(builder.allocator());
    let class = |key: &str| {
        mapped.get(key).map_or_else(
            || Expression::new_string_literal(SPAN, "", None, builder),
            |value| value.clone_in(builder.allocator()),
        )
    };
    let mut keys = Vec::new();
    match &mut table {
        Expression::ObjectExpression(object) => {
            for property in &mut object.properties {
                let ObjectPropertyKind::ObjectProperty(property) = property else {
                    return None;
                };
                let key = super::get_str_by_property_key(&property.key)?.into_owned();
                if key == "__proto__"
                    || property.kind != PropertyKind::Init
                    || property.method
                    || !tag(&property.value, &key)
                {
                    return None;
                }
                property.value = class(&key);
                keys.push(key);
            }
        }
        Expression::ArrayExpression(array) => {
            for (index, element) in array.elements.iter_mut().enumerate() {
                if matches!(element, oxc_ast::ast::ArrayExpressionElement::Elision(_)) {
                    continue;
                }
                let key = index.to_string();
                if !element
                    .as_expression()
                    .is_some_and(|value| tag(value, &key))
                {
                    return None;
                }
                *element = class(&key).into();
                keys.push(key);
            }
        }
        _ => return None,
    }
    if mapped.keys().any(|key| !keys.contains(key)) {
        return None;
    }
    if let Some(key) = super::get_key_by_literal_expression(&selection.expression)
        && keys.iter().any(|own| own == key.as_ref())
    {
        return Some(class(&key));
    }
    // An arbitrary array key can read `length`, methods, or inherited slots.
    // Replacing its tag packets with class strings would expose those as classes.
    if matches!(table, Expression::ArrayExpression(_)) {
        return None;
    }
    Some(Expression::new_computed_member_expression(
        selection.span(),
        table,
        selection.expression.clone_in(builder.allocator()),
        false,
        builder,
    ))
}

struct Strings<'b, 'a>(&'b AstBuilder<'a>);

impl<'a> oxc_ast_visit::VisitMut<'a> for Strings<'_, 'a> {
    fn visit_jsx_expression(&mut self, value: &mut oxc_ast::ast::JSXExpression<'a>) {
        if let Some(value) = value.as_expression_mut() {
            self.visit_expression(value);
        } else {
            oxc_ast_visit::walk_mut::walk_jsx_expression(self, value);
        }
    }

    fn visit_expression(&mut self, value: &mut Expression<'a>) {
        oxc_ast_visit::walk_mut::walk_expression(self, value);
        if value.span() != SPAN {
            return;
        }
        if let Expression::LogicalExpression(logical) = value
            && logical.operator == oxc_syntax::operator::LogicalOperator::Or
            && let Expression::StringLiteral(left) = unwrap_syntax_only(&logical.left)
            && matches!(
                unwrap_syntax_only(&logical.right),
                Expression::StringLiteral(_)
            )
        {
            *value = if left.value.is_empty() {
                super::unwrap_syntax_only_mut(&mut logical.right).take_in(self.0)
            } else {
                super::unwrap_syntax_only_mut(&mut logical.left).take_in(self.0)
            };
        }
        if let Expression::TemplateLiteral(template) = value
            && !template.expressions.is_empty()
            && template
                .expressions
                .iter()
                .all(|value| matches!(unwrap_syntax_only(value), Expression::StringLiteral(_)))
            && let Some(text) = super::get_string_by_literal_expression(value)
        {
            *value = Expression::new_string_literal(
                SPAN,
                oxc_ast::ast::Str::from_in(text.as_ref(), self.0.allocator()),
                None,
                self.0,
            );
        }
    }
}

pub(super) fn direct<'a>(builder: &AstBuilder<'a>, body: &mut Expression<'a>) {
    super::rewrite_expression(body, &mut |value| {
        if let Expression::ComputedMemberExpression(member) = value
            && !member.optional
            && let Some(slot) = slot(&member.expression)
            && let Some(projected) = project(builder, &member.object, &slot.object)
        {
            *value = projected;
            return true;
        }
        false
    });
    oxc_ast_visit::VisitMut::visit_expression(&mut Strings(builder), body);
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;
    use rstest::rstest;

    fn parse<'a>(builder: &AstBuilder<'a>, source: &str) -> Expression<'a> {
        let mut value = Parser::new(
            builder.allocator(),
            builder.allocator().alloc_str(source),
            SourceType::tsx(),
        )
        .parse_expression()
        .unwrap_or_else(|error| panic!("{source}: {error:?}"));
        super::super::clear_generated_spans(&mut value);
        value
    }

    #[rstest]
    #[case("packet[0]", true)]
    #[case("packet?.['0']", true)]
    #[case("packet?.tag", false)]
    #[case("packet?.()", false)]
    #[case("packet[1]", false)]
    #[case("packet", false)]
    fn only_the_branch_tag_slot_is_projected(#[case] source: &str, #[case] expected: bool) {
        let arena = Allocator::default();
        let builder = AstBuilder::new(&arena);
        assert_eq!(slot(&parse(&builder, source)).is_some(), expected);
    }

    #[rstest]
    #[case("[]", "{a:['a']}[key]", None)]
    #[case("{...rest}", "{a:['a']}[key]", None)]
    #[case("{[key()]:'x'}", "{a:['a']}[key]", None)]
    #[case("{__proto__:'x'}", "{a:['a']}[key]", None)]
    #[case("{get a(){return 'x'}}", "{a:['a']}[key]", None)]
    #[case("{a(){return 'x'}}", "{a:['a']}[key]", None)]
    #[case("{a:read()}", "{a:['a']}[key]", None)]
    #[case("{a:'x'}", "table", None)]
    #[case("{a:'x'}", "{...rest}[key]", None)]
    #[case("{a:'x'}", "{[key()]:['a']}[key]", None)]
    #[case("{a:'x'}", "{__proto__:['__proto__']}[key]", None)]
    #[case("{a:'x'}", "{get a(){return ['a']}}[key]", None)]
    #[case("{a:'x'}", "{a(){return ['a']}}[key]", None)]
    #[case("{a:'x'}", "{a:['a',read()]}[key]", None)]
    #[case("{a:'x'}", "{a:['b']}[key]", None)]
    #[case("{a:'x'}", "{a:'a'}[key]", None)]
    #[case("{a:'x'}", "[...rest][key]", None)]
    #[case("{a:'x'}", "table[key]", None)]
    #[case("{a:'x',b:'y'}", "{a:['a']}[key]", None)]
    #[case("{a:'x'}", "{a:['a'],b:['b']}[key]", Some("{ a: 'x', b: '' }[key]"))]
    #[case(
        "{a:'x',b:'y'}",
        "{a:['a'],b:['b']}[key]",
        Some("{ a: 'x', b: 'y' }[key]")
    )]
    #[case("{'0':'x','2':'y'}", "[['0'],,['2']][key]", None)]
    #[case("{'0':'x','2':'y'}", "[['0'],,['2']][2]", Some("'y'"))]
    #[case("{a:'x'}", "{a:['a']}['a']", Some("'x'"))]
    #[case("{'0':'x'}", "[['0']][0]", Some("'x'"))]
    fn projection_requires_static_generated_classes_and_tag_only_branches(
        #[case] classes: &str,
        #[case] selection: &str,
        #[case] expected: Option<&str>,
    ) {
        let arena = Allocator::default();
        let builder = AstBuilder::new(&arena);
        let result = project(
            &builder,
            &parse(&builder, classes),
            &parse(&builder, selection),
        );
        assert_eq!(
            result.as_ref().map(super::super::readable_code),
            expected.map(|source| super::super::readable_code(&parse(&builder, source)))
        );
    }

    #[test]
    fn authored_class_maps_and_packets_and_optional_tables_are_not_projected() {
        let arena = Allocator::default();
        let builder = AstBuilder::new(&arena);
        let mut classes = parse(&builder, "{a:'x'}");
        let mut selection = parse(&builder, "{a:['a']}[key]");
        if let Expression::ObjectExpression(object) = &mut classes {
            object.span = oxc_span::Span::new(1, 2);
        }
        assert!(project(&builder, &classes, &selection).is_none());
        let classes = parse(&builder, "{a:'x'}");
        if let Expression::ComputedMemberExpression(member) = &mut selection {
            member.optional = true;
        }
        assert!(project(&builder, &classes, &selection).is_none());
        if let Expression::ComputedMemberExpression(member) = &mut selection {
            member.optional = false;
            if let Expression::ObjectExpression(object) = &mut member.object
                && let ObjectPropertyKind::ObjectProperty(property) = &mut object.properties[0]
                && let Expression::ArrayExpression(array) = &mut property.value
            {
                array.span = oxc_span::Span::new(1, 2);
            }
        }
        assert!(project(&builder, &classes, &selection).is_none());
    }

    #[rstest]
    #[case("'x'||''", "'x'")]
    #[case("('x')||''", "'x'")]
    #[case("''||'y'", "'y'")]
    #[case("`before ${'x'||''} after`", "'before x after'")]
    #[case("`before ${read()} after`", "`before ${read()} after`")]
    #[case("'x'&&'y'", "'x'&&'y'")]
    #[case("'x'||read()", "'x'||read()")]
    fn projected_static_classes_fold_without_coercing_unknown_values(
        #[case] source: &str,
        #[case] expected: &str,
    ) {
        let arena = Allocator::default();
        let builder = AstBuilder::new(&arena);
        let mut value = parse(&builder, source);
        direct(&builder, &mut value);
        assert_eq!(
            super::super::readable_code(&value),
            super::super::readable_code(&parse(&builder, expected))
        );
        let mut authored = parse(&builder, "'x'||''");
        if let Expression::LogicalExpression(logical) = &mut authored {
            logical.span = oxc_span::Span::new(1, 2);
        }
        direct(&builder, &mut authored);
        assert!(matches!(authored, Expression::LogicalExpression(_)));
    }

    #[rstest]
    #[case("{a:'x'}[{a:['a']}[key]?.[0]]", "{a:'x'}[key]")]
    #[case("{'0':'x'}[[['0']][key]?.[0]]", "{'0':'x'}[[['0']][key]?.[0]]")]
    #[case("{a:'x'}?.[{a:['a']}[key]?.[0]]", "{a:'x'}?.[{a:['a']}[key]?.[0]]")]
    fn already_inlined_tag_selections_are_also_projected(
        #[case] source: &str,
        #[case] expected: &str,
    ) {
        let arena = Allocator::default();
        let builder = AstBuilder::new(&arena);
        let mut value = parse(&builder, source);
        direct(&builder, &mut value);
        assert_eq!(
            super::super::readable_code(&value),
            super::super::readable_code(&parse(&builder, expected))
        );
    }
}
