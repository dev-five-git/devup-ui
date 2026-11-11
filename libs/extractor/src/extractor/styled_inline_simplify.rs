//! Shorten a compiler-owned styled renderer once its stages are inlined: one
//! scope where no name collides, constants where nothing reassigns, and no
//! temporaries a single following declaration reads.

use crate::utils::{rewrite_expression, unwrap_syntax_only};
use oxc_allocator::{CloneIn, GetAllocator, TakeIn, Vec};
use oxc_ast::{
    ast::{
        AssignmentOperator, AssignmentTarget, AssignmentTargetMaybeDefault,
        AssignmentTargetProperty, AssignmentTargetPropertyIdentifier, BindingPattern,
        BindingProperty, BindingRestElement, Expression, IdentifierReference,
        SimpleAssignmentTarget, Statement, VariableDeclarationKind, VariableDeclarator,
    },
    builder::AstBuilder,
};
use oxc_ast_visit::{Visit, walk};
use oxc_span::SPAN;
use rustc_hash::{FxHashMap, FxHashSet};

#[cfg(test)]
#[path = "styled_inline_simplify_tests.rs"]
mod tests;

/// How often each name is referenced and assigned
#[derive(Default)]
struct Uses {
    reads: FxHashMap<String, usize>,
    writes: FxHashMap<String, usize>,
}

impl Uses {
    fn of(statements: &[Statement<'_>]) -> Self {
        let mut uses = Self::default();
        for statement in statements {
            uses.visit_statement(statement);
        }
        uses
    }

    fn reads(&self, name: &str) -> usize {
        self.reads.get(name).copied().unwrap_or_default()
    }

    fn writes(&self, name: &str) -> usize {
        self.writes.get(name).copied().unwrap_or_default()
    }
}

impl<'a> Visit<'a> for Uses {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        *self.reads.entry(it.name.to_string()).or_default() += 1;
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        if let SimpleAssignmentTarget::AssignmentTargetIdentifier(id) = it {
            *self.writes.entry(id.name.to_string()).or_default() += 1;
        }
        walk::walk_simple_assignment_target(self, it);
    }

    fn visit_assignment_target_property_identifier(
        &mut self,
        it: &AssignmentTargetPropertyIdentifier<'a>,
    ) {
        *self.writes.entry(it.binding.name.to_string()).or_default() += 1;
        walk::walk_assignment_target_property_identifier(self, it);
    }
}

fn bound(pattern: &BindingPattern<'_>, names: &mut FxHashSet<String>) {
    match pattern {
        BindingPattern::BindingIdentifier(id) => {
            names.insert(id.name.to_string());
        }
        BindingPattern::ObjectPattern(object) => {
            for property in &object.properties {
                bound(&property.value, names);
            }
            if let Some(rest) = &object.rest {
                bound(&rest.argument, names);
            }
        }
        BindingPattern::ArrayPattern(array) => {
            for element in array.elements.iter().flatten() {
                bound(element, names);
            }
            if let Some(rest) = &array.rest {
                bound(&rest.argument, names);
            }
        }
        BindingPattern::AssignmentPattern(assignment) => bound(&assignment.left, names),
    }
}

fn declared(statements: &[Statement<'_>]) -> FxHashSet<String> {
    let mut names = FxHashSet::default();
    for statement in statements {
        if let Statement::VariableDeclaration(declaration) = statement {
            for declarator in &declaration.declarations {
                bound(&declarator.id, &mut names);
            }
        }
    }
    names
}

/// `name` of `let name;`
fn bare_let<'s>(statement: &'s Statement<'_>) -> Option<&'s str> {
    let Statement::VariableDeclaration(declaration) = statement else {
        return None;
    };
    let [declarator] = declaration.declarations.as_slice() else {
        return None;
    };
    match &declarator.id {
        BindingPattern::BindingIdentifier(id)
            if declaration.kind == VariableDeclarationKind::Let && declarator.init.is_none() =>
        {
            Some(id.name.as_str())
        }
        _ => None,
    }
}

fn declaration<'a>(
    ast: &AstBuilder<'a>,
    kind: VariableDeclarationKind,
    pattern: BindingPattern<'a>,
    init: Expression<'a>,
) -> Statement<'a> {
    Statement::new_variable_declaration(
        SPAN,
        kind,
        Vec::from_array_in(
            [VariableDeclarator::new(
                SPAN,
                pattern,
                None,
                Some(init),
                false,
                ast,
            )],
            ast,
        ),
        false,
        ast,
    )
}

