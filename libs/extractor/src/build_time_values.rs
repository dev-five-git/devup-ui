//! Values the styles of a file compute when the module runs, such as
//! `css({ color: darken(0.1, PRIMARY) })`: the APIs taking them have no element
//! to set a runtime value on, so the build runs the code they read and writes
//! what it gives in their place.

use std::collections::BTreeSet;

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

use crate::stylex::StylexFunction;
use crate::utils::{binding_root, get_string_by_literal_expression, unwrap_syntax_only};
use crate::{ExtractOption, ModuleResolver};

/// An expression whose reads the module can run on its own
struct Found {
    span: Span,
    /// The top-level statements it reads, directly or through each other
    statements: BTreeSet<usize>,
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
    let (code, _) = crate::import_alias_visit::transform_import_aliases_with_edits(
        code,
        filename,
        &option.package,
        &option.import_aliases,
    );
    let allocator = Allocator::default();
    let Some(mut program) = parse(&allocator, filename, &code) else {
        return false;
    };
    let inlined = crate::imported_constants::inline_constants(
        &oxc_ast::builder::AstBuilder::new(&allocator),
        &mut program,
        filename,
        option,
        resolver,
    );
    !find(
        &program,
        &|source| source.starts_with(option.package.as_str()),
        &inlined.unknown,
        &|_| false,
    )
    .is_empty()
}

/// Globals only the running page or process knows
pub(crate) const ENVIRONMENT: [&str; 19] = [
    "window",
    "self",
    "document",
    "navigator",
    "location",
    "history",
    "localStorage",
    "sessionStorage",
    "matchMedia",
    "screen",
    "innerWidth",
    "innerHeight",
    "devicePixelRatio",
    "process",
    "global",
    "globalThis",
    "Deno",
    "Bun",
    "Intl",
];

/// Methods giving what the locale of the running page or process makes them
pub(crate) const LOCALE_METHODS: [&str; 6] = [
    "toLocaleString",
    "toLocaleDateString",
    "toLocaleTimeString",
    "toLocaleUpperCase",
    "toLocaleLowerCase",
    "localeCompare",
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
) -> Vec<Found> {
    let scoping = SemanticBuilder::new()
        .build(program)
        .semantic
        .into_scoping();
    let mut finder = Finder::new(program, &scoping, is_style, unknown, is_changed);
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
    closures: FxHashMap<usize, Option<BTreeSet<usize>>>,
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
                        declare(function.id.as_ref().and_then(|id| id.symbol_id.get()), true);
                    }
                    ExportDefaultDeclarationKind::ClassDeclaration(class) => {
                        declare(class.id.as_ref().and_then(|id| id.symbol_id.get()), true);
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
                            declaration.kind == VariableDeclarationKind::Const,
                        );
                    }
                }
            }
            Declaration::FunctionDeclaration(function) => {
                declare(function.id.as_ref().and_then(|id| id.symbol_id.get()), true);
            }
            Declaration::ClassDeclaration(class) => {
                declare(class.id.as_ref().and_then(|id| id.symbol_id.get()), true);
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
            return member.property.name == "css";
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
                            let key = property.key.static_name();
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

    fn candidate(
        &mut self,
        expression: &Expression<'a>,
        shorthand: Option<&str>,
        rules_only: bool,
    ) -> bool {
        let span = expression.span();
        let mut reads = Reads::new(self.scoping);
        reads.visit_expression(expression);
        if reads.opaque || reads.environment {
            return false;
        }
        let Some(statements) = self.closure(span, &reads.references) else {
            return false;
        };
        self.found.push(Found {
            span,
            statements,
            shorthand: shorthand.map(str::to_string),
            rules_only,
        });
        true
    }

    /// The top-level statements code at `span` reads, `None` when it reads a
    /// binding the module cannot run on its own
    fn closure(&mut self, span: Span, references: &[ReferenceId]) -> Option<BTreeSet<usize>> {
        let mut statements = BTreeSet::new();
        for reference in references {
            let Some(symbol) = self.scoping.get_reference(*reference).symbol_id() else {
                continue;
            };
            if span.contains_inclusive(self.scoping.symbol_span(symbol)) {
                continue;
            }
            let statement = match self.bindings.get(&symbol) {
                Some(Binding {
                    statement,
                    usable: true,
                }) if !(self.is_changed)(self.scoping.symbol_name(symbol)) => *statement,
                _ => return None,
            };
            statements.insert(statement);
            statements.extend(self.statement_closure(statement)?);
        }
        Some(statements)
    }

    fn statement_closure(&mut self, index: usize) -> Option<BTreeSet<usize>> {
        if let Some(closure) = self.closures.get(&index) {
            return closure.clone();
        }
        // Read while it is computed, so statements reading each other stop
        self.closures.insert(index, Some(BTreeSet::new()));
        let statement = &self.statements[index];
        let closure = if matches!(statement, Statement::ImportDeclaration(_)) {
            Some(BTreeSet::new())
        } else {
            let mut reads = Reads::new(self.scoping);
            reads.visit_statement(statement);
            if reads.environment {
                None
            } else {
                self.closure(statement.span(), &reads.references)
            }
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
        if self.is_api(&tagged.tag) {
            for expression in &tagged.quasi.expressions {
                self.values(expression, None);
            }
        }
        walk::walk_tagged_template_expression(self, tagged);
    }
}

/// The bindings code reads
struct Reads<'s> {
    scoping: &'s Scoping,
    references: Vec<ReferenceId>,
    /// Reads what only the code around it knows (`this`, `super`,
    /// `import.meta`), or what differs on every build (`Date`, `Math.random`)
    opaque: bool,
    /// Reads what only the running page or process knows, or its locale
    environment: bool,
}

