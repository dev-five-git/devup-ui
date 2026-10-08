//! Imported constants inlined where styles read them, so a value another
//! module declares with `const` becomes a static class instead of a CSS
//! variable set at runtime.

use std::cell::{Cell, OnceCell};
use std::collections::BTreeSet;
use std::rc::Rc;

use oxc_allocator::{Allocator, FromIn, GetAllocator};
use oxc_ast::ast::{
    Argument, ArrayExpressionElement, Expression, IdentifierReference, ImportDeclarationSpecifier,
    JSXAttributeItem, JSXElementName, ObjectPropertyKind, Program, Statement, Str,
    VariableDeclarationKind,
};
use oxc_ast::builder::AstBuilder;
use oxc_ast_visit::{Visit, VisitMut, walk, walk_mut};
use oxc_parser::Parser;
use oxc_semantic::{Scoping, SemanticBuilder};
use oxc_span::{GetSpan, SPAN, SourceType, Span};
use oxc_syntax::number::NumberBase;
use oxc_syntax::operator::BinaryOperator;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::composition::{Composition, set_prop_order};
use crate::css_prop::{CssProp, CssTakers, binding_of, reads_top_level, root_reference};
use crate::extractor::ExtractResult;
use crate::extractor::extract_style_from_expression::{
    LiteralHandling, extract_style_from_expression,
};
use crate::stylex::StylexFunction;
use crate::{ExtractOption, ExtractStyleValue, ModuleResolver};

pub(crate) mod consumer;
mod initialization;
mod lexical;
#[cfg(test)]
mod require_tests;
#[cfg(test)]
mod safety_tests;

#[derive(Clone, Debug)]
enum Constant {
    String(String),
    Number(f64),
    Null,
    Bool(bool),
    Undefined,
    /// A function an object literal holds: styles only call it, which only
    /// running the module does
    Function,
    /// The properties known of an object, a module namespace or an enum
    Object(Rc<FxHashMap<String, Constant>>),
    /// An object literal every property of which is known, in source order
    Record(Rc<Vec<(String, Constant)>>),
    Array(Rc<Vec<Constant>>),
    /// `StyleX` custom properties by key, which read as their `var()`
    Vars(Rc<FxHashMap<String, String>>),
    /// The class a `StyleX` theme applies
    Theme(String),
    /// What another style API gives: a class, a component or a keyframes
    /// name, never rules; for a `css()` class, the styles behind it when
    /// they are known
    Style(Option<Rc<Vec<ExtractStyleValue>>>),
    /// An object or array code changes, or a value read from one
    Changed(Rc<Change>),
}

impl Constant {
    /// Whether code can change what it holds
    const fn is_mutable(&self) -> bool {
        matches!(self, Self::Object(_) | Self::Record(_) | Self::Array(_))
    }

    /// Whether this value or one it holds is a function
    fn has_function(&self) -> bool {
        match self {
            Self::Function => true,
            Self::Object(object) => object.values().any(Self::has_function),
            Self::Record(entries) => entries.iter().any(|(_, value)| value.has_function()),
            Self::Array(values) => values.iter().any(Self::has_function),
            _ => false,
        }
    }

    /// This value as JavaScript source, when every part of it is known
    fn js_literal(&self) -> Option<String> {
        match self {
            Self::String(text) => serde_json::to_string(text).ok(),
            Self::Number(number) => Some(if number.to_bits() == (-0.0_f64).to_bits() {
                "-0".to_string()
            } else {
                crate::utils::js_number_string(*number)
            }),
            Self::Null => Some("null".to_string()),
            Self::Bool(value) => Some(value.to_string()),
            Self::Undefined => Some("undefined".to_string()),
            Self::Record(entries) => {
                let mut properties = Vec::with_capacity(entries.len());
                for (key, value) in entries.iter() {
                    properties.push(format!(
                        "{}: {}",
                        serde_json::to_string(key).ok()?,
                        value.js_literal()?
                    ));
                }
                Some(format!("({{ {} }})", properties.join(", ")))
            }
            Self::Array(values) => {
                let values: Option<Vec<String>> = values.iter().map(Self::js_literal).collect();
                Some(format!("[{}]", values?.join(", ")))
            }
            _ => None,
        }
    }

    /// Where code changes this value or one it holds
    fn change(&self) -> Option<Rc<Change>> {
        match self {
            Self::Changed(change) => Some(change.clone()),
            Self::Object(object) => object.values().find_map(Self::change),
            Self::Record(entries) => entries.iter().find_map(|(_, value)| value.change()),
            Self::Array(values) => values.iter().find_map(Self::change),
            _ => None,
        }
    }

    /// Whether what `path` leads to (`None` for any key) holds nothing code can
    /// change
    fn reaches_only_primitives(&self, path: &[Option<String>]) -> bool {
        match path.split_first() {
            None => !self.is_mutable(),
            Some((Some(key), rest)) => match member_of(self, key) {
                Some(member) => member.reaches_only_primitives(rest),
                // A key the build knows is missing reads `undefined`
                None => !matches!(self, Self::Object(_)),
            },
            Some((None, rest)) => match self {
                Self::Record(entries) => entries
                    .iter()
                    .all(|(_, value)| value.reaches_only_primitives(rest)),
                Self::Array(values) => values
                    .iter()
                    .all(|value| value.reaches_only_primitives(rest)),
                value => !value.is_mutable(),
            },
        }
    }
}

/// Where code changes the object or array a binding holds, or hands it to code
/// that may change it
#[derive(Debug)]
pub(crate) struct Change {
    /// The binding, as the module changing it names it
    pub name: String,
    pub site: ChangeSite,
    /// Handed on rather than changed there
    pub handed: bool,
}

#[derive(Debug)]
pub(crate) enum ChangeSite {
    /// An offset in the file extracted
    Here(u32),
    /// `file:line:column` in a module it imports
    In(String),
}

/// What inlining found: the files read, and the `StyleX` values imported from
/// other modules, by the name the program binds them to
#[derive(Default)]
pub(crate) struct Inlined {
    pub errors: Vec<(u32, String)>,
    pub dependencies: BTreeSet<String>,
    pub atoms: crate::vanilla_extract::producer_atoms::ProducerAtoms,
    pub references: crate::vanilla_extract::style_references::StyleReferences,
    pub stylex_vars: FxHashMap<String, FxHashMap<String, String>>,
    pub stylex_themes: FxHashMap<String, String>,
    /// The styles behind imported `css()` classes
    pub css_styles: FxHashMap<String, Vec<ExtractStyleValue>>,
    pub unknown: Unknown,
    pub changed: Changed,
    /// The semantic analysis of the program as parsed, so the visitor reuses
    /// it: constants inlined later leave the ids of surviving references as
    /// they are. `None` when no analysis was needed
    pub scoping: Option<Rc<Scoping>>,
}

/// Bindings styles read that hold an object or array code changes, whole or
/// in some member
#[derive(Default, Clone)]
pub(crate) struct Changed {
    whole: FxHashMap<String, Rc<Change>>,
    holding: FxHashMap<String, Constant>,
}

impl Changed {
    pub(crate) fn is_empty(&self) -> bool {
        self.whole.is_empty() && self.holding.is_empty()
    }

    /// Whether `expression` (`x`, `x.y` or `x[y]`) reads, whole or in part, an
    /// object or array code changes, counting only the identifiers
    /// `reads_binding` accepts, so a local named like a binding of the module
    /// does not
    pub(crate) fn read_by_in(
        &self,
        expression: &Expression<'_>,
        reads_binding: &dyn Fn(&IdentifierReference<'_>) -> bool,
    ) -> bool {
        let mut path = Vec::new();
        let mut expression = expression;
        let identifier = loop {
            match expression {
                Expression::StaticMemberExpression(member) => {
                    path.push(Some(member.property.name.as_str()));
                    expression = &member.object;
                }
                Expression::ComputedMemberExpression(member) => {
                    path.push(None);
                    expression = &member.object;
                }
                Expression::Identifier(identifier) => break identifier,
                _ => return false,
            }
        };
        let name = identifier.name.as_str();
        if !reads_binding(identifier) {
            return false;
        }
        if self.whole.contains_key(name) {
            return true;
        }
        let Some(mut value) = self.holding.get(name) else {
            return false;
        };
        for key in path.iter().rev() {
            let Some(key) = key else {
                return true;
            };
            match member_value(value, key) {
                Some(member) => value = member,
                None => return false,
            }
        }
        value.change().is_some()
    }

    /// The changes of the bindings `message` names, by the name the module
    /// changing them uses
    pub(crate) fn named_in(&self, message: &str) -> Vec<Rc<Change>> {
        let is_part = |c: char| c.is_alphanumeric() || c == '_' || c == '$';
        let mut changes: Vec<Rc<Change>> = self
            .whole
            .iter()
            .map(|(name, change)| (name, Some(change.clone())))
            .chain(
                self.holding
                    .iter()
                    .map(|(name, value)| (name, value.change())),
            )
            .filter(|(name, _)| {
                message.match_indices(name.as_str()).any(|(index, _)| {
                    !message[..index].ends_with(is_part)
                        && !message[index + name.len()..].starts_with(is_part)
                })
            })
            .filter_map(|(_, change)| change)
            .collect();
        changes.sort_unstable_by(|a, b| a.name.cmp(&b.name));
        changes.dedup_by(|a, b| Rc::ptr_eq(a, b));
        changes
    }
}

/// Module-level bindings styles read whose value only running the module
/// gives, whole or in part
#[derive(Default, Clone)]
pub(crate) struct Unknown {
    names: FxHashSet<String>,
    /// Objects and namespaces the build knows some members of
    partial: FxHashMap<String, Constant>,
}

impl Unknown {
    pub(crate) fn is_empty(&self) -> bool {
        self.names.is_empty() && self.partial.is_empty()
    }

    /// Whether `expression` (`x`, `x.y.z`, `x[y]` or a call of one) reads what
    /// only running the module gives
    pub(crate) fn read_by(&self, expression: &Expression<'_>) -> bool {
        self.read_by_in(expression, &|_| true)
    }

