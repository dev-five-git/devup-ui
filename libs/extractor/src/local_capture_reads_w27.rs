use oxc_allocator::GetAllocator;
use oxc_ast::{
    ast::{
        ArrowFunctionExpression, BindingIdentifier, CallExpression, Expression, Function,
        IdentifierReference, ObjectProperty, Program,
    },
    builder::AstBuilder,
};
use oxc_ast_visit::{Visit, VisitMut, walk, walk_mut};
use oxc_semantic::Scoping;
use oxc_span::Span;
use oxc_syntax::{reference::ReferenceId, symbol::SymbolId};
use rustc_hash::{FxHashMap, FxHashSet};

#[path = "capture_public_exports_w27.rs"]
mod public_exports;

/// Names generated styled scopes bind after semantic references become strings.
pub(super) fn generated_name(name: &str) -> bool {
    matches!(
        name,
        "rest" | "style" | "className" | "DevupAs" | "forwardedAs"
    ) || name.starts_with("__devup")
}

pub(super) fn aliases(
    program: &Program<'_>,
    scoping: &Scoping,
    shapes: &FxHashMap<SymbolId, Expression<'_>>,
    captures: &FxHashSet<SymbolId>,
) -> FxHashMap<SymbolId, String> {
    #[derive(Default)]
    struct Names(FxHashSet<String>);
    impl<'a> Visit<'a> for Names {
        fn visit_binding_identifier(&mut self, identifier: &BindingIdentifier<'a>) {
            self.0.insert(identifier.name.to_string());
        }
        fn visit_identifier_reference(&mut self, identifier: &IdentifierReference<'a>) {
            self.0.insert(identifier.name.to_string());
        }
    }
    let mut names = Names::default();
    names.visit_program(program);
    let mut symbols: Vec<_> = shapes
        .keys()
        .copied()
        .chain(captures.iter().copied())
        .collect();
    symbols.sort_unstable_by_key(|symbol| scoping.symbol_span(*symbol).start);
    symbols.dedup();
    let mut aliases = FxHashMap::default();
    for symbol in symbols {
        let name = scoping.symbol_name(symbol);
        if !generated_name(name) && !captures.contains(&symbol) {
            continue;
        }
        let base = format!("__devup_{name}");
        let mut alias = base.clone();
        let mut suffix = 0usize;
        while names.0.contains(&alias) || generated_reserved(&alias) {
            suffix += 1;
            alias = format!("{base}{suffix}");
        }
        names.0.insert(alias.clone());
        aliases.insert(symbol, alias);
    }
    aliases
}

fn generated_reserved(name: &str) -> bool {
    matches!(
        name,
        "__devupProps"
            | "__devupContext"
            | "__devupAttrs"
            | "__devupValue"
            | "__devupMixin"
            | "__devupForwardRef"
            | "__devupRefProps"
            | "__devupRef"
            | "__devupDom"
    ) || name.starts_with("__devupOmit")
}

pub(super) struct Rename<'s, 'a> {
    pub builder: &'s AstBuilder<'a>,
    pub scoping: &'s Scoping,
    pub aliases: &'s FxHashMap<SymbolId, String>,
}

impl<'a> VisitMut<'a> for Rename<'_, 'a> {
    fn visit_program(&mut self, program: &mut Program<'a>) {
        public_exports::preserve(program, self);
        walk_mut::walk_program(self, program);
    }

    fn visit_binding_identifier(&mut self, identifier: &mut BindingIdentifier<'a>) {
        if let Some(alias) = identifier
            .symbol_id
            .get()
            .and_then(|symbol| self.aliases.get(&symbol))
        {
            identifier.name = self.builder.allocator().alloc_str(alias).into();
        }
    }
    fn visit_identifier_reference(&mut self, identifier: &mut IdentifierReference<'a>) {
        if let Some(alias) = identifier
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
            .and_then(|symbol| self.aliases.get(&symbol))
        {
            identifier.name = self.builder.allocator().alloc_str(alias).into();
        }
    }
    fn visit_object_property(&mut self, property: &mut ObjectProperty<'a>) {
        walk_mut::walk_object_property(self, property);
        if property.shorthand
            && let Expression::Identifier(identifier) = &property.value
            && property
                .key
                .static_name()
                .is_some_and(|key| key != identifier.name.as_str())
        {
            property.shorthand = false;
        }
    }
}

/// A deferred function permits later outer declarations, but not early reads
/// of declarations inside its own body. An IIFE adds no deferred boundary.
pub(super) fn deferred_reads(program: &Program<'_>, scoping: &Scoping) -> FxHashSet<ReferenceId> {
    struct Reads<'s> {
        scoping: &'s Scoping,
        boundary: Option<Span>,
        deferred: FxHashSet<ReferenceId>,
    }
    impl<'a> Visit<'a> for Reads<'_> {
        fn visit_identifier_reference(&mut self, identifier: &IdentifierReference<'a>) {
            if let Some(boundary) = self.boundary
                && let Some(reference) = identifier.reference_id.get()
                && let Some(symbol) = self.scoping.get_reference(reference).symbol_id()
                && !boundary.contains_inclusive(self.scoping.symbol_span(symbol))
            {
                self.deferred.insert(reference);
            }
        }
        fn visit_arrow_function_expression(&mut self, arrow: &ArrowFunctionExpression<'a>) {
            let outer = self.boundary.replace(arrow.span);
            walk::walk_arrow_function_expression(self, arrow);
            self.boundary = outer;
        }
        fn visit_function(
            &mut self,
            function: &Function<'a>,
            flags: oxc_syntax::scope::ScopeFlags,
        ) {
            let outer = self.boundary.replace(function.span);
            walk::walk_function(self, function, flags);
            self.boundary = outer;
        }
        fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
            match crate::utils::unwrap_syntax_only(&call.callee) {
                Expression::ArrowFunctionExpression(arrow) => {
                    walk::walk_arrow_function_expression(self, arrow);
                }
                Expression::FunctionExpression(function) => {
                    walk::walk_function(self, function, oxc_syntax::scope::ScopeFlags::empty());
                }
                _ => self.visit_expression(&call.callee),
            }
            for argument in &call.arguments {
                self.visit_argument(argument);
            }
        }
    }
    let mut reads = Reads {
        scoping,
        boundary: None,
        deferred: FxHashSet::default(),
    };
    reads.visit_program(program);
    reads.deferred
}
