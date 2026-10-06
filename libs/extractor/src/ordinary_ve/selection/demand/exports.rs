use oxc_ast::ast::{ExportDefaultDeclarationKind, Program, Statement};
use oxc_span::GetSpan;
use oxc_syntax::symbol::SymbolId;

use super::{Index, View};
use crate::module_loader::demand::Demand;

pub(super) fn seed(program: &Program<'_>, index: &Index<'_, '_>, demand: &Demand, view: &mut View) {
    let scoping = index.semantic.scoping();
    let mut local = |exported: String, name: String| {
        if let Some(child) = demand.child(&exported) {
            view.exports.push((exported, name.clone()));
            if let Some(symbol) = scoping.get_root_binding(name.as_str().into()) {
                view.symbols.entry(symbol).or_default().merge(child);
            }
        }
    };
    for statement in &program.body {
        match statement {
            Statement::ExportDeclaration(export) => {
                for name in crate::module_loader::declared_names(&export.declaration) {
                    local(name.clone(), name);
                }
            }
            Statement::ExportNamedDeclaration(export) => {
                for specifier in &export.specifiers {
                    local(
                        specifier.exported.name().to_string(),
                        specifier.local.name().to_string(),
                    );
                }
            }
            _ => {}
        }
    }
    for statement in &program.body {
        match statement {
            Statement::ExportFromDeclaration(export) => {
                let mut forwarded = Demand::default();
                let mut selected = false;
                for specifier in &export.specifiers {
                    if let Some(child) = demand.child(specifier.exported.name().as_str()) {
                        forwarded.merge(&Demand::prefixed(specifier.local.name().as_str(), child));
                        selected = true;
                    }
                }
                if selected {
                    view.forwarded.push((
                        export.source.value.to_string(),
                        forwarded,
                        export.source.span,
                    ));
                    view.reexports.push(export.span);
                }
            }
            Statement::ExportAllDeclaration(export) => {
                let forwarded = match &export.exported {
                    Some(name) => demand.child(name.name().as_str()).cloned(),
                    None => Some(demand.clone()),
                };
                if let Some(forwarded) = forwarded {
                    view.forwarded.push((
                        export.source.value.to_string(),
                        forwarded,
                        export.source.span,
                    ));
                    view.reexports.push(export.span);
                }
            }
            Statement::ExportDefaultDeclaration(export) => {
                let Some(child) = demand.child("default") else {
                    continue;
                };
                let name = match &export.declaration {
                    ExportDefaultDeclarationKind::FunctionDeclaration(function) => {
                        function.id.as_ref().map(|id| id.name.to_string())
                    }
                    ExportDefaultDeclarationKind::ClassDeclaration(class) => {
                        class.id.as_ref().map(|id| id.name.to_string())
                    }
                    _ => None,
                };
                if let Some(name) = name {
                    view.exports.push(("default".to_string(), name.clone()));
                    if let Some(symbol) = scoping.get_root_binding(name.as_str().into()) {
                        view.symbols.entry(symbol).or_default().merge(child);
                    }
                } else {
                    let span = export.declaration.span();
                    let name = crate::fresh_name::fresh_name(
                        "__ve_default__",
                        index.semantic.source_text(),
                    );
                    view.exports.push(("default".to_string(), name.clone()));
                    view.default = Some((span, name));
                    if let Some(owner) = index.owner(export.declaration.node_id()) {
                        view.units.entry(owner).or_default().merge(child);
                    }
                }
            }
            _ => {}
        }
    }
}

pub(super) fn binding_demand(index: &Index<'_, '_>, symbol: SymbolId, demand: &Demand) -> Demand {
    let declaration = index.semantic.scoping().symbol_declaration(symbol);
    if let oxc_ast::AstKind::VariableDeclarator(declarator) =
        index.semantic.nodes().kind(declaration)
        && let Some(path) = super::super::binding_path::find(&declarator.id, symbol)
    {
        return path
            .keys
            .iter()
            .rev()
            .fold(demand.clone(), |child, key| Demand::prefixed(key, &child));
    }
    demand.clone()
}
