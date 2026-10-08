//! What the file binds a style API call to, such as
//! `const fadeIn = keyframes({ ... })`: the build gives it a name, so styles
//! reading the binding read that name

use oxc_allocator::{FromIn, GetAllocator};
use oxc_ast::ast::{BindingPattern, Expression, Str, TemplateLiteral};
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{VisitMut, walk_mut};
use oxc_semantic::Scoping;
use oxc_span::SPAN;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::ExtractStyleValue;

/// The name a style API call gives
pub enum StyleValue {
    /// A class `css()` gives: in CSS text it is a mixin, composed rather than
    /// written as text, so it is read only outside CSS text. The styles behind
    /// it, when the build knows them, let a later style composed with it
    /// replace its declarations.
    Class(String, Option<Vec<ExtractStyleValue>>),
    /// A name `keyframes()` gives
    Keyframes(String),
    /// The selector of a styled component other styles select, read in CSS
    /// text and rule keys alike
    Component(String),
}

#[derive(Default)]
pub struct StyleValues {
    scoping: Option<Scoping>,
    values: FxHashMap<SymbolId, StyleValue>,
    members: FxHashMap<SymbolId, FxHashMap<String, StyleValue>>,
    /// The styles behind `css()` classes the file imports, by binding
    imported: FxHashMap<String, Vec<ExtractStyleValue>>,
}

impl StyleValues {
    pub fn new(scoping: Scoping) -> Self {
        Self {
            scoping: Some(scoping),
            values: FxHashMap::default(),
            members: FxHashMap::default(),
            imported: FxHashMap::default(),
        }
    }

    pub fn import(&mut self, imported: FxHashMap<String, Vec<ExtractStyleValue>>) {
        self.imported = imported;
    }

    /// The `const` `id` binds, whose value no code changes
    pub fn constant(&self, id: &BindingPattern<'_>) -> Option<SymbolId> {
        let symbol = id.get_binding_identifier()?.symbol_id.get()?;
        self.scoping
            .as_ref()?
            .symbol_flags(symbol)
            .is_const_variable()
            .then_some(symbol)
    }

    /// The name the import `symbol` binds
    pub fn import_name(&self, symbol: SymbolId) -> Option<&str> {
        let scoping = self.scoping.as_ref()?;
        scoping
            .symbol_flags(symbol)
            .is_import()
            .then(|| scoping.symbol_name(symbol))
    }

    /// The top-level binding `name` names
    pub fn root_symbol(&self, name: &str) -> Option<SymbolId> {
        self.scoping.as_ref()?.get_root_binding(name.into())
    }

    /// Whether `symbol` is declared below the top level of the module, where
    /// the constants styles read are not
    pub fn is_local(&self, symbol: SymbolId) -> bool {
        self.scoping
            .as_ref()
            .is_some_and(|scoping| scoping.symbol_scope_id(symbol) != scoping.root_scope_id())
    }

    pub fn insert(&mut self, symbol: SymbolId, value: StyleValue) {
        self.values.insert(symbol, value);
    }

    /// The selector of a member of an imported namespace, keyed by its binding.
    pub fn insert_member(&mut self, symbol: SymbolId, member: &str, selector: String) {
        self.members
            .entry(symbol)
            .or_default()
            .insert(member.to_string(), StyleValue::Component(selector));
    }

    /// The binding at the root of a JSX element's name.
    pub fn jsx_symbol(&self, name: &oxc_ast::ast::JSXElementName<'_>) -> Option<SymbolId> {
        match name {
            oxc_ast::ast::JSXElementName::IdentifierReference(reference) => {
                self.reference_symbol(reference)
            }
            oxc_ast::ast::JSXElementName::MemberExpression(member) => match &member.object {
                oxc_ast::ast::JSXMemberExpressionObject::IdentifierReference(reference) => {
                    self.reference_symbol(reference)
                }
                _ => None,
            },
            _ => None,
        }
    }

    pub fn remove(&mut self, symbol: SymbolId) {
        self.values.remove(&symbol);
    }

