use oxc_ast::{
    AstKind,
    ast::{Declaration, Program, Statement, VariableDeclaration},
};
use oxc_semantic::Semantic;
use oxc_span::GetSpan;

use super::selection::Selection;

pub(crate) fn mixed(program: &Program<'_>, semantic: &Semantic<'_>, selection: &Selection) -> bool {
    if semantic
        .scoping()
        .root_unresolved_references()
        .keys()
        .any(|name| {
            !matches!(
                name.as_str(),
                "undefined"
                    | "NaN"
                    | "Infinity"
                    | "Math"
                    | "String"
                    | "Number"
                    | "Boolean"
                    | "Array"
                    | "Object"
                    | "JSON"
                    | "parseInt"
                    | "parseFloat"
                    | "isNaN"
                    | "isFinite"
                    | "encodeURIComponent"
                    | "decodeURIComponent"
                    | "encodeURI"
                    | "decodeURI"
                    | "Error"
                    | "TypeError"
                    | "ReferenceError"
                    | "RangeError"
                    | "console"
            )
        })
    {
        return true;
    }
    if semantic.nodes().iter().any(|node| {
        matches!(
            node.kind(),
            AstKind::JSXElement(_) | AstKind::JSXFragment(_)
        )
    }) {
        return true;
    }
    program.body.iter().any(|statement| match statement {
        Statement::ThrowStatement(_) => true,
        Statement::ImportDeclaration(import) => {
            import.specifiers.iter().flatten().any(|specifier| {
                !selection
                    .imports
                    .iter()
                    .any(|selected| selected.specifier == specifier.span())
            }) && import.source.value != "@vanilla-extract/css"
                && !import.source.value.ends_with(".css")
        }
        Statement::VariableDeclaration(declaration) => effects(declaration, semantic, selection),
        Statement::ExportDeclaration(export) => match &export.declaration {
            Declaration::VariableDeclaration(declaration) => {
                effects(declaration, semantic, selection)
            }
            Declaration::TSTypeAliasDeclaration(_) | Declaration::TSInterfaceDeclaration(_) => {
                false
            }
            _ => true,
        },
        Statement::FunctionDeclaration(_)
        | Statement::ExportNamedDeclaration(_)
        | Statement::ExportFromDeclaration(_)
        | Statement::ExportAllDeclaration(_)
        | Statement::ExportDefaultDeclaration(_)
        | Statement::TSTypeAliasDeclaration(_)
        | Statement::TSInterfaceDeclaration(_)
        | Statement::EmptyStatement(_) => false,
        statement => {
            let span = statement.span();
            !selection.roots.iter().any(|root| root.span == span)
                && !selection.mutations.iter().any(|mutation| {
                    let at = match &mutation.usage {
                        crate::mutations::Use::Changes { at, .. }
                        | crate::mutations::Use::Calls { at, .. }
                        | crate::mutations::Use::Escapes { at, .. } => *at,
                    };
                    (span.start..span.end).contains(&at)
                })
        }
    })
}

fn effects(
    declaration: &VariableDeclaration<'_>,
    semantic: &Semantic<'_>,
    selection: &Selection,
) -> bool {
    declaration.declarations.iter().any(|declarator| {
        !selection
            .units
            .iter()
            .any(|unit| unit.span == declarator.span)
            && semantic.nodes().iter().any(|node| {
                declarator.span.contains_inclusive(node.kind().span())
                    && matches!(
                        node.kind(),
                        AstKind::CallExpression(_)
                            | AstKind::NewExpression(_)
                            | AstKind::ArrowFunctionExpression(_)
                            | AstKind::Function(_)
                            | AstKind::AssignmentExpression(_)
                            | AstKind::UpdateExpression(_)
                            | AstKind::AwaitExpression(_)
                            | AstKind::YieldExpression(_)
                            | AstKind::TaggedTemplateExpression(_)
                    )
            })
    })
}
