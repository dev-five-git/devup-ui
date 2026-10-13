//! Values the styles of a file compute from constants with code the file
//! declares, such as `css({ w: double(SIZE) })`: the APIs taking them have no
//! element to set a runtime value on, so the build runs that code and writes
//! what it gives in their place. Only code whose every step gives the same
//! value on every page and every build runs; anything else is left to the
//! runtime.

use std::collections::{BTreeMap, BTreeSet};

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    ArrayExpressionElement, Declaration, ExportDefaultDeclarationKind, Expression,
    ImportDeclarationSpecifier, ObjectPropertyKind, Program, Statement, VariableDeclarationKind,
};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_semantic::{Scoping, SemanticBuilder};
use oxc_span::{GetSpan, SourceType, Span};
use oxc_syntax::operator::LogicalOperator;
use oxc_syntax::reference::ReferenceId;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

#[path = "build_time_values_execute.rs"]
mod execute;
#[path = "build_time_values_mapping.rs"]
mod mapping;
use execute::compute;
use mapping::Generated;

use crate::stylex::StylexFunction;
use crate::utils::{binding_root, get_string_by_literal_expression, unwrap_syntax_only};
use crate::{ExtractOption, ModuleResolver};

/// What code reads outside itself
#[derive(Clone, Default)]
struct Closure {
    /// The top-level statements it reads, directly or through each other
    statements: BTreeSet<usize>,
    /// The imports it reads, by local name, each as the value the build knows
    imports: BTreeMap<String, String>,
}

/// An expression whose reads the build can run
struct Found {
    span: Span,
    closure: Closure,
    /// The key of the shorthand property it is written as, which the value
    /// written in its place needs
    shorthand: Option<String>,
    /// A part `css()` or `styled()` composes, which a class computed in its
    /// place would turn into CSS text
    rules_only: bool,
}

const UTILS: [&str; 4] = ["css", "globalCss", "keyframes", "createGlobalStyle"];

/// Whether `code` has a style value that only running the module computes:
/// one that the constants it reads do not give once inlined
#[cfg(test)]
pub(crate) fn has_build_time_values(
    filename: &str,
    code: &str,
    option: &ExtractOption,
    resolver: Option<&ModuleResolver>,
) -> bool {
    let aliased = crate::import_alias_visit::transform_import_aliases_with_edits(
        code,
        filename,
        &option.package,
        &option.import_aliases,
    );
    let allocator = Allocator::default();
    let Some(mut program) = parse(&allocator, filename, &aliased.code) else {
        return false;
    };
    let inlined = crate::imported_constants::inline_constants(
        &oxc_ast::builder::AstBuilder::new(&allocator),
        &mut program,
        filename,
        option,
        resolver,
        aliased.css_prop,
    );
    let changes = crate::imported_constants::ChangeCheck::new(&program, filename, option, resolver);
    !find(
        &program,
        &|source| crate::package_specifier::is_package(source, &option.package),
        &inlined.unknown,
        &|name| changes.is_changed(name),
        &|name| changes.known(name),
    )
    .is_empty()
}

/// The globals the build runs: plain data and the functions over it that every
/// engine computes alike
const GLOBALS: [&str; 18] = [
    "undefined",
    "NaN",
    "Infinity",
    "Math",
    "String",
    "Number",
    "Boolean",
    "Array",
    "Object",
    "JSON",
    "parseInt",
    "parseFloat",
    "isNaN",
    "isFinite",
    "encodeURIComponent",
    "decodeURIComponent",
    "encodeURI",
    "decodeURI",
];

/// The members of `Math` every engine gives exactly; the others are
/// approximations that may differ in their last digits
pub(crate) mod exact_math;
#[cfg(test)]
mod w22_tests;
use exact_math::EXACT_MATH;

/// Members giving what the locale, the Unicode data of the engine or chance
/// make them, and `toString`, which engines only approximate with a radix:
/// it runs only called at once without one
const UNCERTAIN_MEMBERS: [&str; 9] = [
    "toLocaleString",
    "toLocaleDateString",
    "toLocaleTimeString",
    "toLocaleUpperCase",
    "toLocaleLowerCase",
    "localeCompare",
    "normalize",
    "random",
    "toString",
];

fn parse<'a>(allocator: &'a Allocator, filename: &str, code: &'a str) -> Option<Program<'a>> {
    let source_type = SourceType::from_path(filename).ok()?;
    let parsed = Parser::new(allocator, code, source_type).parse();
    (!parsed.fatal_error).then_some(parsed.program)
}

