use super::{CssValue, DevupVisitor, JsxCss};
use super::{branch_capture::BranchReads, jsx_order::AttributeOrder};
use crate::extractor::styled_record_evaluation::visit_style_expressions;
use crate::extractor::{
    extract_style_from_styled::{StyledDefinition, forwarding},
    styled_css_slot::CssPropLowering,
};
use crate::prop_modify_utils::{
    add_class_and_style, add_class_and_style_to_object, written_class_name,
};
use crate::{ExtractStyleProp, ExtractStyleValue};
use oxc_allocator::{CloneIn, FromIn, GetAllocator, TakeIn};
use oxc_ast::ast::{
    Argument, CallExpression, Expression, JSXAttributeItem, JSXAttributeName, JSXChild, JSXElement,
    JSXElementName, ObjectPropertyKind, Str,
};
use oxc_ast_visit::{Visit, VisitMut};
use oxc_span::{GetSpan, SPAN};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

#[path = "css_forwarded_props.rs"]
mod props;
use props::{filter_jsx, filter_object};

// Only references to the collision-free bindings created for this exact site.
fn owned_reads(value: &Expression<'_>, names: &FxHashSet<String>) -> Vec<String> {
    struct Reads<'b> {
        names: &'b FxHashSet<String>,
        found: Vec<String>,
    }
    impl<'a> Visit<'a> for Reads<'_> {
        fn visit_identifier_reference(&mut self, value: &oxc_ast::ast::IdentifierReference<'a>) {
            if self.names.contains(value.name.as_str()) {
                self.found.push(value.name.to_string());
            }
        }
    }
    let mut reads = Reads {
        names,
        found: Vec::new(),
    };
    reads.visit_expression(value);
    reads.found
}

// Match gen_styles' reversal of nested StaticArray property lists without
// generating classes/variables or touching their global allocation order.
fn style_slots<'b>(
    style: &'b ExtractStyleProp<'_>,
    reversed: bool,
    slots: &mut Vec<&'b str>,
) -> bool {
    match style {
        ExtractStyleProp::Static(ExtractStyleValue::Dynamic(value)) => {
            slots.push(value.identifier());
            true
        }
        ExtractStyleProp::Static(ExtractStyleValue::Static(_)) => true,
        ExtractStyleProp::StaticArray(styles) if reversed => {
            styles.iter().all(|style| style_slots(style, false, slots))
        }
        ExtractStyleProp::StaticArray(styles) => styles
            .iter()
            .rev()
            .all(|style| style_slots(style, true, slots)),
        _ => false,
    }
}

// Runtime as aliases start with a capital; they remain on the capture fallback.
fn is_intrinsic_tag(element: &JSXElement<'_>) -> bool {
    matches!(&element.opening_element.name, JSXElementName::Identifier(name)
        if name.name.as_str().starts_with(|c: char| c.is_ascii_lowercase()))
}

