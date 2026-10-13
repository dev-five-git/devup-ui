//! Execution-only rendering references: no component value is manufactured.
use oxc_allocator::Allocator;
use oxc_ast::{
    AstKind,
    ast::{Expression, Program},
};
use oxc_ast_visit::VisitMut;
use oxc_codegen::{Codegen, CodegenOptions};
use oxc_semantic::SemanticBuilder;
use oxc_span::SPAN;
use rustc_hash::{FxHashMap, FxHashSet};

mod assignments;
mod visit;
use visit::Deferred;

pub(crate) struct Evaluation {
    pub code: String,
    pub marks: Vec<(usize, usize)>,
    pub namespace: String,
    pub keys: FxHashSet<String>,
    pub exports: Vec<String>,
}

pub(super) fn prepare<'a>(
    program: &mut Program<'a>,
    generated: &FxHashSet<String>,
    allocator: &'a Allocator,
) -> Option<Evaluation> {
    let semantic = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(program)
        .semantic;
    let scoping = semantic.scoping();
    let calls: FxHashMap<_, _> = semantic
        .nodes()
        .iter()
        .filter_map(|node| {
            let AstKind::CallExpression(call) = node.kind() else {
                return None;
            };
            let Expression::Identifier(callee) = &call.callee else {
                return None;
            };
            let symbol = scoping
                .get_reference(callee.reference_id.get()?)
                .symbol_id()?;
            (call.span == SPAN
                && scoping.symbol_scope_id(symbol) == scoping.root_scope_id()
                && generated.contains(callee.name.as_str()))
            .then_some((call.node_id.get(), callee.span))
        })
        .collect();
    if calls.is_empty() {
        return None;
    }
    let bindings: FxHashMap<_, _> = semantic
        .nodes()
        .iter()
        .filter_map(|node| {
            let AstKind::VariableDeclarator(declarator) = node.kind() else {
                return None;
            };
            let Expression::CallExpression(call) = declarator.init.as_ref()?.without_parentheses()
            else {
                return None;
            };
            if !calls.contains_key(&call.node_id.get()) {
                return None;
            }
            let oxc_ast::ast::BindingPattern::BindingIdentifier(binding) = &declarator.id else {
                return None;
            };
            Some((binding.symbol_id.get()?, binding.name.to_string()))
        })
        .collect();
    let namespace = crate::fresh_name::fresh_name("__devupDeferredRendering", program.source_text);
    let scoping = semantic.into_scoping();
    let mut visitor = Deferred {
        ast: oxc_ast::builder::AstBuilder::new(allocator),
        scoping: &scoping,
        calls: &calls,
        bindings: &bindings,
        generated,
        namespace: &namespace,
        keys: FxHashSet::default(),
        exports: Vec::new(),
    };
    visitor.visit_program(program);
    let Deferred { keys, exports, .. } = visitor;
    let generated = Codegen::new()
        .with_options(CodegenOptions {
            source_map_path: Some(std::path::PathBuf::from("stylesheet.tsx")),
            ..CodegenOptions::default()
        })
        .build(program);
    let marks = generated
        .map
        .map(|map| crate::source_map::marks(&map, &generated.code, program.source_text))
        .unwrap_or_default();
    Some(Evaluation {
        code: generated.code,
        marks,
        namespace,
        keys,
        exports,
    })
}