    /// [`Self::read_by`] counting only the identifiers `reads_binding`
    /// accepts, so a local named like a binding of the module does not
    pub(crate) fn read_by_in(
        &self,
        expression: &Expression<'_>,
        reads_binding: &dyn Fn(&IdentifierReference<'_>) -> bool,
    ) -> bool {
        let mut path = Vec::new();
        let mut expression = expression;
        loop {
            match expression {
                Expression::StaticMemberExpression(member) => {
                    path.push(member.property.name.as_str());
                    expression = &member.object;
                }
                Expression::ComputedMemberExpression(member) => {
                    return root_reference(&member.object).is_some_and(|identifier| {
                        reads_binding(identifier)
                            && (self.names.contains(identifier.name.as_str())
                                || self.partial.contains_key(identifier.name.as_str()))
                    });
                }
                Expression::CallExpression(call) => {
                    return self.read_by_in(&call.callee, reads_binding);
                }
                Expression::Identifier(identifier) => {
                    if !reads_binding(identifier) {
                        return false;
                    }
                    let name = identifier.name.as_str();
                    if self.names.contains(name) {
                        return true;
                    }
                    let Some(mut value) = self.partial.get(name) else {
                        return false;
                    };
                    for key in path.iter().rev() {
                        match member_value(value, key) {
                            Some(member) => value = member,
                            None => break,
                        }
                    }
                    return matches!(value, Constant::Object(_)) || value.has_function();
                }
                _ => return false,
            }
        }
    }
}

fn member_value<'c>(value: &'c Constant, key: &str) -> Option<&'c Constant> {
    match value {
        Constant::Object(object) => object.get(key),
        Constant::Record(entries) => entries
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value),
        _ => None,
    }
}

enum Imported {
    Named(String),
    Namespace,
}

/// Inline the primitive constants `program` reads in styles, its own
/// module-level `const`s and those it imports, returning the files read.
/// A program with no style import and no `css` prop is left as it is, with no
/// semantic analysis.
pub(crate) fn inline_constants<'a>(
    ast_builder: &AstBuilder<'a>,
    program: &mut Program<'a>,
    filename: &str,
    option: &ExtractOption,
    resolver: Option<&ModuleResolver>,
    css_prop: CssProp,
) -> Inlined {
    let imports_style_package = program.body.iter().any(|statement| {
        matches!(statement, Statement::ImportDeclaration(import)
            if is_style_package(option, &import.source.value))
    });
    if !imports_style_package && css_prop == CssProp::Off {
        return Inlined::default();
    }
    let scoping = Rc::new(
        SemanticBuilder::new()
            .build(program)
            .semantic
            .into_scoping(),
    );
    let mut inlined = inline_in(
        &scoping,
        ast_builder,
        program,
        filename,
        option,
        resolver,
        css_prop,
    );
    inlined.scoping = Some(scoping);
    inlined
}

fn is_style_package(option: &ExtractOption, source: &str) -> bool {
    crate::package_specifier::is_package(source, &option.package) || source == crate::STYLEX_PACKAGE
}

fn inline_in<'a>(
    scoping: &Scoping,
    ast_builder: &AstBuilder<'a>,
    program: &mut Program<'a>,
    filename: &str,
    option: &ExtractOption,
    resolver: Option<&ModuleResolver>,
    css_prop: CssProp,
) -> Inlined {
    let compat = format!("{}/compat", option.package);
    let css_props = CssTakers::new(program, scoping, css_prop, &compat);
    let style = StyleSymbols::from_program(program, scoping, option);
    let mut read = StyleReads {
        style: &style,
        css_props: &css_props,
        names: FxHashSet::default(),
        symbols: FxHashSet::default(),
        references: FxHashSet::default(),
        depth: 0,
        class_names: Vec::new(),
        slots: Vec::new(),
        callee: false,
        known: None,
    };
    read.visit_program(program);
    let initialization = initialization::Initialization::new(program, scoping);
    let declarations = lexical::declarations(ast_builder, program, scoping);
    loop {
        let before = read.symbols.len();
        for (symbol, init) in &declarations {
            if read.symbols.contains(symbol) {
                read.reading(true, |read| read.visit_expression(init));
            }
        }
        if read.symbols.len() == before {
            break;
        }
    }
    if read.names.is_empty() && read.symbols.is_empty() {
        return Inlined::default();
    }
    let mut inlined = Inlined {
        errors: initialization.errors(&read.references, scoping),
        ..Inlined::default()
    };
    if !inlined.errors.is_empty() {
        return inlined;
    }
    let mut modules = Modules {
        resolver,
        option,
        exports: FxHashMap::default(),
        loading: Vec::new(),
    };
    let mut symbols: FxHashMap<SymbolId, Constant> = FxHashMap::default();
    let reads_math = {
        let mut scope = ModuleScope::new(filename, program, None);
        scope.style_names.extend(
            style
                .roots
                .iter()
                .map(|symbol| scoping.symbol_name(*symbol).to_string()),
        );
        scope.css_prop = Some((css_prop, &compat));
        scope.shared_scoping = Some(scoping);
        let mut bindings: FxHashMap<&str, Vec<&Cell<Option<SymbolId>>>> = FxHashMap::default();
        for statement in &program.body {
            let declaration = match statement {
                Statement::ImportDeclaration(import) => {
                    if !is_style_package(option, &import.source.value) {
                        scope.import(import);
                        for specifier in import.specifiers.iter().flatten() {
                            let local = specifier.local();
                            bindings
                                .entry(local.name.as_str())
                                .or_default()
                                .push(&local.symbol_id);
                        }
                    } else if import.source.value != crate::STYLEX_PACKAGE {
                        scope.style_imports.extend(
                            import
                                .specifiers
                                .iter()
                                .flatten()
                                .map(|specifier| specifier.local().name.to_string()),
                        );
                    }
                    continue;
                }
                Statement::VariableDeclaration(declaration) => declaration,
                Statement::ExportDeclaration(export) => match &export.declaration {
                    oxc_ast::ast::Declaration::VariableDeclaration(declaration) => declaration,
                    oxc_ast::ast::Declaration::TSEnumDeclaration(declaration) => {
                        scope.declare_enum(declaration);
                        bindings
                            .entry(declaration.id.name.as_str())
                            .or_default()
                            .push(&declaration.id.symbol_id);
                        continue;
                    }
                    _ => continue,
                },
                Statement::TSEnumDeclaration(declaration) => {
                    scope.declare_enum(declaration);
                    bindings
                        .entry(declaration.id.name.as_str())
                        .or_default()
                        .push(&declaration.id.symbol_id);
                    continue;
                }
                _ => continue,
            };
            scope.declare(declaration);
            for declarator in &declaration.declarations {
                if let oxc_ast::ast::BindingPattern::BindingIdentifier(identifier) = &declarator.id
                {
                    bindings
                        .entry(identifier.name.as_str())
                        .or_default()
                        .push(&identifier.symbol_id);
                }
            }
        }
        // Constants are only worth reading when a style reads a name that may
        // hold one
        let reads_math = read.names.contains("Math") && !scope.binds("Math");
        if !reads_math
            && read.symbols.is_empty()
            && !read.names.iter().any(|name| scope.binds(name))
        {
            return Inlined::default();
        }
        for name in &read.names {
            let bound = scope.binds(name);
            let constant = scope.lookup(&mut modules, name);
            match &constant {
                Some(Constant::Changed(change)) => {
                    inlined.changed.whole.insert(name.clone(), change.clone());
                    continue;
                }
                Some(constant) if constant.change().is_some() => {
                    inlined
                        .changed
                        .holding
                        .insert(name.clone(), constant.clone());
                }
                _ => {}
            }
            match &constant {
                None if bound => {
                    inlined.unknown.names.insert(name.clone());
                }
                Some(object)
                    if bound
                        && (matches!(object, Constant::Object(_)) || object.has_function()) =>
                {
                    inlined.unknown.partial.insert(name.clone(), object.clone());
                }
                _ => {}
            }
            let Some(constant) = constant else {
                continue;
            };
            match &constant {
                Constant::Vars(vars) => {
                    inlined
                        .stylex_vars
                        .insert(name.clone(), vars.as_ref().clone());
                }
                Constant::Theme(class) => {
                    inlined.stylex_themes.insert(name.clone(), class.clone());
                }
                Constant::Style(Some(styles)) => {
                    inlined
                        .css_styles
                        .insert(name.clone(), styles.as_ref().clone());
                }
                _ => {}
            }
            for symbol in bindings.get(name.as_str()).into_iter().flatten() {
                symbols.extend(symbol.get().map(|symbol| (symbol, constant.clone())));
            }
        }
        reads_math
    };
    inlined.dependencies = modules.exports.into_keys().collect();
    let mut pending = declarations;
    pending.retain(|symbol, _| {
        read.symbols.contains(symbol) && scoping.symbol_scope_id(*symbol) != scoping.root_scope_id()
    });
    loop {
        let inline = Inline {
            ast_builder,
            scoping,
            initialization: &initialization,
            symbols: &symbols,
            style: &style,
            css_props: &css_props,
            objects: false,
            styles: false,
            px: false,
            class_names: Vec::new(),
        };
        let resolved: Vec<_> = pending
            .iter()
            .filter_map(|(symbol, init)| {
                let value = inline.operand(init)?;
                if matches!(&value, Constant::Number(number) if !number.is_finite()) {
                    return None;
                }
                matches!(
                    value,
                    Constant::String(_)
                        | Constant::Number(_)
                        | Constant::Null
                        | Constant::Bool(_)
                        | Constant::Undefined
                )
                .then_some((*symbol, value))
            })
            .collect();
        if resolved.is_empty() {
            break;
        }
        for (symbol, value) in resolved {
            pending.remove(&symbol);
            symbols.insert(symbol, value);
        }
    }
    if !symbols.is_empty() || reads_math {
        Inline {
            ast_builder,
            scoping,
            initialization: &initialization,
            symbols: &symbols,
            style: &style,
            css_props: &css_props,
            objects: false,
            styles: false,
            px: false,
            class_names: Vec::new(),
        }
        .visit_program(program);
    }
    inlined
}

/// Tells which top-level bindings of a program hold, whole or in some member,
/// an object or array code changes, reading only the modules they come from
pub(crate) struct ChangeCheck<'p, 'a, 'r> {
    scope: std::cell::RefCell<ModuleScope<'p, 'a>>,
    modules: std::cell::RefCell<Modules<'r>>,
}

impl<'p, 'a, 'r> ChangeCheck<'p, 'a, 'r> {
    pub(crate) fn new(
        program: &'p Program<'a>,
        filename: &'p str,
        option: &'r ExtractOption,
        resolver: Option<&'r ModuleResolver>,
    ) -> Self {
        let mut scope = ModuleScope::new(filename, program, None);
        for statement in &program.body {
            match statement {
                Statement::ImportDeclaration(import) => scope.import(import),
                Statement::VariableDeclaration(declaration) => {
                    scope.declare(declaration);
                }
                Statement::ExportDeclaration(export) => match &export.declaration {
                    oxc_ast::ast::Declaration::VariableDeclaration(declaration) => {
                        scope.declare(declaration);
                    }
                    oxc_ast::ast::Declaration::TSEnumDeclaration(declaration) => {
                        scope.declare_enum(declaration);
                    }
                    _ => {}
                },
                Statement::TSEnumDeclaration(declaration) => {
                    scope.declare_enum(declaration);
                }
                _ => {}
            }
        }
        Self {
            scope: std::cell::RefCell::new(scope),
            modules: std::cell::RefCell::new(Modules {
                resolver,
                option,
                exports: FxHashMap::default(),
                loading: Vec::new(),
            }),
        }
    }

