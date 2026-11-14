//! Shorten a compiler-owned styled renderer once its stages are inlined: one
//! scope where no name collides, constants where nothing reassigns, and no
//! temporaries a single following declaration reads.

use crate::utils::{rewrite_expression, unwrap_syntax_only};
use oxc_allocator::{CloneIn, GetAllocator, TakeIn, Vec};
use oxc_ast::{
    ast::{
        ArrayExpressionElement, ArrowFunctionBody, ArrowFunctionExpression, AssignmentOperator,
        AssignmentTarget, AssignmentTargetMaybeDefault, AssignmentTargetProperty,
        AssignmentTargetPropertyIdentifier, BindingPattern, BindingProperty, BindingRestElement,
        Expression, Ident, IdentifierReference, ObjectAssignmentTarget, PropertyKey,
        SimpleAssignmentTarget, Statement, VariableDeclarationKind, VariableDeclarator,
    },
    builder::AstBuilder,
};
use oxc_ast_visit::{Visit, VisitMut, walk, walk_mut};
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
    split(ast, statements);
    assign_once(ast, statements);
    omit_ahead(ast, statements);
    // Moving a value into its declaration can bring a sequence to split; each
    // round that goes on adds or removes statements.
    loop {
        let count = statements.len();
        single_use(statements);
        split(ast, statements);
        assign_once(ast, statements);
        if statements.len() == count {
            break;
        }
    }
    single_use(statements);
    constants(statements);
}

/// Whether `statements` still declare one of `names` without a value
pub(super) fn leaves_slot(statements: &[Statement<'_>], names: &[String]) -> bool {
    struct Slots<'n> {
        names: &'n [String],
        found: bool,
    }
    impl<'a> Visit<'a> for Slots<'_> {
        fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
            let mut bound_names = FxHashSet::default();
            bound(&it.id, &mut bound_names);
            self.found |= it.init.is_none()
                && bound_names
                    .iter()
                    .any(|name| self.names.iter().any(|slot| slot == name));
            walk::walk_variable_declarator(self, it);
        }
    }
    let mut slots = Slots {
        names,
        found: false,
    };
    for statement in statements {
        slots.visit_statement(statement);
    }
    slots.found
}

/// Simplifies the frames left around calls that were not inlined, innermost
/// first, once inlining no longer reads their slots.
pub(super) struct Frames<'s, 'a> {
    pub ast: &'s AstBuilder<'a>,
    pub slots: &'s [String],
}

impl<'a> VisitMut<'a> for Frames<'_, 'a> {
    fn visit_arrow_function_expression(&mut self, arrow: &mut ArrowFunctionExpression<'a>) {
        walk_mut::walk_arrow_function_expression(self, arrow);
        if let ArrowFunctionBody::FunctionBody(body) = &mut arrow.body
            && leaves_slot(&body.statements, self.slots)
        {
            let parameters: std::vec::Vec<_> = arrow
                .params
                .items
                .iter()
                .filter_map(|parameter| parameter.pattern.get_binding_identifier())
                .map(|id| id.name.as_str())
                .collect();
            simplify(self.ast, &mut body.statements, &parameters);
        }
    }
}

/// The value a statement evaluates last, as a sequence, then whatever runs
/// before it as statements of their own: a sequence runs its parts in order.
fn split<'a>(ast: &AstBuilder<'a>, statements: &mut Vec<'a, Statement<'a>>) {
    let mut index = 0;
    while index < statements.len() {
        let alone = matches!(&statements[index], Statement::ExpressionStatement(_));
        let value = match &mut statements[index] {
            Statement::ExpressionStatement(statement) => Some(&mut statement.expression),
            Statement::ReturnStatement(statement) => statement.argument.as_mut(),
            Statement::VariableDeclaration(declaration) if declaration.declarations.len() == 1 => {
                declaration.declarations[0].init.as_mut()
            }
            _ => None,
        };
        let Some((parts, unbound)) = value
            .and_then(head_sequence)
            .and_then(|value| unsequence(ast, value, alone))
        else {
            index += 1;
            continue;
        };
        let count = parts.len();
        for (offset, part) in parts.into_iter().enumerate() {
            statements.insert(
                index + offset,
                Statement::new_expression_statement(SPAN, part, ast),
            );
        }
        // What the sequence ended with may be another sequence to split.
        index += count + usize::from(unbound);
    }
}

