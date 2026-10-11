use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::ast::{Expression, Program, VariableDeclaration, VariableDeclarationKind};
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{Visit, walk};
use oxc_semantic::Scoping;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

/// Const initializers keyed by lexical identity, excluding written bindings.
pub(super) fn declarations<'a>(
    builder: &AstBuilder<'a>,
    program: &Program<'a>,
    scoping: &Scoping,
) -> FxHashMap<SymbolId, Expression<'a>> {
    let mut collector = Collector {
        builder,
        scoping,
        found: FxHashMap::default(),
    };
    collector.visit_program(program);
    collector.found
}

struct Collector<'s, 'a> {
    builder: &'s AstBuilder<'a>,
    scoping: &'s Scoping,
    found: FxHashMap<SymbolId, Expression<'a>>,
}

impl<'a> Visit<'a> for Collector<'_, 'a> {
    fn visit_variable_declaration(&mut self, declaration: &VariableDeclaration<'a>) {
        if declaration.kind == VariableDeclarationKind::Const {
            for declarator in &declaration.declarations {
                if let Some(identifier) = declarator.id.get_binding_identifier()
                    && let Some(symbol) = identifier.symbol_id.get()
                    && !self
                        .scoping
                        .get_resolved_reference_ids(symbol)
                        .iter()
                        .any(|reference| self.scoping.get_reference(*reference).is_write())
                    && let Some(init) = &declarator.init
                {
                    self.found.insert(
                        symbol,
                        init.clone_in_with_semantic_ids(self.builder.allocator()),
                    );
                }
            }
        }
        walk::walk_variable_declaration(self, declaration);
    }
}
