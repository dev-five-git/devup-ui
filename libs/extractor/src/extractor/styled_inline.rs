//! Remove intermediate calls from compiler-owned styled renderers. Assignments
//! stay in their original expressions; tail calls retain their lexical blocks.

use super::styled_record_evaluation::Evaluations;
use crate::utils::{readable_code, rewrite_expression, unwrap_syntax_only, unwrap_syntax_only_mut};
use oxc_allocator::{CloneIn, FromIn, GetAllocator, TakeIn, Vec};
use oxc_ast::{
    ast::{
        Argument, ArrayExpressionElement, ArrowFunctionBody, AssignmentTarget,
        AssignmentTargetMaybeDefault, AssignmentTargetProperty, AssignmentTargetRest,
        BindingPattern, Expression, Ident, JSXElement, JSXElementName, ObjectPropertyKind, Program,
        Statement, Str, VariableDeclarationKind, VariableDeclarator,
    },
    builder::AstBuilder,
};
use oxc_span::{SPAN, SourceType};
use oxc_syntax::operator::AssignmentOperator;

#[cfg(test)]
#[path = "styled_inline_tests.rs"]
mod tests;

struct Render<'b, 'a> {
    ast: &'b AstBuilder<'a>,
    names: Evaluations<'b, 'a>,
    slots: std::vec::Vec<String>,
}

fn binding<'a>(ast: &AstBuilder<'a>, name: &str) -> BindingPattern<'a> {
    BindingPattern::new_binding_identifier(SPAN, Ident::from_in(name, ast.allocator()), ast)
}

fn read<'a>(ast: &AstBuilder<'a>, name: &str) -> Expression<'a> {
    Expression::new_identifier(SPAN, Ident::from_in(name, ast.allocator()), ast)
}

fn target<'a>(ast: &AstBuilder<'a>, name: &str) -> AssignmentTarget<'a> {
    AssignmentTarget::new_assignment_target_identifier(
        SPAN,
        Ident::from_in(name, ast.allocator()),
        ast,
    )
}

fn declaration<'a>(
    ast: &AstBuilder<'a>,
    pattern: BindingPattern<'a>,
    value: Option<Expression<'a>>,
) -> Statement<'a> {
    Statement::new_variable_declaration(
        SPAN,
        VariableDeclarationKind::Let,
        Vec::from_array_in(
            [VariableDeclarator::new(
                SPAN, pattern, None, value, false, ast,
            )],
            ast,
        ),
        false,
        ast,
    )
}

fn assignment<'a>(
    ast: &AstBuilder<'a>,
    left: AssignmentTarget<'a>,
    value: Expression<'a>,
) -> Expression<'a> {
    // An argument or a returned anonymous definition has no inferred temporary name.
    let value = if matches!(
        unwrap_syntax_only(&value),
        Expression::ArrowFunctionExpression(_)
            | Expression::FunctionExpression(_)
            | Expression::ClassExpression(_)
    ) {
        crate::utils::unbound_value(ast, value)
    } else {
        value
    };
    Expression::new_assignment_expression(SPAN, AssignmentOperator::Assign, left, value, ast)
}

