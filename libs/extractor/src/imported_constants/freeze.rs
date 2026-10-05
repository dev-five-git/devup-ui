//! Only unconditional, early shallow freezes prove immutable own scalar slots.

use super::provenance::Proof;
use crate::{
    css_prop::binding_of,
    mutations::{Use, callees},
    utils::unwrap_syntax_only,
};
use oxc_ast::{
    AstKind,
    ast::{Argument, Expression, Program, Statement},
};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

pub(super) fn before(
    proof: &Proof<'_, '_>,
    program: &Program<'_>,
    symbol: SymbolId,
    hazard: u32,
    uses: &FxHashMap<String, Vec<Use>>,
) -> bool {
    if !proof.binding(symbol).plain() {
        return false;
    }
    let identity = identity(proof, symbol);
    let frozen = program
        .body
        .iter()
        .filter_map(|statement| {
            let expression = match statement {
                Statement::ExpressionStatement(statement) => &statement.expression,
                Statement::VariableDeclaration(declaration) => {
                    return declaration
                        .declarations
                        .iter()
                        .filter_map(|declaration| {
                            let name = declaration.id.get_binding_identifier()?.symbol_id.get()?;
                            (identity == self::identity(proof, name))
                                .then_some(declaration.init.as_ref()?)
                                .and_then(|init| freeze_call(proof, init))
                        })
                        .min();
                }
                _ => return None,
            };
            let Expression::CallExpression(call) = unwrap_syntax_only(expression) else {
                return None;
            };
            let Argument::Identifier(argument) = call.arguments.first()? else {
                return None;
            };
            let target = binding_of(proof.scoping, argument)?;
            (identity == self::identity(proof, target))
                .then_some(expression)
                .and_then(|expression| freeze_call(proof, expression))
        })
        .min();
    let Some(frozen) = frozen else {
        return false;
    };
    if frozen >= hazard {
        return false;
    }
    for (name, found) in uses {
        let Some(symbol) = proof.scoping.get_root_binding(name.as_str().into()) else {
            continue;
        };
        if identity != self::identity(proof, symbol) {
            continue;
        }
        if found.iter().any(|found| match found {
            Use::Changes { at, .. } | Use::Calls { at, .. } => *at < frozen,
            Use::Escapes { at, into, .. } => into.is_none() && *at < frozen,
        }) {
            return false;
        }
    }
    true
}

fn freeze_call(proof: &Proof<'_, '_>, expression: &Expression<'_>) -> Option<u32> {
    let Expression::CallExpression(call) = unwrap_syntax_only(expression) else {
        return None;
    };
    (callees::global(proof, &call.callee) == Some(("Object", "freeze"))
        && call.arguments.len() == 1)
        .then_some(call.span.start)
}

fn identity(proof: &Proof<'_, '_>, symbol: SymbolId) -> SymbolId {
    let mut symbol = symbol;
    let mut seen = FxHashSet::default();
    while seen.insert(symbol) {
        let AstKind::VariableDeclarator(declaration) =
            proof.nodes.kind(proof.scoping.symbol_declaration(symbol))
        else {
            break;
        };
        let Some(init) = declaration.init.as_ref().map(unwrap_syntax_only) else {
            break;
        };
        let identifier = match init {
            Expression::Identifier(identifier) => identifier,
            Expression::CallExpression(call)
                if callees::global(proof, &call.callee) == Some(("Object", "freeze")) =>
            {
                let Some(Argument::Identifier(identifier)) = call.arguments.first() else {
                    break;
                };
                identifier
            }
            _ => break,
        };
        let Some(alias) = binding_of(proof.scoping, identifier) else {
            break;
        };
        symbol = alias;
    }
    symbol
}
