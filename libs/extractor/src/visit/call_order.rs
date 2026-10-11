//! Source evaluation of React props, before style selection changes the object.

use super::DevupVisitor;
use super::capture::Captured;
use super::jsx_order::AttributeOrder;
use super::order::{
    Item, Reach, Role, classify, is_merged, last_captured, reach, suspends, written_properties,
};
use super::spread_slots::is_unknown_spread;
use crate::utils::unwrap_syntax_only_mut;
use css::disassemble_property;
use oxc_allocator::TakeIn;
use oxc_ast::ast::{Expression, ObjectExpression, ObjectPropertyKind, PropertyKind};
use rustc_hash::FxHashSet;

impl<'a> DevupVisitor<'a> {
    fn property_items(&self, object: &ObjectExpression<'a>) -> Vec<Item> {
        let kept = FxHashSet::default();
        let mut written = FxHashSet::default();
        let mut items = Vec::new();
        for property in object.properties.iter().rev() {
            let item = match property {
                ObjectPropertyKind::SpreadProperty(spread) => {
                    written.extend(written_properties(&spread.argument));
                    Item {
                        role: if is_unknown_spread(&spread.argument) {
                            Role::Stays
                        } else {
                            Role::Moved
                        },
                        reach: reach(&self.bindings, &spread.argument),
                        lost: !is_unknown_spread(&spread.argument),
                        snapshot: is_unknown_spread(&spread.argument),
                        suspends: suspends(&spread.argument),
                    }
                }
                ObjectPropertyKind::ObjectProperty(property) => {
                    let key = property.key.static_name();
                    let (role, style) = key
                        .as_ref()
                        .map_or((Role::Stays, false), |key| classify(key, &kept));
                    let keys: Vec<_> = key
                        .as_ref()
                        .filter(|_| style)
                        .into_iter()
                        .flat_map(|key| disassemble_property(key).map(std::borrow::Cow::into_owned))
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
    pub(super) fn order_call_props(&mut self, props: &mut Expression<'a>) -> AttributeOrder<'a> {
        let mut order = AttributeOrder::default();
        let style_order = match crate::utils::unwrap_syntax_only(props) {
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
        for property in &mut object.properties {
            if let ObjectPropertyKind::ObjectProperty(property) = property
                && property.key.static_name().is_none_or(|key| key != "css")
            {
                self.style_values.read_in(&self.ast, &mut property.value);
            }
        }
        let items = self.property_items(object);
        let Some(mut last) = last_captured(&items, false) else {
            return order;
        };
        if items
            .iter()
            .any(|item| item.role == Role::Hoisted && item.reach != Reach::Constant)
        {
            last = items.len() - 1;
        }
        for (item, property) in items.iter().zip(&mut object.properties).take(last + 1) {
            if item.reach == Reach::Constant && !item.snapshot {
                continue;
            }
            let mut captured = Vec::new();
            match property {
                ObjectPropertyKind::SpreadProperty(spread)
                    if is_unknown_spread(&spread.argument) =>
                {
                    let name = self.names.fresh("__devupSpread");
                    if unsafe_to_extract(&spread.argument) {
                        self.report_hidden_selector(&spread.argument);
                        captured.push(self.snapshot_as(name, &mut spread.argument));
                    } else {
                        order.spreads.insert(spread.span.start, order.values.len());
                        order.values.push((name, None));
                    }
                }
                ObjectPropertyKind::SpreadProperty(spread) => {
                    self.capture_shape(&mut spread.argument, &mut captured);
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
                    let merged = property
                        .key
                        .static_name()
                        .is_some_and(|key| is_merged(&key));
                    if property
                        .key
                        .static_name()
                        .is_some_and(|key| key == "styleOrder")
                        && matches!(
                            self.parsed_order(&property.value),
                            crate::utils::ParsedStyleOrder::Unsupported
                        )
                    {
                        continue;
                    }
                    property.shorthand = false;
                    if item.role == Role::Moved && !merged {
                        self.capture_shape(&mut property.value, &mut captured);
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
        if let Expression::ObjectExpression(object) = props {
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

impl DevupVisitor<'_> {
    pub(super) fn report_hidden_selector(&mut self, value: &Expression<'_>) {
        if let Expression::ObjectExpression(object) = crate::utils::unwrap_syntax_only(value) {
            for property in &object.properties {
                if let ObjectPropertyKind::ObjectProperty(property) = property
                    && let Some(key) = property.key.static_name()
                    && (key.starts_with('_')
                        || key.starts_with('@')
                        || key.contains('&')
                        || matches!(key.as_ref(), "selectors" | "typography" | "css"))
                {
                    self.errors.push((property.span.start,crate::utils::build_time_error("style spread",&key,"an unknown computed key or accessor can replace or hide this whole style object; write the selector as an explicit prop after the spread")));
                }
            }
        }
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
