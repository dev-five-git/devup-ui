//! `W = A.withComponent(tag)` on a component made with Emotion's `styled`:
//! Emotion creates `W` with `A`'s options, so a component selector reads the
//! same target for both, and `W` takes `A`'s marker instead of its own

use oxc_ast::ast::{Declaration, Expression, Program, Statement, VariableDeclaration};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

use crate::style_values::StyleValues;
use crate::utils::unwrap_syntax_only;

/// The `withComponent` results of Emotion components the module declares at
/// its top level, by name, each with the component it shares the target of:
/// the first in the chain of `withComponent` calls that the module declares.
/// `emotion` tells whether a declaration's value is a component Emotion's
/// `styled` makes
pub fn read(
    program: &Program<'_>,
    values: &StyleValues,
    emotion: &dyn Fn(&Expression<'_>) -> bool,
) -> Vec<(SymbolId, String, SymbolId)> {
    // Each Emotion component, with the one whose target it shares
    let mut roots = FxHashMap::default();
    let mut retargets = Vec::new();
    for declarator in program
        .body
        .iter()
        .filter_map(declaration)
        .flat_map(|declaration| &declaration.declarations)
    {
        let (Some(symbol), Some(init), Some(id)) = (
            values.constant(&declarator.id),
            declarator.init.as_ref().map(unwrap_syntax_only),
            declarator.id.get_binding_identifier(),
        ) else {
            continue;
        };
        if let Some(root) = receiver(init)
            .and_then(|receiver| values.symbol(receiver))
            .and_then(|receiver| roots.get(&receiver).copied())
        {
            roots.insert(symbol, root);
            retargets.push((symbol, id.name.to_string(), root));
        } else if emotion(init) {
            roots.insert(symbol, symbol);
        }
    }
    retargets
}

/// The declaration a top-level statement makes, exported or not
fn declaration<'s, 'a>(statement: &'s Statement<'a>) -> Option<&'s VariableDeclaration<'a>> {
    match statement {
        Statement::VariableDeclaration(declaration) => Some(declaration),
        Statement::ExportDeclaration(export) => match &export.declaration {
            Declaration::VariableDeclaration(declaration) => Some(declaration),
            _ => None,
        },
        _ => None,
    }
}

/// The component `init` calls `withComponent` on
pub fn receiver<'e, 'a>(init: &'e Expression<'a>) -> Option<&'e Expression<'a>> {
    let Expression::CallExpression(call) = init else {
        return None;
    };
    let Expression::StaticMemberExpression(member) = &call.callee else {
        return None;
    };
    (member.property.name == "withComponent").then(|| unwrap_syntax_only(&member.object))
}