/// The ref adapter is the actual render function. Its intermediate component
/// calls do not need independent function objects or function-local parameters.
pub(super) fn body<'a>(
    ast: &AstBuilder<'a>,
    component: Expression<'a>,
    props: Expression<'a>,
) -> ArrowFunctionBody<'a> {
    let mut source = readable_code(&component);
    source.push(';');
    source.push_str(&readable_code(&props));
    let mut render = Render {
        ast,
        slots: std::vec::Vec::new(),
        names: Evaluations::new(ast, source),
    };
    let mut component = component;
    rewrite_expression(&mut component, &mut |value| {
        if let Expression::Identifier(id) = value
            && !oxc_syntax::identifier::is_identifier_name(id.name.as_str())
            && let Ok(mut parsed) =
                oxc_parser::Parser::new(ast.allocator(), id.name.as_str(), SourceType::tsx())
                    .parse_expression()
        {
            crate::utils::clear_generated_spans(&mut parsed);
            *value = parsed;
        }
        false
    });
    let mut eval = false;
    rewrite_expression(&mut component, &mut |value| {
        eval |= matches!(value, Expression::CallExpression(call) if matches!(unwrap_syntax_only(&call.callee), Expression::Identifier(id) if id.name == "eval"));
        false
    });
    if !eval {
        render.frames(&mut component);
    }
    let mut call = Expression::new_call_expression(
        SPAN,
        component,
        None,
        Vec::from_array_in([Argument::from(props)], ast),
        false,
        ast,
    );
    if eval || !Render::tail_call(&call) {
        return call.into();
    }
    let mut statements = Vec::new_in(ast);
    render.complete(&mut statements, call.take_in(ast), None);
    super::styled_inline_simplify::simplify(
        ast,
        &mut statements,
        &["__devupRefProps", "__devupRef"],
    );
    ArrowFunctionBody::new_function_body(SPAN, Vec::new_in(ast), statements, ast)
}

