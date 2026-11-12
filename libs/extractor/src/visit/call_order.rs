//! Source evaluation of React props, before style selection changes the object.

use super::DevupVisitor;
use super::capture::Captured;
use super::element_props::{flatten_known, is_unknown_spread};
use super::jsx_order::AttributeOrder;
use super::order::{
    Item, Reach, Role, classify, is_merged, last_captured, reach, suspends, written_properties,
};
use crate::utils::{get_str_by_property_key, unwrap_syntax_only_mut};
use css::disassemble_property;
use oxc_allocator::TakeIn;
use oxc_ast::ast::{Expression, ObjectExpression, ObjectPropertyKind, PropertyKind};
use rustc_hash::FxHashSet;

impl<'a> DevupVisitor<'a> {
    fn property_items(
        &self,
        object: &ObjectExpression<'a>,
        kept: &FxHashSet<String>,
        css_only: bool,
    ) -> Vec<Item> {
        let mut written = FxHashSet::default();
        let mut items = Vec::new();
        for property in object.properties.iter().rev() {
            let item = match property {
                ObjectPropertyKind::SpreadProperty(spread) => {
                    written.extend(written_properties(&spread.argument));
                    let branch = !css_only && self.is_rule_branch(&spread.argument);
                    let unknown = is_unknown_spread(&spread.argument) && !branch;
                    Item {
                        role: if unknown { Role::Stays } else { Role::Moved },
                        reach: reach(&self.bindings, &spread.argument),
                        lost: !unknown && !branch,
                        snapshot: unknown,
                        suspends: suspends(&spread.argument),
                    }
                }
                ObjectPropertyKind::ObjectProperty(property) => {
                    let key = get_str_by_property_key(&property.key);
                    let (role, style) = key
                        .as_ref()
                        .map_or((Role::Stays, false), |key| classify(key.as_ref(), kept));
                    let keys: Vec<_> = key
                        .as_ref()
                        .filter(|_| style)
                        .into_iter()
                        .flat_map(|key| {
                            disassemble_property(key.as_ref()).map(std::borrow::Cow::into_owned)
                        })
                        .collect();
                    let lost = keys.len() > 1
                        || (!keys.is_empty() && keys.iter().all(|key| written.contains(key)));
                    written.extend(keys);
                    Item {
                        role,
                        reach: reach(&self.bindings, &property.value),
                        lost,
                        snapshot: property.computed && key.is_none(),
                        suspends: suspends(&property.value)
                            || property.key.as_expression().is_some_and(suspends),
                    }
                }
            };
            items.push(item);
        }
        items.reverse();
        items
    }

