//! The members of the objects the build writes and reads again: a generated
//! constant bound to an object literal no code changes holds what the literal
//! wrote, so reading a member of it reads that value.

use oxc_allocator::{CloneIn, GetAllocator, Vec};
use oxc_ast::{
    ast::{
        Expression, IdentifierReference, ObjectPropertyKind, Program, PropertyKind,
        SimpleAssignmentTarget, Statement, StaticMemberExpression, UnaryOperator,
        VariableDeclarationKind,
    },
    builder::AstBuilder,
};
use oxc_ast_visit::{Visit, walk};
use oxc_semantic::Scoping;
use oxc_span::SPAN;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use super::{generated, unwrapped};

#[path = "peephole_sink.rs"]
mod sink;

/// The members every object takes from `Object.prototype`
const INHERITED: [&str; 12] = [
    "constructor",
    "hasOwnProperty",
    "isPrototypeOf",
    "propertyIsEnumerable",
    "toLocaleString",
    "toString",
    "valueOf",
    "__proto__",
    "__defineGetter__",
    "__defineSetter__",
    "__lookupGetter__",
    "__lookupSetter__",
];

/// What a property of a known object holds
enum Value<'a> {
    /// A value that reads nothing, written again wherever it is read
    Literal(Expression<'a>),
    /// An object literal of such values, spread again wherever it is spread
    Object(Expression<'a>),
    Other,
}

enum Entry<'a> {
    /// A property under its key, or under a key the build cannot read
    Property(Option<String>, Value<'a>),
    /// What a spread copies: a known object, or anything else
    Spread(Option<SymbolId>),
}

/// What reading a member of a known object reads
enum Found<'s, 'a> {
    Literal(&'s Expression<'a>),
    Object(&'s Expression<'a>),
    /// No property: what `Object.prototype` gives
    Absent,
    /// What the object copied from the known object, or a value the build
    /// cannot read
    Copied(Option<SymbolId>),
}

/// Whether `value` reads nothing and gives the same value written again
fn literal(value: &Expression<'_>) -> bool {
    match unwrapped(value) {
        Expression::StringLiteral(_)
        | Expression::NumericLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NullLiteral(_) => true,
        Expression::TemplateLiteral(template) => template.expressions.is_empty(),
        Expression::UnaryExpression(unary) => {
            unary.operator == UnaryOperator::Void && literal(&unary.argument)
        }
        _ => false,
    }
}

/// The key a property is written under, when it is one the build reads
fn key(property: &oxc_ast::ast::ObjectProperty<'_>) -> Option<String> {
    match &property.key {
        oxc_ast::ast::PropertyKey::StringLiteral(text) => text.value.as_str().map(str::to_string),
        _ if property.computed => None,
        key => key.static_name()?.as_str().map(str::to_string),
    }
}

/// Whether `value` is an object literal of literals under keys the build reads
fn literal_object(value: &Expression<'_>) -> bool {
    matches!(unwrapped(value), Expression::ObjectExpression(object)
        if object.properties.iter().all(|property| matches!(property,
            ObjectPropertyKind::ObjectProperty(property)
                if property.kind == PropertyKind::Init
                    && !property.method
                    && key(property).is_some_and(|key| key != "__proto__")
                    && (literal(&property.value) || literal_object(&property.value)))))
}

/// How often each generated binding is read, and how often only for a
/// member or to be spread, which changes nothing it holds
struct Reads<'s> {
    scoping: &'s Scoping,
    all: FxHashMap<SymbolId, usize>,
    plain: FxHashMap<SymbolId, usize>,
}

impl Reads<'_> {
    fn symbol(&self, value: &Expression<'_>) -> Option<SymbolId> {
        let Expression::Identifier(id) = unwrapped(value) else {
            return None;
        };
        self.scoping
            .get_reference(id.reference_id.get()?)
            .symbol_id()
    }

    fn plain(&mut self, value: &Expression<'_>) {
        if let Some(symbol) = self.symbol(value) {
            *self.plain.entry(symbol).or_default() += 1;
        }
    }

    /// Counts a read of `value` once more, so it never reads only plainly
    fn changes(&mut self, value: &Expression<'_>) {
        if let Some(symbol) = self.symbol(value) {
            *self.all.entry(symbol).or_default() += 1;
        }
    }
}