impl<'a> Render<'_, 'a> {
    fn frames(&mut self, expression: &mut Expression<'a>) {
        rewrite_expression(expression, &mut |value| match value {
            Expression::ArrowFunctionExpression(arrow) => {
                let mut slots = std::vec::Vec::new();
                let ast = self.ast;
                let mut rewrite = |inner: &mut Expression<'a>| {
                    if matches!(inner, Expression::ArrowFunctionExpression(_)) {
                        self.frames(inner);
                        return true;
                    }
                    if matches!(
                        inner,
                        Expression::FunctionExpression(_) | Expression::ClassExpression(_)
                    ) {
                        return true;
                    }
                    if let Expression::CallExpression(call) = inner {
                        self.frames(&mut call.callee);
                    }
                    if let Some(replacement) = self.expression(inner, &mut slots) {
                        *inner = replacement;
                    }
                    false
                };
                if let Some(body) = arrow.body.as_expression_mut() {
                    rewrite_expression(body, &mut rewrite);
                } else if let ArrowFunctionBody::FunctionBody(body) = &mut arrow.body {
                    let mut program = Program::new(
                        SPAN,
                        SourceType::tsx(),
                        "",
                        Vec::new_in(ast),
                        None,
                        Vec::new_in(ast),
                        body.statements.take_in(ast),
                        ast,
                    );
                    crate::utils::rewrite_program(&mut program, &mut rewrite);
                    body.statements = program.body;
                }
                if !slots.is_empty() {
                    let mut statements = Vec::new_in(self.ast);
                    for name in slots {
                        self.slots.push(name.clone());
                        statements.push(declaration(self.ast, binding(self.ast, &name), None));
                    }
                    let mut directives = Vec::new_in(self.ast);
                    match arrow.body.take_in(self.ast) {
                        ArrowFunctionBody::FunctionBody(mut body) => {
                            directives = body.directives.take_in(self.ast);
                            statements.append(&mut body.statements);
                        }
                        body => statements.push(Statement::new_return_statement(
                            SPAN,
                            Some(body.into_expression()),
                            self.ast,
                        )),
                    }
                    arrow.body = ArrowFunctionBody::new_function_body(
                        SPAN, directives, statements, self.ast,
                    );
                }
                true
            }
            Expression::FunctionExpression(_) | Expression::ClassExpression(_) => true,
            _ => false,
        });
    }

    fn expression(
        &mut self,
        value: &mut Expression<'a>,
        slots: &mut std::vec::Vec<String>,
    ) -> Option<Expression<'a>> {
        let Expression::CallExpression(call) = unwrap_syntax_only_mut(value) else {
            return None;
        };
        let Expression::ArrowFunctionExpression(arrow) = unwrap_syntax_only(&call.callee) else {
            return None;
        };
        if arrow.r#async || arrow.params.rest.is_some() || call.optional {
            return None;
        }
        let [argument] = call.arguments.as_slice() else {
            return None;
        };
        let argument = argument.as_expression()?;
        let mut promoted = std::vec::Vec::new();
        let mut records = std::vec::Vec::new();
        let body = match &arrow.body {
            ArrowFunctionBody::FunctionBody(body) => {
                if !body.directives.is_empty() {
                    return None;
                }
                let (Statement::ReturnStatement(returned), declarations) =
                    body.statements.split_last()?
                else {
                    return None;
                };
                for statement in declarations {
                    let Statement::VariableDeclaration(declaration) = statement else {
                        return None;
                    };
                    for declaration in &declaration.declarations {
                        let BindingPattern::BindingIdentifier(id) = &declaration.id else {
                            return None;
                        };
                        if declaration.init.is_none()
                            && self.slots.iter().any(|name| name == id.name.as_str())
                        {
                            promoted.push(id.name.to_string());
                        } else if let Some(Expression::ArrayExpression(array)) = &declaration.init
                            && array.elements.iter().all(|element| {
                                matches!(element, ArrayExpressionElement::StringLiteral(_))
                            })
                        {
                            records.push((
                                id.name.to_string(),
                                declaration.init.as_ref()?.clone_in(self.ast.allocator()),
                            ));
                        } else {
                            return None;
                        }
                    }
                }
                returned.argument.as_ref()?
            }
            body => body.as_expression()?,
        };
        let mut body = body.clone_in(self.ast.allocator());
        if !crate::utils::capture_elision::can_transport(&mut body, &records) {
            return None;
        }
        if !records.is_empty() {
            let mut prefix = Vec::new_in(self.ast);
            for (old, value) in records {
                let name = self.names.fresh_name();
                if !self.rename(&mut body, &old, &name) {
                    return None;
                }
                prefix.push(assignment(self.ast, target(self.ast, &name), value));
                promoted.push(name);
            }
            prefix.push(body);
            body = Expression::new_sequence_expression(SPAN, prefix, self.ast);
        }
        if arrow.params.items.is_empty() {
            slots.extend(promoted);
            return Some(Expression::new_sequence_expression(
                SPAN,
                Vec::from_array_in([argument.clone_in(self.ast.allocator()), body], self.ast),
                self.ast,
            ));
        }
        let [parameter] = arrow.params.items.as_slice() else {
            return None;
        };
        if parameter.initializer.is_some() {
            return None;
        }
        if let BindingPattern::BindingIdentifier(id) = &parameter.pattern {
            let old = id.name.to_string();
            let mut values = vec![(old.clone(), argument.clone_in(self.ast.allocator()))];
            crate::utils::capture_elision::inline(self.ast, &mut values, &mut body);
            if values.is_empty() {
                slots.extend(promoted);
                // A call produces a value, never the member reference that a
                // following call/delete/typeof could otherwise observe.
                return Some(
                    if matches!(
                        unwrap_syntax_only(&body),
                        Expression::Identifier(_)
                            | Expression::StaticMemberExpression(_)
                            | Expression::ComputedMemberExpression(_)
                            | Expression::ChainExpression(_)
                    ) {
                        crate::utils::unbound_value(self.ast, body)
                    } else {
                        body
                    },
                );
            }
            if !crate::utils::capture_elision::can_transport(&mut body, &values) {
                return None;
            }
            let name = self.names.fresh_name();
            if !self.rename(&mut body, &old, &name) {
                return None;
            }
            slots.push(name.clone());
            slots.extend(promoted);
            let (_, argument) = values.pop()?;
            return Some(Expression::new_sequence_expression(
                SPAN,
                Vec::from_array_in(
                    [
                        assignment(self.ast, target(self.ast, &name), argument),
                        body,
                    ],
                    self.ast,
                ),
                self.ast,
            ));
        }
        let mut names = std::vec::Vec::new();
        let pattern = self.pattern(&parameter.pattern, &mut names)?;
        if !names
            .iter()
            .all(|(old, _)| crate::utils::capture_elision::can_transport_name(&mut body, old))
        {
            return None;
        }
        for (old, name) in names {
            if !self.rename(&mut body, &old, &name) {
                return None;
            }
            slots.push(name);
        }
        slots.extend(promoted);
        Some(Expression::new_sequence_expression(
            SPAN,
            Vec::from_array_in(
                [
                    assignment(self.ast, pattern, argument.clone_in(self.ast.allocator())),
                    body,
                ],
                self.ast,
            ),
            self.ast,
        ))
    }

    fn rename(&self, body: &mut Expression<'a>, old: &str, name: &str) -> bool {
        let mut opaque_tag = false;
        rewrite_expression(body, &mut |value| {
            opaque_tag |= matches!(value, Expression::JSXElement(element) if !matches!(element.opening_element.name, JSXElementName::Identifier(_) | JSXElementName::IdentifierReference(_)));
            false
        });
        if opaque_tag {
            return false;
        }
        rewrite_expression(body, &mut |value| {
            if matches!(value, Expression::Identifier(id) if id.name == old) {
                *value = read(self.ast, name);
            }
            if let Expression::ObjectExpression(object) = value {
                for property in &mut object.properties {
                    if let ObjectPropertyKind::ObjectProperty(property) = property
                        && matches!(&property.value, Expression::Identifier(id) if id.name == old)
                    {
                        property.shorthand = false;
                    }
                }
            }
            if let Expression::JSXElement(element) = value {
                let JSXElement {
                    opening_element,
                    closing_element,
                    ..
                } = &mut **element;
                for tag in std::iter::once(&mut opening_element.name)
                    .chain(closing_element.iter_mut().map(|closing| &mut closing.name))
                {
                    match tag {
                        JSXElementName::Identifier(id)
                            if id.name == old
                                && !id
                                    .name
                                    .as_str()
                                    .starts_with(|c: char| c.is_ascii_lowercase()) =>
                        {
                            id.name = Str::from_in(name, self.ast.allocator());
                        }
                        JSXElementName::IdentifierReference(id) if id.name == old => {
                            id.name = Ident::from_in(name, self.ast.allocator());
                        }
                        _ => {}
                    }
                }
            }
            false
        });

        true
    }

    fn pattern(
        &mut self,
        pattern: &BindingPattern<'a>,
        names: &mut std::vec::Vec<(String, String)>,
    ) -> Option<AssignmentTarget<'a>> {
        Some(match pattern {
            BindingPattern::BindingIdentifier(id) => {
                let name = self.names.fresh_name();
                let result = target(self.ast, &name);
                names.push((id.name.to_string(), name));
                result
            }
            BindingPattern::ObjectPattern(object) => {
                let mut properties = Vec::new_in(self.ast);
                for property in &object.properties {
                    if property.computed {
                        return None;
                    }
                    let value = self.pattern(&property.value, names)?;
                    properties.push(
                        AssignmentTargetProperty::new_assignment_target_property_property(
                            SPAN,
                            property.key.clone_in(self.ast.allocator()),
                            AssignmentTargetMaybeDefault::from(value),
                            false,
                            self.ast,
                        ),
                    );
                }
                let rest = match &object.rest {
                    Some(rest) => Some(AssignmentTargetRest::boxed(
                        SPAN,
                        self.pattern(&rest.argument, names)?,
                        self.ast,
                    )),
                    None => None,
                };
                AssignmentTarget::new_object_assignment_target(SPAN, properties, rest, self.ast)
            }
            BindingPattern::ArrayPattern(array) => {
                let mut elements = Vec::new_in(self.ast);
                for element in &array.elements {
                    elements.push(match element {
                        Some(element) => Some(AssignmentTargetMaybeDefault::from(
                            self.pattern(element, names)?,
                        )),
                        None => None,
                    });
                }
                let rest = match &array.rest {
                    Some(rest) => Some(AssignmentTargetRest::boxed(
                        SPAN,
                        self.pattern(&rest.argument, names)?,
                        self.ast,
                    )),
                    None => None,
                };
                AssignmentTarget::new_array_assignment_target(SPAN, elements, rest, self.ast)
            }
            BindingPattern::AssignmentPattern(_) => return None,
        })
    }

    fn tail_call(value: &Expression<'a>) -> bool {
        let Expression::CallExpression(call) = unwrap_syntax_only(value) else {
            return false;
        };
        let Expression::ArrowFunctionExpression(arrow) = unwrap_syntax_only(&call.callee) else {
            return false;
        };
        if call.optional
            || arrow.r#async
            || arrow.params.rest.is_some()
            || arrow.params.items.len() != 1
            || call.arguments.len() != 1
            || !call.arguments[0].is_expression()
            || arrow.params.items[0].initializer.is_some()
        {
            return false;
        }
        match &arrow.body {
            ArrowFunctionBody::FunctionBody(body) => body.directives.is_empty() && matches!(body.statements.last(), Some(Statement::ReturnStatement(_)))
                && body.statements.iter().take(body.statements.len().saturating_sub(1)).all(|statement| matches!(statement, Statement::VariableDeclaration(declaration) if declaration.kind != VariableDeclarationKind::Var))
                && match &arrow.params.items[0].pattern {
                    BindingPattern::ObjectPattern(_) => true,
                    BindingPattern::BindingIdentifier(id) => id.name.starts_with("__devup"),
                    _ => false,
                },
            _ => true,
        }
    }

    fn complete(
        &mut self,
        statements: &mut Vec<'a, Statement<'a>>,
        mut value: Expression<'a>,
        destination: Option<&str>,
    ) {
        if Self::tail_call(&value) {
            let Expression::CallExpression(mut call) =
                unwrap_syntax_only_mut(&mut value).take_in(self.ast)
            else {
                unreachable!()
            };
            let Expression::ArrowFunctionExpression(mut arrow) =
                unwrap_syntax_only_mut(&mut call.callee).take_in(self.ast)
            else {
                unreachable!()
            };
            let name = self.names.fresh_name();
            statements.push(declaration(self.ast, binding(self.ast, &name), None));
            self.complete(
                statements,
                call.arguments.remove(0).into_expression(),
                Some(&name),
            );
            let mut block = Vec::new_in(self.ast);
            let parameter = declaration(
                self.ast,
                arrow.params.items.remove(0).pattern,
                Some(read(self.ast, &name)),
            );
            block.push(parameter);
            let (mut body_statements, returned, separate_body) = match arrow.body.take_in(self.ast)
            {
                ArrowFunctionBody::FunctionBody(mut body) => {
                    let Some(Statement::ReturnStatement(mut returned)) = body.statements.pop()
                    else {
                        unreachable!()
                    };
                    let returned = returned.argument.take().unwrap_or_else(|| {
                        Expression::new_unary_expression(
                            SPAN,
                            oxc_syntax::operator::UnaryOperator::Void,
                            Expression::new_numeric_literal(
                                SPAN,
                                0.0,
                                None,
                                oxc_syntax::number::NumberBase::Decimal,
                                self.ast,
                            ),
                            self.ast,
                        )
                    });
                    (body.statements.take_in(self.ast), returned, true)
                }
                body => (Vec::new_in(self.ast), body.into_expression(), false),
            };
            if separate_body {
                self.complete(&mut body_statements, returned, destination);
                block.push(Statement::new_block_statement(
                    SPAN,
                    body_statements,
                    self.ast,
                ));
            } else {
                self.complete(&mut block, returned, destination);
            }
            statements.push(Statement::new_block_statement(SPAN, block, self.ast));
        } else {
            statements.push(match destination {
                None => Statement::new_return_statement(SPAN, Some(value), self.ast),
                Some(name) => Statement::new_expression_statement(
                    SPAN,
                    assignment(self.ast, target(self.ast, name), value),
                    self.ast,
                ),
            });
        }
    }
}
