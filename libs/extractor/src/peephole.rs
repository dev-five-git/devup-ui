//! What the build writes in a longer form than it means: a check whose answer
//! is written beside it, a fallback never reached, a value wrapped to be
//! unwrapped again. Each is written as the shorter code it means, which runs
//! the same steps in the same order.

use oxc_allocator::{CloneIn, GetAllocator, TakeIn, Vec};
use oxc_ast::{
    ast::{
        Argument, ArrayExpressionElement, BindingPattern, CallExpression, Expression,
        JSXAttributeItem, JSXAttributeName, JSXAttributeValue, JSXOpeningElement, LogicalOperator,
        ObjectPropertyKind, Program, PropertyKind, TemplateElementValue, TemplateLiteral,
        UnaryOperator, VariableDeclarationKind,
    },
    builder::AstBuilder,
};
use oxc_ast_visit::{Visit, VisitMut, walk, walk_mut};
use oxc_span::{ContentEq, SPAN};
use rustc_hash::{FxHashMap, FxHashSet};

/// Write what `program` writes in a longer form than it means as the code it means
pub(crate) fn simplify<'a>(ast: &AstBuilder<'a>, program: &mut Program<'a>) {
    let mut literals = Literals::default();
    literals.visit_program(program);
    if !literals.alias.is_empty() {
        inline_aliases(ast, program);
        literals = Literals::default();
        literals.visit_program(program);
    }
    Simplify {
        ast,
        guards: literals.guards(),
        declared: literals.bound.keys().cloned().collect(),
        objects: literals.objects(),
    }
    .visit_program(program);
}

/// Whether `name` is one the build gives what it declares, which no code of
/// the module assigns
fn generated(name: &str) -> bool {
    name.starts_with("__devup")
}

fn unwrapped<'s, 'a>(mut value: &'s Expression<'a>) -> &'s Expression<'a> {
    while let Expression::ParenthesizedExpression(inner) = value {
        value = &inner.expression;
    }
    value
}

fn unwrapped_mut<'s, 'a>(mut value: &'s mut Expression<'a>) -> &'s mut Expression<'a> {
    while let Expression::ParenthesizedExpression(inner) = value {
        value = &mut inner.expression;
    }
    value
}

/// The generated names bound only to an object or array the build writes,
/// which no code assigns: never `null` nor `undefined`
#[derive(Default)]
struct Literals {
    bound: FxHashMap<String, usize>,
    literal: FxHashMap<String, usize>,
    written: FxHashSet<String>,
    /// The generated constants declared as another generated name
    alias: FxHashMap<String, String>,
    /// The generated constants declared as `guard ? value : undefined`,
    /// with that guard
    guard: FxHashMap<String, String>,
}

impl Literals {
    /// Whether `name` is bound once and never assigned
    fn constant(&self, name: &str) -> bool {
        self.bound.get(name) == Some(&1) && !self.written.contains(name)
    }

    fn objects(&self) -> FxHashSet<String> {
        self.literal
            .iter()
            .filter(|(name, count)| {
                self.bound.get(*name) == Some(*count) && !self.written.contains(*name)
            })
            .map(|(name, _)| name.clone())
            .collect()
    }

    fn guards(&self) -> FxHashMap<String, String> {
        self.guard
            .iter()
            .filter(|(name, guard)| self.constant(name) && self.constant(guard))
            .map(|(name, guard)| (name.clone(), guard.clone()))
            .collect()
    }

    fn literal(&mut self, pattern: &BindingPattern<'_>, value: Option<&Expression<'_>>) {
        if let BindingPattern::BindingIdentifier(id) = pattern
            && generated(&id.name)
            && value.is_some_and(|value| {
                matches!(
                    unwrapped(value),
                    Expression::ObjectExpression(_) | Expression::ArrayExpression(_)
                )
            })
        {
            *self.literal.entry(id.name.to_string()).or_default() += 1;
        }
    }
}

impl<'a> Visit<'a> for Literals {
    fn visit_binding_identifier(&mut self, it: &oxc_ast::ast::BindingIdentifier<'a>) {
        *self.bound.entry(it.name.to_string()).or_default() += 1;
    }