impl<'a> Visit<'a> for Reads<'_> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if let Some(symbol) = it
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
        {
            *self.all.entry(symbol).or_default() += 1;
        }
    }

    fn visit_expression(&mut self, it: &Expression<'a>) {
        match it {
            Expression::StaticMemberExpression(member) => self.plain(&member.object),
            // `delete` changes what it reads.
            Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::Delete => {
                if let Expression::StaticMemberExpression(member) = unwrapped(&unary.argument) {
                    self.changes(&member.object);
                }
            }
            // A method called on it may change it.
            Expression::CallExpression(call) => {
                if let Expression::StaticMemberExpression(member) = unwrapped(&call.callee) {
                    self.changes(&member.object);
                }
            }
            _ => {}
        }
        walk::walk_expression(self, it);
    }

    fn visit_spread_element(&mut self, it: &oxc_ast::ast::SpreadElement<'a>) {
        self.plain(&it.argument);
        walk::walk_spread_element(self, it);
    }

    fn visit_jsx_spread_attribute(&mut self, it: &oxc_ast::ast::JSXSpreadAttribute<'a>) {
        self.plain(&it.argument);
        walk::walk_jsx_spread_attribute(self, it);
    }

    fn visit_simple_assignment_target(&mut self, it: &SimpleAssignmentTarget<'a>) {
        if let SimpleAssignmentTarget::StaticMemberExpression(member) = it {
            self.changes(&member.object);
        }
        walk::walk_simple_assignment_target(self, it);
    }
}

/// How often the statements read `symbol`
struct Count<'k, 'a> {
    known: &'k Known<'a>,
    symbol: SymbolId,
    reads: usize,
}

impl<'a> Visit<'a> for Count<'_, 'a> {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if it
            .reference_id
            .get()
            .and_then(|reference| self.known.scoping.get_reference(reference).symbol_id())
            == Some(self.symbol)
        {
            self.reads += 1;
        }
    }
}

/// Puts `value` where an object spreads `symbol`
struct Place<'k, 'a> {
    known: &'k Known<'a>,
    symbol: SymbolId,
    value: Option<Expression<'a>>,
}

impl<'a> oxc_ast_visit::VisitMut<'a> for Place<'_, 'a> {
    fn visit_spread_element(&mut self, it: &mut oxc_ast::ast::SpreadElement<'a>) {
        if self.known.symbol(&it.argument) == Some(self.symbol)
            && let Some(value) = self.value.take()
        {
            it.argument = value;
        }
        oxc_ast_visit::walk_mut::walk_spread_element(self, it);
    }
}

pub(super) struct Known<'a> {
    scoping: Scoping,
    /// The generated bindings read only for members or to be spread
    unchanged: FxHashSet<SymbolId>,
    records: FxHashMap<SymbolId, std::vec::Vec<Entry<'a>>>,
    order: sink::Order,
}

impl<'a> Known<'a> {
    pub(super) fn new(program: &Program<'a>) -> Self {
        let scoping = oxc_semantic::SemanticBuilder::new()
            .build(program)
            .semantic
            .into_scoping();
        let mut reads = Reads {
            scoping: &scoping,
            all: FxHashMap::default(),
            plain: FxHashMap::default(),
        };
        reads.visit_program(program);
        let unchanged = reads
            .all
            .iter()
            .filter(|(symbol, count)| {
                generated(scoping.symbol_name(**symbol))
                    && !scoping.symbol_is_mutated(**symbol)
                    && reads.plain.get(*symbol) == Some(*count)
            })
            .map(|(symbol, _)| *symbol)
            .collect();
        let order = sink::Order::of(program, &scoping);
        Self {
            order,
            scoping,
            unchanged,
            records: FxHashMap::default(),
        }
    }

    fn symbol(&self, value: &Expression<'_>) -> Option<SymbolId> {
        let Expression::Identifier(id) = unwrapped(value) else {
            return None;
        };
        self.scoping
            .get_reference(id.reference_id.get()?)
            .symbol_id()
    }

    /// Whether `value` reads the global `undefined`, which no code assigns
    fn undefined(&self, value: &Expression<'_>) -> bool {
        matches!(unwrapped(value), Expression::Identifier(id)
        if id.name == "undefined"
            && id.reference_id.get().is_some_and(|reference| {
                self.scoping.get_reference(reference).symbol_id().is_none()
            }))
    }

    /// Keeps what `statement` binds when it binds a generated constant no code
    /// changes to an object literal
    pub(super) fn register(&mut self, ast: &AstBuilder<'a>, statement: &Statement<'a>) {
        let Statement::VariableDeclaration(declaration) = statement else {
            return;
        };
        if declaration.kind != VariableDeclarationKind::Const {
            return;
        }
        for declarator in &declaration.declarations {
            if let Some(symbol) = declarator
                .id
                .get_binding_identifier()
                .and_then(|id| id.symbol_id.get())
                .filter(|symbol| self.unchanged.contains(symbol))
                && let Some(Expression::ObjectExpression(object)) =
                    declarator.init.as_ref().map(unwrapped)
                && let Some(record) = self.record(ast, &object.properties)
            {
                self.records.insert(symbol, record);
            }
        }
    }

