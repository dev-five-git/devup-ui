use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use rustc_hash::FxHashMap;
use std::collections::BTreeSet;

use super::{Facts, Shape, Terminal, Walker};
use crate::{ModuleResolver, ResolvedModule};

mod aliases;
mod carrier_provenance;
mod facts;
mod terminals;

const PACKAGE: &str = "@vanilla-extract/css";
type Modules<'a> = &'a [(&'a str, &'a str, &'a str)];
type TestResult = Result<(), &'static str>;

fn resolver(modules: Modules<'_>) -> impl Fn(&str, &str) -> Option<ResolvedModule> + use<> {
    let modules: Vec<_> = modules
        .iter()
        .map(|(specifier, path, code)| (specifier.to_string(), path.to_string(), code.to_string()))
        .collect();
    move |source, _| {
        modules.iter().find_map(|(specifier, path, code)| {
            (specifier == source).then(|| ResolvedModule {
                path: path.clone(),
                code: code.clone(),
            })
        })
    }
}

fn walker(resolver: &ModuleResolver) -> Walker<'_, 'static> {
    Walker {
        resolver,
        package: PACKAGE,
        native: true,
        modules: FxHashMap::default(),
        dependencies: BTreeSet::new(),
    }
}

fn facts(source: &str, modules: Modules<'_>) -> Facts {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{source}");
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program);
    assert_eq!(built.diagnostics.len(), 0, "{source}");
    Facts::resolve(
        &parsed.program,
        ("/entry.ts", PACKAGE),
        Some(&resolver(modules)),
    )
}
