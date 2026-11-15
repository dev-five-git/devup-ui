//! `calc` of `@vanilla-extract/css-utils`, compiled to the strings the
//! package builds.
//!
//! A `.css.ts` stylesheet is evaluated, and evaluates the package's own
//! `calc` (see `module_loader::css_utils`). Any other module is not, so each
//! `calc` there is written as the string it gives, which only literal
//! operands let the build know; whatever else reads `calc` is a build error.

use oxc_allocator::Allocator;
use oxc_ast::ast::{
    Argument, BindingPattern, CallExpression, Declaration, Expression, IdentifierReference,
    ImportDeclaration, ImportDeclarationSpecifier, Program, Statement, TaggedTemplateExpression,
    TemplateLiteral, VariableDeclaration, VariableDeclarationKind,
};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_semantic::{Scoping, SemanticBuilder};
use oxc_span::GetSpan;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::HashMap;

use crate::ImportAlias;
pub(crate) use crate::module_loader::css_utils::CSS_UTILS;
use crate::utils::{
    get_string_by_literal_expression, is_vanilla_extract_file, readable_code, unwrap_syntax_only,
};
use crate::vanilla_extract::json_string;

#[cfg(test)]
#[path = "vanilla_extract_calc_tests.rs"]
mod tests;

type Error = (u32, String);

/// What compiling `calc` writes: the replacements of the source it makes,
/// and the errors where it cannot
#[derive(Default)]
pub(crate) struct Compiled {
    pub replacements: Vec<(usize, usize, String)>,
    pub errors: Vec<Error>,
}

/// What a `calc` expression gives: a string, or the chainable expression
/// `calc(x)` builds, which is `x` as a string
#[derive(Debug, PartialEq)]
enum Value {
    Text(String),
    Chain(String),
}

impl Value {
    fn text(self) -> String {
        match self {
            Self::Text(text) | Self::Chain(text) => text,
        }
    }
}

/// The operator of the `calc` operation `name`
fn operator(name: &str) -> Option<&'static str> {
    match name {
        "add" => Some("+"),
        "subtract" => Some("-"),
        "multiply" => Some("*"),
        "divide" => Some("/"),
        _ => None,
    }
}

/// `calc.add(...)` and its siblings as the package writes them: the operands
/// joined by the operator, with every `calc` in them dropped
fn operation(operator: &str, operands: &[String]) -> String {
    let joined = operands.join(&format!(" {operator} ")).replace("calc", "");
    format!("calc({joined})")
}

fn error(expression: &Expression<'_>, requirement: &str) -> Error {
    (
        expression.span().start,
        format!(
            "`{CSS_UTILS}` cannot compile `{}` at build time outside a `.css.ts` stylesheet: {requirement}. Fix: write it with literal operands, as `calc.add('100%', '-4px')` or `calc('4px').multiply(2).toString()`, or build it in a `.css.ts` stylesheet, where vanilla-extract values are evaluated",
            readable_code(expression)
        ),
    )
}

fn chain_error(expression: &Expression<'_>) -> Error {
    (
        expression.span().start,
        format!(
            "`{CSS_UTILS}` cannot use `{}` as a value: `calc(x)` builds an expression, which is a string only once `.toString()` is called or it is interpolated in a template. Fix: call `.toString()` on it, as `calc(x).multiply(2).toString()`",
            readable_code(expression)
        ),
    )
}

fn import_error(at: u32, requirement: &str) -> Error {
    (
        at,
        format!(
            "`{CSS_UTILS}` {requirement}. Fix: import `calc` by name, as `import {{ calc }} from '{CSS_UTILS}'`, or set `importAliases: {{ '{CSS_UTILS}': false }}`"
        ),
    )
}

/// Whether `code` may read `@vanilla-extract/css-utils`, which the build
/// compiles while it is aliased
pub(crate) fn reads(code: &str, import_aliases: &HashMap<String, ImportAlias>) -> bool {
    import_aliases.contains_key(CSS_UTILS) && code.contains(CSS_UTILS)
}