    /// The value of `name` as JavaScript source, when the build knows all of
    /// it and no code changes it
    pub(crate) fn known(&self, name: &str) -> Option<String> {
        let mut modules = self.modules.borrow_mut();
        self.scope
            .borrow_mut()
            .lookup(&mut modules, name)?
            .js_literal()
    }

    /// The modules read for the values of imports
    pub(crate) fn dependencies(&self) -> BTreeSet<String> {
        self.modules.borrow().exports.keys().cloned().collect()
    }

    pub(crate) fn is_changed(&self, name: &str) -> bool {
        let mut modules = self.modules.borrow_mut();
        let mut scope = self.scope.borrow_mut();
        scope.change(&mut modules, name).is_some()
            || scope
                .lookup(&mut modules, name)
                .is_some_and(|value| value.change().is_some())
    }
}

/// What the imports of the style packages bind, told by the binding an
/// identifier reads and not by its spelling: a local named like an import is
/// not a style API
struct StyleSymbols<'s> {
    scoping: &'s Scoping,
    /// Everything the packages give
    roots: FxHashSet<SymbolId>,
    /// The style APIs that read style objects at build time
    functions: FxHashSet<SymbolId>,
    /// Namespace and default imports, `true` for `StyleX`
    namespaces: FxHashMap<SymbolId, bool>,
}

fn takes_style_objects(stylex: bool, export: &str) -> bool {
    if stylex {
        StylexFunction::from_export_name(export)
            .is_some_and(|function| function.requirement().is_some())
    } else {
        matches!(
            export,
            "css" | "globalCss" | "keyframes" | "createGlobalStyle" | "styled"
        )
    }
}

impl<'s> StyleSymbols<'s> {
    fn from_program(program: &Program<'_>, scoping: &'s Scoping, option: &ExtractOption) -> Self {
        let mut style = Self::new(scoping);
        for statement in &program.body {
            if let Statement::ImportDeclaration(import) = statement
                && is_style_package(option, &import.source.value)
                && !import.import_kind.is_type()
            {
                let stylex = import.source.value == crate::STYLEX_PACKAGE;
                for specifier in import.specifiers.iter().flatten() {
                    if let Some(local) = specifier.local().symbol_id.get() {
                        style.roots.insert(local);
                        match specifier {
                            ImportDeclarationSpecifier::ImportSpecifier(specifier) => {
                                if takes_style_objects(stylex, &specifier.imported.name()) {
                                    style.functions.insert(local);
                                }
                            }
                            _ => {
                                style.namespaces.insert(local, stylex);
                            }
                        }
                    }
                }
            }
        }
        style
    }

    fn new(scoping: &'s Scoping) -> Self {
        Self {
            scoping,
            roots: FxHashSet::default(),
            functions: FxHashSet::default(),
            namespaces: FxHashMap::default(),
        }
    }

    /// Whether `identifier` reads something a style package gives
    fn has(&self, identifier: &IdentifierReference<'_>) -> bool {
        binding_of(self.scoping, identifier).is_some_and(|symbol| self.roots.contains(&symbol))
    }

    /// Whether `expression` is a style API: a root the package gives, or a
    /// member or call of one
    fn is_root(&self, expression: &Expression<'_>) -> bool {
        match expression {
            Expression::Identifier(identifier) => self.has(identifier),
            Expression::StaticMemberExpression(member) => self.is_root(&member.object),
            Expression::CallExpression(call) => self.is_root(&call.callee),
            _ => false,
        }
    }

    /// Whether `name` is a component of the packages: `<Box>` or `<Devup.Box>`
    fn is_component(&self, name: &JSXElementName<'_>) -> bool {
        jsx_root_identifier(name).is_some_and(|identifier| self.has(identifier))
    }

    /// Whether calling `callee` reads its arguments as style objects:
    /// `css(...)`, `styled.div(...)`, `styled(Link).attrs(...)`,
    /// `stylex.create(...)`
    fn reads(&self, callee: &Expression<'_>) -> bool {
        let mut expression = callee;
        let mut member = None;
        loop {
            match crate::utils::unwrap_syntax_only(expression) {
                Expression::Identifier(identifier) => {
                    return binding_of(self.scoping, identifier).is_some_and(|symbol| {
                        self.functions.contains(&symbol)
                            || self.namespaces.get(&symbol).is_some_and(|stylex| {
                                member.is_some_and(|member| takes_style_objects(*stylex, member))
                            })
                    });
                }
                Expression::StaticMemberExpression(inner) => {
                    member = Some(inner.property.name.as_str());
                    expression = &inner.object;
                }
                Expression::CallExpression(call) => {
                    member = None;
                    expression = &call.callee;
                }
                _ => return false,
            }
        }
    }
}
/// Names read inside the props of the package's components and the arguments
/// of its functions
struct StyleReads<'s> {
    style: &'s StyleSymbols<'s>,
    css_props: &'s CssTakers<'s>,
    names: FxHashSet<String>,
    symbols: FxHashSet<SymbolId>,
    references: FxHashSet<oxc_syntax::reference::ReferenceId>,
    depth: usize,
    /// The bindings the `<ClassNames>` child functions around take `css` and
    /// `cx` by
    class_names: Vec<SymbolId>,
    slots: Vec<Span>,
    callee: bool,
    known: Option<&'s dyn Fn(&IdentifierReference<'_>) -> bool>,
}

impl StyleReads<'_> {
    fn reading<T>(&mut self, style: bool, walk: impl FnOnce(&mut Self) -> T) -> T {
        self.depth += usize::from(style);
        let result = walk(self);
        self.depth -= usize::from(style);
        result
    }
}

impl<'a> Visit<'a> for StyleReads<'_> {
    fn visit_expression(&mut self, expression: &Expression<'a>) {
        if self.depth > 0
            && !self.callee
            && !self.style.is_root(expression)
            && let Some(known) = self.known
            && consumer::closed(expression, self.style, known)
        {
            self.slots.push(expression.span());
        }
        walk::walk_expression(self, expression);
    }

    fn visit_identifier_reference(&mut self, identifier: &IdentifierReference<'a>) {
        if self.depth > 0 {
            self.references.extend(identifier.reference_id.get());
            self.symbols
                .extend(binding_of(self.style.scoping, identifier));
            if reads_top_level(self.style.scoping, identifier) {
                self.names.insert(identifier.name.to_string());
                if !self.callee
                    && binding_of(self.style.scoping, identifier).is_some()
                    && !self.style.has(identifier)
                    && self.known.is_some_and(|known| known(identifier))
                {
                    self.slots.push(identifier.span);
                }
            }
        }
    }

    fn visit_static_member_expression(
        &mut self,
        member: &oxc_ast::ast::StaticMemberExpression<'a>,
    ) {
        if self.depth > 0
            && !self.callee
            && let Some(known) = self.known
            && consumer::member((&member.object, None), self.style, known)
        {
            self.slots.push(member.span);
        }
        walk::walk_static_member_expression(self, member);
    }

    fn visit_computed_member_expression(
        &mut self,
        member: &oxc_ast::ast::ComputedMemberExpression<'a>,
    ) {
        if self.depth > 0
            && !self.callee
            && let Some(known) = self.known
            && consumer::member(
                (&member.object, Some(&member.expression)),
                self.style,
                known,
            )
        {
            self.slots.push(member.span);
        }
        walk::walk_computed_member_expression(self, member);
    }

    fn visit_template_literal(&mut self, template: &oxc_ast::ast::TemplateLiteral<'a>) {
        if self.depth > 0
            && !self.callee
            && let Some(known) = self.known
            && consumer::template(template, self.style, known)
        {
            self.slots.push(template.span);
        }
        walk::walk_template_literal(self, template);
    }

    fn visit_jsx_element(&mut self, element: &oxc_ast::ast::JSXElement<'a>) {
        let calls = self.css_props.class_names_calls(element);
        let taken = calls.len();
        self.class_names.extend(calls);
        oxc_ast_visit::walk::walk_jsx_element(self, element);
        self.class_names.truncate(self.class_names.len() - taken);
    }

    fn visit_call_expression(&mut self, call: &oxc_ast::ast::CallExpression<'a>) {
        if self.depth > 0
            && !self.callee
            && !self.style.is_root(&call.callee)
            && let Some(known) = self.known
            && consumer::call(call, self.style, known)
        {
            self.slots.push(call.span);
        }
        let outer = std::mem::replace(&mut self.callee, true);
        self.visit_expression(&call.callee);
        self.callee = outer;
        let style = self.style.is_root(&call.callee)
            || self
                .css_props
                .calls_class_names(&self.class_names, &call.callee);
        let css = self
            .css_props
            .property(call, |identifier| self.style.has(identifier));
        self.reading(style, |reads| {
            for (index, argument) in call.arguments.iter().enumerate() {
                match (css, argument) {
                    (Some(css), Argument::ObjectExpression(props)) if index == 1 => {
                        for (at, property) in props.properties.iter().enumerate() {
                            reads.reading(at == css, |reads| {
                                reads.visit_object_property_kind(property);
                            });
                        }
                    }
                    _ => reads.visit_argument(argument),
                }
            }
        });
    }

    fn visit_tagged_template_expression(
        &mut self,
        tagged: &oxc_ast::ast::TaggedTemplateExpression<'a>,
    ) {
        let outer = std::mem::replace(&mut self.callee, true);
        self.visit_expression(&tagged.tag);
        self.callee = outer;
        let style = self.style.is_root(&tagged.tag)
            || self
                .css_props
                .calls_class_names(&self.class_names, &tagged.tag);
        self.reading(style, |reads| reads.visit_template_literal(&tagged.quasi));
    }

    fn visit_jsx_opening_element(&mut self, element: &oxc_ast::ast::JSXOpeningElement<'a>) {
        let component = self.style.is_component(&element.name);
        for attribute in &element.attributes {
            let style = (component
                && (self.known.is_none()
                    || match attribute {
                        JSXAttributeItem::Attribute(attribute) => matches!(&attribute.name,
                    oxc_ast::ast::JSXAttributeName::Identifier(name)
                        if !css::is_special_property::is_special_property(&name.name)
                            && !matches!(name.name.as_str(), "as" | "props" | "styleVars")),
                        JSXAttributeItem::SpreadAttribute(_) => true,
                    }))
                || self
                    .css_props
                    .attribute(&element.name, attribute, |identifier| {
                        self.style.has(identifier)
                    });
            self.reading(style, |reads| match attribute {
                JSXAttributeItem::Attribute(attribute) => {
                    if let Some(value) = &attribute.value {
                        reads.visit_jsx_attribute_value(value);
                    }
                }
                JSXAttributeItem::SpreadAttribute(spread) => {
                    reads.visit_expression(&spread.argument);
                }
            });
        }
    }
}

