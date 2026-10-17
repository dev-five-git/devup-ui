//! The order an element evaluates the props it is written with. Compiling
//! moves, repeats and drops what props evaluate, so what could be told apart
//! by when it is evaluated is evaluated once, in the order written, before the
//! element is built; the build reads the values it captured instead.

use crate::scope::Bindings;
use oxc_ast::ast::{
    Argument, ArrayExpressionElement, ChainElement, Expression, ObjectPropertyKind, PropertyKind,
};
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};
use rustc_hash::FxHashSet;

/// What evaluating an expression can reach
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Reach {
    /// Nothing code can change: literals, functions, `const`s
    Constant,
    /// Variables and properties, which code run before can change
    Reads,
    /// Code that runs: calls, assignments, coercions, getters it copies
    Runs,
}

impl Reach {
    /// Whether evaluating two expressions in the other order can be told
    pub(super) fn conflicts(self, other: Self) -> bool {
        (self == Self::Runs && other != Self::Constant)
            || (other == Self::Runs && self != Self::Constant)
    }
}

/// What an operation reaches that runs the code of what it is given (a
/// coercion), unless that is constant
fn constant_or_runs(reach: Reach) -> Reach {
    if reach == Reach::Constant {
        Reach::Constant
    } else {
        Reach::Runs
    }
}

/// What evaluating `expression` reaches
pub(super) fn reach(bindings: &Bindings, expression: &Expression<'_>) -> Reach {
    let of = |inner: &Expression<'_>| reach(bindings, inner);
    match expression {
        Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::BigIntLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::RegExpLiteral(_)
        | Expression::ArrowFunctionExpression(_)
        | Expression::FunctionExpression(_) => Reach::Constant,
        Expression::Identifier(identifier) => {
            if bindings.is_global_undefined(identifier) {
                Reach::Constant
            } else {
                Reach::Reads
            }
        }
        Expression::ThisExpression(_) => Reach::Reads,
        Expression::TemplateLiteral(template) => constant_or_runs(
            template
                .expressions
                .iter()
                .map(of)
                .max()
                .unwrap_or(Reach::Constant),
        ),
        Expression::UnaryExpression(unary) => match unary.operator {
            UnaryOperator::LogicalNot | UnaryOperator::Typeof | UnaryOperator::Void => {
                of(&unary.argument)
            }
            UnaryOperator::Delete => Reach::Runs,
            _ => constant_or_runs(of(&unary.argument)),
        },
        Expression::BinaryExpression(binary) => {
            let operands = of(&binary.left).max(of(&binary.right));
            if matches!(
                binary.operator,
                BinaryOperator::StrictEquality | BinaryOperator::StrictInequality
            ) {
                operands
            } else {
                constant_or_runs(operands)
            }
        }
        Expression::LogicalExpression(logical) => of(&logical.left).max(of(&logical.right)),
        Expression::ConditionalExpression(conditional) => of(&conditional.test)
            .max(of(&conditional.consequent))
            .max(of(&conditional.alternate)),
        Expression::SequenceExpression(sequence) => sequence
            .expressions
            .iter()
            .map(of)
            .max()
            .unwrap_or(Reach::Constant),
        Expression::ArrayExpression(array) => array
            .elements
            .iter()
            .map(|element| match element {
                ArrayExpressionElement::SpreadElement(_) => Reach::Runs,
                ArrayExpressionElement::Elision(_) => Reach::Constant,
                element => element.as_expression().map_or(Reach::Runs, of),
            })
            .max()
            .unwrap_or(Reach::Constant),
        Expression::ObjectExpression(object) => object
            .properties
            .iter()
            .map(|property| match property {
                ObjectPropertyKind::SpreadProperty(_) => Reach::Runs,
                ObjectPropertyKind::ObjectProperty(property) => {
                    let key = property
                        .key
                        .as_expression()
                        .filter(|_| property.computed)
                        .map_or(Reach::Constant, |key| constant_or_runs(of(key)));
                    let value = if property.kind == PropertyKind::Init {
                        of(&property.value)
                    } else {
                        Reach::Constant
                    };
                    key.max(value)
                }
            })
            .max()
            .unwrap_or(Reach::Constant),
        Expression::ParenthesizedExpression(inner) => of(&inner.expression),
        Expression::TSAsExpression(inner) => of(&inner.expression),
        Expression::TSSatisfiesExpression(inner) => of(&inner.expression),
        Expression::TSNonNullExpression(inner) => of(&inner.expression),
        Expression::TSTypeAssertion(inner) => of(&inner.expression),
        Expression::TSInstantiationExpression(inner) => of(&inner.expression),
        Expression::ChainExpression(chain) => match &chain.expression {
            ChainElement::StaticMemberExpression(_)
            | ChainElement::PrivateFieldExpression(_)
            | ChainElement::ComputedMemberExpression(_)
            | ChainElement::CallExpression(_) => Reach::Runs,
            ChainElement::TSNonNullExpression(inner) => of(&inner.expression),
        },
        Expression::CallExpression(call)
            if bindings.compiles(&call.callee)
                || bindings.stylex_function(&call.callee).is_some() =>
        {
            call.arguments
                .iter()
                .map(|argument| match argument {
                    Argument::SpreadElement(_) => Reach::Runs,
                    argument => of(argument.to_expression()),
                })
                .max()
                .unwrap_or(Reach::Constant)
        }
        _ => Reach::Runs,
    }
}

