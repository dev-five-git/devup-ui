//! What the file binds a style API call to, such as
//! `const fadeIn = keyframes({ ... })`: the build gives it a name, so styles
//! reading the binding read that name

use oxc_allocator::{CloneIn, FromIn, GetAllocator};
use oxc_ast::ast::{Argument, BindingPattern, Expression, Str, TemplateLiteral};
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{VisitMut, walk_mut};
use oxc_semantic::Scoping;
use oxc_span::SPAN;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashMap;

use crate::ExtractStyleValue;

/// The name a style API call gives
#[derive(Clone)]
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
pub struct StyleValues<'a> {
    scoping: Option<Scoping>,
    values: FxHashMap<SymbolId, StyleValue>,
    /// Known imported styles, including top-level constant aliases, by binding
    imported: FxHashMap<String, Vec<ExtractStyleValue>>,
    /// The `const`s the file binds to a string, which a `className` reads as
    /// the classes they hold
    texts: FxHashMap<SymbolId, String>,
    rules: FxHashMap<SymbolId, Expression<'a>>,
    calls: FxHashMap<u32, Expression<'a>>,
}

impl<'a> StyleValues<'a> {
    pub fn new(scoping: Scoping) -> Self {
        Self {
            scoping: Some(scoping),
            values: FxHashMap::default(),
            imported: FxHashMap::default(),
            texts: FxHashMap::default(),
            rules: FxHashMap::default(),
            calls: FxHashMap::default(),
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
            None => {
                let flags = scoping.symbol_flags(symbol);
                (flags.is_import()
                    || (flags.is_const_variable()
                        && scoping.symbol_scope_id(symbol) == scoping.root_scope_id()))
                .then(|| self.imported.get(scoping.symbol_name(symbol)))
                .flatten()
                .map(Vec::as_slice)
            }
        }
    }

    pub fn insert_text(&mut self, symbol: SymbolId, text: String) {
        self.texts.insert(symbol, text);
    }

    /// `expression`, a `className`, reading the strings the constants hold
    pub fn read_in_class(&self, ast: &AstBuilder<'a>, expression: &mut Expression<'a>) {
        if let Some(scoping) = self.scoping.as_ref().filter(|_| !self.texts.is_empty()) {
            ClassReads {
                ast,
                scoping,
                texts: &self.texts,
            }
            .visit_expression(expression);
        }
    }

    /// `expression` reading what the bindings recorded hold
    pub fn read_in(&self, ast: &AstBuilder<'a>, expression: &mut Expression<'a>) {
        if let Some(mut reads) = self.reads(ast, false) {
            reads.visit_expression(expression);
        }
    }

    /// `expression` as class names, which read a class in a template literal
    /// too: there it is a string to join, not CSS text
    pub fn read_in_class_names(&self, ast: &AstBuilder<'a>, expression: &mut Expression<'a>) {
        if let Some(mut reads) = self.reads(ast, true) {
            reads.visit_expression(expression);
        }
    }

    /// The CSS text `template` reading what the bindings recorded hold
    pub fn read_in_text(&self, ast: &AstBuilder<'a>, template: &mut TemplateLiteral<'a>) {
        if let Some(mut reads) = self.reads(ast, false) {
            reads.visit_template_literal(template);
        }
    }

