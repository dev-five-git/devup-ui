use super::super::{Link, reference_symbol};
use oxc_ast::ast::{BindingPattern, Declaration, Expression, Statement, VariableDeclarationKind};
use oxc_semantic::Semantic;
use oxc_span::GetSpan;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

pub(in crate::barrel) fn expression(
    value: &Expression<'_>,
    links: &FxHashMap<SymbolId, Link>,
    semantic: &Semantic<'_>,
) -> Option<Link> {
    match crate::utils::unwrap_syntax_only(value) {
        Expression::Identifier(id) => links.get(&reference_symbol(id, semantic)?).cloned(),
        Expression::StaticMemberExpression(member) => Some(Link::Member {
            base: Box::new(expression(&member.object, links, semantic)?),
            name: member.property.name.to_string(),
        }),
        Expression::ComputedMemberExpression(member) => Some(Link::Member {
            base: Box::new(expression(&member.object, links, semantic)?),
            name: crate::ordinary_ve::selection::static_key::resolve(&member.expression, semantic)?,
        }),
        _ => None,
    }
}

fn bind(
    pattern: &BindingPattern<'_>,
    link: Link,
    context: (&mut FxHashMap<SymbolId, Link>, &Semantic<'_>),
) {
    let (links, semantic) = context;
    match pattern {
        BindingPattern::BindingIdentifier(id) => {
            if let Some(symbol) = id.symbol_id.get() {
                links.insert(symbol, link);
            }
        }
        BindingPattern::ObjectPattern(object) => {
            for property in &object.properties {
                let name = if property.computed {
                    property.key.as_expression().and_then(|key| {
                        crate::ordinary_ve::selection::static_key::resolve(key, semantic)
                    })
                } else {
                    property.key.static_name().map(std::borrow::Cow::into_owned)
                };
                if let Some(name) = name {
                    bind(
                        &property.value,
                        Link::Member {
                            base: Box::new(link.clone()),
                            name,
                        },
                        (links, semantic),
                    );
                }
            }
        }
        BindingPattern::AssignmentPattern(pattern) => bind(&pattern.left, link, (links, semantic)),
        BindingPattern::ArrayPattern(_) => {}
    }
}

pub(in crate::barrel) fn extend(
    statement: &Statement<'_>,
    links: &mut FxHashMap<SymbolId, Link>,
    semantic: &Semantic<'_>,
) {
    let declaration = match statement {
        Statement::VariableDeclaration(declaration) => declaration.as_ref(),
        Statement::ExportDeclaration(export) => match &export.declaration {
            Declaration::VariableDeclaration(declaration) => declaration.as_ref(),
            _ => return,
        },
        _ => return,
    };
    if declaration.kind != VariableDeclarationKind::Const || declaration.declare {
        return;
    }
    for declarator in &declaration.declarations {
        if let Some(link) = declarator
            .init
            .as_ref()
            .and_then(|init| expression(init, links, semantic))
        {
            bind(&declarator.id, link, (links, semantic));
        }
    }
}

pub(in crate::barrel) fn changes(
    program: &oxc_ast::ast::Program<'_>,
    links: &mut FxHashMap<SymbolId, Link>,
    origin: (&Semantic<'_>, &crate::ResolvedModule),
) {
    let (semantic, module) = origin;
    let scoping = semantic.scoping();
    let names: rustc_hash::FxHashSet<_> = links
        .keys()
        .map(|symbol| scoping.symbol_name(*symbol))
        .collect();
    for (name, uses) in crate::mutations::uses(program, &|name| names.contains(name), None) {
        let Some(symbol) = scoping.get_root_binding(name.as_str().into()) else {
            continue;
        };
        let Some(at) = uses
            .into_iter()
            .filter_map(|usage| match usage {
                crate::mutations::Use::Changes { at, .. } => Some(at),
                crate::mutations::Use::Escapes { at, into: None, .. } => {
                    let transfer = scoping.get_resolved_reference_ids(symbol).iter().any(|reference| {
                        let data = scoping.get_reference(*reference);
                        let span = semantic.nodes().kind(data.node_id()).span();
                        span.start == at && semantic.nodes().ancestor_kinds(data.node_id()).any(|kind| {
                            matches!(kind, oxc_ast::AstKind::VariableDeclarator(declarator)
                                if declarator.init.as_ref().is_some_and(|init|
                                    init.span().contains_inclusive(span)
                                        && expression(init, links, semantic).is_some())
                                    && declarator.id.get_binding_identifiers().iter().all(|id|
                                        id.symbol_id.get().is_some_and(|symbol| links.contains_key(&symbol))))
                        })
                    });
                    (!transfer).then_some(at)
                }
                crate::mutations::Use::Escapes { into: Some(_), .. }
                | crate::mutations::Use::Calls { .. } => None,
            })
            .min()
        else {
            continue;
        };
        if let Some(link) = links.get_mut(&symbol) {
            let base = link.clone();
            *link = Link::Changed {
                base: Box::new(base),
                place: crate::locate(&module.path, &module.code, usize::try_from(at).unwrap_or(0)),
            };
        }
    }
}