    fn visit_variable_declaration(&mut self, it: &oxc_ast::ast::VariableDeclaration<'a>) {
        if it.kind == VariableDeclarationKind::Const {
            for declarator in &it.declarations {
                self.literal(&declarator.id, declarator.init.as_ref());
                if let BindingPattern::BindingIdentifier(id) = &declarator.id
                    && generated(&id.name)
                    && let Some(init) = &declarator.init
                {
                    match unwrapped(init) {
                        Expression::Identifier(target) if generated(&target.name) => {
                            self.alias
                                .insert(id.name.to_string(), target.name.to_string());
                        }
                        Expression::ConditionalExpression(conditional)
                            if is_undefined(&conditional.alternate) =>
                        {
                            if let Some(guard) = pure_name(&conditional.test) {
                                self.guard.insert(id.name.to_string(), guard.to_string());
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        walk::walk_variable_declaration(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if let Expression::ArrowFunctionExpression(arrow) = unwrapped(&it.callee)
            && arrow.span == SPAN
            && arrow.params.rest.is_none()
            && !it.optional
        {
            for (parameter, argument) in arrow.params.items.iter().zip(&it.arguments) {
                if parameter.initializer.is_none() {
                    self.literal(&parameter.pattern, argument.as_expression());
                }
            }
        }
        walk::walk_call_expression(self, it);
    }

    fn visit_simple_assignment_target(&mut self, it: &oxc_ast::ast::SimpleAssignmentTarget<'a>) {
        if let oxc_ast::ast::SimpleAssignmentTarget::AssignmentTargetIdentifier(id) = it {
            self.written.insert(id.name.to_string());
        }
        walk::walk_simple_assignment_target(self, it);
    }

    fn visit_assignment_target_property_identifier(
        &mut self,
        it: &oxc_ast::ast::AssignmentTargetPropertyIdentifier<'a>,
    ) {
        self.written.insert(it.binding.name.to_string());
        walk::walk_assignment_target_property_identifier(self, it);
    }
}

/// Read each generated constant declared as another as the one it holds,
/// where that name reads it, and drop its declaration
fn inline_aliases<'a>(ast: &AstBuilder<'a>, program: &mut Program<'a>) {
    let scoping = oxc_semantic::SemanticBuilder::new()
        .build(program)
        .semantic
        .into_scoping();
    let mut declared = Declared::default();
    declared.visit_program(program);
    let mut targets = FxHashMap::default();
    for (alias, target) in declared.aliases {
        if let Some(symbol) = scoping.get_reference(target).symbol_id()
            && !scoping.symbol_is_mutated(alias)
            && !scoping.symbol_is_mutated(symbol)
        {
            targets.insert(alias, symbol);
        }
    }
    let mut renames = FxHashMap::default();
    let mut removed = FxHashSet::default();
    for alias in targets.keys() {
        let mut target = targets[alias];
        while let Some(next) = targets.get(&target) {
            target = *next;
        }
        let name: &'a str = ast.allocator().alloc_str(scoping.symbol_name(target));
        let references = scoping.get_resolved_reference_ids(*alias);
        if references.iter().all(|reference| {
            scoping.find_binding(scoping.get_reference(*reference).scope_id(), name.into())
                == Some(target)
        }) {
            for reference in references {
                renames.insert(*reference, name);
            }
            removed.insert(*alias);
        }
    }
    if !removed.is_empty() {
        Alias { renames, removed }.visit_program(program);
    }
}

/// The generated constants declared as another generated name
#[derive(Default)]
struct Declared {
    aliases: std::vec::Vec<(oxc_semantic::SymbolId, oxc_semantic::ReferenceId)>,
}

impl<'a> Visit<'a> for Declared {
    fn visit_variable_declaration(&mut self, it: &oxc_ast::ast::VariableDeclaration<'a>) {
        if it.kind == VariableDeclarationKind::Const {
            for declarator in &it.declarations {
                if let BindingPattern::BindingIdentifier(id) = &declarator.id
                    && generated(&id.name)
                    && let Some(symbol) = id.symbol_id.get()
                    && let Some(Expression::Identifier(target)) =
                        declarator.init.as_ref().map(unwrapped)
                    && generated(&target.name)
                    && let Some(reference) = target.reference_id.get()
                {
                    self.aliases.push((symbol, reference));
                }
            }
        }
        walk::walk_variable_declaration(self, it);
    }
}

/// Read each alias as the name it holds
struct Alias<'a> {
    renames: FxHashMap<oxc_semantic::ReferenceId, &'a str>,
    removed: FxHashSet<oxc_semantic::SymbolId>,
}

impl<'a> Alias<'a> {
    fn renamed(&self, it: &oxc_ast::ast::IdentifierReference<'a>) -> bool {
        it.reference_id
            .get()
            .is_some_and(|reference| self.renames.contains_key(&reference))
    }
}

impl<'a> VisitMut<'a> for Alias<'a> {
    fn visit_identifier_reference(&mut self, it: &mut oxc_ast::ast::IdentifierReference<'a>) {
        if let Some(target) = it
            .reference_id
            .get()
            .and_then(|reference| self.renames.get(&reference))
        {
            it.name = (*target).into();
        }
    }

    fn visit_object_property(&mut self, it: &mut oxc_ast::ast::ObjectProperty<'a>) {
        if it.shorthand && matches!(&it.value, Expression::Identifier(id) if self.renamed(id)) {
            it.shorthand = false;
        }
        walk_mut::walk_object_property(self, it);
    }

    fn visit_statements(&mut self, it: &mut Vec<'a, oxc_ast::ast::Statement<'a>>) {
        it.retain_mut(|statement| {
            let oxc_ast::ast::Statement::VariableDeclaration(declaration) = statement else {
                return true;
            };
            declaration.declarations.retain(|declarator| {
                !matches!(&declarator.id, BindingPattern::BindingIdentifier(id)
                    if id.symbol_id.get().is_some_and(|symbol| self.removed.contains(&symbol)))
            });
            !declaration.declarations.is_empty()
        });
        walk_mut::walk_statements(self, it);
    }
}

struct Simplify<'s, 'a> {
    ast: &'s AstBuilder<'a>,
    /// The generated constants holding `undefined` unless their guard holds
    guards: FxHashMap<String, String>,
    /// Every name the module declares
    declared: FxHashSet<String>,
    /// The generated names holding an object or array
    objects: FxHashSet<String>,
}

/// Whether `value` is truthy whatever it reads
fn truthy(value: &Expression<'_>) -> bool {
    match unwrapped(value) {
        Expression::StringLiteral(text) => !text.value.is_empty(),
        Expression::TemplateLiteral(template) => template.quasis.iter().any(|quasi| {
            quasi
                .value
                .cooked
                .as_ref()
                .is_some_and(|text| !text.is_empty())
        }),
        Expression::NumericLiteral(number) => number.value != 0.0 && !number.value.is_nan(),
        Expression::BooleanLiteral(boolean) => boolean.value,
        Expression::ObjectExpression(_)
        | Expression::ArrayExpression(_)
        | Expression::ArrowFunctionExpression(_)
        | Expression::FunctionExpression(_)
        | Expression::ClassExpression(_) => true,
        // Text joined to a nonempty string is a nonempty string.
        Expression::BinaryExpression(binary)
            if binary.operator == oxc_syntax::operator::BinaryOperator::Addition =>
        {
            [&binary.left, &binary.right].into_iter().any(|side| {
                matches!(
                    unwrapped(side),
                    Expression::StringLiteral(_) | Expression::TemplateLiteral(_)
                ) && truthy(side)
            })
        }
        Expression::ConditionalExpression(conditional) => {
            truthy(&conditional.consequent) && truthy(&conditional.alternate)
        }
        Expression::LogicalExpression(logical) => match logical.operator {
            LogicalOperator::Or => truthy(&logical.right),
            // A left side given but falsy, such as `0`, is what it gives.
            LogicalOperator::Coalesce => truthy(&logical.left),
            LogicalOperator::And => truthy(&logical.left) && truthy(&logical.right),
        },
        _ => false,
    }
}

/// Whether `value` is never `null` nor `undefined`
fn given(value: &Expression<'_>) -> bool {
    truthy(value)
        || match unwrapped(value) {
            Expression::StringLiteral(_)
            | Expression::TemplateLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::BinaryExpression(_) => true,
            Expression::UnaryExpression(unary) => unary.operator != UnaryOperator::Void,
            _ => false,
        }
}

/// Whether `value` is `undefined` by `void` of a literal
fn is_void(value: &Expression<'_>) -> bool {
    matches!(unwrapped(value), Expression::UnaryExpression(unary)
        if unary.operator == UnaryOperator::Void
            && matches!(unwrapped(&unary.argument), Expression::NumericLiteral(_) | Expression::StringLiteral(_)))
}

/// Whether `value` is `undefined` or `false` written as such
fn is_undefined(value: &Expression<'_>) -> bool {
    is_void(value)
        || matches!(unwrapped(value), Expression::Identifier(id) if id.name == "undefined")
        || matches!(unwrapped(value), Expression::BooleanLiteral(b) if !b.value)
}

/// What a condition of literals is, which reading runs no code
fn known_test(value: &Expression<'_>) -> Option<bool> {
    match unwrapped(value) {
        Expression::BooleanLiteral(boolean) => Some(boolean.value),
        Expression::NumericLiteral(number) => Some(number.value != 0.0 && !number.value.is_nan()),
        Expression::StringLiteral(text) => Some(!text.value.is_empty()),
        Expression::NullLiteral(_) => Some(false),
        value if is_void(value) => Some(false),
        _ => None,
    }
}

/// Whether reading `value` runs no code nor throws: a literal, a generated
/// name, or literals compared or combined
fn inert(value: &Expression<'_>) -> bool {
    let literal = |value: &Expression<'_>| {
        matches!(
            unwrapped(value),
            Expression::StringLiteral(_)
                | Expression::NumericLiteral(_)
                | Expression::BooleanLiteral(_)
                | Expression::NullLiteral(_)
        )
    };
    match unwrapped(value) {
        Expression::BinaryExpression(binary) => {
            !matches!(
                binary.operator,
                oxc_syntax::operator::BinaryOperator::In
                    | oxc_syntax::operator::BinaryOperator::Instanceof
            ) && literal(&binary.left)
                && literal(&binary.right)
        }
        value => literal(value) || pure_name(value).is_some(),
    }
}

/// The generated name `value` reads, which reading runs no code
fn pure_name<'s>(value: &'s Expression<'_>) -> Option<&'s str> {
    match unwrapped(value) {
        Expression::Identifier(id) if generated(&id.name) => Some(id.name.as_str()),
        _ => None,
    }
}

/// The text `[...].filter(Boolean).join(separator)` gives of strings
fn joined(call: &CallExpression<'_>) -> Option<String> {
    let (parts, separator) = join_parts(call)?;
    let parts = parts
        .iter()
        .map(|part| match part {
            ArrayExpressionElement::StringLiteral(text) => text.value.as_str(),
            _ => None,
        })
        .collect::<Option<std::vec::Vec<_>>>()?;
    Some(
        parts
            .into_iter()
            .filter(|text| !text.is_empty())
            .collect::<std::vec::Vec<_>>()
            .join(separator),
    )
}

/// `[...texts, name, ...texts].filter(Boolean).join(separator)` as
/// `name ? `texts name texts` : "texts texts"`, reading `name` once more to
/// test it as `filter` does
fn joined_around<'a>(
    ast: &AstBuilder<'a>,
    call: &CallExpression<'a>,
    declared: &FxHashSet<String>,
) -> Option<Expression<'a>> {
    let (parts, separator) = join_parts(call)?;
    let mut name = None;
    let (mut before, mut after) = (std::vec::Vec::new(), std::vec::Vec::new());
    for part in parts {
        match part {
            ArrayExpressionElement::Identifier(id)
                if name.is_none()
                    && (generated(&id.name) || declared.contains(id.name.as_str())) =>
            {
                name = Some(id);
            }
            ArrayExpressionElement::StringLiteral(text) => {
                let text = text.value.as_str()?;
                if !text.is_empty() {
                    if name.is_some() {
                        &mut after
                    } else {
                        &mut before
                    }
                    .push(text);
                }
            }
            _ => return None,
        }
    }
    let name = name?;
    // Texts a template reads as they are written
    if std::iter::once(&separator)
        .chain(&before)
        .chain(&after)
        .any(|text| text.contains(['\\', '`', '$']))
    {
        return None;
    }
    let (before, after) = (before.join(separator), after.join(separator));
    let quasis = [
        if before.is_empty() {
            before.clone()
        } else {
            format!("{before}{separator}")
        },
        if after.is_empty() {
            after.clone()
        } else {
            format!("{separator}{after}")
        },
    ];
    let template = Expression::new_template_literal(
        SPAN,
        Vec::from_iter_in(
            quasis.iter().enumerate().map(|(index, text)| {
                oxc_ast::ast::TemplateElement::new(
                    SPAN,
                    crate::utils::template_element_value(text, ast),
                    index == 1,
                    ast,
                )
            }),
            ast,
        ),
        Vec::from_array_in(
            [Expression::Identifier(name.clone_in(ast.allocator()))],
            ast,
        ),
        ast,
    );
    let rest = [before, after]
        .into_iter()
        .filter(|text| !text.is_empty())
        .collect::<std::vec::Vec<_>>()
        .join(separator);
    Some(Expression::new_conditional_expression(
        SPAN,
        Expression::Identifier(name.clone_in(ast.allocator())),
        template,
        Expression::new_string_literal(SPAN, ast.allocator().alloc_str(&rest), None, ast),
        ast,
    ))
}

/// The elements `[...].filter(Boolean).join(separator)` joins, and the separator
fn join_parts<'s, 'a>(
    call: &'s CallExpression<'a>,
) -> Option<(&'s [ArrayExpressionElement<'a>], &'s str)> {
    let Expression::StaticMemberExpression(join) = unwrapped(&call.callee) else {
        return None;
    };
    let [Argument::StringLiteral(separator)] = call.arguments.as_slice() else {
        return None;
    };
    let Expression::CallExpression(filter) = unwrapped(&join.object) else {
        return None;
    };
    let Expression::StaticMemberExpression(member) = unwrapped(&filter.callee) else {
        return None;
    };
    let [Argument::Identifier(boolean)] = filter.arguments.as_slice() else {
        return None;
    };
    let Expression::ArrayExpression(array) = unwrapped(&member.object) else {
        return None;
    };
    if join.property.name != "join"
        || member.property.name != "filter"
        || boolean.name != "Boolean"
        || call.optional
        || filter.optional
    {
        return None;
    }
    Some((array.elements.as_slice(), separator.value.as_str()?))
}

