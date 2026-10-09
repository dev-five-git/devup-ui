//! Styling-only read slots; symbols are rebound to each original parsed tree.

use oxc_ast::{
    AstKind,
    ast::{IdentifierReference, Program},
};
use oxc_ast_visit::Visit;
use oxc_semantic::Semantic;
use oxc_span::{GetSpan, Span};

use super::{StyleReads, StyleSymbols};

mod proof;
#[cfg(test)]
mod tests;
use proof::Closed;
pub(super) use proof::{call, closed, member, template};

#[derive(Default)]
pub(crate) struct ReadPlan {
    pub slots: Vec<Span>,
    pub reads: Vec<Span>,
    pub guards: Vec<(Span, Vec<Guard>)>,
    pub failures: Vec<(Span, String)>,
}

pub(crate) struct Guard {
    pub test: Span,
    pub kind: GuardKind,
}

#[derive(Clone, Copy)]
pub(crate) enum GuardKind {
    Truthy,
    Falsy,
    Nullish,
}

pub(crate) fn style_names(
    program: &Program<'_>,
    scoping: &oxc_semantic::Scoping,
    option: &crate::ExtractOption,
) -> rustc_hash::FxHashSet<String> {
    let style = StyleSymbols::from_program(program, scoping, option);
    style
        .roots
        .iter()
        .map(|symbol| scoping.symbol_name(*symbol).to_string())
        .collect()
}

pub(crate) fn plan(
    program: &Program<'_>,
    semantic: &Semantic<'_>,
    option: &crate::ExtractOption,
) -> ReadPlan {
    let scoping = semantic.scoping();
    let compat = format!("{}/compat", option.package);
    let css_prop = crate::css_prop::CssProp::of(&option.import_aliases, program.source_text, false);
    let css_props = crate::css_prop::CssTakers::new(program, scoping, css_prop, &compat);
    let style = StyleSymbols::from_program(program, scoping, option);
    let known = |identifier: &IdentifierReference<'_>| proof::input(identifier, semantic, &style);
    let mut collector = StyleReads {
        style: &style,
        css_props: &css_props,
        names: Default::default(),
        symbols: Default::default(),
        references: Default::default(),
        depth: 0,
        class_names: Vec::new(),
        slots: Vec::new(),
        callee: false,
        known: Some(&known),
    };
    collector.visit_program(program);
    collector
        .slots
        .sort_by_key(|span| (span.start, std::cmp::Reverse(span.end)));
    let mut slots: Vec<Span> = Vec::new();
    for span in collector.slots {
        if !slots
            .last()
            .is_some_and(|outer| outer.contains_inclusive(span))
        {
            slots.push(span);
        }
    }
    let mut reads = ReadReferences {
        slots: &slots,
        reads: Vec::new(),
    };
    reads.visit_program(program);
    let mut reads = reads.reads;
    let mut guards = Vec::new();
    let mut failures = Vec::new();
    for slot in &slots {
        let Some(node) = semantic
            .nodes()
            .iter()
            .find(|node| node.kind().span() == *slot)
        else {
            continue;
        };
        let mut conditions = Vec::new();
        for ancestor in semantic.nodes().ancestor_kinds(node.id()) {
            if matches!(ancestor, AstKind::ForStatement(_) | AstKind::ForInStatement(_) | AstKind::ForOfStatement(_) | AstKind::WhileStatement(_) | AstKind::DoWhileStatement(_))
                && semantic.nodes().iter().any(|node| matches!(node.kind(), AstKind::CallExpression(call) if slot.contains_inclusive(call.span)))
            {
                failures.push((ancestor.span(), "required consumer computation has an application iteration schedule. Fix: observe the helper in an exact initializer outside the runtime loop".to_string()));
            }
            let condition = match ancestor {
                AstKind::IfStatement(statement)
                    if statement.consequent.span().contains_inclusive(*slot) =>
                {
                    Some((&statement.test, GuardKind::Truthy))
                }
                AstKind::IfStatement(statement)
                    if statement
                        .alternate
                        .as_ref()
                        .is_some_and(|alternate| alternate.span().contains_inclusive(*slot)) =>
                {
                    Some((&statement.test, GuardKind::Falsy))
                }
                AstKind::ConditionalExpression(expression)
                    if expression.consequent.span().contains_inclusive(*slot) =>
                {
                    Some((&expression.test, GuardKind::Truthy))
                }
                AstKind::ConditionalExpression(expression)
                    if expression.alternate.span().contains_inclusive(*slot) =>
                {
                    Some((&expression.test, GuardKind::Falsy))
                }
                AstKind::LogicalExpression(expression)
                    if expression.right.span().contains_inclusive(*slot) =>
                {
                    Some((
                        &expression.left,
                        match expression.operator {
                            oxc_syntax::operator::LogicalOperator::And => GuardKind::Truthy,
                            oxc_syntax::operator::LogicalOperator::Or => GuardKind::Falsy,
                            oxc_syntax::operator::LogicalOperator::Coalesce => GuardKind::Nullish,
                        },
                    ))
                }
                _ => None,
            };
            if let Some((test, kind)) = condition {
                let mut proof = Closed {
                    style: &style,
                    known: &known,
                    exact: true,
                    input: false,
                };
                proof.visit_expression(test);
                proof.exact &= !semantic.nodes().iter().any(|node| matches!(node.kind(), AstKind::CallExpression(call) if test.span().contains_inclusive(call.span)));
                if proof.exact { conditions.push(Guard { test: test.span(), kind }); }
                else if semantic.nodes().iter().any(|node| matches!(node.kind(), AstKind::CallExpression(call) if slot.contains_inclusive(call.span))) {
                    failures.push((test.span(), "required consumer computation has a runtime execution guard. Fix: keep helper observations behind exact module-level guards".to_string()));
                }
            }
        }
        conditions.reverse();
        guards.push((*slot, conditions));
    }
    for node in semantic.nodes().iter() {
        if let AstKind::IdentifierReference(identifier) = node.kind()
            && guards.iter().any(|(_, guards)| {
                guards
                    .iter()
                    .any(|guard| guard.test.contains_inclusive(identifier.span))
            })
        {
            reads.push(identifier.span);
        }
    }
    ReadPlan {
        slots,
        reads,
        guards,
        failures,
    }
}

struct ReadReferences<'s> {
    slots: &'s [Span],
    reads: Vec<Span>,
}

impl<'a> Visit<'a> for ReadReferences<'_> {
    fn visit_identifier_reference(&mut self, identifier: &IdentifierReference<'a>) {
        if self
            .slots
            .iter()
            .any(|slot| slot.contains_inclusive(identifier.span))
        {
            self.reads.push(identifier.span);
        }
    }
}
