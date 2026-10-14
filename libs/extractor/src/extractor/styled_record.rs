use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::{
    ast::{Expression, ObjectProperty, ObjectPropertyKind, PropertyKind},
    builder::AstBuilder,
};
use oxc_span::GetSpan;
use oxc_syntax::operator::{BinaryOperator, LogicalOperator};

use super::styled_record_evaluation::Evaluations;
use crate::utils::{
    get_str_by_property_key, get_string_by_literal_expression, readable_code, unwrap_syntax_only,
};

struct Entry<'a> {
    key: String,
    property: oxc_allocator::Box<'a, ObjectProperty<'a>>,
    present: Expression<'a>,
    conditional: bool,
}

/// A prepared record is committed only after every spread and key was readable.
pub(super) struct PreparedRecord<'a> {
    pub rules: Expression<'a>,
    pub bindings: Vec<(String, Expression<'a>)>,
}

struct Preparation<'b, 'a> {
    reads: Evaluations<'b, 'a>,
    affected: bool,
}

pub(super) fn prepare<'a>(
    builder: &AstBuilder<'a>,
    rules: &Expression<'a>,
    scope: &Expression<'a>,
) -> Option<PreparedRecord<'a>> {
    let mut preparation = Preparation {
        reads: Evaluations::new(builder, readable_code(scope)),
        affected: false,
    };
    let entries = preparation.record(rules, &preparation.reads.boolean(true))?;
    if !preparation.affected {
        return None;
    }
    Some(PreparedRecord {
        rules: preparation.materialize(rules.span(), entries),
        bindings: preparation.reads.bindings,
    })
}