/// The identifier `<Box>` or `<Devup.Box>` starts with
pub(crate) fn jsx_root_identifier<'n, 'a>(
    name: &'n JSXElementName<'a>,
) -> Option<&'n IdentifierReference<'a>> {
    match name {
        JSXElementName::IdentifierReference(identifier) => Some(identifier),
        JSXElementName::MemberExpression(member) => {
            let mut object = &member.object;
            while let oxc_ast::ast::JSXMemberExpressionObject::MemberExpression(inner) = object {
                object = &inner.object;
            }
            match object {
                oxc_ast::ast::JSXMemberExpressionObject::IdentifierReference(identifier) => {
                    Some(identifier)
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// The name `<Box>` or `<Devup.Box>` starts with
pub(crate) fn jsx_root<'n>(name: &'n JSXElementName<'_>) -> Option<&'n str> {
    jsx_root_identifier(name).map(|identifier| identifier.name.as_str())
}

/// The constant exports of the modules read, by path
struct Modules<'r> {
    resolver: Option<&'r ModuleResolver>,
    option: &'r ExtractOption,
    exports: FxHashMap<String, Rc<FxHashMap<String, Constant>>>,
    loading: Vec<String>,
}

impl Modules<'_> {
    fn exports(
        &mut self,
        specifier: &str,
        importer: &str,
    ) -> Option<Rc<FxHashMap<String, Constant>>> {
        let module = (self.resolver?)(specifier, importer)?;
        if let Some(exports) = self.exports.get(&module.path) {
            return Some(exports.clone());
        }
        if self.loading.contains(&module.path) {
            return None;
        }
        self.loading.push(module.path.clone());
        let exports = Rc::new(self.read(&module.path, &module.code));
        self.loading.pop();
        self.exports.insert(module.path, exports.clone());
        Some(exports)
    }

    fn read(&mut self, path: &str, code: &str) -> FxHashMap<String, Constant> {
        let allocator = Allocator::default();
        let source_type = SourceType::from_path(path).unwrap_or_else(|_| SourceType::ts());
        let program = Parser::new(&allocator, code, source_type).parse().program;
        let mut scope = ModuleScope::new(path, &program, Some(code));
        let mut exports = FxHashMap::default();
        let mut exported: Vec<(String, String)> = Vec::new();
        let mut commonjs = CommonJs::new(&program);
        for statement in &program.body {
            match statement {
                Statement::ImportDeclaration(import) => scope.import(import),
                Statement::VariableDeclaration(declaration) => {
                    scope.require(declaration);
                    scope.declare(declaration);
                }
                Statement::ExpressionStatement(statement) => {
                    commonjs.assign(&mut scope, self, &statement.expression);
                }
                Statement::ExportDeclaration(export) => match &export.declaration {
                    oxc_ast::ast::Declaration::VariableDeclaration(declaration) => {
                        for name in scope.declare(declaration) {
                            exported.push((name.clone(), name));
                        }
                    }
                    oxc_ast::ast::Declaration::TSEnumDeclaration(declaration) => {
                        let name = scope.declare_enum(declaration);
                        exported.push((name.clone(), name));
                    }
                    _ => {}
                },
                Statement::TSEnumDeclaration(declaration) => {
                    scope.declare_enum(declaration);
                }
                Statement::ExportNamedDeclaration(export) => {
                    for specifier in &export.specifiers {
                        exported.push((
                            specifier.exported.name().to_string(),
                            specifier.local.name().to_string(),
                        ));
                    }
                }
                Statement::ExportFromDeclaration(export) => {
                    if let Some(from) = self.exports(&export.source.value, path) {
                        for specifier in &export.specifiers {
                            if let Some(constant) = from.get(specifier.local.name().as_str()) {
                                exports.insert(
                                    specifier.exported.name().to_string(),
                                    constant.clone(),
                                );
                            }
                        }
                    }
                }
                Statement::ExportAllDeclaration(export) => {
                    if let Some(from) = self.exports(&export.source.value, path) {
                        match &export.exported {
                            Some(exported) => {
                                exports.insert(exported.name().to_string(), Constant::Object(from));
                            }
                            None => {
                                for (name, constant) in from.iter() {
                                    if name != "default" {
                                        exports
                                            .entry(name.clone())
                                            .or_insert_with(|| constant.clone());
                                    }
                                }
                            }
                        }
                    }
                }
                Statement::ExportDefaultDeclaration(export) => {
                    if let Some(expression) = export.declaration.as_expression()
                        && let Some(constant) = scope.evaluate(self, expression)
                    {
                        exports.insert("default".to_string(), constant);
                    }
                }
                _ => {}
            }
        }
        for (exported, local) in exported {
            if let Some(constant) = scope.lookup(self, &local) {
                exports.insert(exported, constant);
            }
        }
        commonjs.finish(&mut exports);
        exports
    }
}
/// The exports of a `CommonJS` module. A property counts only when the module
/// assigns it once, as anything assigned again may change after it is read.
#[derive(Default)]
struct CommonJs {
    /// Assignments to each exported property anywhere in the module, `*` for
    /// `module.exports` itself; `void 0` placeholders do not count
    writes: FxHashMap<String, usize>,
    exports: FxHashMap<String, Constant>,
    whole: Option<Constant>,
    es_module: bool,
    seen: bool,
}

impl CommonJs {
    fn new(program: &Program<'_>) -> Self {
        let mut commonjs = Self::default();
        commonjs.visit_program(program);
        commonjs
    }

    fn assign(
        &mut self,
        scope: &mut ModuleScope<'_, '_>,
        modules: &mut Modules<'_>,
        expression: &Expression<'_>,
    ) {
        if let Expression::CallExpression(call) = expression
            && is_define_es_module(call)
        {
            self.es_module = true;
            return;
        }
        let Expression::AssignmentExpression(assignment) = expression else {
            return;
        };
        let Some(key) = assignment
            .left
            .as_simple_assignment_target()
            .and_then(|target| target.as_member_expression())
            .and_then(commonjs_target)
        else {
            return;
        };
        self.seen = true;
        if key == "__esModule" {
            self.es_module = true;
            return;
        }
        if self.writes.get(&key) != Some(&1) {
            return;
        }
        let Some(value) = scope.evaluate(modules, &assignment.right) else {
            return;
        };
        if key == "*" {
            // What `module.exports` holds becomes the module's exports
            self.whole = Some(match value {
                Constant::Record(entries) => {
                    Constant::Object(Rc::new(entries.iter().cloned().collect()))
                }
                value => value,
            });
        } else {
            self.exports.insert(key, value);
        }
    }

    fn finish(self, exports: &mut FxHashMap<String, Constant>) {
        if !self.seen {
            return;
        }
        let mut properties = FxHashMap::default();
        if let Some(Constant::Object(object)) = &self.whole {
            for (key, value) in object.iter() {
                if !self.writes.contains_key(key) {
                    properties.insert(key.clone(), value.clone());
                }
            }
        }
        properties.extend(self.exports);
        let default = if self.es_module {
            properties.get("default").cloned()
        } else {
            match self.whole {
                Some(Constant::Object(_)) | None => Some(Constant::Object(Rc::new(
                    properties
                        .clone()
                        .into_iter()
                        .filter(|(key, _)| key != "default")
                        .collect(),
                ))),
                primitive => primitive,
            }
        };
        exports.extend(properties);
        if let Some(default) = default {
            exports.insert("default".to_string(), default);
        }
    }
}

impl<'a> Visit<'a> for CommonJs {
    fn visit_assignment_expression(&mut self, assignment: &oxc_ast::ast::AssignmentExpression<'a>) {
        let mut value = &assignment.right;
        while let Expression::AssignmentExpression(inner) = value {
            value = &inner.right;
        }
        let placeholder = matches!(value, Expression::UnaryExpression(unary)
            if unary.operator == oxc_syntax::operator::UnaryOperator::Void);
        if !placeholder
            && let Some(key) = assignment
                .left
                .as_simple_assignment_target()
                .and_then(|target| target.as_member_expression())
                .and_then(commonjs_target)
        {
            *self.writes.entry(key).or_default() += 1;
        }
        walk::walk_assignment_expression(self, assignment);
    }
}

/// `exports.x` / `module.exports.x` -> `x`, `module.exports` -> `*`
fn commonjs_target(member: &oxc_ast::ast::MemberExpression<'_>) -> Option<String> {
    let key = member.static_property_name()?;
    let object = member.object();
    if matches!(object, Expression::Identifier(identifier) if identifier.name == "module") {
        return (key == "exports").then(|| "*".to_string());
    }
    let is_exports = match object {
        Expression::Identifier(identifier) => identifier.name == "exports",
        Expression::StaticMemberExpression(inner) => {
            inner.property.name == "exports"
                && matches!(&inner.object, Expression::Identifier(identifier) if identifier.name == "module")
        }
        _ => false,
    };
    is_exports.then(|| key.to_string())
}

/// `Object.defineProperty(exports, '__esModule', ...)`
fn is_define_es_module(call: &oxc_ast::ast::CallExpression<'_>) -> bool {
    matches!(&call.callee, Expression::StaticMemberExpression(callee)
        if callee.property.name == "defineProperty"
            && matches!(&callee.object, Expression::Identifier(object) if object.name == "Object"))
        && matches!(call.arguments.first(), Some(Argument::Identifier(target)) if target.name == "exports")
        && matches!(call.arguments.get(1), Some(Argument::StringLiteral(key)) if key.value == "__esModule")
}

/// The top-level constants and imports of a module, each constant evaluated
/// when first read
struct ModuleScope<'p, 'a> {
    path: &'p str,
    program: &'p Program<'a>,
    /// The code of an imported module, where changes are located as
    /// `file:line:column`; the file extracted keeps offsets
    source: Option<&'p str>,
    locals: FxHashMap<String, Constant>,
    declarations: FxHashMap<String, &'p Expression<'a>>,
    enums: FxHashMap<String, &'p oxc_ast::ast::TSEnumDeclaration<'a>>,
    enum_members: Option<(
        &'p oxc_ast::ast::TSEnumDeclaration<'a>,
        FxHashMap<String, Constant>,
    )>,
    imports: FxHashMap<String, (String, Imported)>,
    style_imports: FxHashSet<String>,
    /// Style APIs besides the imports, which never run what they are given
    style_names: FxHashSet<String>,
    /// The `css` props of the file extracted, which never run what they hold,
    /// and the entry absorbing Emotion's own `jsx`
    css_prop: Option<(CssProp, &'p str)>,
    uses: Option<Rc<FxHashMap<String, Vec<crate::mutations::Use>>>>,
    changes: FxHashMap<String, Option<Rc<Change>>>,
    /// The semantic analysis the program extracted already has, which the
    /// visitor reuses and building another over the same program would reset
    shared_scoping: Option<&'p Scoping>,
    /// The semantic analysis of a module read, built when a `StyleX` callee
    /// first needs the binding it reads told
    scoping: OnceCell<Scoping>,
}

impl<'p, 'a> ModuleScope<'p, 'a> {
    fn new(path: &'p str, program: &'p Program<'a>, source: Option<&'p str>) -> Self {
        Self {
            path,
            program,
            source,
            locals: FxHashMap::default(),
            declarations: FxHashMap::default(),
            enums: FxHashMap::default(),
            enum_members: None,
            imports: FxHashMap::default(),
            style_imports: FxHashSet::default(),
            style_names: FxHashSet::default(),
            css_prop: None,
            uses: None,
            changes: FxHashMap::default(),
            shared_scoping: None,
            scoping: OnceCell::new(),
        }
    }

    fn is_style_api(&self, modules: &Modules<'_>, callee: &Expression<'_>) -> bool {
        let mut expression = callee;
        let name = loop {
            match expression {
                Expression::Identifier(identifier) => break identifier.name.as_str(),
                Expression::StaticMemberExpression(member) => expression = &member.object,
                Expression::CallExpression(call) => expression = &call.callee,
                _ => return false,
            }
        };
        self.style_imports.contains(name)
            || self.imports.get(name).is_some_and(|(source, _)| {
                source != crate::STYLEX_PACKAGE
                    && (crate::package_specifier::is_package(source, &modules.option.package)
                        || modules.option.import_aliases.contains_key(source))
            })
    }

    /// The styles behind `css(rules)`, the package's own `css` given one
    /// rule object every value of which is known, as the module's class names
    /// do not tell them
    fn css_styles(
        &mut self,
        modules: &mut Modules<'_>,
        call: &oxc_ast::ast::CallExpression<'_>,
    ) -> Option<Rc<Vec<ExtractStyleValue>>> {
        let Expression::Identifier(callee) = &call.callee else {
            return None;
        };
        let (source, Imported::Named(export)) = self.imports.get(callee.name.as_str())? else {
            return None;
        };
        if export != "css" || !crate::package_specifier::is_package(source, &modules.option.package)
        {
            return None;
        }
        let [argument] = call.arguments.as_slice() else {
            return None;
        };
        let rules = self.evaluate(modules, argument.as_expression()?)?;
        let allocator = Allocator::default();
        let builder = AstBuilder::new(&allocator);
        let mut rules = match constant_literal(&builder, &rules, true)? {
            rules @ Expression::ObjectExpression(_) => rules,
            _ => return None,
        };
        let ExtractResult {
            mut styles,
            style_order,
            ..
        } = extract_style_from_expression(
            &builder,
            None,
            &mut rules,
            0,
            &None,
            LiteralHandling::ExpandResponsiveThemeToken,
        );
        if let Some(order) = style_order {
            for prop in &mut styles {
                set_prop_order(prop, order);
            }
        }
        let mut composition = Composition::default();
        composition.apply(&builder, styles);
        composition.unconditional().map(Rc::new)
    }

    fn is_style_import(&self, option: &ExtractOption, name: &str) -> bool {
        self.style_imports.contains(name)
            || self.style_names.contains(name)
            || self.imports.get(name).is_some_and(|(source, _)| {
                source == crate::STYLEX_PACKAGE
                    || crate::package_specifier::is_package(source, &option.package)
                    || option.import_aliases.contains_key(source)
            })
    }

    fn site(&self, name: &str, at: u32, handed: bool) -> Rc<Change> {
        Rc::new(Change {
            name: name.to_string(),
            site: match self.source {
                Some(code) => ChangeSite::In(crate::locate(self.path, code, at as usize)),
                None => ChangeSite::Here(at),
            },
            handed,
        })
    }

    /// Where code changes the object or array `name` holds, directly or
    /// through the `const` it is put in; handing on a value the build does
    /// not know, such as a function, does not count. A namespace import can
    /// only have its members changed
    fn change(&mut self, modules: &mut Modules<'_>, name: &str) -> Option<Rc<Change>> {
        if let Some(change) = self.changes.get(name) {
            return change.clone();
        }
        self.changes.insert(name.to_string(), None);
        let uses = if let Some(uses) = &self.uses {
            uses.clone()
        } else {
            let option = modules.option;
            let uses = Rc::new(crate::mutations::uses(
                self.program,
                &|name| self.is_style_import(option, name),
                self.css_prop,
            ));
            self.uses = Some(uses.clone());
            uses
        };
        let namespace = matches!(self.imports.get(name), Some((_, Imported::Namespace)));
        let value = self.lookup_raw(modules, name);
        let mut change = None;
        for found in uses.get(name).into_iter().flatten() {
            change = match found {
                crate::mutations::Use::Changes { at, depth } => {
                    (!namespace || *depth > 1).then(|| self.site(name, *at, false))
                }
                crate::mutations::Use::Calls { at, path } => (!namespace
                    && value
                        .as_ref()
                        .is_some_and(|value| !value.reaches_only_primitives(path)))
                .then(|| self.site(name, *at, true)),
                crate::mutations::Use::Escapes { at, path, into } => {
                    let mut path = path.clone();
                    if namespace && path.is_empty() {
                        path.push(None);
                    }
                    if value
                        .as_ref()
                        .is_none_or(|value| value.reaches_only_primitives(&path))
                    {
                        continue;
                    }
                    match into {
                        Some(into) => self.change(modules, into),
                        None => Some(self.site(name, *at, true)),
                    }
                }
            };
            if change.is_some() {
                break;
            }
        }
        self.changes.insert(name.to_string(), change.clone());
        change
    }

    fn binds(&self, name: &str) -> bool {
        self.declarations.contains_key(name)
            || self.enums.contains_key(name)
            || self.imports.contains_key(name)
            || self.locals.contains_key(name)
            || self
                .semantic_scoping()
                .get_root_binding(name.into())
                .is_some()
    }

    fn is_global_math(&self, expression: &Expression<'_>) -> bool {
        matches!(expression, Expression::Identifier(identifier)
            if identifier.name == "Math" && !self.binds("Math"))
    }

    /// Record an enum for lazy evaluation after module bindings are collected.
    fn declare_enum(&mut self, declaration: &'p oxc_ast::ast::TSEnumDeclaration<'a>) -> String {
        let name = declaration.id.name.to_string();
        if !declaration.declare {
            self.enums.insert(name.clone(), declaration);
        }
        name
    }

    fn evaluate_enum(
        &mut self,
        modules: &mut Modules<'_>,
        declaration: &'p oxc_ast::ast::TSEnumDeclaration<'a>,
    ) -> Constant {
        let name = declaration.id.name.to_string();
        let mut members = FxHashMap::default();
        let mut next = Some(0.0);
        let outer = self.enum_members.take();
        for member in &declaration.body.members {
            self.locals
                .insert(name.clone(), Constant::Object(Rc::new(members.clone())));
            self.enum_members = Some((declaration, members.clone()));
            let value = match &member.initializer {
                None => next.map(Constant::Number),
                Some(initializer) => self.evaluate(modules, initializer),
            };
            let Some(value @ (Constant::String(_) | Constant::Number(_))) = value else {
                break;
            };
            if matches!(&value, Constant::Number(number) if !number.is_finite()) {
                break;
            }
            next = match &value {
                Constant::Number(number) => Some(number + 1.0),
                _ => None,
            };
            members.insert(member.id.static_name().to_string(), value);
        }
        self.enum_members = outer;
        Constant::Object(Rc::new(members))
    }

    fn import(&mut self, import: &oxc_ast::ast::ImportDeclaration<'_>) {
        for specifier in import.specifiers.iter().flatten() {
            let imported = match specifier {
                ImportDeclarationSpecifier::ImportSpecifier(specifier) => {
                    Imported::Named(specifier.imported.name().to_string())
                }
                ImportDeclarationSpecifier::ImportDefaultSpecifier(_) => {
                    Imported::Named("default".to_string())
                }
                ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) => Imported::Namespace,
            };
            self.imports.insert(
                specifier.local().name.to_string(),
                (import.source.value.to_string(), imported),
            );
        }
    }
    /// `const x = require('m')` and `const { a, b: c } = require('m')`
    fn require(&mut self, declaration: &oxc_ast::ast::VariableDeclaration<'_>) {
        for declarator in &declaration.declarations {
            let Some(Expression::CallExpression(call)) = &declarator.init else {
                continue;
            };
            let (Expression::Identifier(callee), [Argument::StringLiteral(source)]) =
                (&call.callee, call.arguments.as_slice())
            else {
                continue;
            };
            let scoping = self.semantic_scoping();
            if callee.name != "require"
                || callee
                    .reference_id
                    .get()
                    .is_none_or(|reference| scoping.get_reference(reference).symbol_id().is_some())
            {
                continue;
            }
            let source = source.value.to_string();
            match &declarator.id {
                oxc_ast::ast::BindingPattern::BindingIdentifier(identifier) => {
                    self.imports
                        .insert(identifier.name.to_string(), (source, Imported::Namespace));
                }
                oxc_ast::ast::BindingPattern::ObjectPattern(pattern) => {
                    for property in &pattern.properties {
                        if let Some(key) = property.key.static_name()
                            && let Some(local) = property.value.get_identifier_name()
                        {
                            self.imports.insert(
                                local.to_string(),
                                (source.clone(), Imported::Named(key.to_string())),
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Record the `const`s of `declaration`, returning the names it binds
    fn declare(&mut self, declaration: &'p oxc_ast::ast::VariableDeclaration<'a>) -> Vec<String> {
        let mut names = Vec::new();
        if declaration.kind != VariableDeclarationKind::Const {
            return names;
        }
        for declarator in &declaration.declarations {
            if let Some(name) = declarator.id.get_identifier_name()
                && let Some(init) = &declarator.init
            {
                self.declarations.insert(name.to_string(), init);
                names.push(name.to_string());
            }
        }
        names
    }

    /// What `name` holds, or where code changes it when it is an object or
    /// array the module changes
    fn lookup(&mut self, modules: &mut Modules<'_>, name: &str) -> Option<Constant> {
        let value = self.lookup_raw(modules, name)?;
        if value.is_mutable()
            && let Some(change) = self.change(modules, name)
        {
            return Some(Constant::Changed(change));
        }
        Some(value)
    }

    fn lookup_raw(&mut self, modules: &mut Modules<'_>, name: &str) -> Option<Constant> {
        if let Some(constant) = self.locals.get(name) {
            return Some(constant.clone());
        }
        if let Some(declaration) = self.enums.remove(name) {
            let value = self.evaluate_enum(modules, declaration);
            self.locals.insert(name.to_string(), value.clone());
            return Some(value);
        }
        // Taken out while it is evaluated, so a constant reading itself stops
        if let Some(init) = self.declarations.remove(name)
            && let Some(constant) = self.evaluate(modules, init)
        {
            self.locals.insert(name.to_string(), constant.clone());
            return Some(constant);
        }
        let (source, imported) = self.imports.get(name)?;
        let exports = modules.exports(source, self.path)?;
        match imported {
            Imported::Named(export) => exports.get(export).cloned(),
            Imported::Namespace => Some(Constant::Object(exports)),
        }
    }

    /// The `StyleX` API `callee` reads, as the import it reads binds it and
    /// not as it is spelled: a local named like an import is not the API
    fn stylex_function(&self, callee: &Expression<'_>) -> Option<StylexFunction> {
        let (identifier, member) = match callee {
            Expression::Identifier(identifier) => (identifier, None),
            Expression::StaticMemberExpression(member) => match &member.object {
                Expression::Identifier(object) => (object, Some(member.property.name.as_str())),
                _ => return None,
            },
            _ => return None,
        };
        let export = match (self.imports.get(identifier.name.as_str())?, member) {
            ((source, _), _) if source != crate::STYLEX_PACKAGE => return None,
            ((_, Imported::Named(export)), None) => export.as_str(),
            ((_, Imported::Namespace), Some(export)) => export,
            ((_, Imported::Named(export)), Some(member)) if export == "default" => member,
            _ => return None,
        };
        let function = StylexFunction::from_export_name(export)?;
        self.reads_top_level_binding(identifier).then_some(function)
    }

    /// Whether `identifier` reads a binding of the module's top level, where
    /// the imports bind, and not a local of a function or block
    fn reads_top_level_binding(&self, identifier: &IdentifierReference<'_>) -> bool {
        let scoping = self.semantic_scoping();
        binding_of(scoping, identifier)
            .is_some_and(|symbol| scoping.symbol_scope_id(symbol) == scoping.root_scope_id())
    }

    fn semantic_scoping(&self) -> &Scoping {
        self.shared_scoping.unwrap_or_else(|| {
            self.scoping.get_or_init(|| {
                SemanticBuilder::new()
                    .build(self.program)
                    .semantic
                    .into_scoping()
            })
        })
    }

    /// A value `StyleX` gives when this module's own extraction reads it, with
    /// the names that extraction generates
    fn evaluate_stylex(
        &mut self,
        modules: &mut Modules<'_>,
        call: &oxc_ast::ast::CallExpression<'_>,
    ) -> Option<Constant> {
        let function = self.stylex_function(&call.callee)?;
        let split_filename = crate::css_bucket(self.path, modules.option);
        match (function, call.arguments.as_slice()) {
            (
                function @ (StylexFunction::DefineVars | StylexFunction::CreateThemeContract),
                [Argument::ObjectExpression(object)],
            ) => {
                let mut vars = FxHashMap::default();
                for property in &object.properties {
                    if let ObjectPropertyKind::ObjectProperty(property) = property
                        && let Some(key) = self.property_key(modules, &property.key)
                        && (function == StylexFunction::CreateThemeContract
                            || self.variable_value(modules, &property.value))
                    {
                        let variable = crate::stylex::define_vars_variable(
                            self.path,
                            &key,
                            split_filename.as_deref(),
                        );
                        vars.insert(key, variable);
                    }
                }
                Some(Constant::Vars(Rc::new(vars)))
            }
            (StylexFunction::DefineConsts, [Argument::ObjectExpression(object)]) => {
                let mut values = FxHashMap::default();
                for property in &object.properties {
                    if let ObjectPropertyKind::ObjectProperty(property) = property
                        && let Some(key) = self.property_key(modules, &property.key)
                        && let Some(value) = self.literal_text(modules, &property.value)
                    {
                        values.insert(key, Constant::String(value));
                    }
                }
                Some(Constant::Object(Rc::new(values)))
            }
            (
                StylexFunction::CreateTheme,
                [
                    Argument::Identifier(contract),
                    Argument::ObjectExpression(_),
                ],
            ) => matches!(self.lookup(modules, &contract.name)?, Constant::Vars(_)).then(|| {
                Constant::Theme(crate::stylex::create_theme_class(
                    self.path,
                    &contract.name,
                    split_filename.as_deref(),
                ))
            }),
            _ => None,
        }
    }

    fn property_key(
        &mut self,
        modules: &mut Modules<'_>,
        key: &oxc_ast::ast::PropertyKey<'_>,
    ) -> Option<String> {
        match crate::utils::get_string_by_property_key(key) {
            Some(key) => Some(key),
            None => self.literal_text(modules, key.as_expression()?),
        }
    }

    /// Whether the module's own extraction reads `value` as the value of a
    /// `StyleX` variable, as [`crate::stylex::variable_values`] reads it once
    /// constants are inlined
    fn variable_value(&mut self, modules: &mut Modules<'_>, value: &Expression<'_>) -> bool {
        let value = crate::stylex::unwrap_types_call(value, &|callee| self.stylex_function(callee));
        if matches!(value, Expression::NullLiteral(_))
            || self.literal_text(modules, value).is_some()
        {
            return true;
        }
        let Expression::ObjectExpression(object) = value else {
            return false;
        };
        for property in &object.properties {
            let ObjectPropertyKind::ObjectProperty(property) = property else {
                return false;
            };
            let Some(key) = self.property_key(modules, &property.key) else {
                return false;
            };
            if (key != "default" && css::at_rule::split_at_rule_key(&key).is_none())
                || !self.variable_value(modules, &property.value)
            {
                return false;
            }
        }
        true
    }

    /// `value` as the text the module's own extraction reads it as, when that
    /// extraction knows it
    fn literal_text(
        &mut self,
        modules: &mut Modules<'_>,
        value: &Expression<'_>,
    ) -> Option<String> {
        match crate::utils::get_string_by_literal_expression(value) {
            Some(text) => Some(text.into_owned()),
            None => js_string(&self.evaluate(modules, value)?),
        }
    }

    fn evaluate(
        &mut self,
        modules: &mut Modules<'_>,
        expression: &Expression<'_>,
    ) -> Option<Constant> {
        match expression {
            Expression::StringLiteral(literal) => Some(Constant::String(literal.value.to_string())),
            Expression::NumericLiteral(literal) => Some(Constant::Number(literal.value)),
            Expression::UnaryExpression(unary)
                if unary.operator == oxc_syntax::operator::UnaryOperator::UnaryNegation =>
            {
                match self.evaluate(modules, &unary.argument)? {
                    Constant::Number(number) => Some(Constant::Number(-number)),
                    _ => None,
                }
            }
            Expression::TemplateLiteral(template) => {
                let mut values = Vec::with_capacity(template.expressions.len());
                for expression in &template.expressions {
                    values.push(self.evaluate(modules, expression)?);
                }
                fold_template(template, &values)
            }
            Expression::BinaryExpression(binary) => {
                let left = self.evaluate(modules, &binary.left)?;
                let right = self.evaluate(modules, &binary.right)?;
                fold_binary(binary.operator, &left, &right)
            }
            Expression::NullLiteral(_) => Some(Constant::Null),
            Expression::BooleanLiteral(literal) => Some(Constant::Bool(literal.value)),
            Expression::Identifier(identifier)
                if matches!(identifier.name.as_str(), "undefined" | "NaN" | "Infinity")
                    && !self.binds(&identifier.name) =>
            {
                Some(match identifier.name.as_str() {
                    "NaN" => Constant::Number(f64::NAN),
                    "Infinity" => Constant::Number(f64::INFINITY),
                    _ => Constant::Undefined,
                })
            }
            Expression::ObjectExpression(object) => Some(self.object(modules, object)),
            Expression::ArrayExpression(array) => {
                let mut values = Vec::with_capacity(array.elements.len());
                for element in &array.elements {
                    match element {
                        ArrayExpressionElement::SpreadElement(spread) => {
                            let Constant::Array(spread) =
                                self.evaluate(modules, &spread.argument)?
                            else {
                                return None;
                            };
                            values.extend(spread.iter().cloned());
                        }
                        element => values.push(self.evaluate(modules, element.as_expression()?)?),
                    }
                }
                Some(Constant::Array(Rc::new(values)))
            }
            Expression::ComputedMemberExpression(member) => {
                let key = js_string(&self.evaluate(modules, &member.expression)?)?;
                if self.is_global_math(&member.object) {
                    return math_constant(&key);
                }
                member_of(&self.evaluate(modules, &member.object)?, &key)
            }
            Expression::Identifier(identifier) => {
                if let Some((declaration, members)) = &self.enum_members
                    && declaration.span.contains_inclusive(identifier.span)
                    && declaration
                        .body
                        .members
                        .iter()
                        .any(|member| member.id.static_name() == identifier.name)
                {
                    return members.get(identifier.name.as_str()).cloned();
                }
                self.lookup(modules, &identifier.name)
            }
            Expression::StaticMemberExpression(member) if self.is_global_math(&member.object) => {
                math_constant(member.property.name.as_str())
            }
            Expression::StaticMemberExpression(member) => member_of(
                &self.evaluate(modules, &member.object)?,
                member.property.name.as_str(),
            ),
            Expression::CallExpression(call) => {
                if let Some(name) = math_member(&call.callee, &|object| self.is_global_math(object))
                {
                    let mut arguments = Vec::with_capacity(call.arguments.len());
                    for argument in &call.arguments {
                        arguments.push(self.evaluate(modules, argument.as_expression()?)?);
                    }
                    return fold_math(&name, &arguments);
                }
                if self.is_style_api(modules, &call.callee) {
                    return Some(Constant::Style(self.css_styles(modules, call)));
                }
                self.evaluate_stylex(modules, call)
            }
            Expression::TaggedTemplateExpression(tagged)
                if self.is_style_api(modules, &tagged.tag) =>
            {
                Some(Constant::Style(None))
            }
            Expression::TSAsExpression(inner) => self.evaluate(modules, &inner.expression),
            Expression::TSSatisfiesExpression(inner) => self.evaluate(modules, &inner.expression),
            Expression::ParenthesizedExpression(inner) => self.evaluate(modules, &inner.expression),
            _ => None,
        }
    }

    /// Every property of `object` when all are known, the known ones otherwise;
    /// a key written twice holds its last value at its first place, as in
    /// JavaScript, and what an unknown spread or key may replace is unknown
    fn object(
        &mut self,
        modules: &mut Modules<'_>,
        object: &oxc_ast::ast::ObjectExpression<'_>,
    ) -> Constant {
        let mut entries: Vec<(String, Constant)> = Vec::with_capacity(object.properties.len());
        let mut complete = true;
        for property in &object.properties {
            let (key, value) = match property {
                ObjectPropertyKind::ObjectProperty(property)
                    if property.kind == oxc_ast::ast::PropertyKind::Init =>
                {
                    let key = self.property_key(modules, &property.key);
                    let value = if property.method
                        || matches!(
                            crate::utils::unwrap_syntax_only(&property.value),
                            Expression::ArrowFunctionExpression(_)
                                | Expression::FunctionExpression(_)
                        ) {
                        Some(Constant::Function)
                    } else {
                        self.evaluate(modules, &property.value)
                    };
                    (key, value)
                }
                ObjectPropertyKind::SpreadProperty(spread) => {
                    if let Some(Constant::Record(spread)) = self.evaluate(modules, &spread.argument)
                    {
                        for (key, value) in spread.iter() {
                            set_entry(&mut entries, key.clone(), value.clone());
                        }
                        continue;
                    }
                    (None, None)
                }
                ObjectPropertyKind::ObjectProperty(property) => {
                    (self.property_key(modules, &property.key), None)
                }
            };
            match (key, value) {
                (Some(key), Some(value)) if key != "__proto__" => {
                    set_entry(&mut entries, key, value);
                }
                (Some(key), _) => {
                    complete = false;
                    entries.retain(|(existing, _)| *existing != key);
                }
                (None, _) => {
                    complete = false;
                    entries.clear();
                }
            }
        }
        if complete {
            Constant::Record(Rc::new(entries))
        } else {
            Constant::Object(Rc::new(entries.into_iter().collect()))
        }
    }
}

fn set_entry(entries: &mut Vec<(String, Constant)>, key: String, value: Constant) {
    match entries.iter_mut().find(|(existing, _)| *existing == key) {
        Some(entry) => entry.1 = value,
        None => entries.push((key, value)),
    }
}

/// `value` as JavaScript turns it into a string
fn js_string(value: &Constant) -> Option<String> {
    match value {
        Constant::String(text) => Some(text.clone()),
        Constant::Number(number) => Some(crate::utils::js_number_string(*number)),
        Constant::Null => Some("null".to_string()),
        Constant::Bool(value) => Some(value.to_string()),
        Constant::Undefined => Some("undefined".to_string()),
        _ => None,
    }
}

fn member_of(object: &Constant, key: &str) -> Option<Constant> {
    match object {
        Constant::Object(object) => object.get(key).cloned(),
        Constant::Record(entries) => entries
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.clone()),
        Constant::Array(values) => key
            .parse::<usize>()
            .ok()
            .filter(|index| index.to_string() == key)
            .and_then(|index| values.get(index).cloned()),
        Constant::Vars(vars) => vars
            .get(key)
            .map(|variable| Constant::String(format!("var({variable})"))),
        _ => None,
    }
}

fn fold_template(
    template: &oxc_ast::ast::TemplateLiteral<'_>,
    values: &[Constant],
) -> Option<Constant> {
    let mut text = String::new();
    for (index, quasi) in template.quasis.iter().enumerate() {
        text.push_str(quasi.value.cooked.as_deref()?);
        if let Some(value) = values.get(index) {
            text.push_str(&js_string(value)?);
        }
    }
    Some(Constant::String(text))
}

fn math_constant(name: &str) -> Option<Constant> {
    crate::build_time_values::exact_math::evaluate(name, None).map(Constant::Number)
}

fn math_member<'a>(
    expression: &Expression<'a>,
    is_math: &dyn Fn(&Expression<'a>) -> bool,
) -> Option<String> {
    match expression {
        Expression::StaticMemberExpression(member) if is_math(&member.object) => {
            Some(member.property.name.to_string())
        }
        Expression::ComputedMemberExpression(member) if is_math(&member.object) => {
            crate::utils::get_string_by_literal_expression(&member.expression)
                .map(std::borrow::Cow::into_owned)
        }
        _ => None,
    }
}

/// `Math.{name}(...arguments)`, folded only where every engine computes the
/// same result, so the CSS never depends on the platform that builds it
fn fold_math(name: &str, arguments: &[Constant]) -> Option<Constant> {
    use crate::build_time_values::exact_math::{Operand, evaluate};
    if name == "pow" {
        let (Some(Constant::Number(base)), Some(Constant::Number(exponent))) =
            (arguments.first(), arguments.get(1))
        else {
            return None;
        };
        return exact_power(*base, *exponent).map(Constant::Number);
    }
    let mut operands = Vec::with_capacity(arguments.len());
    for argument in arguments {
        let operand = match argument {
            Constant::Number(number) => Operand::Number(*number),
            Constant::String(text) => Operand::String(text),
            Constant::Bool(value) => Operand::Bool(*value),
            Constant::Null => Operand::Null,
            Constant::Undefined => Operand::Undefined,
            _ => return None,
        };
        operands.push(operand);
    }
    evaluate(name, Some(&operands)).map(Constant::Number)
}

/// An integer raised to a whole power, when the result is an exact integer
fn exact_power(base: f64, exponent: f64) -> Option<f64> {
    const EXACT: f64 = 9_007_199_254_740_992.0;
    if base.fract() != 0.0 || exponent.fract() != 0.0 || !(0.0..=64.0).contains(&exponent) {
        return None;
    }
    let mut result = 1.0_f64;
    for _ in 0..exponent as u8 {
        result *= base;
        if result.abs() > EXACT {
            return None;
        }
    }
    Some(result)
}

fn fold_binary(operator: BinaryOperator, left: &Constant, right: &Constant) -> Option<Constant> {
    let number = match (operator, left, right) {
        (BinaryOperator::Addition, Constant::Number(a), Constant::Number(b)) => a + b,
        (BinaryOperator::Subtraction, Constant::Number(a), Constant::Number(b)) => a - b,
        (BinaryOperator::Multiplication, Constant::Number(a), Constant::Number(b)) => a * b,
        (BinaryOperator::Division, Constant::Number(a), Constant::Number(b)) => a / b,
        (BinaryOperator::Remainder, Constant::Number(a), Constant::Number(b)) => a % b,
        (BinaryOperator::Addition, Constant::String(_), _)
        | (BinaryOperator::Addition, _, Constant::String(_)) => {
            return Some(Constant::String(js_string(left)? + &js_string(right)?));
        }
        _ => return None,
    };
    number.is_finite().then_some(Constant::Number(number))
}
/// Replaces reads of constants where styles read them: primitives in the
/// arguments of style APIs and in style props, objects and arrays where they
/// are read as style objects, as a copy elsewhere would change their identity.
/// Other code keeps reading the binding itself
struct Inline<'s, 'a> {
    ast_builder: &'s AstBuilder<'a>,
    scoping: &'s Scoping,
    initialization: &'s initialization::Initialization,
    symbols: &'s FxHashMap<SymbolId, Constant>,
    style: &'s StyleSymbols<'s>,
    css_props: &'s CssTakers<'s>,
    /// Inside what the build reads as style objects
    objects: bool,
    /// Inside the arguments of a style API or a style prop
    styles: bool,
    /// Inside a `css` prop, whose numbers Emotion reads as `px` lengths
    px: bool,
    /// The bindings the `<ClassNames>` child functions around take `css` and
    /// `cx` by
    class_names: Vec<SymbolId>,
}

impl<'a> Inline<'_, 'a> {
    fn constant(&self, expression: &Expression<'a>) -> Option<Constant> {
        match expression {
            Expression::Identifier(identifier) => {
                let reference = identifier.reference_id.get()?;
                let symbol = self.scoping.get_reference(reference).symbol_id()?;
                if !self.initialization.allows(identifier) {
                    return None;
                }
                self.symbols.get(&symbol).cloned()
            }
            Expression::StaticMemberExpression(member) if self.is_global_math(&member.object) => {
                math_constant(member.property.name.as_str())
            }
            Expression::StaticMemberExpression(member) => member_of(
                &self.constant(&member.object)?,
                member.property.name.as_str(),
            ),
            Expression::CallExpression(call) => {
                let name = math_member(&call.callee, &|object| self.is_global_math(object))?;
                let arguments: Option<Vec<Constant>> = call
                    .arguments
                    .iter()
                    .map(|argument| self.operand(argument.as_expression()?))
                    .collect();
                fold_math(&name, &arguments?)
            }
            Expression::ComputedMemberExpression(member) => {
                let key = js_string(&self.operand(&member.expression)?)?;
                if self.is_global_math(&member.object) {
                    return math_constant(&key);
                }
                member_of(&self.constant(&member.object)?, &key)
            }
            // Folded only when they read a constant, leaving other code as written
            Expression::TemplateLiteral(template) => {
                let values: Option<Vec<Constant>> = template
                    .expressions
                    .iter()
                    .map(|expression| self.operand(expression))
                    .collect();
                fold_template(template, &values?)
            }
            Expression::BinaryExpression(binary) => fold_binary(
                binary.operator,
                &self.operand(&binary.left)?,
                &self.operand(&binary.right)?,
            ),
            Expression::UnaryExpression(unary)
                if unary.operator == oxc_syntax::operator::UnaryOperator::UnaryNegation =>
            {
                match self.constant(&unary.argument)? {
                    Constant::Number(number) => Some(Constant::Number(-number)),
                    _ => None,
                }
            }
            Expression::ParenthesizedExpression(inner) => self.constant(&inner.expression),
            Expression::TSAsExpression(inner) => self.operand(&inner.expression),
            Expression::TSSatisfiesExpression(inner) => self.operand(&inner.expression),
            _ => None,
        }
    }

    /// A constant read or a literal
    fn operand(&self, expression: &Expression<'a>) -> Option<Constant> {
        self.constant(expression).or_else(|| match expression {
            Expression::StringLiteral(literal) => Some(Constant::String(literal.value.to_string())),
            Expression::BooleanLiteral(literal) => Some(Constant::Bool(literal.value)),
            Expression::NullLiteral(_) => Some(Constant::Null),
            Expression::Identifier(identifier)
                if binding_of(self.scoping, identifier).is_none() =>
            {
                match identifier.name.as_str() {
                    "undefined" => Some(Constant::Undefined),
                    "NaN" => Some(Constant::Number(f64::NAN)),
                    "Infinity" => Some(Constant::Number(f64::INFINITY)),
                    _ => None,
                }
            }
            _ => crate::utils::js_number_literal(expression).map(Constant::Number),
        })
    }

    /// Whether a condition reading `expression` holds, when it is a constant
    /// read or a literal
    fn holds(&self, expression: &Expression<'a>) -> Option<bool> {
        Some(match self.operand(expression)? {
            Constant::String(text) => !text.is_empty(),
            Constant::Number(number) => number != 0.0 && !number.is_nan(),
            Constant::Null | Constant::Undefined => false,
            Constant::Bool(value) => value,
            _ => true,
        })
    }

    /// The side of a condition or of `&&`, `||` or `??` a constant or literal
    /// chooses, taken out of `expression`
    fn chosen(&self, expression: &mut Expression<'a>) -> Option<Expression<'a>> {
        use oxc_allocator::TakeIn;

        let allocator = self.ast_builder;
        match expression {
            Expression::ConditionalExpression(conditional) => {
                Some(if self.holds(&conditional.test)? {
                    conditional.consequent.take_in(allocator)
                } else {
                    conditional.alternate.take_in(allocator)
                })
            }
            Expression::LogicalExpression(logical) => {
                let left = self.operand(&logical.left)?;
                let keep_left = match logical.operator {
                    oxc_syntax::operator::LogicalOperator::And => !self.holds(&logical.left)?,
                    oxc_syntax::operator::LogicalOperator::Or => self.holds(&logical.left)?,
                    oxc_syntax::operator::LogicalOperator::Coalesce => {
                        !matches!(left, Constant::Null | Constant::Undefined)
                    }
                };
                Some(if keep_left {
                    logical.left.take_in(allocator)
                } else {
                    logical.right.take_in(allocator)
                })
            }
            _ => None,
        }
    }

    fn is_global_math(&self, expression: &Expression<'a>) -> bool {
        matches!(expression, Expression::Identifier(identifier)
            if identifier.name == "Math"
                && identifier
                    .reference_id
                    .get()
                    .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
                    .is_none())
    }

    fn literal(&self, constant: &Constant) -> Option<Expression<'a>> {
        constant_literal(self.ast_builder, constant, self.objects)
    }

    fn reading_objects<T>(&mut self, objects: bool, visit: impl FnOnce(&mut Self) -> T) -> T {
        let outer = std::mem::replace(&mut self.objects, objects);
        let result = visit(self);
        self.objects = outer;
        result
    }

    fn reading_styles<T>(&mut self, styles: bool, visit: impl FnOnce(&mut Self) -> T) -> T {
        let outer = self.styles;
        self.styles |= styles;
        let result = visit(self);
        self.styles = outer;
        result
    }

    /// `visit` reading a `css` prop when `css`
    fn reading_css<T>(&mut self, css: bool, visit: impl FnOnce(&mut Self) -> T) -> T {
        let outer = std::mem::replace(&mut self.px, css);
        let result = self.reading_styles(css, |inline| inline.reading_objects(css, visit));
        self.px = outer;
        result
    }
}

/// `property` holding a number as the `px` length Emotion reads it as
fn px_value<'a>(ast_builder: &AstBuilder<'a>, property: &mut oxc_ast::ast::ObjectProperty<'a>) {
    if let Some(number) = crate::utils::js_number_literal(&property.value)
        && number != 0.0
        && property
            .key
            .static_name()
            .is_some_and(|key| !crate::utils::keeps_bare_number(&key))
    {
        property.value = Expression::new_string_literal(
            SPAN,
            Str::from_in(format!("{number}px").as_str(), ast_builder.allocator()),
            None,
            ast_builder,
        );
    }
}

/// The rules `rules` with their numbers as the `px` lengths Emotion reads them
/// as, nested rules included
fn px_rules<'a>(ast_builder: &AstBuilder<'a>, rules: &mut Expression<'a>) {
    if let Expression::ObjectExpression(object) = rules {
        for property in &mut object.properties {
            if let ObjectPropertyKind::ObjectProperty(property) = property {
                px_value(ast_builder, property);
                px_rules(ast_builder, &mut property.value);
            }
        }
    }
}

/// `constant` written as a literal, objects and arrays too when `objects`
fn constant_literal<'a>(
    builder: &AstBuilder<'a>,
    constant: &Constant,
    objects: bool,
) -> Option<Expression<'a>> {
    match constant {
        Constant::String(value) => Some(Expression::new_string_literal(
            SPAN,
            Str::from_in(value.as_str(), builder.allocator()),
            None,
            builder,
        )),
        Constant::Number(value) if value.is_finite() => Some(Expression::new_numeric_literal(
            SPAN,
            *value,
            None,
            NumberBase::Decimal,
            builder,
        )),
        Constant::Null => Some(Expression::new_null_literal(SPAN, builder)),
        Constant::Bool(value) => Some(Expression::new_boolean_literal(SPAN, *value, builder)),
        Constant::Record(entries) if objects => {
            let mut properties = oxc_allocator::Vec::with_capacity_in(entries.len(), builder);
            for (key, value) in entries.iter() {
                properties.push(ObjectPropertyKind::new_object_property(
                    SPAN,
                    oxc_ast::ast::PropertyKind::Init,
                    oxc_ast::ast::PropertyKey::StringLiteral(oxc_ast::ast::StringLiteral::boxed(
                        SPAN,
                        Str::from_in(key.as_str(), builder.allocator()),
                        None,
                        builder,
                    )),
                    constant_literal(builder, value, objects)?,
                    false,
                    false,
                    false,
                    builder,
                ));
            }
            Some(Expression::new_object_expression(SPAN, properties, builder))
        }
        Constant::Array(values) if objects => {
            let mut elements = oxc_allocator::Vec::with_capacity_in(values.len(), builder);
            for value in values.iter() {
                elements.push(constant_literal(builder, value, objects)?.into());
            }
            Some(Expression::new_array_expression(SPAN, elements, builder))
        }
        _ => None,
    }
}

impl<'a> VisitMut<'a> for Inline<'_, 'a> {
    fn visit_expression(&mut self, expression: &mut Expression<'a>) {
        if self.styles {
            if let Some(literal) = self
                .constant(expression)
                .and_then(|constant| self.literal(&constant))
            {
                *expression = literal;
                if self.px {
                    px_rules(self.ast_builder, expression);
                }
                return;
            }
            if let Some(chosen) = self.chosen(expression) {
                *expression = chosen;
                self.visit_expression(expression);
                return;
            }
        }
        walk_mut::walk_expression(self, expression);
    }

    fn visit_tagged_template_expression(
        &mut self,
        tagged: &mut oxc_ast::ast::TaggedTemplateExpression<'a>,
    ) {
        self.visit_expression(&mut tagged.tag);
        if self
            .css_props
            .calls_class_names(&self.class_names, &tagged.tag)
        {
            self.reading_css(true, |inline| {
                inline.visit_template_literal(&mut tagged.quasi);
            });
            return;
        }
        let styles = self.style.is_root(&tagged.tag);
        self.reading_styles(styles, |inline| {
            inline.visit_template_literal(&mut tagged.quasi);
        });
    }

    fn visit_member_expression(&mut self, member: &mut oxc_ast::ast::MemberExpression<'a>) {
        self.reading_objects(false, |inline| {
            walk_mut::walk_member_expression(inline, member);
        });
    }

    fn visit_jsx_element(&mut self, element: &mut oxc_ast::ast::JSXElement<'a>) {
        let calls = self.css_props.class_names_calls(element);
        let taken = calls.len();
        self.class_names.extend(calls);
        walk_mut::walk_jsx_element(self, element);
        self.class_names.truncate(self.class_names.len() - taken);
    }

    fn visit_call_expression(&mut self, call: &mut oxc_ast::ast::CallExpression<'a>) {
        self.visit_expression(&mut call.callee);
        if self
            .css_props
            .calls_class_names(&self.class_names, &call.callee)
        {
            self.reading_css(true, |inline| {
                for argument in &mut call.arguments {
                    inline.visit_argument(argument);
                }
            });
            return;
        }
        let objects = self.style.reads(&call.callee);
        let styles = self.style.is_root(&call.callee);
        let css = self
            .css_props
            .property(call, |identifier| self.style.has(identifier));
        self.reading_styles(styles, |inline| {
            inline.reading_objects(objects, |inline| {
                for (index, argument) in call.arguments.iter_mut().enumerate() {
                    match (css, argument) {
                        (Some(css), Argument::ObjectExpression(props)) if index == 1 => {
                            for (at, property) in props.properties.iter_mut().enumerate() {
                                inline.reading_css(at == css, |inline| {
                                    inline.visit_object_property_kind(property);
                                });
                            }
                        }
                        (_, argument) => inline.visit_argument(argument),
                    }
                }
            });
        });
    }

    fn visit_jsx_opening_element(&mut self, element: &mut oxc_ast::ast::JSXOpeningElement<'a>) {
        let styled = self.style.is_component(&element.name);
        let element_name = &element.name;
        for attribute in &mut element.attributes {
            if self
                .css_props
                .attribute(element_name, attribute, |identifier| {
                    self.style.has(identifier)
                })
            {
                self.reading_css(true, |inline| inline.visit_jsx_attribute_item(attribute));
                continue;
            }
            let objects = styled
                && match attribute {
                    JSXAttributeItem::Attribute(attribute) => {
                        matches!(&attribute.name, oxc_ast::ast::JSXAttributeName::Identifier(name)
                        if !css::is_special_property::is_special_property(&name.name)
                            && !matches!(name.name.as_str(), "as" | "props" | "styleVars"))
                    }
                    JSXAttributeItem::SpreadAttribute(_) => true,
                };
            self.reading_styles(objects, |inline| {
                inline
                    .reading_objects(objects, |inline| inline.visit_jsx_attribute_item(attribute));
            });
        }
    }

    fn visit_object_property(&mut self, property: &mut oxc_ast::ast::ObjectProperty<'a>) {
        let inlined_number = self.px
            && self.styles
            && matches!(self.constant(&property.value), Some(Constant::Number(_)));
        walk_mut::walk_object_property(self, property);
        if property.shorthand && !matches!(property.value, Expression::Identifier(_)) {
            property.shorthand = false;
        }
        if inlined_number {
            px_value(self.ast_builder, property);
        }
    }
}

#[cfg(test)]
mod exact_edge_tests;
#[cfg(test)]
mod exact_math_tests;
#[cfg(test)]
mod exact_tests;
#[cfg(test)]
mod numeric_semantics_tests;
#[cfg(test)]
#[path = "imported_constants_package_boundary_tests.rs"]
mod package_boundary_tests;
#[cfg(test)]
mod scope_tests;
#[cfg(test)]
mod stylex_scope_tests;
#[cfg(test)]
mod tdz_tests;
