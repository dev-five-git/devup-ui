use oxc_ast::ast::{Expression, ObjectPropertyKind};
use oxc_semantic::Scoping;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;

/// Only function leaves move into new scopes; scalar leaves read the original object.
pub(in crate::imported_constants) fn collect(
    shape: &Expression<'_>,
    scoping: &Scoping,
) -> Option<FxHashSet<SymbolId>> {
    let mut captures = FxHashSet::default();
    match shape {
        Expression::ObjectExpression(object) => {
            for property in &object.properties {
                if let ObjectPropertyKind::ObjectProperty(property) = property {
                    captures.extend(collect(&property.value, scoping)?);
                }
            }
        }
        Expression::ArrayExpression(array) => {
            for value in array
                .elements
                .iter()
                .filter_map(|element| element.as_expression())
            {
                captures.extend(collect(value, scoping)?);
            }
        }
        Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_) => {
            captures.extend(super::super::bound_callback_captures::collect(
                shape, scoping,
            )?);
        }
        _ => {}
    }
    Some(captures)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxc_ast::ast::Statement;

    #[test]
    fn array_function_leaves_collect_captures_without_scalar_initializer_reads()
    -> Result<(), Box<dyn std::error::Error>> {
        // Given
        let allocator = oxc_allocator::Allocator::default();
        let parsed = oxc_parser::Parser::new(
            &allocator,
            "let rest, scalar; const inner = [{color: [() => rest], width: scalar}];",
            oxc_span::SourceType::ts(),
        )
        .parse();
        let scoping = oxc_semantic::SemanticBuilder::new()
            .build(&parsed.program)
            .semantic
            .into_scoping();
        let Statement::VariableDeclaration(declaration) = &parsed.program.body[1] else {
            panic!("expected shape declaration");
        };
        let shape = declaration.declarations[0]
            .init
            .as_ref()
            .ok_or("missing shape initializer")?;
        let rest = scoping
            .symbol_ids()
            .find(|symbol| scoping.symbol_name(*symbol) == "rest")
            .ok_or("missing captured rest binding")?;
        // When
        let captured =
            collect(shape, &scoping).ok_or("readable function captures were rejected")?;
        // Then
        assert_eq!(captured, FxHashSet::from_iter([rest]));
        Ok(())
    }
}