/// Whether `key` can be written as the name of a JSX attribute
fn attribute_name(key: &str) -> bool {
    key != "key"
        && key.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_' || c == '$')
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$' || c == '-')
}

/// Whether `text` reads the same written between the quotes of a JSX
/// attribute, which takes no escapes and reads character references
fn quotable(text: &oxc_ast::ast::StringLiteral<'_>) -> bool {
    text.value.as_str().is_some_and(|text| {
        text.chars()
            .all(|c| (' '..='~').contains(&c) && !matches!(c, '"' | '&' | '\\'))
    })
}

/// Whether spreading `argument` as props passes what writing each of its
/// properties as a prop passes: an object written in place without `key`,
/// which React reads apart
fn spreads_props(argument: &Expression<'_>) -> bool {
    matches!(unwrapped(argument), Expression::ObjectExpression(object) if spreads_props_of(object))
}

fn spreads_props_of(object: &oxc_ast::ast::ObjectExpression<'_>) -> bool {
    plain_properties(&object.properties)
        && object.properties.iter().all(|property| match property {
            ObjectPropertyKind::SpreadProperty(_) => true,
            ObjectPropertyKind::ObjectProperty(property) => {
                !property.computed
                    && property
                        .key
                        .static_name()
                        .and_then(|key| key.as_str().map(attribute_name))
                        == Some(true)
            }
        })
}