    /// The binding `expression` reads, when it reads one
    pub fn symbol(&self, expression: &Expression<'_>) -> Option<SymbolId> {
        let Expression::Identifier(identifier) = expression else {
            return None;
        };
        self.scoping
            .as_ref()?
            .get_reference(identifier.reference_id.get()?)
            .symbol_id()
    }

    /// The binding the JSX element name `identifier` reads
    pub fn reference_symbol(
        &self,
        identifier: &oxc_ast::ast::IdentifierReference<'_>,
    ) -> Option<SymbolId> {
        self.scoping
            .as_ref()?
            .get_reference(identifier.reference_id.get()?)
            .symbol_id()
    }

    /// The styles behind the `css()` class `expression` reads
    pub fn styles(&self, expression: &Expression<'_>) -> Option<&[ExtractStyleValue]> {
        let Expression::Identifier(identifier) = expression else {
            return None;
        };
        let symbol = self
            .scoping
            .as_ref()?
            .get_reference(identifier.reference_id.get()?)
            .symbol_id()?;
        let scoping = self.scoping.as_ref()?;
        match self.values.get(&symbol) {
            Some(StyleValue::Class(_, Some(styles))) => Some(styles),
            Some(_) => None,
            None => scoping
                .symbol_flags(symbol)
                .is_import()
                .then(|| self.imported.get(scoping.symbol_name(symbol)))
                .flatten()
                .map(Vec::as_slice),
        }
    }

    /// `expression` reading what the bindings recorded hold
    pub fn read_in<'a>(&self, ast: &AstBuilder<'a>, expression: &mut Expression<'a>) {
        if let Some(mut reads) = self.reads(ast) {
            reads.visit_expression(expression);
        }
    }

    /// The CSS text `template` reading what the bindings recorded hold
    pub fn read_in_text<'a>(&self, ast: &AstBuilder<'a>, template: &mut TemplateLiteral<'a>) {
        if let Some(mut reads) = self.reads(ast) {
            reads.visit_template_literal(template);
        }
    }

    fn reads<'s, 'a>(&'s self, ast: &'s AstBuilder<'a>) -> Option<Reads<'s, 'a>> {
        let scoping = self.scoping.as_ref()?;
        (!self.values.is_empty() || !self.members.is_empty()).then_some(Reads {
            ast,
            scoping,
            values: &self.values,
            members: &self.members,
            in_text: false,
            in_rules: false,
            in_key: false,
        })
    }
}

/// The bindings styles read as selectors: interpolated in CSS text where a
/// value cannot stand, or as the computed key of a rule object
pub fn selected(program: &oxc_ast::ast::Program<'_>, values: &StyleValues) -> FxHashSet<SymbolId> {
    let mut symbols = FxHashSet::default();
    selectors(program, &mut |expression| {
        symbols.extend(values.symbol(expression));
    });
    symbols
}

/// Visit the expressions styles read as selectors: interpolated in CSS text
/// where a value cannot stand, or as the computed key of a rule object
pub fn selectors<'a>(program: &oxc_ast::ast::Program<'a>, found: &mut dyn FnMut(&Expression<'a>)) {
    oxc_ast_visit::Visit::visit_program(&mut Selected { found }, program);
}

struct Selected<'s, 'a> {
    found: &'s mut dyn FnMut(&Expression<'a>),
}

impl<'a> oxc_ast_visit::Visit<'a> for Selected<'_, 'a> {
    fn visit_tagged_template_expression(
        &mut self,
        it: &oxc_ast::ast::TaggedTemplateExpression<'a>,
    ) {
        let quasi = &it.quasi;
        let mut text = String::new();
        for (index, expression) in quasi.expressions.iter().enumerate() {
            text.push_str(quasi.quasis[index].value.raw.as_str());
            if matches!(
                crate::css_utils::interpolation_place(&text, &quasi.quasis[index + 1..]),
                crate::css_utils::Place::Other
            ) {
                (self.found)(expression);
            }
        }
        oxc_ast_visit::walk::walk_tagged_template_expression(self, it);
    }

