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
use rustc_hash::FxHashMap;

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
}

#[derive(Default)]
pub struct StyleValues {
    scoping: Option<Scoping>,
    values: FxHashMap<SymbolId, StyleValue>,
    /// The styles behind `css()` classes the file imports, by binding
    imported: FxHashMap<String, Vec<ExtractStyleValue>>,
}

impl StyleValues {
    pub fn new(scoping: Scoping) -> Self {
        Self {
            scoping: Some(scoping),
            values: FxHashMap::default(),
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

    pub fn insert(&mut self, symbol: SymbolId, value: StyleValue) {
        self.values.insert(symbol, value);
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
        (!self.values.is_empty()).then_some(Reads {
            ast,
            scoping,
            values: &self.values,
            in_text: false,
            in_rules: false,
        })
    }
}

struct Reads<'s, 'a> {
    ast: &'s AstBuilder<'a>,
    scoping: &'s Scoping,
    values: &'s FxHashMap<SymbolId, StyleValue>,
    in_text: bool,
    in_rules: bool,
}

impl<'a> VisitMut<'a> for Reads<'_, 'a> {
    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        if let Expression::Identifier(identifier) = it
            && let Some(value) = identifier
                .reference_id
                .get()
                .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
                .and_then(|symbol| self.values.get(&symbol))
        {
            let value = match value {
                StyleValue::Class(..) if self.in_text => return,
                StyleValue::Class(value, _) | StyleValue::Keyframes(value) => value,
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