fn find(
    program: &Program<'_>,
    is_style: &dyn Fn(&str) -> bool,
    unknown: &crate::imported_constants::Unknown,
    is_changed: &dyn Fn(&str) -> bool,
    known: &dyn Fn(&str) -> Option<String>,
) -> Vec<Found> {
    let scoping = SemanticBuilder::new()
        .build(program)
        .semantic
        .into_scoping();
    let mut finder = Finder::new(program, &scoping, is_style, unknown, is_changed, known);
    finder.visit_program(program);
    finder.found
}

/// A top-level binding: the statement declaring it, and whether the module
/// can run it without the style packages
struct Binding {
    statement: usize,
    usable: bool,
}

struct Finder<'s, 'a> {
    scoping: &'s Scoping,
    statements: &'s [Statement<'a>],
    bindings: FxHashMap<SymbolId, Binding>,
    /// Style APIs imported by name, and the namespaces holding them
    apis: FxHashSet<SymbolId>,
    namespaces: FxHashSet<SymbolId>,
    stylex_namespaces: FxHashSet<SymbolId>,
    /// `css` and `styled` imported by name, which compose their arguments
    css: FxHashSet<SymbolId>,
    styled: FxHashSet<SymbolId>,
    /// What the style packages other than `StyleX` are imported as by name
    components: FxHashSet<SymbolId>,
    /// Bindings whose value only running the module gives
    unknown: &'s crate::imported_constants::Unknown,
    /// Whether a binding holds an object or array code changes, which the
    /// module would compute from as declared
    is_changed: &'s dyn Fn(&str) -> bool,
    /// The value of an import as JavaScript source, when the build knows all
    /// of it; the module it comes from never runs
    known: &'s dyn Fn(&str) -> Option<String>,
    closures: FxHashMap<usize, Option<Closure>>,
    found: Vec<Found>,
}

fn is_evaluated_stylex(name: &str) -> bool {
    StylexFunction::from_export_name(name).is_some_and(|function| function.requirement().is_some())
}

impl<'s, 'a> Finder<'s, 'a> {
    fn new(
        program: &'s Program<'a>,
        scoping: &'s Scoping,
        is_style: &dyn Fn(&str) -> bool,
        unknown: &'s crate::imported_constants::Unknown,
        is_changed: &'s dyn Fn(&str) -> bool,
        known: &'s dyn Fn(&str) -> Option<String>,
    ) -> Self {
        let mut finder = Self {
            scoping,
            statements: &program.body,
            bindings: FxHashMap::default(),
            apis: FxHashSet::default(),
            namespaces: FxHashSet::default(),
            stylex_namespaces: FxHashSet::default(),
            css: FxHashSet::default(),
            styled: FxHashSet::default(),
            components: FxHashSet::default(),
            unknown,
            is_changed,
            known,
            closures: FxHashMap::default(),
            found: Vec::new(),
        };
        for (index, statement) in program.body.iter().enumerate() {
            finder.bind(index, statement, is_style);
        }
        finder
    }