    /// Retain the static source, not the generated class text, for nested reuse.
    pub fn record_rules(
        &mut self,
        ast: &AstBuilder<'a>,
        offset: u32,
        arguments: &[oxc_ast::ast::Argument<'a>],
    ) {
        let mut arguments = oxc_allocator::Vec::from_iter_in(
            arguments
                .iter()
                .map(|argument| argument.clone_in_with_semantic_ids(ast.allocator())),
            ast,
        );
        for argument in &mut arguments {
            let expression = match argument {
                Argument::SpreadElement(spread) => &mut spread.argument,
                argument => argument.to_expression_mut(),
            };
            if let Some(mut reads) = self.reads(ast, false) {
                reads.source = true;
                reads.visit_expression(expression);
            }
        }
        let rules = if crate::utils::reads_directly(&arguments) {
            arguments
                .first()
                .map(|argument| match argument {
                    Argument::SpreadElement(spread) => &spread.argument,
                    argument => argument.to_expression(),
                })
                .map(|rule| rule.clone_in_with_semantic_ids(ast.allocator()))
        } else {
            crate::utils::style_arguments(ast, &arguments)
                .filter(|parts| parts.classes.is_empty())
                .map(|parts| parts.rules)
        };
        if let Some(rules) = rules {
            self.calls.insert(offset, rules);
        }
    }

    pub fn bind_rules(&mut self, ast: &AstBuilder<'a>, symbol: SymbolId, value: &Expression<'a>) {
        let rules = match crate::utils::unwrap_syntax_only(value) {
            Expression::StringLiteral(text) => self.calls.get(&text.span.start),
            Expression::Identifier(identifier) => {
                let source = identifier.reference_id.get().and_then(|reference| {
                    self.scoping.as_ref()?.get_reference(reference).symbol_id()
                });
                if let Some(value) = source.and_then(|source| self.values.get(&source)).cloned() {
                    self.values.insert(symbol, value);
                }
                source.and_then(|source| self.rules.get(&source))
            }
            _ => None,
        };
        if let Some(rules) = rules {
            self.rules
                .insert(symbol, rules.clone_in_with_semantic_ids(ast.allocator()));
        }
    }

    /// `program` with each `name.getName()` on a `keyframes()` binding read as
    /// the name it gives, as styled-components' keyframes object returns it
    pub fn read_names(&self, ast: &AstBuilder<'a>, program: &mut oxc_ast::ast::Program<'a>) {
        let Some(scoping) = self.scoping.as_ref() else {
            return;
        };
        if self
            .values
            .values()
            .any(|value| matches!(value, StyleValue::Keyframes(_)))
        {
            Names {
                ast,
                scoping,
                values: &self.values,
            }
            .visit_program(program);
        }
    }

    fn reads<'s>(&'s self, ast: &'s AstBuilder<'a>, in_rules: bool) -> Option<Reads<'s, 'a>> {
        let scoping = self.scoping.as_ref()?;
        (!self.values.is_empty() || !self.calls.is_empty()).then_some(Reads {
            ast,
            scoping,
            values: &self.values,
            context: if in_rules {
                ReadContext::Rules
            } else {
                ReadContext::Value
            },
            rules: &self.rules,
            calls: &self.calls,
            source: false,
            nested_rules: !in_rules,
        })
    }
}

struct Names<'s, 'a> {
    ast: &'s AstBuilder<'a>,
    scoping: &'s Scoping,
    values: &'s FxHashMap<SymbolId, StyleValue>,
}

