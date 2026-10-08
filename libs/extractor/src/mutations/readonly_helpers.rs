//! A single pure scalar return cannot write, retain, or invoke its parameters.

use crate::{
    css_prop::binding_of,
    imported_constants::provenance::{Proof, Shape},
    utils::unwrap_syntax_only,
};
use oxc_ast::{
    AstKind,
    ast::{
        ArrowFunctionBody, CallExpression, Expression, FormalParameters, FunctionBody, Statement,
    },
};
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

pub(super) fn reads_arguments(proof: &Proof<'_, '_>, call: &CallExpression<'_>) -> bool {
    if !super::callees::pristine(proof, "Object") || !super::callees::pristine(proof, "Array") {
        return false;
    }
    let Expression::Identifier(callee) = unwrap_syntax_only(&call.callee) else {
        return false;
    };
    let Some(symbol) = binding_of(proof.scoping, callee) else {
        return false;
    };
    if proof
        .scoping
        .get_resolved_reference_ids(symbol)
        .iter()
        .any(|id| proof.scoping.get_reference(*id).is_write())
    {
        return false;
    }
    let (params, returned) = match proof.nodes.kind(proof.scoping.symbol_declaration(symbol)) {
        AstKind::VariableDeclarator(declaration) => {
            match declaration.init.as_ref().map(unwrap_syntax_only) {
                Some(Expression::ArrowFunctionExpression(arrow)) if !arrow.r#async => {
                    let returned = match &arrow.body {
                        ArrowFunctionBody::FunctionBody(body) => scalar_return(body),
                        expression => expression.as_expression(),
                    };
                    (&arrow.params, returned)
                }
                Some(Expression::FunctionExpression(function))
                    if !function.r#async && !function.generator =>
                {
                    (
                        &function.params,
                        function.body.as_ref().and_then(|body| scalar_return(body)),
                    )
                }
                _ => return false,
            }
        }
        AstKind::Function(function) if !function.r#async && !function.generator => (
            &function.params,
            function.body.as_ref().and_then(|body| scalar_return(body)),
        ),
        _ => return false,
    };
    let Some(returned) = returned else {
        return false;
    };
    let Some(parameters) = parameters(proof, params, call) else {
        return false;
    };
    scalar(proof, &parameters, returned)
}

#[cfg(test)]
#[path = "readonly_helper_tests.rs"]
mod tests;

fn scalar_return<'s, 'a>(body: &'s FunctionBody<'a>) -> Option<&'s Expression<'a>> {
    match body.statements.as_slice() {
        [Statement::ReturnStatement(statement)] => statement.argument.as_ref(),
        _ => None,
    }
}

fn parameters(
    proof: &Proof<'_, '_>,
    params: &FormalParameters<'_>,
    call: &CallExpression<'_>,
) -> Option<FxHashMap<SymbolId, Shape>> {
    if params.rest.is_some() || params.items.len() != call.arguments.len() {
        return None;
    }
    params
        .items
        .iter()
        .zip(&call.arguments)
        .map(|(parameter, argument)| {
            let symbol = parameter
                .pattern
                .get_binding_identifier()?
                .symbol_id
                .get()?;
            let shape = proof.expression(argument.as_expression()?);
            shape.plain().then_some((symbol, shape))
        })
        .collect()
}

fn scalar(
    proof: &Proof<'_, '_>,
    parameters: &FxHashMap<SymbolId, Shape>,
    expression: &Expression<'_>,
) -> bool {
    match unwrap_syntax_only(expression) {
        Expression::NumericLiteral(_)
        | Expression::StringLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_) => true,
        Expression::Identifier(identifier) => {
            binding_of(proof.scoping, identifier).is_some_and(|symbol| {
                parameters
                    .get(&symbol)
                    .is_some_and(|shape| matches!(shape, Shape::Primitive))
            })
        }
        Expression::StaticMemberExpression(_) | Expression::ComputedMemberExpression(_) => {
            let mut expression = unwrap_syntax_only(expression);
            let mut path = Vec::new();
            loop {
                match expression {
                    Expression::StaticMemberExpression(member) if !member.optional => {
                        path.push(Some(member.property.name.to_string()));
                        expression = unwrap_syntax_only(&member.object);
                    }
                    Expression::ComputedMemberExpression(member) if !member.optional => {
                        let Some(key) =
                            crate::utils::get_string_by_literal_expression(&member.expression)
                        else {
                            return false;
                        };
                        path.push(Some(key.into_owned()));
                        expression = unwrap_syntax_only(&member.object);
                    }
                    Expression::Identifier(identifier) => {
                        path.reverse();
                        return binding_of(proof.scoping, identifier)
                            .and_then(|symbol| parameters.get(&symbol))
                            .is_some_and(|shape| shape.primitive_path(&path));
                    }
                    _ => return false,
                }
            }
        }
        Expression::BinaryExpression(binary) => {
            scalar(proof, parameters, &binary.left) && scalar(proof, parameters, &binary.right)
        }
        Expression::UnaryExpression(unary)
            if matches!(
                unary.operator,
                oxc_syntax::operator::UnaryOperator::UnaryNegation
                    | oxc_syntax::operator::UnaryOperator::UnaryPlus
                    | oxc_syntax::operator::UnaryOperator::LogicalNot
                    | oxc_syntax::operator::UnaryOperator::BitwiseNot
                    | oxc_syntax::operator::UnaryOperator::Typeof
                    | oxc_syntax::operator::UnaryOperator::Void
            ) =>
        {
            scalar(proof, parameters, &unary.argument)
        }
        _ => false,
    }
}
