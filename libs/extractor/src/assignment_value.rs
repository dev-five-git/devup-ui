use oxc_allocator::{CloneIn, FromIn, GetAllocator};
use oxc_ast::ast::{ArrayExpression, ArrayExpressionElement, BinaryOperator, Expression, Str};
use oxc_span::{GetSpan, SPAN};

use crate::{
    ExtractStyleProp, ExtractStyleValue,
    assignment_lowering::{Classes, Lowering, empty, projection},
    assignment_member::selected_array,
    gen_class_name::{gen_class_names, merge_expression_for_class_name},
    gen_style::gen_styles,
    utils::{call_with_values, merge_object_expressions},
};

#[path = "assignment_class_reference.rs"]
mod assignment_class_reference;

#[path = "assignment_presence.rs"]
mod assignment_presence;

impl<'a> Lowering<'_, 'a> {
    pub(super) fn array(
        &self,
        source: &ArrayExpression<'a>,
        styles: &[ExtractStyleProp<'a>],
    ) -> Expression<'a> {
        let ast = self.ast;
        let mut values = Vec::new();
        let mut refs = Vec::new();
        let mut raw = source.clone_in(ast.allocator());
        for (index, element) in source.elements.iter().enumerate() {
            if let Some(value) = element.as_expression() {
                let mut selected: Vec<_> = styles
                    .iter()
                    .filter(|style| {
                        if crate::assignment_owner::contains_consumer(value.span(), style) {
                            return true;
                        }
                        style.extract().iter().any(|value| match value {
                            ExtractStyleValue::Dynamic(style) => {
                                usize::from(style.level()) == index
                            }
                            ExtractStyleValue::Static(style) => usize::from(style.level()) == index,
                            ExtractStyleValue::Typography(_) => index == 0,
                            _ => false,
                        })
                    })
                    .map(|style| style.clone_in(ast.allocator()))
                    .collect();
                let binding = format!("__devupLevel{index}");
                values.push((binding.clone(), self.lower(value, &mut selected)));
                raw.elements[index] = ArrayExpressionElement::from(projection(ast, &binding, 2));
                refs.push(binding);
            }
        }
        let class =
            merge_expression_for_class_name(ast, refs.iter().map(|name| projection(ast, name, 0)))
                .unwrap_or_else(|| Expression::new_string_literal(SPAN, "", None, ast));
        let alternate = self.alternate_order.map(|_| {
            merge_expression_for_class_name(ast, refs.iter().map(|name| projection(ast, name, 3)))
                .unwrap_or_else(|| Expression::new_string_literal(SPAN, "", None, ast))
        });
        let inline: Vec<_> = refs.iter().map(|name| projection(ast, name, 1)).collect();
        let inline = merge_object_expressions(ast, &inline).unwrap_or_else(|| empty(ast));
        call_with_values(
            ast,
            values,
            self.pair(
                Classes {
                    primary: class,
                    alternate,
                },
                inline,
                Expression::ArrayExpression(oxc_allocator::Box::new_in(raw, ast)),
            ),
        )
    }

    pub(super) fn leaf(
        &self,
        source: &Expression<'a>,
        styles: &[ExtractStyleProp<'a>],
    ) -> Expression<'a> {
        let ast = self.ast;
        let mut values = styles
            .iter()
            .flat_map(ExtractStyleProp::extract)
            .collect::<Vec<_>>();
        let normalization = values.iter().find_map(|value| match value {
            ExtractStyleValue::Dynamic(style) => {
                crate::source_normalization::suffix(ast, source, style).map(|suffix| {
                    (
                        Expression::new_identifier(
                            source.span(),
                            Str::from_in(style.identifier(), ast.allocator()),
                            ast,
                        ),
                        suffix,
                    )
                })
            }
            _ => None,
        });
        let (input, suffix) =
            normalization.unwrap_or_else(|| (source.clone_in(ast.allocator()), String::new()));
        let responsive = selected_array(source);
        let mut class_props = styles
            .iter()
            .map(|style| style.clone_in(ast.allocator()))
            .collect::<Vec<_>>();
        assignment_presence::rewrite(&mut class_props, &values, responsive);
        for value in &mut values {
            if let ExtractStyleValue::Dynamic(style) = value {
                style.replace_identifier(&assignment_presence::reference(style, responsive));
            }
        }
        let mut props: Vec<_> = values.into_iter().map(ExtractStyleProp::Static).collect();
        let class_alternatives = self.alternate_order.map(|_| {
            class_props
                .iter()
                .map(|style| style.clone_in(ast.allocator()))
                .collect::<Vec<_>>()
        });
        let mut class = gen_class_names(ast, &mut class_props, self.order, self.filename)
            .unwrap_or_else(|| Expression::new_string_literal(SPAN, "", None, ast));
        assignment_class_reference::replace(ast, source, &mut class);
        let alternate =
            self.alternate_order
                .zip(class_alternatives)
                .map(|(order, mut alternative)| {
                    let mut class =
                        gen_class_names(ast, &mut alternative, order.value, self.filename)
                            .unwrap_or_else(|| Expression::new_string_literal(SPAN, "", None, ast));
                    assignment_class_reference::replace(ast, source, &mut class);
                    class
                });
        props.reverse();
        let inline = gen_styles(ast, &props, self.filename).unwrap_or_else(|| empty(ast));
        let mut value = Expression::new_identifier(SPAN, "__devupValue", ast);
        if !suffix.is_empty() {
            value = Expression::new_binary_expression(
                SPAN,
                value,
                BinaryOperator::Addition,
                Expression::new_string_literal(
                    SPAN,
                    Str::from_in(suffix.as_str(), ast.allocator()),
                    None,
                    ast,
                ),
                ast,
            );
        }
        call_with_values(
            ast,
            vec![("__devupValue".to_string(), input)],
            self.pair(
                Classes {
                    primary: class,
                    alternate,
                },
                inline,
                value,
            ),
        )
    }

    pub(super) fn pair(
        &self,
        classes: Classes<'a>,
        inline: Expression<'a>,
        value: Expression<'a>,
    ) -> Expression<'a> {
        let mut slots = oxc_allocator::Vec::from_iter_in(
            [classes.primary, inline, value]
                .into_iter()
                .map(ArrayExpressionElement::from),
            self.ast,
        );
        if let Some(alternate) = classes.alternate {
            slots.push(ArrayExpressionElement::from(alternate));
        }
        Expression::new_array_expression(SPAN, slots, self.ast)
    }
}