    fn bind(&mut self, statement: usize, node: &Statement<'a>, is_style: &dyn Fn(&str) -> bool) {
        let mut declare = |symbol: Option<SymbolId>, usable: bool| {
            if let Some(symbol) = symbol {
                self.bindings.insert(symbol, Binding { statement, usable });
            }
        };
        let declaration = match node {
            Statement::ImportDeclaration(import) => {
                let source = import.source.value.as_str();
                let stylex = source == crate::STYLEX_PACKAGE;
                let style = stylex || is_style(source);
                for specifier in import.specifiers.iter().flatten() {
                    let symbol = specifier.local().symbol_id.get();
                    let (usable, api, namespace) = match specifier {
                        ImportDeclarationSpecifier::ImportSpecifier(specifier) => {
                            let name = specifier.imported.name();
                            (
                                !specifier.import_kind.is_type(),
                                if stylex {
                                    is_evaluated_stylex(&name)
                                } else {
                                    UTILS.contains(&name.as_str())
                                },
                                false,
                            )
                        }
                        _ => (true, false, true),
                    };
                    declare(symbol, usable && !style && !import.import_kind.is_type());
                    if let Some(symbol) = symbol.filter(|_| style) {
                        if api {
                            self.apis.insert(symbol);
                        } else if namespace && stylex {
                            self.stylex_namespaces.insert(symbol);
                        } else if namespace {
                            self.namespaces.insert(symbol);
                        }
                        if let ImportDeclarationSpecifier::ImportSpecifier(specifier) = specifier
                            && !stylex
                        {
                            self.components.insert(symbol);
                            match specifier.imported.name().as_str() {
                                "css" => {
                                    self.css.insert(symbol);
                                }
                                "styled" => {
                                    self.styled.insert(symbol);
                                }
                                _ => {}
                            }
                        }
                    }
                }
                return;
            }
            Statement::ExportDeclaration(export) => &export.declaration,
            Statement::ExportDefaultDeclaration(export) => {
                match &export.declaration {
                    ExportDefaultDeclarationKind::FunctionDeclaration(function) => {
                        declare(
                            function.id.as_ref().and_then(|id| id.symbol_id.get()),
                            function.body.is_some(),
                        );
                    }
                    ExportDefaultDeclarationKind::ClassDeclaration(class) => {
                        declare(class.id.as_ref().and_then(|id| id.symbol_id.get()), false);
                    }
                    _ => {}
                }
                return;
            }
            node => match node.as_declaration() {
                Some(declaration) => declaration,
                None => return,
            },
        };
        match declaration {
            Declaration::VariableDeclaration(declaration) => {
                for declarator in &declaration.declarations {
                    for identifier in declarator.id.get_binding_identifiers() {
                        declare(
                            identifier.symbol_id.get(),
                            declaration.kind == VariableDeclarationKind::Const
                                && !declaration.declare
                                && declarator.init.is_some(),
                        );
                    }
                }
            }
            Declaration::FunctionDeclaration(function) => {
                declare(
                    function.id.as_ref().and_then(|id| id.symbol_id.get()),
                    function.body.is_some(),
                );
            }
            Declaration::ClassDeclaration(class) => {
                declare(class.id.as_ref().and_then(|id| id.symbol_id.get()), false);
            }
            Declaration::TSEnumDeclaration(declaration) => {
                declare(declaration.id.symbol_id.get(), !declaration.declare);
            }
            _ => {}
        }
    }

    fn symbol(&self, expression: &Expression<'_>) -> Option<SymbolId> {
        let Expression::Identifier(identifier) = expression else {
            return None;
        };
        self.scoping
            .get_reference(identifier.reference_id.get()?)
            .symbol_id()
    }

    /// Whether `callee` is a style API with no element to set a runtime value on
    fn is_api(&self, callee: &Expression<'_>) -> bool {
        match callee {
            Expression::Identifier(_) => {
                self.symbol(callee).is_some_and(|s| self.apis.contains(&s))
            }
            Expression::StaticMemberExpression(member) => {
                self.symbol(&member.object).is_some_and(|symbol| {
                    let name = member.property.name.as_str();
                    (self.namespaces.contains(&symbol) && UTILS.contains(&name))
                        || (self.stylex_namespaces.contains(&symbol) && is_evaluated_stylex(name))
                })
            }
            _ => false,
        }
    }

    fn is_css(&self, callee: &Expression<'_>) -> bool {
        if let Expression::StaticMemberExpression(member) = callee {
            return member.property.name == "css"
                && self
                    .symbol(&member.object)
                    .is_some_and(|symbol| self.namespaces.contains(&symbol));
        }
        self.symbol(callee)
            .is_some_and(|symbol| self.css.contains(&symbol))
    }

    /// Whether calling `callee` gives a `styled` component its styles:
    /// `styled.div(...)`, `styled(Link)(...)`, `styled.div.attrs({})(...)`
    fn is_styled(&self, callee: &Expression<'_>) -> bool {
        match unwrap_syntax_only(callee) {
            Expression::StaticMemberExpression(member) => self.is_styled_function(&member.object),
            Expression::CallExpression(call) => match unwrap_syntax_only(&call.callee) {
                Expression::StaticMemberExpression(member)
                    if matches!(member.property.name.as_str(), "attrs" | "withConfig") =>
                {
                    self.is_styled(&member.object)
                }
                callee => self.is_styled_function(callee),
            },
            _ => false,
        }
    }

    /// Whether `name` is a component of the style packages: `<Box>`, `<Devup.Box>`
    fn is_component(&self, name: &oxc_ast::ast::JSXElementName<'_>) -> bool {
        let mut object = match name {
            oxc_ast::ast::JSXElementName::IdentifierReference(identifier) => {
                return self
                    .symbol_of(identifier)
                    .is_some_and(|symbol| self.components.contains(&symbol));
            }
            oxc_ast::ast::JSXElementName::MemberExpression(member) => &member.object,
            _ => return false,
        };
        loop {
            match object {
                oxc_ast::ast::JSXMemberExpressionObject::MemberExpression(member) => {
                    object = &member.object;
                }
                oxc_ast::ast::JSXMemberExpressionObject::IdentifierReference(identifier) => {
                    return self
                        .symbol_of(identifier)
                        .is_some_and(|symbol| self.namespaces.contains(&symbol));
                }
                oxc_ast::ast::JSXMemberExpressionObject::ThisExpression(_) => return false,
            }
        }
    }