pub(super) fn simplify<'a>(
    ast: &AstBuilder<'a>,
    statements: &mut Vec<'a, Statement<'a>>,
    parameters: &[&str],
) {
    flatten(statements, parameters);
    assign_once(ast, statements);
    omit_ahead(ast, statements);
    single_use(statements);
    constants(statements);
}

/// Splice a trailing block into its list when what it declares neither
/// collides with nor is read before it there.
fn flatten<'a>(statements: &mut Vec<'a, Statement<'a>>, parameters: &[&str]) {
    while let Some(Statement::BlockStatement(block)) = statements.last() {
        let before = &statements[..statements.len() - 1];
        let mut outer = declared(before);
        outer.extend(parameters.iter().map(|name| (*name).to_string()));
        let uses = Uses::of(before);
        let inner = declared(&block.body);
        if inner
            .iter()
            .any(|name| outer.contains(name) || uses.reads(name) > 0)
        {
            if let Some(Statement::BlockStatement(block)) = statements.last_mut() {
                flatten(&mut block.body, &[]);
            }
            return;
        }
        if let Some(Statement::BlockStatement(mut block)) = statements.pop() {
            statements.append(&mut block.body);
        }
    }
}

/// `let x; …; x = value;` as `let x = value;` where that is its only assignment
/// and nothing between reads it.
fn assign_once<'a>(ast: &AstBuilder<'a>, statements: &mut Vec<'a, Statement<'a>>) {
    let mut index = 0;
    while index < statements.len() {
        let Some(name) = bare_let(&statements[index]).map(str::to_string) else {
            index += 1;
            continue;
        };
        let uses = Uses::of(statements);
        let first = (index + 1..statements.len())
            .find(|&later| Uses::of(&statements[later..=later]).reads(&name) > 0);
        let Some(later) = first.filter(|&later| {
            uses.writes(&name) == 1
                && matches!(&statements[later], Statement::ExpressionStatement(statement)
                    if matches!(&statement.expression, Expression::AssignmentExpression(assignment)
                        if assignment.operator == AssignmentOperator::Assign
                            && matches!(&assignment.left, AssignmentTarget::AssignmentTargetIdentifier(id) if id.name == name.as_str())))
                && Uses::of(&statements[later..=later]).reads(&name) == 1
        }) else {
            index += 1;
            continue;
        };
        if let Statement::ExpressionStatement(statement) = &mut statements[later]
            && let Expression::AssignmentExpression(assignment) = &mut statement.expression
        {
            let value = assignment.right.take_in(ast);
            let pattern =
                BindingPattern::new_binding_identifier(SPAN, ast.allocator().alloc_str(&name), ast);
            statements[later] = declaration(ast, VariableDeclarationKind::Let, pattern, value);
        }
        statements.remove(index);
    }
}

/// The trailing return's `({ theme: a, ...b } = rest, b)`, where `rest` is an
/// object rest no one else reads, as `let { theme: a, ...b } = rest;` before it:
/// destructuring a fresh data-only object observes nothing, wherever it runs.
fn omit_ahead<'a>(ast: &AstBuilder<'a>, statements: &mut Vec<'a, Statement<'a>>) {
    let Some((Statement::ReturnStatement(_), before)) = statements.split_last() else {
        return;
    };
    let uses = Uses::of(statements);
    let mut rests = FxHashSet::default();
    for statement in before {
        if let Statement::VariableDeclaration(declaration) = statement {
            for declarator in &declaration.declarations {
                if let BindingPattern::ObjectPattern(object) = &declarator.id
                    && let Some(rest) = &object.rest
                    && let BindingPattern::BindingIdentifier(id) = &rest.argument
                {
                    rests.insert(id.name.to_string());
                }
            }
        }
    }
    let slots: FxHashSet<String> = before
        .iter()
        .filter_map(bare_let)
        .map(str::to_string)
        .collect();
    let mut hoisted = std::vec::Vec::new();
    let Some(Statement::ReturnStatement(returned)) = statements.last_mut() else {
        return;
    };
    let Some(argument) = returned.argument.as_mut() else {
        return;
    };
    rewrite_expression(argument, &mut |value| {
        if matches!(
            value,
            Expression::ArrowFunctionExpression(_)
                | Expression::FunctionExpression(_)
                | Expression::ClassExpression(_)
        ) {
            return true;
        }
        let Some(pattern) = omission(ast, value, &rests, &slots, &uses) else {
            return false;
        };
        let Expression::SequenceExpression(sequence) = value else {
            return false;
        };
        let Some(read) = sequence.expressions.pop() else {
            return false;
        };
        let source = match sequence.expressions.pop() {
            Some(Expression::AssignmentExpression(mut assignment)) => assignment.right.take_in(ast),
            _ => return false,
        };
        hoisted.push((pattern, source));
        *value = read;
        true
    });
    for (pattern, source) in hoisted {
        let mut names = FxHashSet::default();
        bound(&pattern, &mut names);
        statements.retain(|statement| bare_let(statement).is_none_or(|name| !names.contains(name)));
        let at = statements.len() - 1;
        statements.insert(
            at,
            declaration(ast, VariableDeclarationKind::Let, pattern, source),
        );
    }
}

