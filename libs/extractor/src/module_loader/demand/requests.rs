use oxc_ast::{
    AstKind,
    ast::{ImportDeclarationSpecifier, Program, Statement},
};
use oxc_semantic::Semantic;
use oxc_span::{GetSpan, Span};

use super::Demand;
use crate::ordinary_ve::selection::plan::{ImportName, MemberDemand};

pub(super) fn entry<'a>(
    parsed: (&Program<'a>, &Semantic<'a>),
    selected: bool,
    selection: &crate::ordinary_ve::selection::Selection,
) -> Vec<(String, Demand, Span)> {
    let (program, semantic) = parsed;
    if selected {
        return selection
            .imports
            .iter()
            .filter(|import| import.native.is_none() || import.source != "@vanilla-extract/css")
            .map(|import| {
                let mut child = Demand::default();
                for demand in selection
                    .demands
                    .iter()
                    .filter(|demand| demand.symbol == import.binding.symbol)
                {
                    match &demand.member {
                        MemberDemand::Path(path) => {
                            child.insert(path);
                        }
                        MemberDemand::Whole => {
                            child.insert(&[]);
                        }
                    }
                }
                let demand = match &import.imported {
                    ImportName::Named(name) => Demand::prefixed(name, &child),
                    ImportName::Default => Demand::prefixed("default", &child),
                    ImportName::Namespace => child,
                };
                (import.source.clone(), demand, import.specifier)
            })
            .collect();
    }
    let mut requests = Vec::new();
    for statement in &program.body {
        let Statement::ImportDeclaration(import) = statement else {
            continue;
        };
        let mut request = Demand::default();
        for specifier in import.specifiers.iter().flatten() {
            let Some(symbol) = specifier.local().symbol_id.get() else {
                continue;
            };
            for reference in semantic.scoping().get_resolved_reference_ids(symbol) {
                let reference = semantic.scoping().get_reference(*reference);
                if !reference.is_value() {
                    continue;
                }
                let path = read_path(semantic, reference.node_id());
                let mut child = Demand::default();
                child.insert(&path);
                match specifier {
                    ImportDeclarationSpecifier::ImportSpecifier(specifier) => {
                        request.merge(&Demand::prefixed(
                            specifier.imported.name().as_str(),
                            &child,
                        ));
                    }
                    ImportDeclarationSpecifier::ImportDefaultSpecifier(_) => {
                        request.merge(&Demand::prefixed("default", &child));
                    }
                    ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => {
                        request.merge(&child);
                    }
                }
            }
        }
        if request.whole || !request.members.is_empty() {
            requests.push((import.source.value.to_string(), request, import.source.span));
        }
    }
    requests
}

fn read_path(semantic: &Semantic<'_>, node: oxc_syntax::node::NodeId) -> Vec<String> {
    let mut span = semantic.nodes().kind(node).span();
    let mut path = Vec::new();
    for ancestor in semantic.nodes().ancestor_ids(node) {
        let kind = semantic.nodes().kind(ancestor);
        match kind {
            AstKind::StaticMemberExpression(member) if member.object.span() == span => {
                path.push(member.property.name.to_string());
            }
            AstKind::ComputedMemberExpression(member) if member.object.span() == span => {
                let Some(key) = crate::utils::get_string_by_literal_expression(
                    crate::utils::unwrap_syntax_only(&member.expression),
                ) else {
                    return Vec::new();
                };
                path.push(key.into_owned());
            }
            AstKind::ParenthesizedExpression(_)
            | AstKind::TSAsExpression(_)
            | AstKind::TSSatisfiesExpression(_)
            | AstKind::TSNonNullExpression(_)
            | AstKind::TSInstantiationExpression(_) => {}
            _ => break,
        }
        span = kind.span();
    }
    path
}