    fn visit_object_property(&mut self, it: &oxc_ast::ast::ObjectProperty<'a>) {
        if it.computed
            && let Some(key) = it.key.as_expression()
        {
            (self.found)(key);
            if let Expression::TemplateLiteral(template) = crate::utils::unwrap_syntax_only(key) {
                for expression in &template.expressions {
                    (self.found)(expression);
                }
            }
        }
        oxc_ast_visit::walk::walk_object_property(self, it);
    }
}

struct Reads<'s, 'a> {
    ast: &'s AstBuilder<'a>,
    scoping: &'s Scoping,
    values: &'s FxHashMap<SymbolId, StyleValue>,
    members: &'s FxHashMap<SymbolId, FxHashMap<String, StyleValue>>,
    in_text: bool,
    in_rules: bool,
    /// Inside the key of a rule, which a component is read in as its selector
    in_key: bool,
}

/// A namespace member whose property name is written as an exact string.
pub fn component_member<'e, 'a>(
    expression: &'e Expression<'a>,
) -> Option<(&'e Expression<'a>, &'e str)> {
    match crate::utils::unwrap_syntax_only(expression) {
        Expression::StaticMemberExpression(member) => {
            Some((&member.object, member.property.name.as_str()))
        }
        Expression::ComputedMemberExpression(member) => {
            match crate::utils::unwrap_syntax_only(&member.expression) {
                Expression::StringLiteral(property) => {
                    Some((&member.object, property.value.as_str()))
                }
                _ => None,
            }
        }
        _ => None,
    }
}

impl Reads<'_, '_> {
    fn value(&self, expression: &Expression<'_>) -> Option<&StyleValue> {
        let (identifier, member) = match crate::utils::unwrap_syntax_only(expression) {
            Expression::Identifier(identifier) => (identifier, None),
            expression => {
                let (object, member) = component_member(expression)?;
                match crate::utils::unwrap_syntax_only(object) {
                    Expression::Identifier(identifier) => (identifier, Some(member)),
                    _ => return None,
                }
            }
        };
        let symbol = self
            .scoping
            .get_reference(identifier.reference_id.get()?)
            .symbol_id()?;
        match member {
            Some(member) => self.members.get(&symbol)?.get(member),
            None => self.values.get(&symbol),
        }
    }
}

impl<'a> VisitMut<'a> for Reads<'_, 'a> {
    fn visit_property_key(&mut self, it: &mut oxc_ast::ast::PropertyKey<'a>) {
        // A component standing for the whole key selects it within the rule
        if let Some(expression) = it.as_expression()
            && let Some(StyleValue::Component(selector)) = self.value(expression)
        {
            *it = oxc_ast::ast::PropertyKey::StringLiteral(oxc_ast::ast::StringLiteral::boxed(
                SPAN,
                Str::from_in(format!("& {selector}").as_str(), self.ast.allocator()),
                None,
                self.ast,
            ));
            return;
        }
        let outer = std::mem::replace(&mut self.in_key, true);
        walk_mut::walk_property_key(self, it);
        self.in_key = outer;
    }

    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        if let Some(value) = self.value(it) {
            let value = match value {
                StyleValue::Class(..) if self.in_text => return,
                // Elsewhere it is the component itself
                StyleValue::Component(_) if !self.in_text && !self.in_key => return,
                StyleValue::Class(value, _)
                | StyleValue::Keyframes(value)
                | StyleValue::Component(value) => value,
            };
            *it = Expression::new_string_literal(
                SPAN,
                Str::from_in(value.as_str(), self.ast.allocator()),
                None,
                self.ast,
            );
            return;
        }
        walk_mut::walk_expression(self, it);
    }

    fn visit_template_literal(&mut self, it: &mut TemplateLiteral<'a>) {
        let outer = std::mem::replace(&mut self.in_text, !self.in_rules);
        walk_mut::walk_template_literal(self, it);
        self.in_text = outer;
    }

    /// A rule object holds strings, not CSS text: a template in its keys or
    /// values reads a class as the text it is
    fn visit_object_expression(&mut self, it: &mut oxc_ast::ast::ObjectExpression<'a>) {
        let outer = (
            std::mem::replace(&mut self.in_rules, true),
            std::mem::replace(&mut self.in_text, false),
        );
        walk_mut::walk_object_expression(self, it);
        (self.in_rules, self.in_text) = outer;
    }
}