/// Whether an expression suspends in its current execution scope.
pub(super) fn suspends(expression: &Expression<'_>) -> bool {
    let mut found = crate::utils::Suspends::default();
    oxc_ast_visit::Visit::visit_expression(&mut found, expression);
    found.found
}

/// The style keys a written object gives without executing it.
pub(super) fn written_properties(expression: &Expression<'_>) -> Vec<String> {
    match crate::utils::unwrap_syntax_only(expression) {
        Expression::ObjectExpression(object) => object
            .properties
            .iter()
            .flat_map(|property| match property {
                ObjectPropertyKind::ObjectProperty(property) => property
                    .key
                    .static_name()
                    .filter(|name| !css::is_special_property::is_special_property(name))
                    .map(|name| vec![name.to_string()])
                    .unwrap_or_default(),
                ObjectPropertyKind::SpreadProperty(spread) => written_properties(&spread.argument),
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Where a prop stands once the element is built
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Role {
    /// Becomes classes or style, which come after the props that stay
    Moved,
    /// Stays a prop of the element built
    Stays,
    /// Is evaluated before everything else that stays (`as`)
    Hoisted,
}

/// A prop of an element, for the order it is evaluated in
pub(super) struct Item {
    pub role: Role,
    pub reach: Reach,
    /// Whether the value is written again or expanded to several properties,
    /// so that evaluating it where it stands would drop or repeat it
    pub lost: bool,
    /// Whether it is a spread the element copies once instead of reading
    /// again, which runs the getters it copies
    pub snapshot: bool,
    /// Whether it holds an `await` or `yield`
    pub suspends: bool,
}

impl Item {
    const fn effective(&self) -> Reach {
        if self.snapshot {
            Reach::Runs
        } else {
            self.reach
        }
    }

    /// Whether it is evaluated before a prop written before it that evaluating
    /// it after would be told from
    pub(super) fn hoisted_over(&self, before: &[Item]) -> bool {
        self.role == Role::Hoisted
            && before
                .iter()
                .any(|item| self.effective().conflicts(item.effective()))
    }

    fn must_capture(&self, before: &[Item], after: &[Item]) -> bool {
        if self.effective() == Reach::Constant {
            return false;
        }
        let evaluated_later =
            |item: &Item| item.role == Role::Stays && self.effective().conflicts(item.effective());
        self.snapshot
            || self.lost
            || (self.role == Role::Moved && self.reach == Reach::Runs)
            || (self.role == Role::Moved && after.iter().any(evaluated_later))
            || self.hoisted_over(before)
    }
}

/// Props that become the classes and style of the element, whatever they hold
pub(super) fn is_merged(name: &str) -> bool {
    matches!(name, "className" | "style")
}

/// Whether the prop `name` is a style the build reads, when `kept` are the
/// ones a styled component takes as they are
pub(super) fn is_style(name: &str, kept: &FxHashSet<String>) -> bool {
    !css::is_special_property::is_special_property(name) && !kept.contains(name)
}

/// Where the prop `name` stands once the element is built, and whether it
/// writes a style that a spread after it can replace
pub(super) fn classify(name: &str, kept: &FxHashSet<String>) -> (Role, bool) {
    if !is_style(name, kept) {
        let role = if is_merged(name) {
            Role::Moved
        } else {
            Role::Stays
        };
        return (role, false);
    }
    if name == "as" {
        return (Role::Hoisted, false);
    }
    (
        Role::Moved,
        !matches!(name, "styleOrder" | "props" | "styleVars"),
    )
}

/// The last prop to capture so that the props evaluate in the order written,
/// when any must: everything before it that is not constant is captured too.
/// `children_suspend` tells children that hold an `await` or `yield`, which
/// only the arguments of the call building the element can hold
pub(super) fn last_captured(items: &[Item], children_suspend: bool) -> Option<usize> {
    let last = (0..items.len())
        .rev()
        .find(|index| items[*index].must_capture(&items[..*index], &items[*index + 1..]))?;
    let suspending = items.iter().rposition(|item| item.suspends).unwrap_or(0);
    Some(if children_suspend {
        items.len() - 1
    } else {
        last.max(suspending)
    })
}