/// The parts the sequence `value` runs before its last, which `value` is left
/// as, and whether that last is a definition no longer named after a binding
fn unsequence<'a>(
    ast: &AstBuilder<'a>,
    value: &mut Expression<'a>,
    alone: bool,
) -> Option<(std::vec::Vec<Expression<'a>>, bool)> {
    // A string alone in a statement could read as a directive.
    if let Expression::SequenceExpression(sequence) = value
        && !(alone
            && matches!(
                sequence.expressions.last().map(unwrap_syntax_only),
                Some(Expression::StringLiteral(_))
            ))
        && let Some(tail) = sequence.expressions.pop()
    {
        // An anonymous definition read through a sequence is not named after its binding.
        let unbound = matches!(
            unwrap_syntax_only(&tail),
            Expression::ArrowFunctionExpression(_)
                | Expression::FunctionExpression(_)
                | Expression::ClassExpression(_)
        );
        let parts = sequence
            .expressions
            .drain(..)
            .filter(|part| !unwrap_syntax_only(part).is_literal())
            .collect();
        *value = if unbound {
            crate::utils::unbound_value(ast, tail)
        } else {
            tail
        };
        return Some((parts, unbound));
    }
    None
}

/// The sequence that `value` evaluates before anything else in it
fn head_sequence<'b, 'a>(value: &'b mut Expression<'a>) -> Option<&'b mut Expression<'a>> {
    let value = crate::utils::unwrap_syntax_only_mut(value);
    if matches!(value, Expression::SequenceExpression(_)) {
        return Some(value);
    }
    match value {
        Expression::ArrayExpression(array) => array
            .elements
            .first_mut()
            .and_then(ArrayExpressionElement::as_expression_mut)
            .and_then(head_sequence),
        Expression::LogicalExpression(logical) => head_sequence(&mut logical.left),
        Expression::BinaryExpression(binary) => head_sequence(&mut binary.left),
        Expression::ConditionalExpression(conditional) => head_sequence(&mut conditional.test),
        Expression::StaticMemberExpression(member) => head_sequence(&mut member.object),
        Expression::ComputedMemberExpression(member) => head_sequence(&mut member.object),
        _ => None,
    }
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

/// `let x; …; x = value;` as `let x = value;`, and likewise for a pattern of
/// such names, where that is their only assignment and nothing before reads them.
fn assign_once<'a>(ast: &AstBuilder<'a>, statements: &mut Vec<'a, Statement<'a>>) {
    let mut index = 0;
    while index < statements.len() {
        let uses = Uses::of(statements);
        let before = Uses::of(&statements[..index]);
        let here = Uses::of(&statements[index..=index]);
        let slots: FxHashSet<&str> = statements[..index].iter().filter_map(bare_let).collect();
        let mut names = std::vec::Vec::new();
        let pattern = match &statements[index] {
            Statement::ExpressionStatement(statement) => {
                match unwrap_syntax_only(&statement.expression) {
                    Expression::AssignmentExpression(assignment)
                        if assignment.operator == AssignmentOperator::Assign =>
                    {
                        binding_of(ast, &assignment.left, &mut names)
                    }
                    _ => None,
                }
            }
            _ => None,
        };
        let Some(pattern) = pattern.filter(|_| {
            names.iter().all(|name| {
                slots.contains(name.as_str())
                    && uses.writes(name) == 1
                    && before.reads(name) == 0
                    && here.reads(name) == 1
            })
        }) else {
            index += 1;
            continue;
        };
        if let Statement::ExpressionStatement(statement) = &mut statements[index]
            && let Expression::AssignmentExpression(assignment) =
                crate::utils::unwrap_syntax_only_mut(&mut statement.expression)
        {
            let value = assignment.right.take_in(ast);
            statements[index] = declaration(ast, VariableDeclarationKind::Let, pattern, value);
        }
        let length = statements.len();
        statements.retain(|statement| {
            bare_let(statement).is_none_or(|name| names.iter().all(|taken| taken != name))
        });
        index = index + 1 - (length - statements.len());
    }
}

