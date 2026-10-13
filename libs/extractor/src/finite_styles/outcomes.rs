use crate::utils::{readable_code, unwrap_syntax_only};
use crate::{ExtractStyleProp, ExtractStyleValue};
use oxc_ast::AstKind;
use oxc_ast::ast::{Expression, LogicalOperator, ObjectPropertyKind, UnaryOperator};
use std::collections::BTreeMap;

/// One way the generated conditions and lookups can come out, with the
/// declarations that way sets. A name is decided either as a condition or as
/// a lookup key, never as both.
#[derive(Clone, Default)]
pub(super) struct Row {
    /// Whether each generated condition holds
    flags: BTreeMap<String, bool>,
    /// The key each generated lookup selects, `None` when it selects no key
    keys: BTreeMap<String, Option<String>>,
    pub values: Vec<ExtractStyleValue>,
}

impl Row {
    /// Whether `other` already decided a name of this row in another way
    fn conflicts_with(&self, other: &Self) -> bool {
        self.flags.iter().any(|(name, value)| {
            other.flags.get(name).is_some_and(|known| known != value)
                || other.keys.contains_key(name)
        }) || self.keys.iter().any(|(name, key)| {
            other.keys.get(name).is_some_and(|known| known != key) || other.flags.contains_key(name)
        })
    }
}

fn condition(expression: &Expression<'_>, selected: bool) -> (String, bool) {
    match unwrap_syntax_only(expression) {
        Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::LogicalNot => {
            condition(&unary.argument, !selected)
        }
        expression => (readable_code(expression), selected),
    }
}

fn join(left: Vec<Row>, right: Vec<Row>) -> Vec<Row> {
    let mut rows = Vec::new();
    for left in left {
        for right in &right {
            if right.conflicts_with(&left) {
                continue;
            }
            let mut row = left.clone();
            row.flags.extend(right.flags.clone());
            row.keys.extend(right.keys.clone());
            row.values.extend(right.values.clone());
            rows.push(row);
        }
    }
    rows
}

pub(super) fn styles(props: &[ExtractStyleProp<'_>]) -> Option<Vec<Row>> {
    let mut rows = vec![Row::default()];
    for prop in props {
        rows = join(rows, alternatives(prop)?);
    }
    Some(rows)
}

fn alternatives(prop: &ExtractStyleProp<'_>) -> Option<Vec<Row>> {
    match prop {
        ExtractStyleProp::Static(
            value @ (ExtractStyleValue::Static(_) | ExtractStyleValue::Typography(_)),
        ) => Some(vec![Row {
            values: vec![value.clone()],
            ..Row::default()
        }]),
        ExtractStyleProp::StaticArray(props) => styles(props),
        ExtractStyleProp::Conditional {
            condition: test,
            consequent,
            alternate,
        } => {
            let mut rows = Vec::new();
            for (selected, side) in [(true, consequent), (false, alternate)] {
                let side = match side {
                    Some(side) => alternatives(side)?,
                    None => vec![Row::default()],
                };
                rows.extend(join(
                    vec![Row {
                        flags: BTreeMap::from([condition(test, selected)]),
                        ..Row::default()
                    }],
                    side,
                ));
            }
            Some(rows)
        }
        ExtractStyleProp::Enum { map, condition } => {
            let name = readable_code(condition);
            let mut rows = vec![Row {
                keys: BTreeMap::from([(name.clone(), None)]),
                ..Row::default()
            }];
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort();
            for key in keys {
                let values = styles(map.get(key)?)?;
                rows.extend(join(
                    vec![Row {
                        keys: BTreeMap::from([(name.clone(), Some(key.clone()))]),
                        ..Row::default()
                    }],
                    values,
                ));
            }
            Some(rows)
        }
        ExtractStyleProp::MemberExpression { map, expression } => {
            let name = readable_code(expression);
            let mut rows = vec![Row {
                keys: BTreeMap::from([(name.clone(), None)]),
                ..Row::default()
            }];
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort();
            for key in keys {
                rows.extend(join(
                    vec![Row {
                        keys: BTreeMap::from([(name.clone(), Some(key.clone()))]),
                        ..Row::default()
                    }],
                    alternatives(map.get(key)?)?,
                ));
            }
            Some(rows)
        }
        ExtractStyleProp::Static(_)
        | ExtractStyleProp::Expression { .. }
        | ExtractStyleProp::Unreadable { .. }
        | ExtractStyleProp::Diagnostic { .. } => None,
    }
}

fn truth(expression: &Expression<'_>, row: &Row) -> Option<bool> {
    match unwrap_syntax_only(expression) {
        Expression::BooleanLiteral(value) => Some(value.value),
        Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::LogicalNot => {
            Some(!truth(&unary.argument, row)?)
        }
        expression => row.flags.get(&readable_code(expression)).copied(),
    }
}

/// The text a generated class expression evaluates to in `row`.
///
/// Each shape is read by its own function that answers `None` for every other
/// shape, so the shapes are tried in turn and an expression that is none of
/// them is declined by the end of the chain.
pub(super) fn text(expression: &Expression<'_>, row: &Row) -> Option<String> {
    let node = AstKind::from_expression(unwrap_syntax_only(expression));
    literal(node)
        .or_else(|| template(node, row))
        .or_else(|| conditional(node, row))
        .or_else(|| either(node, row))
        .or_else(|| lookup(node, row))
}

fn literal(node: AstKind<'_>) -> Option<String> {
    Some(node.as_string_literal()?.value.to_string())
}

fn template(node: AstKind<'_>, row: &Row) -> Option<String> {
    let template = node.as_template_literal()?;
    let mut result = String::new();
    for (index, quasi) in template.quasis.iter().enumerate() {
        result.push_str(quasi.value.cooked.as_ref().unwrap_or(&quasi.value.raw));
        if let Some(value) = template.expressions.get(index) {
            result.push_str(&text(value, row)?);
        }
    }
    Some(result)
}

fn conditional(node: AstKind<'_>, row: &Row) -> Option<String> {
    let conditional = node.as_conditional_expression()?;
    text(
        if truth(&conditional.test, row)? {
            &conditional.consequent
        } else {
            &conditional.alternate
        },
        row,
    )
}

/// `left || right`: the right side stands in for an empty left side.
fn either(node: AstKind<'_>, row: &Row) -> Option<String> {
    let logical = node
        .as_logical_expression()
        .filter(|logical| logical.operator == LogicalOperator::Or)?;
    let left = text(&logical.left, row)?;
    if left.is_empty() {
        text(&logical.right, row)
    } else {
        Some(left)
    }
}

/// `({ key: value })[name]`: the value of the key the row selects for `name`.
fn lookup(node: AstKind<'_>, row: &Row) -> Option<String> {
    let member = node.as_computed_member_expression()?;
    let Some(key) = row.keys.get(&readable_code(&member.expression))? else {
        return Some(String::new());
    };
    let object =
        AstKind::from_expression(unwrap_syntax_only(&member.object)).as_object_expression()?;
    for property in &object.properties {
        if let ObjectPropertyKind::ObjectProperty(property) = property
            && property.key.static_name().as_deref() == Some(key.as_str())
        {
            return text(&property.value, row);
        }
    }
    Some(String::new())
}