impl<'a> DevupVisitor<'a> {
    /// Put each owned read back in its original unconditional JSX attribute.
    /// Local metadata is rewritten before `gen_class_names/gen_styles/register`.
    fn inline_css_reads(
        &self,
        element: &mut JSXElement<'a>,
        props: &mut [ExtractStyleProp<'a>],
        raw: &mut [ExtractStyleProp<'a>],
        order: &mut AttributeOrder<'a>,
        offset: u32,
    ) -> Option<Vec<Expression<'a>>> {
        if order.values.is_empty()
            || order.values.iter().any(|(_, value)| value.is_none())
            || !is_intrinsic_tag(element)
            || element.opening_element.type_arguments.is_some()
            || element
                .children
                .iter()
                .any(|child| !matches!(child, JSXChild::Text(_)))
        {
            return None;
        }
        let names: FxHashSet<_> = order.values.iter().map(|(name, _)| name.clone()).collect();
        if order
            .values
            .iter()
            .filter_map(|(_, value)| value.as_ref())
            .any(|value| !owned_reads(value, &names).is_empty() || super::order::suspends(value))
        {
            return None;
        }
        let mut slots = Vec::new();
        for attribute in &element.opening_element.attributes {
            let JSXAttributeItem::Attribute(attribute) = attribute else {
                return None;
            };
            let JSXAttributeName::Identifier(name) = &attribute.name else {
                return None;
            };
            if matches!(
                name.name.as_str(),
                "key" | "ref" | "as" | "forwardedAs" | "style"
            ) {
                return None;
            }
            if let Some(oxc_ast::ast::JSXAttributeValue::ExpressionContainer(container)) =
                &attribute.value
            {
                if name.name == "className" {
                    return None;
                }
                if let Some(value) = container.expression.as_expression() {
                    slots.push((attribute.span.start, owned_reads(value, &names)));
                }
            }
        }
        let mut dynamic = Vec::new();
        if !props
            .iter()
            .all(|style| style_slots(style, true, &mut dynamic))
        {
            return None;
        }
        let mut style_reads = Vec::new();
        let mut emitted = Vec::new();
        for identifier in dynamic {
            let source = self.ast.allocator().alloc_str(identifier);
            let Ok(value) =
                oxc_parser::Parser::new(self.ast.allocator(), source, oxc_span::SourceType::tsx())
                    .parse_expression()
            else {
                return None;
            };
            style_reads.extend(owned_reads(&value, &names));
            emitted.push(value);
        }
        slots.push((offset, style_reads));
        slots.sort_by_key(|(offset, _)| *offset);
        let actual: Vec<_> = slots.into_iter().flat_map(|(_, reads)| reads).collect();
        if actual
            != order
                .values
                .iter()
                .map(|(name, _)| name.clone())
                .collect::<Vec<_>>()
        {
            return None;
        }
        let reads = order
            .values
            .iter()
            .filter_map(|(name, value)| {
                value
                    .as_ref()
                    .map(|value| (name.clone(), value.clone_in(self.ast.allocator())))
            })
            .collect::<FxHashMap<_, _>>();
        let mut replace = BranchReads {
            ast: &self.ast,
            reads,
        };
        for value in &mut emitted {
            replace.visit_expression(value);
        }
        for style in props.iter_mut().chain(raw.iter_mut()) {
            visit_style_expressions(&self.ast, style, &mut |value| {
                replace.visit_expression(value);
            });
        }
        for attribute in &mut element.opening_element.attributes {
            replace.visit_jsx_attribute_item(attribute);
        }
        order.values.clear();
        Some(emitted)
    }

    /// Keep a necessary shared read inside the original style object. The
    /// outer JSX retains the downstream factory/type lookup before that read.
    pub(super) fn capture_in_style(
        &self,
        element: &mut JSXElement<'a>,
        values: &mut Vec<super::capture::Captured<'a>>,
    ) {
        if values.is_empty()
            || self.pending_replacement.is_some()
            || !is_intrinsic_tag(element)
            || element.opening_element.type_arguments.is_some()
            || element
                .children
                .iter()
                .any(|child| !matches!(child, JSXChild::Text(_)))
        {
            return;
        }
        let names: FxHashSet<_> = values.iter().map(|(name, _)| name.clone()).collect();
        let mut slot = None;
        for (index, attribute) in element.opening_element.attributes.iter().enumerate() {
            let JSXAttributeItem::Attribute(attribute) = attribute else {
                return;
            };
            let JSXAttributeName::Identifier(name) = &attribute.name else {
                return;
            };
            if matches!(name.name.as_str(), "key" | "ref" | "as" | "forwardedAs") {
                return;
            }
            match &attribute.value {
                None | Some(oxc_ast::ast::JSXAttributeValue::StringLiteral(_)) => {}
                Some(oxc_ast::ast::JSXAttributeValue::ExpressionContainer(container))
                    if name.name == "style" && slot.is_none() =>
                {
                    let Some(value @ Expression::ObjectExpression(object)) =
                        container.expression.as_expression()
                    else {
                        return;
                    };
                    if !object.properties.iter().all(|property| {
                        let ObjectPropertyKind::ObjectProperty(property) = property else {
                            return false;
                        };
                        !property.computed
                            && !property.method
                            && property.kind == oxc_ast::ast::PropertyKind::Init
                            && matches!(
                                property.value,
                                Expression::Identifier(_)
                                    | Expression::StringLiteral(_)
                                    | Expression::NumericLiteral(_)
                                    | Expression::BooleanLiteral(_)
                                    | Expression::NullLiteral(_)
                            )
                    }) {
                        return;
                    }
                    let reads = owned_reads(value, &names);
                    if reads.len() <= names.len() || !names.iter().all(|name| reads.contains(name))
                    {
                        return;
                    }
                    slot = Some(index);
                }
                _ => return,
            }
        }
        if let Some(index) = slot
            && let JSXAttributeItem::Attribute(attribute) =
                &mut element.opening_element.attributes[index]
            && let Some(oxc_ast::ast::JSXAttributeValue::ExpressionContainer(container)) =
                &mut attribute.value
            && let Some(value) = container.expression.as_expression_mut()
        {
            let object = value.take_in(&self.ast);
            *value = crate::utils::call_with_values(&self.ast, std::mem::take(values), object);
        }
    }

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

    pub(super) fn lower_css_prop(
        &mut self,
        element: &mut JSXElement<'a>,
        css: CssValue<'a>,
        order: &mut AttributeOrder<'a>,
    ) {
        let offset = css.offset;
        let name = element.opening_element.name.to_string();
        let attributes = &element.opening_element.attributes;
        let class_name = written_class_name(&self.ast, attributes);
        let renders = attributes.iter().all(|attribute| match attribute {
            JSXAttributeItem::Attribute(attribute) => !matches!(&attribute.name, JSXAttributeName::Identifier(name) if name.name == "as" || name.name == "forwardedAs"),
            JSXAttributeItem::SpreadAttribute(_) => false,
        });
        let symbol = self.style_values.jsx_symbol(&element.opening_element.name);
        let mut raw = Vec::new();
        let Some((lowering, mut props)) = self.css_prop_styles(
            (&name, &name),
            css,
            symbol,
            renders,
            class_name.as_ref(),
            Some(&mut raw),
        ) else {
            self.styles
                .extend(raw.into_iter().flat_map(ExtractStyleProp::into_extract));
            return;
        };
        let inline_reads = if matches!(&lowering, CssPropLowering::Retained) {
            self.inline_css_reads(element, &mut props, &mut raw, order, offset)
        } else {
            None
        };
        self.styles
            .extend(raw.into_iter().flat_map(ExtractStyleProp::into_extract));
        let (class_name, mut style) = self.css_class_and_style(props);
        // Metadata above has identical expression code; keep the original
        // captured expression AST/spans in emitted slots, including guards.
        if let (Some(expressions), Some(Expression::ObjectExpression(object))) =
            (&inline_reads, &mut style)
        {
            // Accepted leaves generate exactly one slot each; never truncate the rewrite.
            assert_eq!(object.properties.len(), expressions.len());
            for (property, value) in object.properties.iter_mut().zip(expressions) {
                if let ObjectPropertyKind::ObjectProperty(property) = property {
                    property.value = value.clone_in(self.ast.allocator());
                }
            }
        }
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
        if inline_reads.is_some() {
            let attributes = &mut element.opening_element.attributes;
            if let Some(index) = attributes.iter().position(|attribute| {
                attribute
                    .as_attribute()
                    .and_then(|attribute| attribute.name.as_identifier())
                    .is_some_and(|name| name.name == "style")
            }) {
                let style = attributes.remove(index);
                let position = attributes
                    .iter()
                    .position(|attribute| attribute.span().start > offset)
                    .unwrap_or(attributes.len());
                attributes.insert(position, style);
            }
        }
    }

    pub(super) fn lower_jsx_css_prop(&mut self, call: &mut CallExpression<'a>, jsx: JsxCss<'a>) {
        let JsxCss {
            css,
            element,
            lookup,
            symbol,
            class_name,
            renders,
            spread_error,
        } = jsx;
        let Some((lowering, props)) = self.css_prop_styles(
            (&element, &lookup),
            css,
            symbol,
            renders,
            class_name.as_ref(),
            None,
        ) else {
            return;
        };
        if let Some((index, offset, error)) = spread_error {
            self.errors.insert(index, (offset, error));
        }
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
                if let Some(Expression::ObjectExpression(props)) = call
                    .arguments
                    .get_mut(1)
                    .and_then(Argument::as_expression_mut)
                    .map(crate::utils::unwrap_syntax_only_mut)
                {
                    props.properties.push(slot.property(&self.ast, class_name));
                }
                None
            }
        };
        if let Some(Expression::ObjectExpression(props)) = call
            .arguments
            .get_mut(1)
            .and_then(Argument::as_expression_mut)
            .map(crate::utils::unwrap_syntax_only_mut)
        {
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

#[cfg(test)]
#[path = "css_boundary_contract_tests.rs"]
mod contract_tests;