/// Where `code` of `filename` uses `@vanilla-extract/css-utils` in a way the
/// build cannot compile
pub(crate) fn errors(
    code: &str,
    filename: &str,
    import_aliases: &HashMap<String, ImportAlias>,
) -> Vec<Error> {
    if !reads(code, import_aliases) {
        return Vec::new();
    }
    let allocator = Allocator::default();
    let source_type = crate::parsing::source_type(filename).unwrap_or_default();
    let program = Parser::new(&allocator, code, source_type).parse().program;
    let scoping = SemanticBuilder::new()
        .build(&program)
        .semantic
        .into_scoping();
    compile(&program, &scoping, is_vanilla_extract_file(filename)).errors
}

/// The declaration `statement` makes of variables, exported or not
fn variables<'s, 'a>(statement: &'s Statement<'a>) -> Option<&'s VariableDeclaration<'a>> {
    match statement {
        Statement::VariableDeclaration(declaration) => Some(declaration),
        Statement::ExportDeclaration(export) => match &export.declaration {
            Declaration::VariableDeclaration(declaration) => Some(declaration),
            _ => None,
        },
        _ => None,
    }
}

/// The strings the module-level `const`s of `program` hold as literals,
/// which no code can change
fn constants(program: &Program<'_>) -> FxHashMap<SymbolId, String> {
    program
        .body
        .iter()
        .filter_map(variables)
        .filter(|declaration| declaration.kind == VariableDeclarationKind::Const)
        .flat_map(|declaration| &declaration.declarations)
        .filter_map(|declarator| match (&declarator.id, &declarator.init) {
            (BindingPattern::BindingIdentifier(id), Some(init)) => Some((
                id.symbol_id.get()?,
                get_string_by_literal_expression(unwrap_syntax_only(init))?.into_owned(),
            )),
            _ => None,
        })
        .collect()
}

/// Compile the `calc` of `program`, whose symbols `scoping` resolves; a
/// `stylesheet` evaluates it, so only what it imports is checked there
pub(crate) fn compile(program: &Program<'_>, scoping: &Scoping, stylesheet: bool) -> Compiled {
    let mut compiler = Compiler {
        scoping,
        calc: FxHashSet::default(),
        constants: constants(program),
        interpolated: false,
        compiled: Compiled::default(),
    };
    for statement in &program.body {
        if let Statement::ImportDeclaration(import) = statement
            && import.source.value == CSS_UTILS
            && !import.import_kind.is_type()
        {
            compiler.import(import, stylesheet);
        }
    }
    if stylesheet {
        return compiler.compiled;
    }
    let imports: FxHashSet<u32> = program
        .body
        .iter()
        .filter_map(|statement| match statement {
            Statement::ImportDeclaration(import) => Some(import.span.start),
            _ => None,
        })
        .collect();
    for (at, _) in crate::module_loads::module_loads(program, &[CSS_UTILS]) {
        if !imports.contains(&at) {
            compiler.compiled.errors.push(import_error(
                at,
                "is loaded where the build cannot read `calc`",
            ));
        }
    }
    compiler.visit_program(program);
    compiler.compiled
}

struct Compiler<'s> {
    scoping: &'s Scoping,
    /// The import bindings of `calc`
    calc: FxHashSet<SymbolId>,
    /// The strings module-level `const`s hold as literals
    constants: FxHashMap<SymbolId, String>,
    /// Whether the expression visited next is interpolated in a template,
    /// which reads it as a string
    interpolated: bool,
    compiled: Compiled,
}

