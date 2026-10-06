use super::{DevupVisitor, Text, jsx_order::AttributeOrder};
use oxc_ast::ast::{Expression, JSXAttributeItem};

impl<'a> DevupVisitor<'a> {
    pub(super) fn prepare_css_capture(
        &mut self,
        element: &str,
        value: &mut Expression<'a>,
    ) -> bool {
        self.css_prop_value(element, value);
        let supported = self
            .css_part(value, &|visitor, part| {
                !visitor.reads_known_styles(part)
                    && (visitor.reads_local_styles(part)
                        || visitor.unknown_bindings.read_by_in(part, &|identifier| {
                            visitor.bindings.reads_module(identifier)
                        })
                        || visitor.changed_bindings.read_by_in(part, &|identifier| {
                            visitor.bindings.reads_module(identifier)
                        }))
            })
            .is_none();
        supported
            && self
                .known_parts(value, &mut Vec::new(), Text::Rules)
                .is_some()
    }

    pub(super) fn snapshot_css_attributes(
        &mut self,
        attributes: &mut [JSXAttributeItem<'a>],
        order: &mut AttributeOrder<'a>,
    ) {
        for attribute in attributes {
            if let JSXAttributeItem::SpreadAttribute(spread) = attribute
                && let Some(slot) = order.spreads.remove(&spread.span.start)
            {
                let name = order.values[slot].0.clone();
                order.values[slot].1 = Some(self.snapshot_as(name, &mut spread.argument).1);
            }
        }
    }
}
