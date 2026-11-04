//! The order a JSX element evaluates its attributes and children in.

use super::DevupVisitor;
use super::capture::Captured;
use super::element_props::{flatten_known, is_unknown_spread};
use super::order::{
    Item, Reach, Role, classify, is_merged, is_style, last_captured_with_lone_moved, reach,
    suspends, written_properties,
};
use crate::utils::Suspends;
use css::disassemble_property;
use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::ast::JSXAttributeItem::{self, Attribute, SpreadAttribute};
use oxc_ast::ast::JSXAttributeName::{self, Identifier};
use oxc_ast::ast::{
    Expression, JSXAttribute, JSXAttributeValue, JSXChild, JSXElement, JSXExpressionContainer,
};
use oxc_ast_visit::Visit;
use oxc_span::SPAN;
use rustc_hash::{FxHashMap, FxHashSet};
use std::borrow::Cow;

/// What an element evaluates once, in the order its attributes are written
#[derive(Default)]
pub(super) struct AttributeOrder<'a> {
    /// The names and values; the value of a spread is read from the element
    /// once the build is done with it
    pub values: Vec<(String, Option<Expression<'a>>)>,
    /// Where in `values` the snapshot of each spread is, by where it starts
    pub spreads: FxHashMap<u32, usize>,
    /// The name the type `as` gives was captured under
    pub as_name: Option<String>,
}

/// Where the attribute `name` stands once the element is built, and whether
/// it writes a style that a spread after it can replace
fn classify_attribute(name: &JSXAttributeName<'_>, kept: &FxHashSet<String>) -> (Role, bool) {
    match name {
        Identifier(name) => classify(&name.name, kept),
        JSXAttributeName::NamespacedName(_) => (Role::Stays, false),
    }
}

fn value_of<'b, 'a>(attribute: &'b JSXAttribute<'a>) -> Option<&'b Expression<'a>> {
    match &attribute.value {
        Some(JSXAttributeValue::ExpressionContainer(container)) => {
            container.expression.as_expression()
        }
        _ => None,
    }
}

fn child_suspends(child: &JSXChild<'_>) -> bool {
    let mut suspends = Suspends::default();
    suspends.visit_jsx_child(child);
    suspends.found
}