/// The binding pattern of `({ key: a, ...b } = rest, b)`
fn omission<'a>(
    ast: &AstBuilder<'a>,
    value: &Expression<'a>,
    rests: &FxHashSet<String>,
    slots: &FxHashSet<String>,
    uses: &Uses,
) -> Option<BindingPattern<'a>> {
    let Expression::SequenceExpression(sequence) = value else {
        return None;
    };
    let [
        Expression::AssignmentExpression(assignment),
        Expression::Identifier(read),
    ] = sequence.expressions.as_slice()
    else {
        return None;
    };
    let Expression::Identifier(source) = unwrap_syntax_only(&assignment.right) else {
        return None;
    };
    let AssignmentTarget::ObjectAssignmentTarget(target) = &assignment.left else {
        return None;
    };
    let slot = |name: &str| slots.contains(name) && uses.writes(name) == 1;
    if assignment.operator != AssignmentOperator::Assign
        || !rests.contains(source.name.as_str())
        || uses.reads(source.name.as_str()) != 1
        || uses.writes(source.name.as_str()) != 0
    {
        return None;
    }
    let rest = target.rest.as_ref()?;
    let AssignmentTarget::AssignmentTargetIdentifier(rest) = &rest.target else {
        return None;
    };
    if rest.name != read.name || !slot(rest.name.as_str()) {
        return None;
    }
    let mut properties = Vec::new_in(ast);
    for property in &target.properties {
        let AssignmentTargetProperty::AssignmentTargetPropertyProperty(property) = property else {
            return None;
        };
        let AssignmentTargetMaybeDefault::AssignmentTargetIdentifier(id) = &property.binding else {
            return None;
        };
        if property.computed || !slot(id.name.as_str()) {
            return None;
        }
        properties.push(BindingProperty::new(
            SPAN,
            property.name.clone_in(ast.allocator()),
            BindingPattern::new_binding_identifier(SPAN, id.name, ast),
            false,
            false,
            ast,
        ));
    }
    Some(BindingPattern::new_object_pattern(
        SPAN,
        properties,
        Some(BindingRestElement::boxed(
            SPAN,
            BindingPattern::new_binding_identifier(SPAN, rest.name, ast),
            ast,
        )),
        ast,
    ))
}

/// `let x = value; let pattern = x;` as `let pattern = value;` where nothing
/// else reads `x`
fn single_use(statements: &mut Vec<'_, Statement<'_>>) {
    let mut index = 0;
    while index + 1 < statements.len() {
        let uses = Uses::of(statements);
        let removable = matches!(&statements[index], Statement::VariableDeclaration(declaration)
            if matches!(declaration.declarations.as_slice(), [declarator]
                if declarator.init.is_some()
                    && matches!(&declarator.id, BindingPattern::BindingIdentifier(id)
                        if uses.reads(id.name.as_str()) == 1
                            && matches!(&statements[index + 1], Statement::VariableDeclaration(next)
                                if matches!(next.declarations.as_slice(), [next]
                                    if matches!(&next.init, Some(Expression::Identifier(read)) if read.name == id.name)
                                        && (!matches!(next.id, BindingPattern::BindingIdentifier(_))
                                            || !matches!(declarator.init.as_ref().map(unwrap_syntax_only),
                                                Some(Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_) | Expression::ClassExpression(_)))))))));
        if !removable {
            index += 1;
            continue;
        }
        let Statement::VariableDeclaration(mut declaration) = statements.remove(index) else {
            continue;
        };
        if let Statement::VariableDeclaration(next) = &mut statements[index] {
            next.declarations[0].init = declaration.declarations[0].init.take();
        }
    }
}

/// `let` to `const` where every declared name is initialized and never reassigned
fn constants(statements: &mut Vec<'_, Statement<'_>>) {
    let uses = Uses::of(statements);
    for statement in statements.iter_mut() {
        if let Statement::VariableDeclaration(declaration) = statement
            && declaration.kind == VariableDeclarationKind::Let
            && declaration.declarations.iter().all(|declarator| {
                let mut names = FxHashSet::default();
                bound(&declarator.id, &mut names);
                declarator.init.is_some() && names.iter().all(|name| uses.writes(name) == 0)
            })
        {
            declaration.kind = VariableDeclarationKind::Const;
        }
    }
}