    fn record(
        &self,
        ast: &AstBuilder<'a>,
        properties: &Vec<'a, ObjectPropertyKind<'a>>,
    ) -> Option<std::vec::Vec<Entry<'a>>> {
        properties
            .iter()
            .map(|property| match property {
                ObjectPropertyKind::SpreadProperty(spread) => Some(Entry::Spread(
                    self.symbol(&spread.argument)
                        .filter(|symbol| self.records.contains_key(symbol)),
                )),
                ObjectPropertyKind::ObjectProperty(property) => {
                    let key = key(property);
                    // `__proto__` written so sets what the object inherits.
                    if key.as_deref() == Some("__proto__") && !property.computed {
                        return None;
                    }
                    let value = if property.kind != PropertyKind::Init || property.method {
                        Value::Other
                    } else if literal(&property.value) || self.undefined(&property.value) {
                        Value::Literal(property.value.clone_in(ast.allocator()))
                    } else if literal_object(&property.value) {
                        Value::Object(property.value.clone_in(ast.allocator()))
                    } else {
                        Value::Other
                    };
                    Some(Entry::Property(key, value))
                }
            })
            .collect()
    }

    fn find(&self, symbol: SymbolId, key: &str) -> Option<Found<'_, 'a>> {
        for entry in self.records.get(&symbol)?.iter().rev() {
            match entry {
                Entry::Property(Some(written), value) if written == key => {
                    return Some(match value {
                        Value::Literal(value) => Found::Literal(value),
                        Value::Object(value) => Found::Object(value),
                        Value::Other => Found::Copied(None),
                    });
                }
                Entry::Property(Some(_), _) => {}
                Entry::Property(None, _) | Entry::Spread(None) => {
                    return Some(Found::Copied(None));
                }
                Entry::Spread(Some(source)) => match self.find(*source, key)? {
                    Found::Absent => {}
                    Found::Copied(None) => return Some(Found::Copied(Some(*source))),
                    found => return Some(found),
                },
            }
        }
        Some(Found::Absent)
    }

    fn member(&self, member: &StaticMemberExpression<'_>) -> Option<Found<'_, 'a>> {
        match self.find(self.symbol(&member.object)?, &member.property.name)? {
            Found::Absent if INHERITED.contains(&member.property.name.as_str()) => None,
            Found::Copied(None) => None,
            found => Some(found),
        }
    }

    /// Whether `source` is the only binding of its name inside its scope, so
    /// its name reads it anywhere the object copying it is read
    fn alone(&self, source: SymbolId) -> bool {
        let name = self.scoping.symbol_name(source);
        let scope = self.scoping.symbol_scope_id(source);
        self.scoping.symbol_ids().all(|other| {
            other == source
                || self.scoping.symbol_name(other) != name
                || !self
                    .scoping
                    .scope_ancestors(self.scoping.symbol_scope_id(other))
                    .any(|ancestor| ancestor == scope)
        })
    }

    /// Whether evaluating `properties` reads nothing that may change: literals,
    /// and copies of known objects
    fn settled(&self, properties: &Vec<'a, ObjectPropertyKind<'a>>) -> bool {
        properties.iter().all(|property| match property {
            ObjectPropertyKind::SpreadProperty(spread) => self
                .symbol(&spread.argument)
                .is_some_and(|symbol| self.records.contains_key(&symbol)),
            ObjectPropertyKind::ObjectProperty(property) => {
                property.kind == PropertyKind::Init
                    && !property.method
                    && key(property).is_some_and(|key| key != "__proto__")
                    && self.reads_nothing(&property.value)
            }
        })
    }

    /// Whether `value` reads nothing that may change: a literal, or a
    /// member of a known object, which holds what it copied
    fn reads_nothing(&self, value: &Expression<'_>) -> bool {
        literal(value)
            || self.undefined(value)
            || literal_object(value)
            || matches!(unwrapped(value), Expression::StaticMemberExpression(member)
                if self.symbol(&member.object).is_some_and(|symbol| self.records.contains_key(&symbol)))
    }

    /// `const { a, ...rest } = { a: 1, ...b, a: c }` as `{ ...b, a: c }`:
    /// the pattern reads `a` by its name wherever the object holds it, and the
    /// last value written for it is the one it holds
    pub(super) fn drop_overridden(&self, statement: &mut Statement<'a>) {
        let Statement::VariableDeclaration(declaration) = statement else {
            return;
        };
        for declarator in &mut declaration.declarations {
            let oxc_ast::ast::BindingPattern::ObjectPattern(pattern) = &declarator.id else {
                continue;
            };
            let Some(Expression::ObjectExpression(object)) =
                declarator.init.as_mut().map(super::unwrapped_mut)
            else {
                continue;
            };
            let named = pattern
                .properties
                .iter()
                .filter(|property| !property.computed)
                .filter_map(|property| property.key.static_name()?.as_str().map(str::to_string))
                .collect::<FxHashSet<_>>();
            let keys = object
                .properties
                .iter()
                .map(|property| match property {
                    ObjectPropertyKind::ObjectProperty(property)
                        if property.kind == PropertyKind::Init && !property.method =>
                    {
                        key(property).filter(|key| named.contains(key) && key != "__proto__")
                    }
                    _ => None,
                })
                .collect::<std::vec::Vec<_>>();
            let mut index = 0;
            object.properties.retain(|property| {
                let key = &keys[index];
                index += 1;
                let overridden = key.as_ref().is_some_and(|key| {
                    keys[index..]
                        .iter()
                        .any(|later| later.as_ref() == Some(key))
                });
                !(overridden
                    && matches!(property, ObjectPropertyKind::ObjectProperty(property)
                        if self.reads_nothing(&property.value)))
            });
        }
    }

    /// `const __devupA = { ... }; ... { ...__devupA }` as `{ ...{ ... } }`
    /// where nothing else reads it: building the object reads nothing that
    /// may change, so it builds the same where it is spread
    pub(super) fn inline_objects(&self, statements: &mut Vec<'a, Statement<'a>>) -> bool {
        let mut inlined = false;
        let mut index = 0;
        while index < statements.len() {
            let symbol = match &statements[index] {
                Statement::VariableDeclaration(declaration)
                    if declaration.kind == VariableDeclarationKind::Const =>
                {
                    match declaration.declarations.as_slice() {
                        [declarator] => declarator
                            .id
                            .get_binding_identifier()
                            .and_then(|id| id.symbol_id.get())
                            .filter(|symbol| {
                                self.unchanged.contains(symbol)
                                    && matches!(declarator.init.as_ref().map(unwrapped),
                                        Some(Expression::ObjectExpression(object))
                                            if self.settled(&object.properties))
                            }),
                        _ => None,
                    }
                }
                _ => None,
            };
            // What is read only to be spread, the statements now read once.
            if let Some(symbol) = symbol
                && {
                    let mut count = Count {
                        known: self,
                        symbol,
                        reads: 0,
                    };
                    for statement in statements.iter() {
                        count.visit_statement(statement);
                    }
                    count.reads == 1
                }
                && let (head, rest) = statements.split_at_mut(index + 1)
                && let Statement::VariableDeclaration(declaration) = &mut head[index]
                && let Some(value) = declaration.declarations[0].init.take()
            {
                let mut place = Place {
                    known: self,
                    symbol,
                    value: Some(value),
                };
                for statement in rest {
                    oxc_ast_visit::VisitMut::visit_statement(&mut place, statement);
                }
                // Nothing read it where the statements reach.
                if let Some(value) = place.value {
                    declaration.declarations[0].init = Some(value);
                    index += 1;
                } else {
                    statements.remove(index);
                    inlined = true;
                }
            } else {
                index += 1;
            }
        }
        inlined
    }

    /// `member` written as the value it reads, when the build knows it
    pub(super) fn read(
        &self,
        ast: &AstBuilder<'a>,
        member: &StaticMemberExpression<'a>,
    ) -> Option<Expression<'a>> {
        match self.member(member)? {
            Found::Literal(value) => Some(value.clone_in(ast.allocator())),
            Found::Absent => Some(Expression::new_unary_expression(
                SPAN,
                UnaryOperator::Void,
                Expression::new_numeric_literal(
                    SPAN,
                    0.0,
                    None,
                    oxc_syntax::number::NumberBase::Decimal,
                    ast,
                ),
                ast,
            )),
            Found::Copied(Some(source)) if self.alone(source) => {
                let name = IdentifierReference::new(
                    SPAN,
                    ast.allocator().alloc_str(self.scoping.symbol_name(source)),
                    ast,
                );
                name.reference_id.set(
                    self.scoping
                        .get_resolved_reference_ids(source)
                        .first()
                        .copied(),
                );
                Some(Expression::new_static_member_expression(
                    SPAN,
                    Expression::Identifier(oxc_allocator::Box::new_in(name, ast)),
                    member.property.clone_in(ast.allocator()),
                    false,
                    ast,
                ))
            }
            _ => None,
        }
    }

    /// The object literal spreading `member` copies
    pub(super) fn spread(
        &self,
        ast: &AstBuilder<'a>,
        member: &StaticMemberExpression<'a>,
    ) -> Option<Expression<'a>> {
        match self.member(member)? {
            Found::Object(value) => Some(value.clone_in(ast.allocator())),
            _ => None,
        }
    }
}
