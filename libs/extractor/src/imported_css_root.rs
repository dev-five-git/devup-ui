use super::{Inline, NumericRole};
use oxc_ast::ast::{Expression, ObjectExpression, ObjectPropertyKind};
use oxc_ast_visit::VisitMut;

impl<'a> Inline<'_, 'a> {
    pub(super) fn compiled_css_props(&mut self, props: &mut ObjectExpression<'a>, css: usize) {
        for (index, property) in props.properties.iter_mut().enumerate() {
            let root = index == css;
            let outer = std::mem::replace(
                &mut self.numeric_role,
                if root {
                    NumericRole::ElementCssProp
                } else {
                    NumericRole::Other
                },
            );
            self.reading_css(root, |inline| match property {
                ObjectPropertyKind::ObjectProperty(property) if root => {
                    inline.visit_expression(&mut property.value);
                    if property.shorthand && !matches!(property.value, Expression::Identifier(_)) {
                        property.shorthand = false;
                    }
                }
                property => inline.visit_object_property_kind(property),
            });
            self.numeric_role = outer;
        }
    }
}