impl<'a> VisitMut<'a> for Names<'_, 'a> {
    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        if let Expression::CallExpression(call) = it
            && call.arguments.is_empty()
            && let Expression::StaticMemberExpression(member) = &call.callee
            && member.property.name == "getName"
            && let Expression::Identifier(identifier) = &member.object
            && let Some(StyleValue::Keyframes(name)) = identifier
                .reference_id
                .get()
                .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
                .and_then(|symbol| self.values.get(&symbol))
        {
            *it = Expression::new_string_literal(
                SPAN,
                Str::from_in(name.as_str(), self.ast.allocator()),
                None,
                self.ast,
            );
            return;
        }
        walk_mut::walk_expression(self, it);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ReadContext {
    Value,
    CssText,
    Rules,
    RuleRecord,
}

struct Reads<'s, 'a> {
    ast: &'s AstBuilder<'a>,
    scoping: &'s Scoping,
    values: &'s FxHashMap<SymbolId, StyleValue>,
    context: ReadContext,
    rules: &'s FxHashMap<SymbolId, Expression<'a>>,
    calls: &'s FxHashMap<u32, Expression<'a>>,
    source: bool,
    nested_rules: bool,
}

impl<'a> VisitMut<'a> for Reads<'_, 'a> {
    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        if self.source
            && let Expression::Identifier(identifier) = it
            && let Some(rules) = identifier
                .reference_id
                .get()
                .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
                .and_then(|symbol| self.rules.get(&symbol))
        {
            *it = rules.clone_in_with_semantic_ids(self.ast.allocator());
            return;
        }
        if self.source
            && let Expression::StringLiteral(text) = it
            && let Some(rules) = self.calls.get(&text.span.start)
        {
            *it = rules.clone_in_with_semantic_ids(self.ast.allocator());
            return;
        }
        if let Expression::Identifier(identifier) = it
            && let Some(value) = identifier
                .reference_id
                .get()
                .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
                .and_then(|symbol| self.values.get(&symbol))
        {
            let value = match value {
                StyleValue::Class(..) if self.context == ReadContext::CssText => return,
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
        let context = match self.context {
            ReadContext::Value | ReadContext::CssText => ReadContext::CssText,
            ReadContext::Rules | ReadContext::RuleRecord => ReadContext::Rules,
        };
        let outer = std::mem::replace(&mut self.context, context);
        walk_mut::walk_template_literal(self, it);
        self.context = outer;
    }

    /// A rule object holds strings, not CSS text: a template in its keys or
    /// values reads a class as the text it is
    fn visit_object_expression(&mut self, it: &mut oxc_ast::ast::ObjectExpression<'a>) {
        let context = match self.context {
            ReadContext::RuleRecord => ReadContext::RuleRecord,
            ReadContext::Value | ReadContext::CssText | ReadContext::Rules => ReadContext::Rules,
        };
        let outer = (
            std::mem::replace(&mut self.context, context),
            std::mem::replace(&mut self.source, false),
        );
        walk_mut::walk_object_expression(self, it);
        (self.context, self.source) = outer;
    }

    fn visit_object_property(&mut self, it: &mut oxc_ast::ast::ObjectProperty<'a>) {
        let key = it.key.static_name();
        let nested = self.nested_rules
            && (self.context == ReadContext::RuleRecord
                || key.as_deref().is_some_and(|key| {
                    key.contains('&') || key.starts_with([':', '_', '@']) || key == "selectors"
                }));
        if !nested {
            walk_mut::walk_object_property(self, it);
            return;
        }
        if let Expression::Identifier(identifier) = &it.value
            && let Some(symbol) = identifier
                .reference_id
                .get()
                .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
            && matches!(self.values.get(&symbol), Some(StyleValue::Class(..)))
            && !self.rules.contains_key(&symbol)
        {
            return;
        }
        let outer = std::mem::replace(&mut self.source, true);
        let context = std::mem::replace(
            &mut self.context,
            if key.as_deref().is_some_and(|key| {
                key == "selectors"
                    || key == "@layer"
                    || crate::extractor::extract_style_from_expression::at_rule_record_kind(key)
                        .is_some()
            }) {
                ReadContext::RuleRecord
            } else {
                ReadContext::Rules
            },
        );
        self.visit_expression(&mut it.value);
        self.context = context;
        self.source = outer;
        self.visit_property_key(&mut it.key);
    }
}

struct ClassReads<'s, 'a> {
    ast: &'s AstBuilder<'a>,
    scoping: &'s Scoping,
    texts: &'s FxHashMap<SymbolId, String>,
}

impl<'a> VisitMut<'a> for ClassReads<'_, 'a> {
    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        if let Expression::Identifier(identifier) = it
            && let Some(text) = identifier
                .reference_id
                .get()
                .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
                .and_then(|symbol| self.texts.get(&symbol))
        {
            *it = Expression::new_string_literal(
                SPAN,
                Str::from_in(text.as_str(), self.ast.allocator()),
                None,
                self.ast,
            );
            return;
        }
        walk_mut::walk_expression(self, it);
    }
}
