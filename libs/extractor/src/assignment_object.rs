use crate::{
    ExtractStyleProp, ExtractStyleValue,
    assignment_lowering::{Classes, Lowering, empty, projection},
    gen_class_name::merge_expression_for_class_name,
    utils::{call_with_values, get_string_by_property_key, merge_object_expressions},
};
use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::ast::{Expression, ObjectExpression, ObjectPropertyKind};
use oxc_span::GetSpan;

impl<'a> Lowering<'_, 'a> {
    pub(super) fn object(
        &self,
        source: &ObjectExpression<'a>,
        styles: &[ExtractStyleProp<'a>],
    ) -> Expression<'a> {
        let ast = self.ast;
        let mut raw = source.clone_in(ast.allocator());
        let mut values = Vec::new();
        let mut refs = Vec::new();
        for (index, property) in raw.properties.iter_mut().enumerate() {
            if let ObjectPropertyKind::ObjectProperty(property) = property {
                let names = get_string_by_property_key(&property.key)
                    .map(|key| {
                        css::disassemble_property(&key)
                            .map(|name| css::utils::to_kebab_case(&name).into_owned())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let mut consumers = styles
                    .iter()
                    .filter(|style| {
                        if crate::assignment_owner::contains_consumer(source.span(), style) {
                            return crate::assignment_owner::contains_consumer(
                                property.value.span(),
                                style,
                            );
                        }
                        style.extract().iter().any(|value| match value {
                            ExtractStyleValue::Static(value) => names.contains(&value.property),
                            ExtractStyleValue::Dynamic(value) => {
                                names.iter().any(|name| name == value.property())
                            }
                            _ => false,
                        })
                    })
                    .map(|style| style.clone_in(ast.allocator()))
                    .collect::<Vec<_>>();
                let binding = format!("__devupField{index}");
                values.push((binding.clone(), self.lower(&property.value, &mut consumers)));
                property.value = projection(ast, &binding, 2);
                refs.push(binding);
            }
        }
        let classes = |slot| {
            merge_expression_for_class_name(
                ast,
                refs.iter().map(|name| projection(ast, name, slot)),
            )
            .unwrap_or_else(|| Expression::new_string_literal(oxc_span::SPAN, "", None, ast))
        };
        let inline = merge_object_expressions(
            ast,
            &refs
                .iter()
                .map(|name| projection(ast, name, 1))
                .collect::<Vec<_>>(),
        )
        .unwrap_or_else(|| empty(ast));
        call_with_values(
            ast,
            values,
            self.pair(
                Classes {
                    primary: classes(0),
                    alternate: self.alternate_order.map(|_| classes(3)),
                },
                inline,
                Expression::ObjectExpression(oxc_allocator::Box::new_in(raw, ast)),
            ),
        )
    }
}
