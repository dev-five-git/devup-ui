use super::{CssValue, DevupVisitor, JsxCss};
use crate::extractor::{
    extract_style_from_styled::{StyledDefinition, forwarding},
    styled_css_slot::CssPropLowering,
};
use crate::prop_modify_utils::{
    add_class_and_style, add_class_and_style_to_object, written_class_name,
};
use oxc_allocator::{FromIn, GetAllocator};
use oxc_ast::ast::{
    Argument, CallExpression, Expression, JSXAttributeItem, JSXAttributeName, JSXElement,
    JSXElementName, ObjectPropertyKind, Str,
};
use oxc_span::SPAN;
use oxc_syntax::symbol::SymbolId;

#[path = "css_forwarded_props.rs"]
mod props;
use props::{filter_jsx, filter_object};

impl<'a> DevupVisitor<'a> {
    pub(super) fn css_definition(
        &self,
        symbol: Option<SymbolId>,
        lookup: &str,
    ) -> Option<&StyledDefinition<'a>> {
        symbol.and_then(|symbol| {
            if let Some((namespace, _)) = lookup.split_once('.') {
                (self.style_values.import_name(symbol) == Some(namespace))
                    .then(|| self.imported_styled.get(lookup))
                    .flatten()
                    .and_then(|component| component.definition.as_ref())
            } else {
                self.definition(symbol)
            }
        })
    }

    pub(super) fn lower_css_prop(&mut self, element: &mut JSXElement<'a>, css: CssValue<'a>) {
        let name = element.opening_element.name.to_string();
        let attributes = &element.opening_element.attributes;
        let class_name = written_class_name(&self.ast, attributes);
        let renders = attributes.iter().all(|attribute| match attribute {
            JSXAttributeItem::Attribute(attribute) => !matches!(&attribute.name, JSXAttributeName::Identifier(name) if name.name == "as" || name.name == "forwardedAs"),
            JSXAttributeItem::SpreadAttribute(_) => false,
        });
        let symbol = self.style_values.jsx_symbol(&element.opening_element.name);
        let Some((lowering, props)) =
            self.css_prop_styles((&name, &name), css, symbol, renders, class_name.as_ref())
        else {
            return;
        };
        let (class_name, style) = self.css_class_and_style(props);
        let local = matches!(
            element.opening_element.name,
            JSXElementName::IdentifierReference(_)
        );
        let inline = match &lowering {
            CssPropLowering::Inline(_) => true,
            CssPropLowering::Retained | CssPropLowering::Slot(_) => false,
        };
        if (local || inline)
            && let Some(definition) = self.css_definition(symbol, &name)
        {
            let site = if local {
                forwarding::Site::LocalJsx
            } else {
                forwarding::Site::Renderer
            };
            let names = element
                .opening_element
                .attributes
                .iter()
                .filter_map(|attribute| match attribute {
                    JSXAttributeItem::Attribute(attribute) => attribute
                        .name
                        .as_identifier()
                        .map(|name| std::borrow::Cow::Borrowed(name.name.as_str())),
                    JSXAttributeItem::SpreadAttribute(_) => None,
                });
            let denied = forwarding::denied(definition, names, site);
            filter_jsx(&self.ast, &mut element.opening_element.attributes, &denied);
        }
        let class_name = match lowering {
            CssPropLowering::Retained => class_name,
            CssPropLowering::Inline(tag) => {
                crate::as_visit::rename(
                    &self.ast,
                    element,
                    JSXElementName::new_identifier(
                        SPAN,
                        Str::from_in(tag.as_str(), self.ast.allocator()),
                        &self.ast,
                    ),
                );
                class_name
            }
            CssPropLowering::Slot(slot) => {
                element
                    .opening_element
                    .attributes
                    .push(slot.attribute(&self.ast, class_name));
                None
            }
        };
        add_class_and_style(
            &self.ast,
            &mut element.opening_element.attributes,
            class_name,
            style,
        );
    }

    pub(super) fn lower_jsx_css_prop(&mut self, call: &mut CallExpression<'a>, jsx: JsxCss<'a>) {
        let JsxCss {
            css,
            element,
            lookup,
            symbol,
            class_name,
            renders,
        } = jsx;
        let Some((lowering, props)) = self.css_prop_styles(
            (&element, &lookup),
            css,
            symbol,
            renders,
            class_name.as_ref(),
        ) else {
            return;
        };
        let (class_name, style) = self.css_class_and_style(props);
        let inline = match &lowering {
            CssPropLowering::Inline(_) => true,
            CssPropLowering::Retained | CssPropLowering::Slot(_) => false,
        };
        let class_name = match lowering {
            CssPropLowering::Retained => class_name,
            CssPropLowering::Inline(tag) => {
                call.arguments[0] = Argument::from(Expression::new_string_literal(
                    SPAN,
                    Str::from_in(tag.as_str(), self.ast.allocator()),
                    None,
                    &self.ast,
                ));
                class_name
            }
            CssPropLowering::Slot(slot) => {
                if let Some(Argument::ObjectExpression(props)) = call.arguments.get_mut(1) {
                    props.properties.push(slot.property(&self.ast, class_name));
                }
                None
            }
        };
        if let Some(Argument::ObjectExpression(props)) = call.arguments.get_mut(1) {
            add_class_and_style_to_object(&self.ast, &mut props.properties, class_name, style);
            if inline && let Some(definition) = self.css_definition(symbol, &lookup) {
                let names = props
                    .properties
                    .iter()
                    .filter_map(ObjectPropertyKind::as_property)
                    .filter_map(|property| {
                        property
                            .key
                            .static_name()
                            .and_then(oxc_ast::StaticPropertyName::into_cow_str)
                    });
                let denied = forwarding::denied(definition, names, forwarding::Site::Renderer);
                filter_object(&self.ast, props, &denied);
            }
        }
    }
}