    fn symbol_of(&self, identifier: &oxc_ast::ast::IdentifierReference<'_>) -> Option<SymbolId> {
        self.scoping
            .get_reference(identifier.reference_id.get()?)
            .symbol_id()
    }

    fn is_styled_function(&self, expression: &Expression<'_>) -> bool {
        if let Expression::StaticMemberExpression(member) = unwrap_syntax_only(expression) {
            return member.property.name == "styled"
                && self
                    .symbol(&member.object)
                    .is_some_and(|symbol| self.namespaces.contains(&symbol));
        }
        self.symbol(expression)
            .is_some_and(|symbol| self.styled.contains(&symbol))
    }

    /// The parts `css()` and `styled()` compose: rule objects, whose values
    /// are read when `rules` is set, and classes, which only rules the module
    /// computes replace
    fn parts(&mut self, expression: &Expression<'a>, rules: bool) {
        match unwrap_syntax_only(expression) {
            Expression::ObjectExpression(object) => {
                if rules {
                    self.values(expression, None);
                    return;
                }
                // Values an element holds at runtime stay; what gives it styles
                // is computed: spreads, selectors and computed keys
                for property in &object.properties {
                    match property {
                        ObjectPropertyKind::SpreadProperty(spread) => {
                            self.parts(&spread.argument, false);
                        }
                        ObjectPropertyKind::ObjectProperty(property) => {
                            if property.computed
                                && let Some(key) = property.key.as_expression()
                            {
                                self.values(key, None);
                            }
                            if property
                                .key
                                .static_name()
                                .is_some_and(|key| key.starts_with('_'))
                            {
                                self.parts(&property.value, false);
                            }
                        }
                    }
                }
            }
            Expression::ArrayExpression(array) => {
                for element in &array.elements {
                    if let Some(element) = element.as_expression() {
                        self.parts(element, rules);
                    }
                }
            }
            Expression::ConditionalExpression(conditional) => {
                self.parts(&conditional.consequent, rules);
                self.parts(&conditional.alternate, rules);
            }
            Expression::LogicalExpression(logical) => {
                if logical.operator != LogicalOperator::And {
                    self.parts(&logical.left, rules);
                }
                self.parts(&logical.right, rules);
            }
            // CSS text, whose values are read as `values` reads them
            Expression::TemplateLiteral(_) => {
                if rules {
                    self.values(expression, None);
                }
            }
            Expression::NullLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::StringLiteral(_) => {}
            inner => {
                if binding_root(inner).is_none() || self.unknown.read_by(inner) {
                    self.candidate(expression, None, true);
                }
            }
        }
    }

    /// The values `expression` holds: itself, or those of the object, array or
    /// condition it writes
    fn values(&mut self, expression: &Expression<'a>, shorthand: Option<&str>) {
        match unwrap_syntax_only(expression) {
            Expression::ObjectExpression(object) => {
                for property in &object.properties {
                    match property {
                        ObjectPropertyKind::ObjectProperty(property) => {
                            if property.computed
                                && let Some(key) = property.key.as_expression()
                            {
                                self.values(key, None);
                            }
                            let key = property
                                .key
                                .static_name()
                                .or_else(|| crate::utils::get_str_by_property_key(&property.key));
                            if key.as_deref().is_some_and(crate::style_order::reserved) {
                                self.order_values(&property.value);
                                continue;
                            }
                            self.values(
                                &property.value,
                                key.as_deref().filter(|_| property.shorthand),
                            );
                        }
                        ObjectPropertyKind::SpreadProperty(spread) => {
                            self.values(&spread.argument, None);
                        }
                    }
                }
            }
            Expression::ArrayExpression(array) => {
                for element in &array.elements {
                    match element {
                        ArrayExpressionElement::SpreadElement(spread) => {
                            self.values(&spread.argument, None);
                        }
                        element => {
                            if let Some(element) = element.as_expression() {
                                self.values(element, None);
                            }
                        }
                    }
                }
            }
            Expression::NullLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::ArrowFunctionExpression(_)
            | Expression::FunctionExpression(_) => {}
            Expression::TemplateLiteral(template) if self.metadata_template(template) => {}
            inner if get_string_by_literal_expression(inner).is_some() => {}
            inner => {
                if self.candidate(expression, shorthand, false) {
                    return;
                }
                // A condition only known at runtime still chooses between
                // values the module computes
                match inner {
                    Expression::ConditionalExpression(conditional) => {
                        self.values(&conditional.consequent, None);
                        self.values(&conditional.alternate, None);
                    }
                    Expression::LogicalExpression(logical) => {
                        self.values(&logical.left, None);
                        self.values(&logical.right, None);
                    }
                    _ => {}
                }
            }
        }
    }

