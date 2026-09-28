//! Values the styles of a file compute when the module runs, such as
//! `css({ color: darken(0.1, PRIMARY) })`: the APIs taking them have no element
//! to set a runtime value on, so the full engine runs the code they read and
//! writes what it gives in their place.

use std::collections::BTreeSet;

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    Declaration, ExportDefaultDeclarationKind, Expression, ImportDeclarationSpecifier,
    ObjectPropertyKind, Program, Statement, VariableDeclarationKind,
};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_semantic::{Scoping, SemanticBuilder};
use oxc_span::{GetSpan, SourceType, Span};
use oxc_syntax::reference::ReferenceId;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::stylex::StylexFunction;
use crate::utils::get_string_by_literal_expression;

/// What an expression of the source computes, by the span of that expression
#[cfg(feature = "vanilla-extract")]
type Values = FxHashMap<(u32, u32), Value>;

#[cfg(feature = "vanilla-extract")]
enum Value {
    String(String),
    Number(f64),
}

/// An expression whose reads the module can run on its own
#[cfg_attr(not(feature = "vanilla-extract"), allow(dead_code))]
struct Found {
    span: Span,
    /// The top-level statements it reads, directly or through each other
    statements: BTreeSet<usize>,
    /// Whether it calls code, which only running the module can compute
    computes: bool,
}

const UTILS: [&str; 4] = ["css", "globalCss", "keyframes", "createGlobalStyle"];

/// Whether `code` has a style value that only running the module computes
#[must_use]
pub fn has_build_time_values(
    filename: &str,
    code: &str,
    package: &str,
    alias_sources: &[String],
) -> bool {
    let allocator = Allocator::default();
    let Some(program) = parse(&allocator, filename, code) else {
        return false;
    };
    let is_style = |source: &str| {
        source.starts_with(package) || alias_sources.iter().any(|alias| alias == source)
    };
    find(&program, &is_style).iter().any(|found| found.computes)
}

fn parse<'a>(allocator: &'a Allocator, filename: &str, code: &'a str) -> Option<Program<'a>> {
    let source_type = SourceType::from_path(filename).ok()?;
    let parsed = Parser::new(allocator, code, source_type).parse();
    (!parsed.fatal_error).then_some(parsed.program)
}

fn find(program: &Program<'_>, is_style: &dyn Fn(&str) -> bool) -> Vec<Found> {
    let scoping = SemanticBuilder::new()
        .build(program)
        .semantic
        .into_scoping();
    let mut finder = Finder::new(program, &scoping, is_style);
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
    ) -> Self {
        let mut finder = Self {
            scoping,
            statements: &program.body,
            bindings: FxHashMap::default(),
            apis: FxHashSet::default(),
            namespaces: FxHashSet::default(),
            stylex_namespaces: FxHashSet::default(),
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
            Statement::VariableDeclaration(declaration) => {
                for declarator in &declaration.declarations {
                    for identifier in declarator.id.get_binding_identifiers() {
                        declare(
                            identifier.symbol_id.get(),
                            declaration.kind == VariableDeclarationKind::Const,
                        );
                    }
                }
                return;
            }
            Statement::FunctionDeclaration(function) => {
                declare(function.id.as_ref().and_then(|id| id.symbol_id.get()), true);
                return;
            }
            Statement::ClassDeclaration(class) => {
                declare(class.id.as_ref().and_then(|id| id.symbol_id.get()), true);
                return;
            }
            _ => return,
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

    /// The values `expression` holds: itself, or those of the object or array
    /// it writes
    fn values(&mut self, expression: &Expression<'a>) {
        match crate::utils::unwrap_syntax_only(expression) {
            Expression::ObjectExpression(object) => {
                for property in &object.properties {
                    if let ObjectPropertyKind::ObjectProperty(property) = property {
                        self.values(&property.value);
                    }
                }
            }
            Expression::ArrayExpression(array) => {
                for element in &array.elements {
                    if let Some(element) = element.as_expression() {
                        self.values(element);
                    }
                }
            }
            Expression::NullLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::ArrowFunctionExpression(_)
            | Expression::FunctionExpression(_) => {}
            inner if get_string_by_literal_expression(inner).is_some() => {}
            _ => {
                let span = expression.span();
                let mut reads = Reads::new(self.scoping);
                reads.visit_expression(expression);
                if !reads.opaque
                    && let Some(statements) = self.closure(span, &reads.references)
                {
                    self.found.push(Found {
                        span,
                        statements,
                        computes: reads.computes,
                    });
                }
            }
        }
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
                }) => *statement,
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
            self.closure(statement.span(), &reads.references)
        };
        self.closures.insert(index, closure.clone());
        closure
    }
}

impl<'a> Visit<'a> for Finder<'_, 'a> {
    fn visit_call_expression(&mut self, call: &oxc_ast::ast::CallExpression<'a>) {
        if self.is_api(&call.callee) {
            for argument in &call.arguments {
                if let Some(argument) = argument.as_expression() {
                    self.values(argument);
                }
            }
        }
        walk::walk_call_expression(self, call);
    }

    fn visit_tagged_template_expression(
        &mut self,
        tagged: &oxc_ast::ast::TaggedTemplateExpression<'a>,
    ) {
        if self.is_api(&tagged.tag) {
            for expression in &tagged.quasi.expressions {
                self.values(expression);
            }
        }
        walk::walk_tagged_template_expression(self, tagged);
    }
}

/// The bindings code reads, and whether it calls code
struct Reads<'s> {
    scoping: &'s Scoping,
    references: Vec<ReferenceId>,
    /// Reads what only the code around it knows: `this`, `super`, `import.meta`
    opaque: bool,
    computes: bool,
}

