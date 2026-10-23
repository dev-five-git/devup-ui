use super::StyleValues;
use crate::utils::unwrap_syntax_only;
use oxc_ast::ast::{Expression, IdentifierReference, Program};
use oxc_ast_visit::Visit;
use oxc_semantic::{Scoping, SemanticBuilder};
use oxc_span::Span;
use rustc_hash::FxHashSet;

impl StyleValues<'_> {
    pub(crate) fn absence_proof(&self) -> Self {
        Self {
            scoping: self.scoping.clone(),
            absent_references: self.absent_references.clone(),
            ..Self::default()
        }
    }

    /// Prove that an authored `undefined` reads the unbound global, not a binding.
    pub(crate) fn is_unbound_undefined(&self, expression: &Expression<'_>) -> bool {
        let Expression::Identifier(identifier) = unwrap_syntax_only(expression) else {
            return false;
        };
        if identifier.name != "undefined" {
            return false;
        }
        self.absent_references.contains(&identifier.span)
            || match (&self.scoping, identifier.reference_id.get()) {
                (Some(scoping), Some(reference)) => {
                    scoping.get_reference(reference).symbol_id().is_none()
                }
                _ => false,
            }
    }

    /// Keep semantic proof tied to the authored span when extraction clones clear IDs.
    pub(crate) fn record_absence(&mut self, program: &Program<'_>) {
        if !program.source_text.contains("undefined") {
            return;
        }
        let temporary;
        let scoping: &Scoping = if let Some(scoping) = &self.scoping {
            scoping
        } else {
            temporary = SemanticBuilder::new()
                .build(program)
                .semantic
                .into_scoping();
            &temporary
        };
        let mut references = AbsenceReferences {
            scoping,
            spans: FxHashSet::default(),
        };
        references.visit_program(program);
        self.absent_references = references.spans;
    }
}

struct AbsenceReferences<'s> {
    scoping: &'s Scoping,
    spans: FxHashSet<Span>,
}

impl<'a> Visit<'a> for AbsenceReferences<'_> {
    fn visit_identifier_reference(&mut self, identifier: &IdentifierReference<'a>) {
        if identifier.name == "undefined"
            && let Some(reference) = identifier.reference_id.get()
            && self.scoping.get_reference(reference).symbol_id().is_none()
        {
            self.spans.insert(identifier.span);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxc_allocator::Allocator;
    use oxc_ast::ast::Statement;
    use oxc_parser::Parser;
    use oxc_semantic::SemanticBuilder;
    use oxc_span::SourceType;
    use rstest::rstest;

    #[rstest]
    #[case("undefined;", true)]
    #[case("let undefined = runtime; undefined;", false)]
    #[case("other;", false)]
    #[case("null;", false)]
    fn absence_requires_an_unbound_undefined_reference(#[case] source: &str, #[case] absent: bool) {
        let arena = Allocator::default();
        let parsed = Parser::new(&arena, source, SourceType::tsx()).parse();
        let scoping = SemanticBuilder::new()
            .build(&parsed.program)
            .semantic
            .into_scoping();
        let values = StyleValues::new(std::rc::Rc::new(scoping));
        let Some(Statement::ExpressionStatement(statement)) = parsed.program.body.last() else {
            panic!("expected expression");
        };
        let result = values.is_unbound_undefined(&statement.expression);
        assert_eq!(result, absent);
    }

    #[rstest]
    #[case(false)]
    #[case(true)]
    fn missing_metadata_never_proves_absence(#[case] with_scoping: bool) {
        let arena = Allocator::default();
        let parsed = Parser::new(&arena, "undefined;", SourceType::tsx()).parse();
        let values = if with_scoping {
            StyleValues::new(std::rc::Rc::new(
                SemanticBuilder::new()
                    .build(&parsed.program)
                    .semantic
                    .into_scoping(),
            ))
        } else {
            StyleValues::default()
        };
        let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
            panic!("expected expression");
        };
        let Expression::Identifier(identifier) = &statement.expression else {
            panic!("expected identifier");
        };
        identifier.reference_id.set(None);
        assert!(!values.is_unbound_undefined(&statement.expression));
    }
}