impl<'s> Reads<'s> {
    const fn new(scoping: &'s Scoping) -> Self {
        Self {
            scoping,
            references: Vec::new(),
            opaque: false,
            environment: false,
        }
    }

    fn is_global(&self, expression: &Expression<'_>, name: &str) -> bool {
        matches!(expression, Expression::Identifier(identifier)
            if identifier.name == name
                && identifier
                    .reference_id
                    .get()
                    .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
                    .is_none())
    }
}

impl<'a> Visit<'a> for Reads<'_> {
    fn visit_identifier_reference(&mut self, identifier: &oxc_ast::ast::IdentifierReference<'a>) {
        let reference = identifier.reference_id.get();
        let global = reference
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
            .is_none();
        self.opaque |= global && identifier.name == "Date";
        self.environment |= global && ENVIRONMENT.contains(&identifier.name.as_str());
        self.references.extend(reference);
    }

    fn visit_static_member_expression(
        &mut self,
        member: &oxc_ast::ast::StaticMemberExpression<'a>,
    ) {
        self.opaque |= member.property.name == "random" && self.is_global(&member.object, "Math");
        self.environment |= LOCALE_METHODS.contains(&member.property.name.as_str());
        walk::walk_static_member_expression(self, member);
    }

    fn visit_this_expression(&mut self, _: &oxc_ast::ast::ThisExpression) {
        self.opaque = true;
    }

    fn visit_super(&mut self, _: &oxc_ast::ast::Super) {
        self.opaque = true;
    }

    fn visit_import_meta(&mut self, _: &oxc_ast::ast::ImportMeta) {
        self.opaque = true;
    }

    fn visit_new_target(&mut self, _: &oxc_ast::ast::NewTarget) {
        self.opaque = true;
    }

    // Types are erased before the code runs, so what they name is not read
    fn visit_ts_type(&mut self, _: &oxc_ast::ast::TSType<'a>) {}
}

/// `code` with what its style values compute written in their place, the
/// replacements made, and the files read; `None` when running the code it
/// reads computes none of them as a string, a finite number, or a plain
/// object or array of those
pub(crate) fn evaluate(
    code: &str,
    filename: &str,
    option: &ExtractOption,
    resolver: Option<&ModuleResolver>,
    unknown: &crate::imported_constants::Unknown,
) -> Option<(
    String,
    Vec<crate::import_alias_visit::Edit>,
    BTreeSet<String>,
)> {
    let (mut values, dependencies) = compute(code, filename, option, resolver, unknown)?;
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
    Some((result, edits, dependencies))
}

/// A value's source text by the span of the code computing it
type Replacement = (Span, String);

/// Run before the values: nothing that differs between builds or only the
/// running page or process knows, its locale included (reading it throws, so
/// no value depends on the build's environment), a stand-in for the style
/// packages, one throwing for a module the build cannot load, and the source
/// text of a value the build can read. A statement that throws is recorded,
/// and a value reading it is not computed
const PRELUDE: &str = r#"delete globalThis.Date;
Math.random = undefined;
for (const name of ["window", "self", "document", "navigator", "location", "history", "localStorage", "sessionStorage", "matchMedia", "screen", "innerWidth", "innerHeight", "devicePixelRatio", "process", "global", "Deno", "Bun", "Intl"]) Object.defineProperty(globalThis, name, { get() { throw new ReferenceError(`${name} is only known at runtime`); }, configurable: true });
for (const [prototype, names] of [[Object.prototype, ["toLocaleString"]], [Number.prototype, ["toLocaleString"]], [BigInt.prototype, ["toLocaleString"]], [Array.prototype, ["toLocaleString"]], [String.prototype, ["localeCompare", "toLocaleUpperCase", "toLocaleLowerCase"]]]) for (const name of names) Object.defineProperty(prototype, name, { value() { throw new ReferenceError(`${name} depends on the locale`); }, configurable: true, writable: true });
Object.setPrototypeOf(globalThis, new Proxy(Object.getPrototypeOf(globalThis), { get(target, key, receiver) { if (typeof key === "string" && !(key in target)) throw new ReferenceError(`${key} is only known at runtime`); return Reflect.get(target, key, receiver); } }));
globalThis.__vanilla_extract__ = (() => { const style = new Proxy(function () {}, { get: (_, key) => key === Symbol.toPrimitive ? undefined : style, apply: () => style }); return style; })();
globalThis.__unloaded__ = new Proxy({}, { get(_, key) { throw new ReferenceError(`${String(key)} comes from a module the build cannot load`); } });
const __failed__ = (() => { const fail = () => { throw new ReferenceError("its value threw"); }; return new Proxy(function () {}, { get: fail, apply: fail, construct: fail, getPrototypeOf: fail }); })();
const __failed_statements__ = new Set();
const __try__ = (compute, statement) => { try { return compute(); } catch { __failed_statements__.add(statement); return __failed__; } };
const __literal__ = (value) => {
  const plain = (item) => item === undefined || item === null || typeof item === "string" || typeof item === "boolean" || (typeof item === "number" && Number.isFinite(item))
    || (Array.isArray(item) && item.every(plain))
    || (typeof item === "object" && Object.getPrototypeOf(item) === Object.prototype && Object.getOwnPropertySymbols(item).length === 0 && !Object.prototype.hasOwnProperty.call(item, "__proto__") && Object.values(item).every(plain));
  if (typeof value === "number") return Number.isFinite(value) ? String(value) : undefined;
  return value !== undefined && plain(value) ? JSON.stringify(value) : undefined;
};
"#;

