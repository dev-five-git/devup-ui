use oxc_allocator::Allocator;
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::Visit;
use rustc_hash::FxHashSet;

use super::{Reading, Used};
use crate::css_prop::{CssProp, CssTakers};
use crate::{ExtractOption, ModuleResolver};

pub(crate) fn candidate(code: &str) -> bool {
    code.contains("withComponent") && code.contains("import")
}

pub(crate) fn read<'a>(
    allocator: &'a Allocator,
    filename: &str,
    code: &'a str,
    option: &ExtractOption,
    resolver: Option<&ModuleResolver>,
) -> Reading<'a> {
    if resolver.is_none() || !candidate(code) {
        return Reading::default();
    }
    let Ok(program) = crate::parsing::parse(allocator, filename, code) else {
        return Reading::default();
    };
    let scoping = oxc_semantic::SemanticBuilder::new()
        .build(&program)
        .semantic
        .into_scoping();
    let mut used = Used {
        styled: FxHashSet::default(),
        css_prop: false,
        css: CssTakers::new(&program, &scoping, CssProp::Off, ""),
        names: FxHashSet::default(),
        scoping: &scoping,
    };
    used.visit_program(&program);
    if used.names.is_empty() {
        return Reading::default();
    }
    let mut reading = super::read(
        &AstBuilder::new(allocator),
        &program,
        filename,
        option,
        resolver,
        CssProp::Off,
    );
    reading
        .components
        .retain(|name, component| used.names.contains(name) && component.definition.is_some());
    reading
}