/// The declaration pattern binding what `target` assigns, where it assigns
/// only plain names without defaults or computed keys
fn binding_of<'a>(
    ast: &AstBuilder<'a>,
    target: &AssignmentTarget<'a>,
    names: &mut std::vec::Vec<String>,
) -> Option<BindingPattern<'a>> {
    Some(match target {
        AssignmentTarget::AssignmentTargetIdentifier(id) => {
            names.push(id.name.to_string());
            BindingPattern::new_binding_identifier(SPAN, id.name, ast)
        }
        AssignmentTarget::ArrayAssignmentTarget(array) => {
            let mut elements = Vec::new_in(ast);
            for element in &array.elements {
                elements.push(match element {
                    Some(element) => Some(binding_of(ast, element.as_assignment_target()?, names)?),
                    None => None,
                });
            }
            let rest = match &array.rest {
                Some(rest) => Some(BindingRestElement::boxed(
                    SPAN,
                    binding_of(ast, &rest.target, names)?,
                    ast,
                )),
                None => None,
            };
            BindingPattern::new_array_pattern(SPAN, elements, rest, ast)
        }
        AssignmentTarget::ObjectAssignmentTarget(object) => {
            let mut properties = Vec::new_in(ast);
            for property in &object.properties {
                properties.push(match property {
                    AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(property)
                        if property.init.is_none() =>
                    {
                        names.push(property.binding.name.to_string());
                        BindingProperty::new(
                            SPAN,
                            oxc_ast::ast::PropertyKey::new_static_identifier(
                                SPAN,
                                property.binding.name,
                                ast,
                            ),
                            BindingPattern::new_binding_identifier(
                                SPAN,
                                property.binding.name,
                                ast,
                            ),
                            true,
                            false,
                            ast,
                        )
                    }
                    AssignmentTargetProperty::AssignmentTargetPropertyProperty(property)
                        if !property.computed =>
                    {
                        BindingProperty::new(
                            SPAN,
                            property.name.clone_in(ast.allocator()),
                            binding_of(ast, property.binding.as_assignment_target()?, names)?,
                            false,
                            false,
                            ast,
                        )
                    }
                    _ => return None,
                });
            }
            let rest = match &object.rest {
                Some(rest) => Some(BindingRestElement::boxed(
                    SPAN,
                    binding_of(ast, &rest.target, names)?,
                    ast,
                )),
                None => None,
            };
            BindingPattern::new_object_pattern(SPAN, properties, rest, ast)
        }
        _ => return None,
    })
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
    if let Some(Statement::ReturnStatement(returned)) = statements.last_mut()
        && let Some(argument) = returned.argument.as_mut()
    {
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
            if let Expression::SequenceExpression(sequence) = value
                && let Some(read) = sequence.expressions.pop()
                && let Some(Expression::AssignmentExpression(mut assignment)) =
                    sequence.expressions.pop()
            {
                hoisted.push((pattern, assignment.right.take_in(ast)));
                *value = read;
            }
            true
        });
    }
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
    let slot = |name: &str| slots.contains(name) && uses.writes(name) == 1;
    if assignment.operator != AssignmentOperator::Assign
        || !rests.contains(source.name.as_str())
        || uses.reads(source.name.as_str()) != 1
        || uses.writes(source.name.as_str()) != 0
    {
        return None;
    }
    let target = object_target(&assignment.left)?;
    let rest = rest_name(target)?;
    if rest != read.name
        || !slot(rest.as_str())
        || !target
            .properties
            .iter()
            .all(|property| property_name(property).is_some_and(|(_, id)| slot(id.as_str())))
    {
        return None;
    }
    let mut properties = Vec::new_in(ast);
    for (key, id) in target.properties.iter().filter_map(property_name) {
        properties.push(BindingProperty::new(
            SPAN,
            key.clone_in(ast.allocator()),
            BindingPattern::new_binding_identifier(SPAN, id, ast),
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
            BindingPattern::new_binding_identifier(SPAN, rest, ast),
            ast,
        )),
        ast,
    ))
}

/// The object pattern `target` assigns
fn object_target<'s, 'a>(
    target: &'s AssignmentTarget<'a>,
) -> Option<&'s ObjectAssignmentTarget<'a>> {
    match target {
        AssignmentTarget::ObjectAssignmentTarget(target) => Some(target),
        _ => None,
    }
}

/// The name the rest of the object pattern `target` assigns
fn rest_name<'a>(target: &ObjectAssignmentTarget<'a>) -> Option<Ident<'a>> {
    match &target.rest.as_ref()?.target {
        AssignmentTarget::AssignmentTargetIdentifier(rest) => Some(rest.name),
        _ => None,
    }
}

/// The key of `property` and the name it assigns, where it assigns a name
/// under a key written as is
fn property_name<'s, 'a>(
    property: &'s AssignmentTargetProperty<'a>,
) -> Option<(&'s PropertyKey<'a>, Ident<'a>)> {
    match property {
        AssignmentTargetProperty::AssignmentTargetPropertyProperty(property)
            if !property.computed =>
        {
            match &property.binding {
                AssignmentTargetMaybeDefault::AssignmentTargetIdentifier(id) => {
                    Some((&property.name, id.name))
                }
                _ => None,
            }
        }
        _ => None,
    }
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
        if let Statement::VariableDeclaration(mut declaration) = statements.remove(index)
            && let Statement::VariableDeclaration(next) = &mut statements[index]
        {
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