fn compute(
    code: &str,
    filename: &str,
    option: &ExtractOption,
    resolver: Option<&ModuleResolver>,
    unknown: &crate::imported_constants::Unknown,
) -> Option<(Vec<Replacement>, BTreeSet<String>)> {
    use std::fmt::Write;

    use boa_engine::{Context, JsObject, Source};

    use crate::module_loader::{Evaluating, ModuleLoader, module_script};

    let allocator = Allocator::default();
    let program = parse(&allocator, filename, code)?;
    let is_style = |source: &str| {
        source.starts_with(option.package.as_str()) || option.import_aliases.contains_key(source)
    };
    let changes = crate::imported_constants::ChangeCheck::new(&program, filename, option, resolver);
    let found = find(&program, &is_style, unknown, &|name| {
        changes.is_changed(name)
    });
    if found.is_empty() {
        return None;
    }
    let statements: BTreeSet<usize> = found
        .iter()
        .flat_map(|found| found.statements.iter().copied())
        .collect();
    let mut module = String::new();
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
            for declarator in &declaration.declarations {
                let Some(init) = &declarator.init else {
                    continue;
                };
                let _ = writeln!(
                    module,
                    "{} {}__try__(() => ({}), {index});",
                    declaration.kind.as_str(),
                    &code[declarator.span.start as usize..init.span().start as usize],
                    &code[init.span().start as usize..init.span().end as usize],
                );
            }
            continue;
        }
        let span = match statement {
            Statement::ExportDeclaration(export) => export.declaration.span(),
            Statement::ExportDefaultDeclaration(export) => export.declaration.span(),
            statement => statement.span(),
        };
        module.push_str(&code[span.start as usize..span.end as usize]);
        module.push('\n');
    }
    // Each runs apart, so one that throws leaves the others
    for (index, found) in found.iter().enumerate() {
        let statements: Vec<String> = found.statements.iter().map(ToString::to_string).collect();
        let _ = writeln!(
            module,
            "const __value_{index}__ = (() => {{ if ([{}].some((statement) => __failed_statements__.has(statement))) return undefined; try {{ return __literal__(({})); }} catch {{ return undefined; }} }})();",
            statements.join(", "),
            &code[found.span.start as usize..found.span.end as usize]
        );
    }
    let names: Vec<String> = (0..found.len())
        .map(|index| format!("__value_{index}__"))
        .collect();
    let _ = writeln!(module, "[{}];", names.join(", "));

    let _evaluating = Evaluating::enter(filename);
    let mut loader = ModuleLoader::new(resolver, option).lenient();
    let script = module_script(
        &crate::vanilla_extract::strip_typescript(&module, filename),
        filename,
        &mut loader,
        false,
    )
    .ok()?;
    let mut context = Context::default();
    context
        .runtime_limits_mut()
        .set_loop_iteration_limit(crate::module_loader::LOOP_ITERATION_LIMIT);
    let values = context
        .eval(Source::from_bytes(
            format!(
                "{}{PRELUDE}{}{}",
                crate::module_loader::CONSOLE,
                loader.prelude(),
                script.body
            )
            .as_bytes(),
        ))
        .ok()?;
    let values = values.as_object().filter(JsObject::is_array)?;
    let mut computed = Vec::new();
    for (index, found) in found.into_iter().enumerate() {
        let Some(literal) = values
            .get(index, &mut context)
            .ok()?
            .as_string()
            .map(|literal| literal.to_std_string_escaped())
        else {
            continue;
        };
        if found.rules_only && !literal.starts_with(['{', '[']) {
            continue;
        }
        computed.push((
            found.span,
            match found.shorthand {
                Some(key) => format!("{key}: {literal}"),
                None => literal,
            },
        ));
    }
    (!computed.is_empty()).then_some((computed, loader.dependencies))
}