    fn order_values(&mut self, expression: &Expression<'a>) {
        match unwrap_syntax_only(expression) {
            Expression::ConditionalExpression(conditional) => {
                self.values(&conditional.test, None);
                self.order_values(&conditional.consequent);
                self.order_values(&conditional.alternate);
            }
            Expression::LogicalExpression(logical) if logical.operator == LogicalOperator::And => {
                self.values(&logical.left, None);
                self.order_values(&logical.right);
            }
            _ => self.values(expression, None),
        }
    }

    fn metadata_template(&mut self, template: &oxc_ast::ast::TemplateLiteral<'a>) -> bool {
        let allocator = Allocator::default();
        let ast = oxc_ast::builder::AstBuilder::new(&allocator);
        let metadata = crate::imported_constants::order_metadata::holes(&ast, template);
        if metadata.is_empty() {
            return false;
        }
        for expression in &template.expressions {
            if metadata.contains(&expression.span()) {
                self.order_values(expression);
            } else {
                self.values(expression, None);
            }
        }
        true
    }

    fn candidate(
        &mut self,
        expression: &Expression<'a>,
        shorthand: Option<&str>,
        rules_only: bool,
    ) -> bool {
        let span = expression.span();
        let mut reads = Reads::new(self.scoping);
        reads.visit_expression(expression);
        if reads.impure {
            return false;
        }
        let Some(closure) = self.closure(span, &reads.references) else {
            return false;
        };
        self.found.push(Found {
            span,
            closure,
            shorthand: shorthand.map(str::to_string),
            rules_only,
        });
        true
    }

    /// What code at `span` reads outside itself, `None` when it reads a
    /// binding the build does not run: one declared other than as a `const`,
    /// a function or an enum, code it does not run, an object code changes,
    /// or an import whose value it does not know whole
    fn closure(&mut self, span: Span, references: &[ReferenceId]) -> Option<Closure> {
        let mut closure = Closure::default();
        for reference in references {
            let Some(symbol) = self.scoping.get_reference(*reference).symbol_id() else {
                continue;
            };
            if span.contains_inclusive(self.scoping.symbol_span(symbol)) {
                continue;
            }
            let name = self.scoping.symbol_name(symbol);
            let statement = match self.bindings.get(&symbol) {
                Some(Binding {
                    statement,
                    usable: true,
                }) if !(self.is_changed)(name) => *statement,
                _ => return None,
            };
            if matches!(self.statements[statement], Statement::ImportDeclaration(_)) {
                closure
                    .imports
                    .insert(name.to_string(), (self.known)(name)?);
                continue;
            }
            closure.statements.insert(statement);
            let inner = self.statement_closure(statement)?;
            closure.statements.extend(inner.statements);
            closure.imports.extend(inner.imports);
        }
        Some(closure)
    }

    fn statement_closure(&mut self, index: usize) -> Option<Closure> {
        if let Some(closure) = self.closures.get(&index) {
            return closure.clone();
        }
        // Read while it is computed, so statements reading each other stop
        self.closures.insert(index, Some(Closure::default()));
        let statement = &self.statements[index];
        let mut reads = Reads::new(self.scoping);
        reads.visit_statement(statement);
        let closure = if reads.impure {
            None
        } else {
            self.closure(statement.span(), &reads.references)
        };
        self.closures.insert(index, closure.clone());
        closure
    }
}

impl<'a> Visit<'a> for Finder<'_, 'a> {
    fn visit_call_expression(&mut self, call: &oxc_ast::ast::CallExpression<'a>) {
        let (api, styled) = (self.is_api(&call.callee), self.is_styled(&call.callee));
        if api || styled {
            let composes = styled || self.is_css(&call.callee);
            for argument in &call.arguments {
                let argument = match argument {
                    oxc_ast::ast::Argument::SpreadElement(spread) => &spread.argument,
                    argument => argument.to_expression(),
                };
                if composes {
                    self.parts(argument, api);
                } else {
                    self.values(argument, None);
                }
            }
        }
        walk::walk_call_expression(self, call);
    }