impl<'s> Reads<'s> {
    const fn new(scoping: &'s Scoping) -> Self {
        Self {
            scoping,
            references: Vec::new(),
            opaque: false,
            computes: false,
        }
    }

    /// `Math.x(...)`, which extraction folds without running the module
    fn is_math(&self, callee: &Expression<'_>) -> bool {
        matches!(callee, Expression::StaticMemberExpression(member)
            if matches!(&member.object, Expression::Identifier(object)
                if object.name == "Math"
                    && object
                        .reference_id
                        .get()
                        .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
                        .is_none()))
    }
}

impl<'a> Visit<'a> for Reads<'_> {
    fn visit_identifier_reference(&mut self, identifier: &oxc_ast::ast::IdentifierReference<'a>) {
        self.references.extend(identifier.reference_id.get());
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

    fn visit_call_expression(&mut self, call: &oxc_ast::ast::CallExpression<'a>) {
        self.computes |= !self.is_math(&call.callee);
        walk::walk_call_expression(self, call);
    }

    fn visit_new_expression(&mut self, new: &oxc_ast::ast::NewExpression<'a>) {
        self.computes = true;
        walk::walk_new_expression(self, new);
    }

    fn visit_tagged_template_expression(
        &mut self,
        tagged: &oxc_ast::ast::TaggedTemplateExpression<'a>,
    ) {
        self.computes = true;
        walk::walk_tagged_template_expression(self, tagged);
    }
}

/// `code` with what its style values compute written in their place, the
/// replacements made, and the files read; `None` when running the code it
/// reads computes none of them as a string or a finite number
#[cfg(feature = "vanilla-extract")]
pub(crate) fn evaluate(
    code: &str,
    filename: &str,
    option: &crate::ExtractOption,
    resolver: Option<&crate::ModuleResolver>,
) -> Option<(
    String,
    Vec<crate::import_alias_visit::Edit>,
    BTreeSet<String>,
)> {
    let (values, dependencies) = compute(code, filename, option, resolver)?;
    let mut spans: Vec<_> = values.into_iter().collect();
    spans.sort_unstable_by_key(|((start, _), _)| *start);
    let mut result = String::with_capacity(code.len());
    let mut edits = Vec::with_capacity(spans.len());
    let mut copied = 0;
    for ((start, end), value) in spans {
        let (start, end) = (start as usize, end as usize);
        let literal = match value {
            Value::String(text) => serde_json::Value::String(text).to_string(),
            Value::Number(number) => crate::utils::js_number_string(number),
        };
        result.push_str(&code[copied..start]);
        result.push_str(&literal);
        edits.push((start, end, literal.len()));
        copied = end;
    }
    result.push_str(&code[copied..]);
    Some((result, edits, dependencies))
}

#[cfg(feature = "vanilla-extract")]
fn compute(
    code: &str,
    filename: &str,
    option: &crate::ExtractOption,
    resolver: Option<&crate::ModuleResolver>,
) -> Option<(Values, BTreeSet<String>)> {
    use std::fmt::Write;

    use boa_engine::{Context, JsObject, Source};

    use crate::module_loader::{Evaluating, ModuleLoader, module_script};

    let allocator = Allocator::default();
    let program = parse(&allocator, filename, code)?;
    let is_style = |source: &str| {
        source.starts_with(option.package.as_str()) || option.import_aliases.contains_key(source)
    };
    let found: Vec<Found> = find(&program, &is_style)
        .into_iter()
        .filter(|found| found.computes)
        .collect();
    if found.is_empty() {
        return None;
    }
    let statements: BTreeSet<usize> = found
        .iter()
        .flat_map(|found| found.statements.iter().copied())
        .collect();
    let mut module = String::new();
    for index in statements {
        let span = match &program.body[index] {
            Statement::ExportDeclaration(export) => export.declaration.span(),
            Statement::ExportDefaultDeclaration(export) => export.declaration.span(),
            statement => statement.span(),
        };
        module.push_str(&code[span.start as usize..span.end as usize]);
        module.push('\n');
    }
    // Each runs apart, so one that throws leaves the others
    for (index, found) in found.iter().enumerate() {
        let _ = writeln!(
            module,
            "const __value_{index}__ = (() => {{ try {{ return ({}); }} catch {{ return undefined; }} }})();",
            &code[found.span.start as usize..found.span.end as usize]
        );
    }
    let names: Vec<String> = (0..found.len())
        .map(|index| format!("__value_{index}__"))
        .collect();
    let _ = writeln!(module, "[{}];", names.join(", "));

    let _evaluating = Evaluating::enter(filename);
    let mut loader = ModuleLoader::new(resolver, option);
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
        .set_loop_iteration_limit(10_000_000);
    // The CSS must be the same on every build
    let values = context
        .eval(Source::from_bytes(
            format!(
                "delete globalThis.Date;\nMath.random = undefined;\n{}{}",
                loader.prelude(),
                script.body
            )
            .as_bytes(),
        ))
        .ok()?;
    let values = values.as_object().filter(JsObject::is_array)?;
    let mut computed = Values::default();
    for (index, found) in found.iter().enumerate() {
        let value = values.get(index, &mut context).ok()?;
        let value = if let Some(text) = value.as_string() {
            Value::String(text.to_std_string_escaped())
        } else if let Some(number) = value.as_number().filter(|number| number.is_finite()) {
            Value::Number(number)
        } else {
            continue;
        };
        computed.insert((found.span.start, found.span.end), value);
    }
    (!computed.is_empty()).then_some((computed, loader.dependencies))
}
