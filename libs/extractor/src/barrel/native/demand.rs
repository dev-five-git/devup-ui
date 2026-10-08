use super::{Facts, Shape, Walker};
use crate::module_loader::demand::Demand;
use oxc_ast::ast::{Program, Statement};
use oxc_span::{GetSpan, Span};
use rustc_hash::{FxHashMap, FxHashSet};
use std::{collections::BTreeSet, rc::Rc};

impl Facts {
    /// Proves requested export terminals with the same walker used for imports.
    pub(crate) fn prove_carrier(
        &mut self,
        parsed: (&Program<'_>, &str),
        origin: (&str, &str),
        request: (&Demand, Option<&crate::ModuleResolver>),
    ) -> bool {
        let (program, source) = parsed;
        let (filename, package) = origin;
        let (demand, resolver) = request;
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
        let names = if demand.whole {
            walker.native_names(&module, &mut FxHashSet::default())
        } else {
            demand.members.keys().cloned().collect()
        };
        let mut proven = false;
        for name in names {
            let Some(child) = demand.child(&name) else {
                continue;
            };
            let terminal = walker.native_origin(&module, &name, &mut Vec::new());
            if let Some(shape) = walker.native_shape(terminal, &mut FxHashSet::default()) {
                let site = program
                    .body
                    .iter()
                    .find_map(|statement| match statement {
                        Statement::ExportFromDeclaration(export) => export
                            .specifiers
                            .iter()
                            .find(|specifier| specifier.exported.name().as_str() == name)
                            .map(GetSpan::span),
                        Statement::ExportAllDeclaration(export) => Some(export.source.span),
                        Statement::ExportNamedDeclaration(export) => export
                            .specifiers
                            .iter()
                            .find(|specifier| specifier.exported.name().as_str() == name)
                            .map(GetSpan::span),
                        Statement::ExportDefaultDeclaration(export) if name == "default" => {
                            Some(export.declaration.span())
                        }
                        _ => None,
                    })
                    .unwrap_or_default();
                proven |= self.requested_terminal(&shape, (child, site));
            }
        }
        self.dependencies.extend(walker.dependencies);
        proven
    }

    fn requested_terminal(&mut self, shape: &Shape, input: (&Demand, Span)) -> bool {
        let (demand, site) = input;
        match shape {
            Shape::Api(_) => demand.whole,
            Shape::Namespace(members) | Shape::PackageNamespace(members) => {
                let mut proven = false;
                for (name, member) in members {
                    if let Some(child) = demand.child(name) {
                        proven |= self.requested_terminal(member, (child, site));
                    }
                }
                proven
            }
            Shape::Failed(message) => {
                self.errors.push((site, message.clone()));
                false
            }
            Shape::OriginalFailure(message) => {
                self.failure.get_or_insert_with(|| message.clone());
                false
            }
        }
    }
}