/// Whether the properties of an object literal define what spreading it
/// copies: plain values under plain keys
fn plain_properties(properties: &[ObjectPropertyKind<'_>]) -> bool {
    properties.iter().all(|property| match property {
        ObjectPropertyKind::SpreadProperty(_) => true,
        ObjectPropertyKind::ObjectProperty(property) => {
            property.kind == PropertyKind::Init
                && !property.method
                && (property.computed
                    || property
                        .key
                        .static_name()
                        .is_none_or(|key| key.as_str() != Some("__proto__")))
        }
    })
}

impl<'a> Simplify<'_, 'a> {
    fn once(&self, it: &mut Expression<'a>) -> bool {
        match it {
            // A name called alone binds no `this`, as calling it through a
            // sequence does not either; `eval` read so is not the direct one.
            Expression::CallExpression(call) if !call.optional => {
                if let Expression::SequenceExpression(sequence) = unwrapped(&call.callee)
                    && let [Expression::NumericLiteral(_), Expression::Identifier(id)] =
                        sequence.expressions.as_slice()
                    && id.name != "eval"
                {
                    let id = Expression::Identifier(id.clone_in(self.ast.allocator()));
                    call.callee = id;
                    return true;
                }
                if let Some(text) = joined(call) {
                    let text: &'a str = self.ast.allocator().alloc_str(&text);
                    *it = Expression::new_string_literal(SPAN, text, None, self.ast);
                    return true;
                }
                if let Some(value) = joined_around(self.ast, call, &self.declared) {
                    *it = value;
                    return true;
                }
                false
            }
            Expression::LogicalExpression(logical) => {
                let left = match logical.operator {
                    LogicalOperator::Or => truthy(&logical.left),
                    LogicalOperator::Coalesce => given(&logical.left) || is_void(&logical.right),
                    LogicalOperator::And => false,
                };
                if left {
                    *it = logical.left.take_in(self.ast);
                }
                left
            }
            Expression::ConditionalExpression(conditional) => {
                let test = unwrapped_mut(&mut conditional.test);
                // A condition is read only for whether it holds.
                if self.condition(test) {
                    return true;
                }
                if let Some(holds) = known_test(&conditional.test) {
                    *it = if holds {
                        conditional.consequent.take_in(self.ast)
                    } else {
                        conditional.alternate.take_in(self.ast)
                    };
                    return true;
                }
                // `typeof` reads a declared name as the condition does.
                if let Expression::Identifier(id) = unwrapped(&conditional.test)
                    && self.declared.contains(id.name.as_str())
                    && matches!(unwrapped(&conditional.alternate), Expression::StringLiteral(text) if text.value.is_empty())
                    && let Expression::ConditionalExpression(inner) =
                        unwrapped(&conditional.consequent)
                    && matches!(unwrapped(&inner.alternate), Expression::StringLiteral(text) if text.value.is_empty())
                    && matches!(unwrapped(&inner.consequent), Expression::Identifier(value) if value.name == id.name)
                    && matches!(unwrapped(&inner.test), Expression::BinaryExpression(binary)
                        if binary.operator == oxc_syntax::operator::BinaryOperator::StrictEquality
                            && matches!(unwrapped(&binary.left), Expression::UnaryExpression(unary)
                                if unary.operator == UnaryOperator::Typeof
                                    && matches!(unwrapped(&unary.argument), Expression::Identifier(read) if read.name == id.name))
                            && matches!(unwrapped(&binary.right), Expression::StringLiteral(kind) if kind.value.as_str() == Some("string")))
                {
                    *it = conditional.consequent.take_in(self.ast);
                    return true;
                }
                let Some(name) = pure_name(&conditional.test).map(str::to_string) else {
                    return false;
                };
                if conditional.consequent.content_eq(&conditional.alternate) {
                    *it = conditional.consequent.take_in(self.ast);
                    return true;
                }
                // The same name read again holds what it held.
                let conditional = &mut **conditional;
                let mut changed = false;
                for (branch, holds) in [
                    (&mut conditional.consequent, true),
                    (&mut conditional.alternate, false),
                ] {
                    if let Expression::ConditionalExpression(inner) = unwrapped_mut(branch)
                        && pure_name(&inner.test) == Some(name.as_str())
                    {
                        *branch = if holds {
                            inner.consequent.take_in(self.ast)
                        } else {
                            inner.alternate.take_in(self.ast)
                        };
                        changed = true;
                    }
                }
                changed
            }
            Expression::TemplateLiteral(template) => self.splice(template),
            // A value read only to be dropped, which reading runs no code of
            // A member or a name read alone binds `this`, calls `eval` directly
            // or reads a name `typeof` would not throw on, and a function
            // assigned alone takes the name it is assigned to: those stay read so.
            Expression::SequenceExpression(sequence)
                if inert(&sequence.expressions[0])
                    && (sequence.expressions.len() > 2
                        || !matches!(
                            unwrapped(&sequence.expressions[1]),
                            Expression::Identifier(_)
                                | Expression::StaticMemberExpression(_)
                                | Expression::ComputedMemberExpression(_)
                                | Expression::PrivateFieldExpression(_)
                                | Expression::ChainExpression(_)
                                | Expression::FunctionExpression(_)
                                | Expression::ArrowFunctionExpression(_)
                                | Expression::ClassExpression(_)
                        )) =>
            {
                sequence.expressions.remove(0);
                if sequence.expressions.len() == 1
                    && let Some(last) = sequence.expressions.pop()
                {
                    *it = last;
                }
                true
            }
            Expression::ObjectExpression(object) => {
                let flat = object.properties.iter().any(|property| {
                    matches!(property, ObjectPropertyKind::SpreadProperty(spread)
                        if matches!(unwrapped(&spread.argument), Expression::ObjectExpression(inner)
                            if plain_properties(&inner.properties)))
                });
                if flat {
                    let properties = object.properties.take_in(self.ast);
                    let mut flattened = Vec::new_in(self.ast);
                    for mut property in properties {
                        if let ObjectPropertyKind::SpreadProperty(spread) = &mut property
                            && let Expression::ObjectExpression(inner) =
                                unwrapped_mut(&mut spread.argument)
                            && plain_properties(&inner.properties)
                        {
                            flattened.extend(inner.properties.take_in(self.ast));
                        } else {
                            flattened.push(property);
                        }
                    }
                    object.properties = flattened;
                }
                flat
            }
            _ => false,
        }
    }

    /// Whether `object` is a name holding an object, never `null` nor `undefined`
    fn object(&self, object: &Expression<'_>) -> bool {
        matches!(unwrapped(object), Expression::Identifier(id)
            if self.objects.contains(id.name.as_str()))
    }

    /// Write `test`, read only for whether it holds, as the shortest
    /// condition that holds when it does
    fn condition(&self, test: &mut Expression<'a>) -> bool {
        let test = unwrapped_mut(test);
        let changed = match test {
            Expression::ConditionalExpression(inner) => {
                match (known_test(&inner.consequent), known_test(&inner.alternate)) {
                    (Some(true), Some(false)) => {
                        *test = inner.test.take_in(self.ast);
                        true
                    }
                    (Some(false), Some(true)) => {
                        let value = inner.test.take_in(self.ast);
                        *test = Expression::new_unary_expression(
                            SPAN,
                            UnaryOperator::LogicalNot,
                            value,
                            self.ast,
                        );
                        true
                    }
                    (None, Some(false)) => {
                        let left = inner.test.take_in(self.ast);
                        let right = inner.consequent.take_in(self.ast);
                        *test = Expression::new_logical_expression(
                            SPAN,
                            left,
                            LogicalOperator::And,
                            right,
                            self.ast,
                        );
                        true
                    }
                    _ => {
                        self.condition(&mut inner.consequent) | self.condition(&mut inner.alternate)
                    }
                }
            }
            Expression::UnaryExpression(outer) if outer.operator == UnaryOperator::LogicalNot => {
                if let Expression::UnaryExpression(inner) = unwrapped_mut(&mut outer.argument)
                    && inner.operator == UnaryOperator::LogicalNot
                {
                    *test = inner.argument.take_in(self.ast);
                    true
                } else {
                    self.condition(&mut outer.argument)
                }
            }
            Expression::LogicalExpression(logical) if logical.operator == LogicalOperator::And => {
                // What holds only when its guard does needs no guard read first.
                if let (Some(guard), Some(value)) =
                    (pure_name(&logical.left), pure_name(&logical.right))
                    && self.guards.get(value).map(String::as_str) == Some(guard)
                {
                    *test = logical.right.take_in(self.ast);
                    true
                } else {
                    self.condition(&mut logical.left) | self.condition(&mut logical.right)
                }
            }
            Expression::LogicalExpression(logical) if logical.operator == LogicalOperator::Or => {
                self.condition(&mut logical.left) | self.condition(&mut logical.right)
            }
            _ => false,
        };
        if changed {
            self.condition(test);
        }
        changed
    }

    /// Write each template a template holds as a value as part of its text
    fn splice(&self, template: &mut TemplateLiteral<'a>) -> bool {
        let Some((index, inner)) =
            template
                .expressions
                .iter_mut()
                .enumerate()
                .find_map(|(index, expression)| match unwrapped_mut(expression) {
                    Expression::TemplateLiteral(inner)
                        if inner
                            .quasis
                            .iter()
                            .all(|quasi| quasi.value.cooked.is_some()) =>
                    {
                        Some((index, inner))
                    }
                    _ => None,
                })
        else {
            return false;
        };
        let mut quasis = inner.quasis.take_in(self.ast);
        let expressions = inner.expressions.take_in(self.ast);
        // A template holds one more text than values, around each of them.
        let before = template.quasis[index].value.clone_in(self.ast.allocator());
        let after = template.quasis[index + 1]
            .value
            .clone_in(self.ast.allocator());
        let join = |left: &str, right: &str| -> &'a str {
            self.ast.allocator().alloc_str(&format!("{left}{right}"))
        };
        let text = |value: &TemplateElementValue<'_>| {
            value
                .cooked
                .as_ref()
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string()
        };
        let last = quasis.len() - 1;
        quasis[0].value.raw = join(before.raw.as_str(), quasis[0].value.raw.as_str()).into();
        quasis[0].value.cooked = Some(join(&text(&before), &text(&quasis[0].value)).into());
        quasis[last].value.raw = join(quasis[last].value.raw.as_str(), after.raw.as_str()).into();
        quasis[last].value.cooked = Some(join(&text(&quasis[last].value), &text(&after)).into());
        quasis[last].tail = template.quasis[index + 1].tail;
        let mut spliced_quasis = Vec::new_in(self.ast);
        for (at, quasi) in template.quasis.take_in(self.ast).into_iter().enumerate() {
            if at == index {
                spliced_quasis.extend(quasis.take_in(self.ast));
            } else if at != index + 1 {
                spliced_quasis.push(quasi);
            }
        }
        let mut spliced_expressions = Vec::new_in(self.ast);
        let mut expressions = Some(expressions);
        for (at, expression) in template
            .expressions
            .take_in(self.ast)
            .into_iter()
            .enumerate()
        {
            if at == index {
                spliced_expressions.extend(expressions.take().into_iter().flatten());
            } else {
                spliced_expressions.push(expression);
            }
        }
        template.quasis = spliced_quasis;
        template.expressions = spliced_expressions;
        true
    }
}

