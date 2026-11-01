use oxc_ast::{ast::Expression, ast::IdentifierReference};
use oxc_ast_visit::Visit;
use oxc_semantic::Scoping;
use oxc_span::{GetSpan, Span};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;

pub(super) fn collect(
    expression: &Expression<'_>,
    scoping: &Scoping,
) -> Option<FxHashSet<SymbolId>> {
    struct Captures<'s> {
        scoping: &'s Scoping,
        span: Span,
        symbols: FxHashSet<SymbolId>,
        safe: bool,
    }
    impl<'a> Visit<'a> for Captures<'_> {
        fn visit_identifier_reference(&mut self, identifier: &IdentifierReference<'a>) {
            match identifier
                .reference_id
                .get()
                .and_then(|id| self.scoping.get_reference(id).symbol_id())
            {
                Some(symbol)
                    if !self
                        .span
                        .contains_inclusive(self.scoping.symbol_span(symbol)) =>
                {
                    self.symbols.insert(symbol);
                }
                None if super::local_capture_reads::generated_name(&identifier.name)
                    || self.scoping.symbol_ids().any(|symbol| {
                        self.scoping.symbol_name(symbol) == identifier.name.as_str()
                    }) =>
                {
                    self.safe = false;
                }
                _ => {}
            }
        }
    }
    let mut collector = Captures {
        scoping,
        span: expression.span(),
        symbols: FxHashSet::default(),
        safe: true,
    };
    collector.visit_expression(expression);
    collector.safe.then_some(collector.symbols)
}