    /// Capture the source prefix before extracting or discarding any prop.
    pub(super) fn order_call_props(
        &mut self,
        props: &mut Expression<'a>,
        kept: &FxHashSet<String>,
        css_only: bool,
        css_element: Option<&str>,
    ) -> AttributeOrder<'a> {
        let mut order = AttributeOrder::default();
        let style_order = match crate::utils::unwrap_syntax_only(props) {
            _ if kept.contains("styleOrder") => None,
            Expression::ObjectExpression(object) => {
                object
                    .properties
                    .iter()
                    .find_map(|property| match property {
                        ObjectPropertyKind::ObjectProperty(property)
                            if property
                                .key
                                .static_name()
                                .is_some_and(|key| key == "styleOrder") =>
                        {
                            self.parsed_order(&property.value).as_static()
                        }
                        _ => None,
                    })
            }
            _ => None,
        };
        let Expression::ObjectExpression(object) = unwrap_syntax_only_mut(props) else {
            return order;
        };
        if !css_only {
            for property in &mut object.properties {
                if let ObjectPropertyKind::SpreadProperty(spread) = property {
                    super::element_props::as_conditional(&self.ast, &mut spread.argument);
                }
                if let ObjectPropertyKind::SpreadProperty(spread) = property
                    && is_unknown_spread(&spread.argument)
                {
                    self.note_hidden_styles(&mut order, &spread.argument);
                }
            }
            flatten_known(&self.ast, object);
        }
        let css_entry = css_element.and_then(|_| object.properties.iter().rposition(|property| matches!(property, ObjectPropertyKind::ObjectProperty(property) if !property.computed && property.key.static_name().is_some_and(|key|key=="css"))));
        let items = self.property_items(object, kept, css_only);
        for property in &mut object.properties {
            if let ObjectPropertyKind::ObjectProperty(property) = property
                && property.key.static_name().is_none_or(|key| key != "css")
            {
                if property
                    .key
                    .static_name()
                    .is_some_and(|key| key == "className")
                {
                    self.style_values
                        .read_in_class(&self.ast, &mut property.value);
                } else {
                    self.style_values.read_in(&self.ast, &mut property.value);
                }
            }
        }
        let Some(mut last) = last_captured(&items, false) else {
            return order;
        };
        if items
            .iter()
            .any(|item| item.role == Role::Hoisted && item.reach != Reach::Constant)
        {
            last = items.len() - 1;
        }
        for (index, (item, property)) in items
            .iter()
            .zip(&mut object.properties)
            .enumerate()
            .take(last + 1)
        {
            if item.reach == Reach::Constant && !item.snapshot {
                continue;
            }
            let mut captured = Vec::new();
            match property {
                ObjectPropertyKind::SpreadProperty(spread)
                    if !css_only && self.is_rule_branch(&spread.argument) =>
                {
                    if !super::element_props::unreadable_branch(&spread.argument) {
                        self.capture_shape(&mut spread.argument, &mut captured);
                    }
                }
                ObjectPropertyKind::SpreadProperty(spread) => {
                    let name = self.names.fresh("__devupSpread");
                    if unsafe_to_extract(&spread.argument) {
                        captured.push(self.snapshot_as(name, &mut spread.argument));
                    } else {
                        order.spreads.insert(spread.span.start, order.values.len());
                        order.values.push((name, None));
                    }
                }
                ObjectPropertyKind::ObjectProperty(written)
                    if written.computed && written.key.static_name().is_none() =>
                {
                    let span = written.span;
                    let original = property.take_in(&self.ast);
                    let mut value = Expression::new_object_expression(
                        span,
                        oxc_allocator::Vec::from_array_in([original], &self.ast),
                        &self.ast,
                    );
                    let name = self.names.fresh("__devupSpread");
                    captured.push(self.snapshot_as(name, &mut value));
                    *property = ObjectPropertyKind::new_spread_property(span, value, &self.ast);
                }
                ObjectPropertyKind::ObjectProperty(property) => {
                    let merged =
                        get_str_by_property_key(&property.key).is_some_and(|key| is_merged(&key));
                    if property
                        .key
                        .static_name()
                        .is_some_and(|key| key == "styleOrder")
                        && !kept.contains("styleOrder")
                        && matches!(
                            self.parsed_order(&property.value),
                            crate::utils::ParsedStyleOrder::Unsupported
                        )
                    {
                        continue;
                    }
                    property.shorthand = false;
                    if item.role == Role::Moved && !merged {
                        let css = self.css_capture.take();
                        if css_entry == Some(index) {
                            self.css_capture = css_element.map(str::to_owned);
                        }
                        self.capture_shape(&mut property.value, &mut captured);
                        self.css_capture = css;
                    } else if property
                        .key
                        .static_name()
                        .is_some_and(|key| key == "className")
                    {
                        self.capture_class_name(&mut property.value, &mut captured, style_order);
                    } else {
                        let name = self.names.fresh("__devupValue");
                        captured.push(self.capture_as(name, &mut property.value));
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

    /// Finish delayed snapshots after their statically written style keys were extracted.
    pub(super) fn finish_call_props(
        &mut self,
        order: AttributeOrder<'a>,
        props: &mut Expression<'a>,
    ) -> Vec<Captured<'a>> {
        let mut values = order.values;
        if let Expression::ObjectExpression(object) = unwrap_syntax_only_mut(props) {
            for property in &mut object.properties {
                if let ObjectPropertyKind::SpreadProperty(spread) = property
                    && let Some(index) = order.spreads.get(&spread.span.start)
                {
                    let name = values[*index].0.clone();
                    values[*index].1 = Some(self.snapshot_as(name, &mut spread.argument).1);
                }
            }
        }
        values
            .into_iter()
            .filter_map(|(name, value)| Some((name, value?)))
            .collect()
    }
}

/// Accessors and computed keys must be evaluated by native object copying.
pub(super) fn unsafe_to_extract(value: &Expression<'_>) -> bool {
    match crate::utils::unwrap_syntax_only(value) {
        Expression::ObjectExpression(object) => {
            object.properties.iter().any(|property| match property {
                ObjectPropertyKind::ObjectProperty(property) => {
                    property.kind != PropertyKind::Init || property.computed || property.method
                }
                ObjectPropertyKind::SpreadProperty(_) => false,
            })
        }
        _ => false,
    }
}