impl<'a> DevupVisitor<'a> {
    /// The items for the order `attributes` evaluate in
    fn attribute_items(
        &self,
        attributes: &[JSXAttributeItem<'a>],
        kept: &FxHashSet<String>,
        css_only: bool,
    ) -> Vec<Item> {
        let mut written: FxHashSet<String> = FxHashSet::default();
        let mut lost = vec![false; attributes.len()];
        for (index, attribute) in attributes.iter().enumerate().rev() {
            let names: Vec<String> = match attribute {
                Attribute(attribute) => match &attribute.name {
                    Identifier(name) if is_style(&name.name, kept) => {
                        let names: Vec<_> = disassemble_property(&name.name)
                            .map(Cow::into_owned)
                            .collect();
                        lost[index] = names.len() > 1 || names.iter().all(|n| written.contains(n));
                        names
                    }
                    _ => Vec::new(),
                },
                SpreadAttribute(spread) => written_properties(&spread.argument)
                    .iter()
                    .flat_map(|key| disassemble_property(key).map(Cow::into_owned))
                    .collect(),
            };
            written.extend(names);
        }
        attributes
            .iter()
            .zip(lost)
            .map(|(attribute, lost)| match attribute {
                Attribute(attribute) => {
                    let value = value_of(attribute);
                    let (role, _) = classify_attribute(&attribute.name, kept);
                    Item {
                        role,
                        reach: value.map_or(Reach::Constant, |value| reach(&self.bindings, value)),
                        lost,
                        snapshot: false,
                        suspends: value.is_some_and(suspends),
                    }
                }
                SpreadAttribute(spread) => {
                    let read = reach(&self.bindings, &spread.argument);
                    let branch = !css_only && self.is_rule_branch(&spread.argument);
                    let unknown = is_unknown_spread(&spread.argument) && !branch;
                    let snapshot = unknown;
                    Item {
                        role: if unknown { Role::Stays } else { Role::Moved },
                        reach: read,
                        lost: !unknown && !branch,
                        snapshot,
                        suspends: suspends(&spread.argument),
                    }
                }
            })
            .collect()
    }

    /// Capture the attributes of `element` that must be evaluated before it is
    /// built for them to evaluate in the order written: the ones compiling
    /// moves behind a later one, drops, or repeats, and everything written
    /// before the last of them. `kept` are the attributes a styled component
    /// takes as they are
    pub(super) fn order_attributes(
        &mut self,
        element: &mut JSXElement<'a>,
        kept: &FxHashSet<String>,
        css_only: bool,
    ) -> AttributeOrder<'a> {
        let mut order = AttributeOrder::default();
        let style_order = element.opening_element.attributes.iter().rev().find_map(|attribute| {
            match attribute {
                Attribute(attribute)
                    if !kept.contains("styleOrder")
                        && matches!(&attribute.name, Identifier(name) if name.name == "styleOrder") =>
                {
                    attribute.value.as_ref().map(|value| {
                        crate::style_order::attribute_order(value, self.ast.allocator()).as_static()
                    })
                }
                _ => None,
            }
        }).flatten();
        let kind = self.bindings.element(&element.opening_element.name);
        let default_tag = kind.as_ref().map_or("div", |kind| kind.to_tag());
        let children_suspend = element.children.iter().any(child_suspends);
        let css_element = self
            .takes_css(&element.opening_element.name)
            .then(|| element.opening_element.name.to_string());
        let attributes = &mut element.opening_element.attributes;
        let css_entry = css_element.as_ref().and_then(|_| attributes.iter().rposition(|attribute| matches!(attribute, Attribute(attribute) if matches!(&attribute.name, Identifier(name) if name.name == "css"))));
        for attribute in attributes.iter_mut() {
            if let SpreadAttribute(spread) = attribute
                && !css_only
                && !is_unknown_spread(&spread.argument)
                && let Expression::ObjectExpression(object) =
                    crate::utils::unwrap_syntax_only_mut(&mut spread.argument)
            {
                flatten_known(&self.ast, object);
            }
        }
        let items = self.attribute_items(attributes, kept, css_only);
        for attribute in attributes.iter_mut() {
            if let Attribute(attribute) = attribute
                && !matches!(&attribute.name, Identifier(name) if name.name == "css")
            {
                let class_name =
                    matches!(&attribute.name, Identifier(name) if name.name == "className");
                if let Some(JSXAttributeValue::ExpressionContainer(container)) =
                    &mut attribute.value
                    && let Some(value) = container.expression.as_expression_mut()
                {
                    if class_name {
                        self.style_values.read_in_class(&self.ast, value);
                    } else {
                        self.style_values.read_in(&self.ast, value);
                    }
                }
            }
        }
        // A lone primitive class call stays in one final `call() || ""` slot;
        // there is no other attribute or child whose evaluation it can cross.
        if kind.is_some()
            && element.children.is_empty()
            && let [Attribute(attribute)] = attributes.as_slice()
            && matches!(&attribute.name, Identifier(name) if name.name == "className")
            && let Some(value @ Expression::CallExpression(_)) = value_of(attribute)
            && !suspends(value)
        {
            return order;
        }
        let allow_lone_moved = attributes.len() == 1
            && element.children.is_empty()
            && matches!(
                attributes.first(),
                Some(Attribute(attribute))
                    if matches!(&attribute.name, Identifier(name) if name.name != "css" && is_style(&name.name, kept))
            );
        let allow_lone_runs = allow_lone_moved
            && attributes.first().is_some_and(|attribute| match attribute {
                Attribute(attribute) => value_of(attribute).is_some_and(|value| {
                    matches!(
                        crate::utils::unwrap_syntax_only(value),
                        Expression::TemplateLiteral(_)
                    )
                }),
                SpreadAttribute(_) => false,
            });
        let Some(last) = last_captured_with_lone_moved(
            &items,
            children_suspend,
            allow_lone_moved,
            allow_lone_runs,
        ) else {
            return order;
        };
        for (index, (item, attribute)) in items
            .iter()
            .zip(attributes.iter_mut())
            .enumerate()
            .take(last + 1)
        {
            let mut captured = Vec::new();
            match attribute {
                SpreadAttribute(spread)
                    if !css_only
                        && self.is_rule_branch(&spread.argument)
                        && super::element_props::unreadable_branch(&spread.argument) =>
                {
                    continue;
                }
                SpreadAttribute(_) if item.reach == Reach::Constant && !item.snapshot => continue,
                SpreadAttribute(spread)
                    if is_unknown_spread(&spread.argument)
                        && (css_only || !self.is_rule_branch(&spread.argument)) =>
                {
                    let name = self.names.fresh("__devupSpread");
                    if super::call_order::unsafe_to_extract(&spread.argument) {
                        captured.push(self.snapshot_as(name, &mut spread.argument));
                    } else {
                        order.spreads.insert(spread.span.start, order.values.len());
                        order.values.push((name, None));
                    }
                }
                SpreadAttribute(spread) => {
                    self.capture_shape(&mut spread.argument, &mut captured);
                }
                Attribute(attribute) => {
                    let merged =
                        matches!(&attribute.name, Identifier(name) if is_merged(&name.name));
                    let class_name =
                        matches!(&attribute.name, Identifier(name) if name.name == "className");
                    let order_value =
                        matches!(&attribute.name, Identifier(name) if name.name == "styleOrder");
                    let Some(value) = attribute.value.as_mut().and_then(|value| match value {
                        JSXAttributeValue::ExpressionContainer(container) => {
                            container.expression.as_expression_mut()
                        }
                        _ => None,
                    }) else {
                        continue;
                    };
                    if item.reach == Reach::Constant {
                        continue;
                    }
                    if order_value
                        && !kept.contains("styleOrder")
                        && matches!(
                            self.parsed_order(value),
                            crate::utils::ParsedStyleOrder::Unsupported
                        )
                    {
                        continue;
                    }
                    match item.role {
                        Role::Hoisted => {
                            let as_name = self.names.fresh("DevupAs");
                            let (name, original) = self.capture_as(as_name.clone(), value);
                            let fallback =
                                Expression::new_string_literal(SPAN, default_tag, None, &self.ast);
                            captured.push((
                                name,
                                Expression::new_logical_expression(
                                    SPAN,
                                    original,
                                    oxc_syntax::operator::LogicalOperator::Or,
                                    fallback,
                                    &self.ast,
                                ),
                            ));
                            order.as_name = Some(as_name);
                        }
                        Role::Moved if !merged => {
                            let css = self.css_capture.take();
                            if css_entry == Some(index) {
                                self.css_capture.clone_from(&css_element);
                            }
                            self.capture_shape(value, &mut captured);
                            self.css_capture = css;
                        }
                        Role::Moved if class_name => {
                            self.capture_class_name(value, &mut captured, style_order);
                        }
                        Role::Moved | Role::Stays => {
                            let fresh = self.names.fresh("__devupValue");
                            captured.push(self.capture_as(fresh, value));
                        }
                    }
                }
            }
            order.values.extend(
                captured
                    .into_iter()
                    .map(|(name, value)| (name, Some(value))),
            );
        }
        order
    }