    fn visit_jsx_opening_element(&mut self, element: &oxc_ast::ast::JSXOpeningElement<'a>) {
        if self.is_component(&element.name) {
            for attribute in &element.attributes {
                match attribute {
                    oxc_ast::ast::JSXAttributeItem::SpreadAttribute(spread) => {
                        self.parts(&spread.argument, false);
                    }
                    oxc_ast::ast::JSXAttributeItem::Attribute(attribute) => {
                        if let oxc_ast::ast::JSXAttributeName::Identifier(name) = &attribute.name
                            && name.name.starts_with('_')
                            && let Some(oxc_ast::ast::JSXAttributeValue::ExpressionContainer(
                                container,
                            )) = &attribute.value
                            && let Some(value) = container.expression.as_expression()
                        {
                            self.parts(value, false);
                        }
                    }
                }
            }
        }
        walk::walk_jsx_opening_element(self, element);
    }

    fn visit_tagged_template_expression(
        &mut self,
        tagged: &oxc_ast::ast::TaggedTemplateExpression<'a>,
    ) {
        if self.is_api(&tagged.tag) && !self.metadata_template(&tagged.quasi) {
            for expression in &tagged.quasi.expressions {
                self.values(expression, None);
            }
        }
        walk::walk_tagged_template_expression(self, tagged);
    }
}

/// The bindings code reads, and whether the build runs it
struct Reads<'s> {
    scoping: &'s Scoping,
    references: Vec<ReferenceId>,
    /// Does what the build does not run: reads what only the code around it,
    /// the page or the process knows (`this`, globals such as `window` or
    /// `Date`), chance, the locale or results engines only approximate; uses
    /// more than plain data and functions (`new`, classes, regular
    /// expressions, `try`, getters, async code, JSX); or writes a binding
    /// outside itself
    impure: bool,
}

impl<'s> Reads<'s> {
    const fn new(scoping: &'s Scoping) -> Self {
        Self {
            scoping,
            references: Vec::new(),
            impure: false,
        }
    }

    fn symbol(&self, identifier: &oxc_ast::ast::IdentifierReference<'_>) -> Option<SymbolId> {
        self.scoping
            .get_reference(identifier.reference_id.get()?)
            .symbol_id()
    }

    fn is_global(&self, expression: &Expression<'_>, name: &str) -> bool {
        matches!(expression, Expression::Identifier(identifier)
            if identifier.name == name && self.symbol(identifier).is_none())
    }

    fn member(&mut self, object: &Expression<'_>, name: &str) {
        self.impure |= UNCERTAIN_MEMBERS.contains(&name)
            || (self.is_global(object, "Math") && !EXACT_MATH.contains(&name));
    }

    /// Whether `expression`, the object of a member written, is a top-level
    /// binding or a member of one
    fn is_outside(&self, expression: &Expression<'_>) -> bool {
        let mut expression = expression;
        loop {
            match unwrap_syntax_only(expression) {
                Expression::StaticMemberExpression(member) => expression = &member.object,
                Expression::ComputedMemberExpression(member) => expression = &member.object,
                Expression::Identifier(identifier) => {
                    return self.symbol(identifier).is_some_and(|symbol| {
                        self.scoping.symbol_scope_id(symbol) == self.scoping.root_scope_id()
                    });
                }
                _ => return false,
            }
        }
    }
}