impl<'a> VisitMut<'a> for Simplify<'_, 'a> {
    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        walk_mut::walk_expression(self, it);
        while self.once(it) {}
    }

    fn visit_static_member_expression(
        &mut self,
        it: &mut oxc_ast::ast::StaticMemberExpression<'a>,
    ) {
        if it.optional && self.object(&it.object) {
            it.optional = false;
        }
        walk_mut::walk_static_member_expression(self, it);
    }

    fn visit_computed_member_expression(
        &mut self,
        it: &mut oxc_ast::ast::ComputedMemberExpression<'a>,
    ) {
        if it.optional && self.object(&it.object) {
            it.optional = false;
        }
        walk_mut::walk_computed_member_expression(self, it);
    }

    fn visit_array_expression_element(&mut self, it: &mut ArrayExpressionElement<'a>) {
        // An element takes any value but a sequence without parentheses.
        if let Some(value) = it.as_expression_mut()
            && let Expression::ParenthesizedExpression(inner) = value
            && !matches!(
                unwrapped(&inner.expression),
                Expression::SequenceExpression(_)
            )
        {
            *value = unwrapped_mut(value).take_in(self.ast);
        }
        walk_mut::walk_array_expression_element(self, it);
    }

    fn visit_tagged_template_expression(
        &mut self,
        it: &mut oxc_ast::ast::TaggedTemplateExpression<'a>,
    ) {
        // A tag reads the parts of its template as written.
        self.visit_expression(&mut it.tag);
        for expression in &mut it.quasi.expressions {
            self.visit_expression(expression);
        }
    }

    fn visit_jsx_opening_element(&mut self, it: &mut JSXOpeningElement<'a>) {
        walk_mut::walk_jsx_opening_element(self, it);
        let spreads_literal = |item: &JSXAttributeItem<'_>| matches!(item, JSXAttributeItem::SpreadAttribute(spread) if spreads_props(&spread.argument));
        if !it.attributes.iter().any(spreads_literal) {
            return;
        }
        let mut attributes = Vec::new_in(self.ast);
        for mut item in it.attributes.take_in(self.ast) {
            let JSXAttributeItem::SpreadAttribute(spread) = &mut item else {
                attributes.push(item);
                continue;
            };
            let Expression::ObjectExpression(object) = unwrapped_mut(&mut spread.argument) else {
                attributes.push(item);
                continue;
            };
            if !spreads_props_of(object) {
                attributes.push(item);
                continue;
            }
            for property in object.properties.take_in(self.ast) {
                match property {
                    ObjectPropertyKind::SpreadProperty(mut inner) => {
                        attributes.push(JSXAttributeItem::new_spread_attribute(
                            SPAN,
                            inner.argument.take_in(self.ast),
                            self.ast,
                        ));
                    }
                    ObjectPropertyKind::ObjectProperty(mut property) => {
                        let key: &'a str = property
                            .key
                            .static_name()
                            .and_then(|key| {
                                key.as_str().map(|key| self.ast.allocator().alloc_str(key))
                            })
                            .unwrap_or_default();
                        let value = match property.value.take_in(self.ast) {
                            Expression::StringLiteral(text) if quotable(&text) => {
                                JSXAttributeValue::StringLiteral(text)
                            }
                            value => JSXAttributeValue::new_expression_container(
                                SPAN,
                                value.into(),
                                self.ast,
                            ),
                        };
                        attributes.push(JSXAttributeItem::new_attribute(
                            SPAN,
                            JSXAttributeName::new_identifier(SPAN, key, self.ast),
                            Some(value),
                            self.ast,
                        ));
                    }
                }
            }
        }
        it.attributes = attributes;
    }
}

#[cfg(test)]
#[path = "peephole_tests.rs"]
mod tests;