impl<'a> Preparation<'_, 'a> {
    fn record(
        &mut self,
        source: &Expression<'a>,
        guard: &Expression<'a>,
    ) -> Option<Vec<Entry<'a>>> {
        match unwrap_syntax_only(source) {
            Expression::ObjectExpression(object) => {
                let mut entries = Vec::new();
                for property in &object.properties {
                    match property {
                        ObjectPropertyKind::ObjectProperty(property) => {
                            let key = get_str_by_property_key(&property.key)?.into_owned();
                            if property.method || property.kind != PropertyKind::Init {
                                return None;
                            }
                            let mut property = property.clone_in(self.reads.builder.allocator());
                            if !matches!(
                                key.as_str(),
                                "styleOrder" | "styleVars" | "props" | "params" | "as"
                            ) {
                                property.value = self.value(&property.value, guard)?;
                            }
                            property.shorthand = false;
                            let entry = Entry {
                                key,
                                property,
                                present: self.reads.boolean(true),
                                conditional: false,
                            };
                            self.append(&mut entries, entry);
                        }
                        ObjectPropertyKind::SpreadProperty(spread) => {
                            for entry in self.record(&spread.argument, guard)? {
                                self.append(&mut entries, entry);
                            }
                        }
                    }
                }
                Some(entries)
            }
            Expression::ConditionalExpression(choice) => {
                let test = self.reads.capture(&choice.test, guard);
                let yes_guard = self.reads.choose(guard, &test, &self.reads.boolean(false));
                let no_guard = self.reads.choose(
                    guard,
                    &self.reads.choose(
                        &test,
                        &self.reads.boolean(false),
                        &self.reads.boolean(true),
                    ),
                    &self.reads.boolean(false),
                );
                let yes = self.record(&choice.consequent, &yes_guard)?;
                let no = self.record(&choice.alternate, &no_guard)?;
                Some(self.choice(&test, yes, no))
            }
            Expression::LogicalExpression(logical) if logical.operator == LogicalOperator::And => {
                let test = self.reads.capture(&logical.left, guard);
                let branch_guard = self.reads.choose(guard, &test, &self.reads.boolean(false));
                let yes = self.record(&logical.right, &branch_guard)?;
                Some(self.choice(&test, yes, Vec::new()))
            }
            Expression::NullLiteral(_) | Expression::BooleanLiteral(_) => Some(Vec::new()),
            Expression::Identifier(identifier) if identifier.name == "undefined" => {
                Some(Vec::new())
            }
            _ => None,
        }
    }

    fn append(&mut self, entries: &mut Vec<Entry<'a>>, mut next: Entry<'a>) {
        if let Some(index) = entries.iter().position(|entry| entry.key == next.key) {
            let previous = entries.remove(index);
            self.affected |= previous.conditional || next.conditional;
            next.property.value = self.reads.choose(
                &next.present,
                &next.property.value,
                &previous.property.value,
            );
            next.present =
                self.reads
                    .choose(&next.present, &self.reads.boolean(true), &previous.present);
            next.conditional |= previous.conditional;
        }
        entries.push(next);
    }

    fn choice(
        &self,
        test: &Expression<'a>,
        yes: Vec<Entry<'a>>,
        mut no: Vec<Entry<'a>>,
    ) -> Vec<Entry<'a>> {
        let mut result = Vec::new();
        for mut entry in yes {
            let other = no
                .iter()
                .position(|other| other.key == entry.key)
                .map(|index| no.remove(index));
            let (present, value) = other.map_or_else(
                || (self.reads.boolean(false), self.reads.undefined()),
                |other| {
                    (
                        other.present,
                        other
                            .property
                            .value
                            .clone_in(self.reads.builder.allocator()),
                    )
                },
            );
            entry.present = self.reads.choose(test, &entry.present, &present);
            entry.property.value = self.reads.choose(test, &entry.property.value, &value);
            entry.conditional = true;
            result.push(entry);
        }
        for mut entry in no {
            entry.present = self
                .reads
                .choose(test, &self.reads.boolean(false), &entry.present);
            entry.property.value =
                self.reads
                    .choose(test, &self.reads.undefined(), &entry.property.value);
            entry.conditional = true;
            result.push(entry);
        }
        result
    }

    fn materialize(&self, span: oxc_span::Span, entries: Vec<Entry<'a>>) -> Expression<'a> {
        let properties = entries.into_iter().map(|mut entry| {
            entry.property.value = self.reads.choose(
                &entry.present,
                &entry.property.value,
                &self.reads.undefined(),
            );
            ObjectPropertyKind::ObjectProperty(entry.property)
        });
        Expression::new_object_expression(
            span,
            oxc_allocator::Vec::from_iter_in(properties, self.reads.builder),
            self.reads.builder,
        )
    }

    fn value(&mut self, source: &Expression<'a>, guard: &Expression<'a>) -> Option<Expression<'a>> {
        let source = unwrap_syntax_only(source);
        let builder = self.reads.builder;
        if get_string_by_literal_expression(source).is_some() {
            return Some(source.clone_in(builder.allocator()));
        }
        match source {
            Expression::ObjectExpression(_) => {
                let entries = self.record(source, guard)?;
                Some(self.materialize(source.span(), entries))
            }
            Expression::ConditionalExpression(choice) => {
                let test = self.reads.capture(&choice.test, guard);
                let yes_guard = self.reads.choose(guard, &test, &self.reads.boolean(false));
                let no_guard = self.reads.choose(
                    guard,
                    &self.reads.choose(
                        &test,
                        &self.reads.boolean(false),
                        &self.reads.boolean(true),
                    ),
                    &self.reads.boolean(false),
                );
                let yes = self.value(&choice.consequent, &yes_guard)?;
                let no = self.value(&choice.alternate, &no_guard)?;
                Some(self.reads.choose(&test, &yes, &no))
            }
            Expression::LogicalExpression(logical) => {
                let left = self.reads.capture(&logical.left, guard);
                let takes_right = match logical.operator {
                    LogicalOperator::And => left.clone_in(builder.allocator()),
                    LogicalOperator::Or => self.reads.choose(
                        &left,
                        &self.reads.boolean(false),
                        &self.reads.boolean(true),
                    ),
                    LogicalOperator::Coalesce => Expression::new_binary_expression(
                        source.span(),
                        left.clone_in(builder.allocator()),
                        BinaryOperator::Equality,
                        Expression::new_null_literal(source.span(), builder),
                        builder,
                    ),
                };
                let right_guard =
                    self.reads
                        .choose(guard, &takes_right, &self.reads.boolean(false));
                let right = self.value(&logical.right, &right_guard)?;
                Some(Expression::new_logical_expression(
                    source.span(),
                    left,
                    logical.operator,
                    right,
                    builder,
                ))
            }
            Expression::ArrayExpression(array) => {
                let mut array = array.clone_in(builder.allocator());
                for element in &mut array.elements {
                    if let Some(value) = element.as_expression_mut() {
                        *value = self.value(value, guard)?;
                    } else if matches!(
                        element,
                        oxc_ast::ast::ArrayExpressionElement::SpreadElement(_)
                    ) {
                        return None;
                    }
                }
                Some(Expression::ArrayExpression(array))
            }
            Expression::NullLiteral(_)
            | Expression::BigIntLiteral(_)
            | Expression::RegExpLiteral(_)
            | Expression::FunctionExpression(_)
            | Expression::ArrowFunctionExpression(_)
            | Expression::ClassExpression(_) => Some(source.clone_in(builder.allocator())),
            Expression::Identifier(identifier)
                if matches!(identifier.name.as_str(), "undefined" | "NaN" | "Infinity") =>
            {
                Some(source.clone_in(builder.allocator()))
            }
            _ => Some(self.reads.capture(source, guard)),
        }
    }
}
