use oxc_ast::ast::{Expression, ObjectPropertyKind, PropertyKind};

use super::Sites;

impl<'a> Sites<'_, 'a> {
    pub(super) fn read_properties(&mut self, expression: &Expression<'a>) {
        let Expression::ObjectExpression(object) = crate::utils::unwrap_syntax_only(expression)
        else {
            return;
        };
        for property in &object.properties {
            let ObjectPropertyKind::ObjectProperty(property) = property else {
                continue;
            };
            let Some(key) = property.key.static_name() else {
                continue;
            };
            if property.method
                || property.kind != PropertyKind::Init
                || key
                    .as_str()
                    .is_some_and(css::is_special_property::is_special_property)
                || matches!(
                    key.as_str(),
                    Some("styleOrder" | "styleVars" | "props" | "params" | "as")
                )
            {
                continue;
            }
            match crate::utils::unwrap_syntax_only(&property.value) {
                Expression::Identifier(identifier) => {
                    if let Some(reference) = identifier.reference_id.get()
                        && let Some(symbol) = self.scoping.get_reference(reference).symbol_id()
                    {
                        self.properties
                            .entry(symbol)
                            .or_default()
                            .push((reference, identifier.span.start));
                    }
                }
                value => self.read_properties(value),
            }
        }
    }
}