impl<'a> Visit<'a> for Reads<'_> {
    fn visit_identifier_reference(&mut self, identifier: &oxc_ast::ast::IdentifierReference<'a>) {
        if let Some(reference) = identifier.reference_id.get() {
            let reference_data = self.scoping.get_reference(reference);
            self.impure |= reference_data.symbol_id().map_or_else(
                // `Math` runs only through the members `member` checks
                || !GLOBALS.contains(&identifier.name.as_str()) || identifier.name == "Math",
                |symbol| {
                    reference_data.is_write()
                        && self.scoping.symbol_scope_id(symbol) == self.scoping.root_scope_id()
                },
            );
            self.references.push(reference);
        }
    }

    fn visit_static_member_expression(
        &mut self,
        member: &oxc_ast::ast::StaticMemberExpression<'a>,
    ) {
        self.member(&member.object, member.property.name.as_str());
        if !self.is_global(&member.object, "Math") {
            walk::walk_static_member_expression(self, member);
        }
    }

    fn visit_computed_member_expression(
        &mut self,
        member: &oxc_ast::ast::ComputedMemberExpression<'a>,
    ) {
        if let Some(key) = get_string_by_literal_expression(&member.expression) {
            self.member(&member.object, &key);
            if self.is_global(&member.object, "Math") {
                return;
            }
        }
        walk::walk_computed_member_expression(self, member);
    }

    fn visit_call_expression(&mut self, call: &oxc_ast::ast::CallExpression<'a>) {
        match unwrap_syntax_only(&call.callee) {
            // A method chosen at runtime may be any
            Expression::ComputedMemberExpression(member) => {
                self.impure |= get_string_by_literal_expression(&member.expression).is_none();
            }
            // Exact only without a radix
            Expression::StaticMemberExpression(member) if member.property.name == "toString" => {
                self.impure |= !call.arguments.is_empty();
                self.visit_expression(&member.object);
                return;
            }
            _ => {}
        }
        walk::walk_call_expression(self, call);
    }

    fn visit_assignment_expression(&mut self, assignment: &oxc_ast::ast::AssignmentExpression<'a>) {
        self.impure |= assignment.operator == oxc_syntax::operator::AssignmentOperator::Exponential
            || assignment
                .left
                .as_simple_assignment_target()
                .and_then(|target| target.as_member_expression())
                .is_some_and(|member| self.is_outside(member.object()));
        walk::walk_assignment_expression(self, assignment);
    }

    fn visit_update_expression(&mut self, update: &oxc_ast::ast::UpdateExpression<'a>) {
        self.impure |= update
            .argument
            .as_member_expression()
            .is_some_and(|member| self.is_outside(member.object()));
        walk::walk_update_expression(self, update);
    }

    fn visit_unary_expression(&mut self, unary: &oxc_ast::ast::UnaryExpression<'a>) {
        self.impure |= unary.operator == oxc_syntax::operator::UnaryOperator::Delete
            && self.is_outside(&unary.argument);
        walk::walk_unary_expression(self, unary);
    }

    fn visit_binary_expression(&mut self, binary: &oxc_ast::ast::BinaryExpression<'a>) {
        self.impure |= binary.operator == oxc_syntax::operator::BinaryOperator::Exponential;
        walk::walk_binary_expression(self, binary);
    }

    fn visit_object_property(&mut self, property: &oxc_ast::ast::ObjectProperty<'a>) {
        self.impure |= property.kind != oxc_ast::ast::PropertyKind::Init;
        walk::walk_object_property(self, property);
    }

    fn visit_binding_property(&mut self, property: &oxc_ast::ast::BindingProperty<'a>) {
        self.impure |= property
            .key
            .static_name()
            .is_some_and(|key| UNCERTAIN_MEMBERS.contains(&key.as_ref()));
        walk::walk_binding_property(self, property);
    }

    fn visit_function(
        &mut self,
        function: &oxc_ast::ast::Function<'a>,
        flags: oxc_syntax::scope::ScopeFlags,
    ) {
        self.impure |= function.r#async || function.generator;
        walk::walk_function(self, function, flags);
    }

    fn visit_arrow_function_expression(
        &mut self,
        arrow: &oxc_ast::ast::ArrowFunctionExpression<'a>,
    ) {
        self.impure |= arrow.r#async;
        walk::walk_arrow_function_expression(self, arrow);
    }

    fn visit_this_expression(&mut self, _: &oxc_ast::ast::ThisExpression) {
        self.impure = true;
    }

    fn visit_super(&mut self, _: &oxc_ast::ast::Super) {
        self.impure = true;
    }

    fn visit_import_meta(&mut self, _: &oxc_ast::ast::ImportMeta) {
        self.impure = true;
    }

    fn visit_new_target(&mut self, _: &oxc_ast::ast::NewTarget) {
        self.impure = true;
    }

    fn visit_new_expression(&mut self, _: &oxc_ast::ast::NewExpression<'a>) {
        self.impure = true;
    }

    fn visit_class(&mut self, _: &oxc_ast::ast::Class<'a>) {
        self.impure = true;
    }

    fn visit_reg_exp_literal(&mut self, _: &oxc_ast::ast::RegExpLiteral<'a>) {
        self.impure = true;
    }

    fn visit_try_statement(&mut self, _: &oxc_ast::ast::TryStatement<'a>) {
        self.impure = true;
    }

    fn visit_await_expression(&mut self, _: &oxc_ast::ast::AwaitExpression<'a>) {
        self.impure = true;
    }

    fn visit_yield_expression(&mut self, _: &oxc_ast::ast::YieldExpression<'a>) {
        self.impure = true;
    }

    fn visit_import_expression(&mut self, _: &oxc_ast::ast::ImportExpression<'a>) {
        self.impure = true;
    }

    fn visit_jsx_element(&mut self, _: &oxc_ast::ast::JSXElement<'a>) {
        self.impure = true;
    }

    fn visit_jsx_fragment(&mut self, _: &oxc_ast::ast::JSXFragment<'a>) {
        self.impure = true;
    }

    // Types are erased before the code runs, so what they name is not read
    fn visit_ts_type(&mut self, _: &oxc_ast::ast::TSType<'a>) {}
}

