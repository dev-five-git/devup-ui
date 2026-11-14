use super::{Inline, NumericRole, component_style_object_property};
use oxc_ast::ast::{Expression, ObjectExpression, ObjectPropertyKind};
use oxc_ast_visit::VisitMut;

impl<'a> Inline<'_, 'a> {
    /// Read the props of a `jsx()` call: the `css` prop at `css`, and the
    /// style props when the call builds a `component` of the package, as
    /// its JSX attributes would be
    pub(super) fn element_props(
        &mut self,
        props: &mut ObjectExpression<'a>,
        css: Option<usize>,
        component: bool,
    ) {
        for (index, property) in props.properties.iter_mut().enumerate() {
            let root = Some(index) == css;
            let styles = !root && component && component_style_object_property(property);
            let outer = std::mem::replace(
                &mut self.numeric_role,
                if root {
                    NumericRole::ElementCssProp
                } else {
                    NumericRole::Other
                },
            );
            self.reading_css(root, |inline| {
                inline.reading_styles(styles, |inline| {
                    inline.reading_objects(root || styles, |inline| match property {
                        ObjectPropertyKind::ObjectProperty(property) if root || styles => {
                            inline.visit_expression(&mut property.value);
                            if property.shorthand
                                && !matches!(property.value, Expression::Identifier(_))
                            {
                                property.shorthand = false;
                            }
                        }
                        ObjectPropertyKind::SpreadProperty(spread) if styles => {
                            inline.style_spread(&mut spread.argument);
                        }
                        property => inline.visit_object_property_kind(property),
                    });
                });
            });
            self.numeric_role = outer;
        }
    }
}