impl Compiler<'_> {
    fn import(&mut self, import: &ImportDeclaration<'_>, stylesheet: bool) {
        for specifier in import.specifiers.iter().flatten() {
            match specifier {
                ImportDeclarationSpecifier::ImportSpecifier(named)
                    if named.import_kind.is_type() => {}
                ImportDeclarationSpecifier::ImportSpecifier(named)
                    if named.imported.name() == "calc" =>
                {
                    self.calc.extend(named.local.symbol_id.get());
                }
                ImportDeclarationSpecifier::ImportNamespaceSpecifier(_) if stylesheet => {}
                ImportDeclarationSpecifier::ImportNamespaceSpecifier(namespace) => {
                    self.compiled.errors.push(import_error(
                        namespace.span.start,
                        "is imported whole where the build reads only `calc` imported by name",
                    ));
                }
                specifier => {
                    let name = match specifier {
                        ImportDeclarationSpecifier::ImportSpecifier(named) => {
                            named.imported.name().to_string()
                        }
                        _ => "default".to_string(),
                    };
                    self.compiled.errors.push(import_error(
                        specifier.span().start,
                        &format!("has no export `{name}`: it exports only `calc`"),
                    ));
                }
            }
        }
        if !stylesheet {
            self.compiled.replacements.push((
                import.span.start as usize,
                import.span.end as usize,
                String::new(),
            ));
        }
    }

    fn is_calc(&self, identifier: &IdentifierReference<'_>) -> bool {
        identifier
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
            .is_some_and(|symbol| self.calc.contains(&symbol))
    }

    /// The string `expression` reads when it names a constant of the module
    fn constant(&self, expression: &Expression<'_>) -> Option<&String> {
        let Expression::Identifier(identifier) = unwrap_syntax_only(expression) else {
            return None;
        };
        let reference = self.scoping.get_reference(identifier.reference_id.get()?);
        self.constants.get(&reference.symbol_id()?)
    }

    /// Whether `expression` reads a binding of `calc`
    fn reads_calc(&self, expression: &Expression<'_>) -> bool {
        matches!(unwrap_syntax_only(expression), Expression::Identifier(identifier) if self.is_calc(identifier))
    }

    /// What `expression` gives when it is a call of `calc`, `calc.add`, ...
    /// or of a method of what `calc(x)` gives
    fn evaluate(&mut self, expression: &Expression<'_>) -> Option<Result<Value, Error>> {
        let Expression::CallExpression(call) = unwrap_syntax_only(expression) else {
            return None;
        };
        if self.reads_calc(&call.callee) {
            return Some(self.chain(expression, call));
        }
        let Expression::StaticMemberExpression(member) = unwrap_syntax_only(&call.callee) else {
            return None;
        };
        let name = member.property.name.as_str();
        if self.reads_calc(&member.object) {
            return Some(self.static_operation(expression, name, call));
        }
        let object = self.evaluate(&member.object)?;
        Some(object.and_then(|object| self.method(expression, object, name, call)))
    }

    /// `calc(x)`
    fn chain(
        &mut self,
        expression: &Expression<'_>,
        call: &CallExpression<'_>,
    ) -> Result<Value, Error> {
        let [operand] = self
            .operands(expression, &call.arguments)?
            .try_into()
            .map_err(|_| {
                error(
                    expression,
                    "`calc()` takes the one value it builds an expression of",
                )
            })?;
        Ok(Value::Chain(operand))
    }

    /// `calc.add(...)`, `calc.negate(x)`, ...
    fn static_operation(
        &mut self,
        expression: &Expression<'_>,
        name: &str,
        call: &CallExpression<'_>,
    ) -> Result<Value, Error> {
        let operands = self.operands(expression, &call.arguments)?;
        if let Some(operator) = operator(name) {
            return Ok(Value::Text(operation(operator, &operands)));
        }
        match (name, operands.as_slice()) {
            ("negate", [operand]) => {
                Ok(Value::Text(operation("*", &[operand.clone(), "-1".into()])))
            }
            ("negate", _) => Err(error(
                expression,
                "`calc.negate()` takes the one value it negates",
            )),
            _ => Err(error(expression, &format!("`calc` has no `{name}`"))),
        }
    }

    /// `name` called on `object`, which `calc(x)` or a string gives
    fn method(
        &mut self,
        expression: &Expression<'_>,
        object: Value,
        name: &str,
        call: &CallExpression<'_>,
    ) -> Result<Value, Error> {
        let operands = self.operands(expression, &call.arguments)?;
        match (object, name, operator(name)) {
            (object, "toString", _) if operands.is_empty() => Ok(Value::Text(object.text())),
            (Value::Chain(x), "negate", _) if operands.is_empty() => {
                Ok(Value::Chain(operation("*", &[x, "-1".into()])))
            }
            (Value::Chain(x), _, Some(operator)) => {
                let operands: Vec<String> = std::iter::once(x).chain(operands).collect();
                Ok(Value::Chain(operation(operator, &operands)))
            }
            _ => Err(error(
                expression,
                &format!(
                    "`.{name}()` is not one the build compiles, which are `add`, `subtract`, `multiply`, `divide` and `negate` of `calc(x)`, and `toString()`"
                ),
            )),
        }
    }

    /// The strings `arguments` pass as operands
    fn operands(
        &mut self,
        expression: &Expression<'_>,
        arguments: &[Argument<'_>],
    ) -> Result<Vec<String>, Error> {
        arguments
            .iter()
            .map(|argument| {
                let operand = argument
                    .as_expression()
                    .ok_or_else(|| error(expression, "its operands cannot be spread"))?;
                self.operand(operand)
            })
            .collect()
    }

    /// The string `operand` gives, as the package concatenates it
    fn operand(&mut self, operand: &Expression<'_>) -> Result<String, Error> {
        if let Some(text) = get_string_by_literal_expression(unwrap_syntax_only(operand)) {
            return Ok(text.into_owned());
        }
        if let Some(text) = self.constant(operand) {
            return Ok(text.clone());
        }
        self.evaluate(operand).map_or_else(
            || {
                Err(error(
                    operand,
                    "its operands must be literals, module-level `const`s holding a literal, or other `calc` results",
                ))
            },
            |value| value.map(Value::text),
        )
    }
}