/// What evaluating computed: the code with the values written in their place,
/// the replacements made, and the files read
pub(crate) type Evaluated = (
    String,
    Vec<crate::import_alias_visit::Edit>,
    BTreeSet<String>,
);

/// An error the build reports: the offset in the code evaluated that it names,
/// and what it tells
pub(crate) type Located = (usize, String);

/// `code` with what its style values compute written in their place, the
/// replacements made, and the files read; `None` when running the code it
/// reads computes none of them as a string, a finite number, or a plain
/// object or array of those. Code that reads what differs between builds or
/// pages (the clock, chance, the environment, the locale) is an error naming
/// the style value it computes, whether or not the code caught what the read
/// threw.
pub(crate) fn evaluate_located(
    code: &str,
    filename: &str,
    option: &ExtractOption,
    resolver: Option<&ModuleResolver>,
    unknown: &crate::imported_constants::Unknown,
) -> Result<Option<Evaluated>, Vec<Located>> {
    let Some((mut values, dependencies)) = compute(code, filename, option, resolver, unknown)?
    else {
        return Ok(None);
    };
    values.sort_unstable_by_key(|(span, _)| span.start);
    let mut result = String::with_capacity(code.len());
    let mut edits = Vec::with_capacity(values.len());
    let mut copied = 0;
    for (span, literal) in values {
        let (start, end) = (span.start as usize, span.end as usize);
        result.push_str(&code[copied..start]);
        result.push_str(&literal);
        edits.push((start, end, literal.len()));
        copied = end;
    }
    result.push_str(&code[copied..]);
    Ok(Some((result, edits, dependencies)))
}

/// A value's source text by the span of the code computing it
type Replacement = (Span, String);

/// The statements the values read, run as they are written, and the imports
/// they read as the build knows them: the modules those come from never run
fn definitions(program: &Program<'_>, code: &str, found: &[Found]) -> Generated {
    use std::fmt::Write;

    let statements: BTreeSet<usize> = found
        .iter()
        .flat_map(|found| found.closure.statements.iter().copied())
        .collect();
    let imports: BTreeMap<&str, &str> = found
        .iter()
        .flat_map(|found| &found.closure.imports)
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();
    let mut module = Generated::default();
    for (name, value) in imports {
        let _ = writeln!(module.text, "const {name} = {value};");
    }
    for index in statements {
        let statement = &program.body[index];
        let declaration = match statement {
            Statement::VariableDeclaration(declaration) => Some(declaration),
            Statement::ExportDeclaration(export) => match &export.declaration {
                Declaration::VariableDeclaration(declaration) => Some(declaration),
                _ => None,
            },
            _ => None,
        };
        if let Some(declaration) = declaration {
            // A binding whose value throws is one reading it throws for,
            // leaving the others
            for (declarator, init) in declaration
                .declarations
                .iter()
                .filter_map(|declarator| declarator.init.as_ref().map(|init| (declarator, init)))
            {
                module.text.push_str(declaration.kind.as_str());
                module.text.push(' ');
                module.copy(
                    code,
                    declarator.span.start as usize,
                    init.span().start as usize,
                );
                module.text.push_str("__try__(() => (");
                module.copy(code, init.span().start as usize, init.span().end as usize);
                let _ = writeln!(module.text, "), {index});");
            }
            continue;
        }
        let span = match statement {
            Statement::ExportDeclaration(export) => export.declaration.span(),
            Statement::ExportDefaultDeclaration(export) => export.declaration.span(),
            statement => statement.span(),
        };
        module.copy(code, span.start as usize, span.end as usize);
        module.text.push('\n');
    }
    module
}

/// The source text of what `found` computes, or nothing when a statement it
/// reads threw. Each value runs apart, so one that throws leaves the others.
fn value_script(code: &str, found: &Found) -> Generated {
    let statements: Vec<String> = found
        .closure
        .statements
        .iter()
        .map(ToString::to_string)
        .collect();
    let mut generated = Generated::default();
    generated.text = format!(
        "(() => {{ if ([{}].some((statement) => __failed_statements__.has(statement))) return undefined; try {{ return __literal__((",
        statements.join(", ")
    );
    generated.copy(code, found.span.start as usize, found.span.end as usize);
    generated
        .text
        .push_str(")); } catch { return undefined; } })();");
    generated
}

#[cfg(test)]
mod scope_tests;

#[cfg(test)]
#[path = "build_time_values_sandbox_tests.rs"]
mod sandbox_tests;
