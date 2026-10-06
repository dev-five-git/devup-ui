use crate::utils::{readable_code, unwrap_syntax_only};
use crate::{ExtractStyleProp, ExtractStyleValue};
use oxc_ast::ast::{Expression, LogicalOperator, UnaryOperator};
use std::collections::BTreeMap;

#[derive(Clone, PartialEq, Eq)]
pub(super) enum Choice {
    Bool(bool),
    Key(Option<String>),
}

#[derive(Clone, Default)]
pub(super) struct Row {
    pub choices: BTreeMap<String, Choice>,
    pub values: Vec<ExtractStyleValue>,
}

fn condition(expression: &Expression<'_>, selected: bool) -> (String, Choice) {
    match unwrap_syntax_only(expression) {
        Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::LogicalNot => {
            condition(&unary.argument, !selected)
        }
        expression => (readable_code(expression), Choice::Bool(selected)),
    }
}

fn join(left: Vec<Row>, right: Vec<Row>) -> Vec<Row> {
    let mut rows = Vec::new();
    for left in left {
        for right in &right {
            if right.choices.iter().any(|(key, value)| {
                left.choices
                    .get(key)
                    .is_some_and(|existing| existing != value)
            }) {
                continue;
            }
            let mut row = left.clone();
            row.choices.extend(right.choices.clone());
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
                        choices: BTreeMap::from([condition(test, selected)]),
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
                choices: BTreeMap::from([(name.clone(), Choice::Key(None))]),
                ..Row::default()
            }];
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort();
            for key in keys {
                let values = styles(map.get(key)?)?;
                rows.extend(join(
                    vec![Row {
                        choices: BTreeMap::from([(name.clone(), Choice::Key(Some(key.clone())))]),
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
                choices: BTreeMap::from([(name.clone(), Choice::Key(None))]),
                ..Row::default()
            }];
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort();
            for key in keys {
                rows.extend(join(
                    vec![Row {
                        choices: BTreeMap::from([(name.clone(), Choice::Key(Some(key.clone())))]),
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

fn truth(expression: &Expression<'_>, choices: &BTreeMap<String, Choice>) -> Option<bool> {
    match unwrap_syntax_only(expression) {
        Expression::BooleanLiteral(value) => Some(value.value),
        Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::LogicalNot => {
            Some(!truth(&unary.argument, choices)?)
        }
        expression => match choices.get(&readable_code(expression))? {
            Choice::Bool(value) => Some(*value),
            Choice::Key(_) => None,
        },
    }
}

pub(super) fn text(
    expression: &Expression<'_>,
    choices: &BTreeMap<String, Choice>,
) -> Option<String> {
    match unwrap_syntax_only(expression) {
        Expression::StringLiteral(value) => Some(value.value.to_string()),
        Expression::TemplateLiteral(template) => {
            let mut result = String::new();
            for (index, quasi) in template.quasis.iter().enumerate() {
                result.push_str(quasi.value.cooked.as_ref().unwrap_or(&quasi.value.raw));
                if let Some(value) = template.expressions.get(index) {
                    result.push_str(&text(value, choices)?);
                }
            }
            Some(result)
        }
        Expression::ConditionalExpression(conditional) => text(
            if truth(&conditional.test, choices)? {
                &conditional.consequent
            } else {
                &conditional.alternate
            },
            choices,
        ),
        Expression::LogicalExpression(logical) if logical.operator == LogicalOperator::Or => {
            let left = text(&logical.left, choices)?;
            if left.is_empty() {
                text(&logical.right, choices)
            } else {
                Some(left)
            }
        }
        Expression::ComputedMemberExpression(member) => {
            let Choice::Key(key) = choices.get(&readable_code(&member.expression))? else {
                return None;
            };
            let Some(key) = key else {
                return Some(String::new());
            };
            let Expression::ObjectExpression(object) = unwrap_syntax_only(&member.object) else {
                return None;
            };
            for property in &object.properties {
                if let oxc_ast::ast::ObjectPropertyKind::ObjectProperty(property) = property
                    && property.key.static_name().as_deref() == Some(key.as_str())
                {
                    return text(&property.value, choices);
                }
            }
            Some(String::new())
        }
        _ => None,
    }
}