impl<'a> Visit<'a> for Compiler<'_> {
    fn visit_expression(&mut self, expression: &Expression<'a>) {
        let interpolated = std::mem::take(&mut self.interpolated);
        match self.evaluate(expression) {
            Some(Ok(Value::Chain(_))) if !interpolated => {
                self.compiled.errors.push(chain_error(expression));
            }
            Some(Ok(value)) => {
                let span = unwrap_syntax_only(expression).span();
                self.compiled.replacements.push((
                    span.start as usize,
                    span.end as usize,
                    json_string(&value.text()),
                ));
            }
            Some(Err(error)) => self.compiled.errors.push(error),
            None => walk::walk_expression(self, expression),
        }
    }

    fn visit_template_literal(&mut self, template: &TemplateLiteral<'a>) {
        for expression in &template.expressions {
            self.interpolated = true;
            self.visit_expression(expression);
        }
    }

    fn visit_tagged_template_expression(&mut self, tagged: &TaggedTemplateExpression<'a>) {
        // A tag takes the interpolated values as they are
        self.visit_expression(&tagged.tag);
        for expression in &tagged.quasi.expressions {
            self.visit_expression(expression);
        }
    }

    /// Types are erased, so what they read stays
    fn visit_ts_type(&mut self, _: &oxc_ast::ast::TSType<'a>) {}

    /// `calc` read by what the build does not compile, which a call it
    /// compiles never walks into
    fn visit_identifier_reference(&mut self, identifier: &IdentifierReference<'a>) {
        if self.is_calc(identifier) {
            self.compiled.errors.push((
                identifier.span.start,
                format!(
                    "`calc` of `{CSS_UTILS}` is read at runtime, where it does not exist: the build compiles it only where it is called. Fix: call `calc.add(...)`, `calc.negate(...)` or `calc(x)` with its methods and `.toString()` where the value is used"
                ),
            ));
        }
    }
}