    /// Capture the children of `element` up to the last that holds an `await`
    /// or `yield`, which only arguments can hold, in the order written
    pub(super) fn capture_children(
        &mut self,
        children: &mut oxc_allocator::Vec<'a, JSXChild<'a>>,
    ) -> Vec<Captured<'a>> {
        let Some(last) = children.iter().rposition(child_suspends) else {
            return Vec::new();
        };
        let mut captured = Vec::new();
        for child in children.iter_mut().take(last + 1) {
            let mut value = match child {
                JSXChild::ExpressionContainer(container) => {
                    let Some(value) = container.expression.as_expression_mut() else {
                        continue;
                    };
                    if reach(&self.bindings, value) == Reach::Constant {
                        continue;
                    }
                    let name = self.names.fresh("__devupValue");
                    captured.push(self.capture_as(name, value));
                    continue;
                }
                JSXChild::Spread(spread) => {
                    let name = self.names.fresh("__devupValue");
                    captured.push(self.capture_as(name, &mut spread.expression));
                    continue;
                }
                JSXChild::Element(element) => {
                    Expression::JSXElement(element.clone_in_with_semantic_ids(self.ast.allocator()))
                }
                JSXChild::Fragment(fragment) => Expression::JSXFragment(
                    fragment.clone_in_with_semantic_ids(self.ast.allocator()),
                ),
                JSXChild::Text(_) => continue,
            };
            let name = self.names.fresh("__devupValue");
            captured.push(self.capture_as(name, &mut value));
            *child = JSXChild::ExpressionContainer(JSXExpressionContainer::boxed(
                SPAN,
                value.into(),
                &self.ast,
            ));
        }
        captured
    }
}
