use super::{Shape, Walker};
use oxc_ast::ast::{Program, Statement};
use oxc_span::{GetSpan, Span};
use rustc_hash::{FxHashMap, FxHashSet};
use std::{collections::BTreeSet, rc::Rc};

pub(crate) fn export_site(program: &Program<'_>, name: &str) -> Option<Span> {
    program.body.iter().find_map(|statement| match statement {
        Statement::ExportFromDeclaration(export) if !export.export_kind.is_type() => export
            .specifiers
            .iter()
            .find(|specifier| {
                !specifier.export_kind.is_type() && specifier.exported.name().as_str() == name
            })
            .map(GetSpan::span),
        Statement::ExportAllDeclaration(export) if !export.export_kind.is_type() => {
            Some(export.source.span)
        }
        Statement::ExportNamedDeclaration(export) if !export.export_kind.is_type() => export
            .specifiers
            .iter()
            .find(|specifier| {
                !specifier.export_kind.is_type() && specifier.exported.name().as_str() == name
            })
            .map(GetSpan::span),
        Statement::ExportDefaultDeclaration(export) if name == "default" => {
            Some(export.declaration.span())
        }
        _ => None,
    })
}

pub(crate) fn exported_terminals(
    parsed: (&Program<'_>, &str),
    origin: (&str, &str),
    resolver: Option<&crate::ModuleResolver>,
) -> Vec<(Span, Rc<Shape>)> {
    let (program, source) = parsed;
    let (filename, package) = origin;
    let no_modules = |_: &str, _: &str| None;
    let mut walker = Walker {
        resolver: resolver.unwrap_or(&no_modules),
        package,
        native: true,
        modules: FxHashMap::default(),
        dependencies: BTreeSet::new(),
    };
    let module = Rc::new(crate::barrel::analyze_mode(
        &crate::ResolvedModule {
            path: filename.to_string(),
            code: source.to_string(),
        },
        package,
        true,
    ));
    walker.modules.insert(filename.to_string(), module.clone());
    let names = walker.native_names(&module, &mut FxHashSet::default());
    let mut terminals = Vec::new();
    for name in names {
        let terminal = walker.native_origin(&module, &name, &mut Vec::new());
        if let Some(shape) = walker.native_shape(terminal, &mut FxHashSet::default())
            && let Some(site) = export_site(program, &name)
        {
            terminals.push((site, shape));
        }
    }
    terminals.sort_by_key(|(site, _)| site.start);
    terminals
}
