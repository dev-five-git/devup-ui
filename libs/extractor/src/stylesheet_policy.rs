//! Which stylesheets (`.css.ts`, `.css.js`) the build extracts as they are
//! written, and which it runs.
//!
//! A stylesheet the build runs has its effects at build time, so extracting it
//! without running it keeps its meaning only when it has none but the ones the
//! extraction compiles: it only declares constants of plain data, creates
//! styles with Devup UI's own `css`, `globalCss`, `keyframes` and `styled`,
//! and exports them. The proof reads the module and nothing runs: any other
//! statement, a call of anything else, an accessor, a mutation or a binding
//! the module does not give the build as plain data is a stylesheet to run.

use oxc_ast::ast::{
    Argument, BindingPattern, CallExpression, Declaration, ExportDefaultDeclarationKind,
    ExportSpecifier, Expression, IdentifierReference, ImportDeclaration,
    ImportDeclarationSpecifier, ModuleExportName, Statement, VariableDeclaration,
    VariableDeclarationKind,
};
use oxc_semantic::Scoping;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::util_type::UtilType;
use crate::utils::unwrap_syntax_only;

#[cfg(test)]
#[path = "stylesheet_policy_tests.rs"]
mod tests;

#[path = "stylesheet_policy_values.rs"]
mod values;
use values::Value;

#[path = "stylesheet_policy_dispatch.rs"]
mod dispatch;
pub(crate) use dispatch::{imports_plain, plan};

/// What the build does with a stylesheet
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Plan {
    /// Extract it as written: it has no effect the extraction does not compile
    Plain,
    /// Run it, and fail when running it fails
    Run,
}

/// The members of `Math` the build folds, which the extraction computes the
/// way every engine does
const MATH_FOLDED: [&str; 10] = [
    "abs", "ceil", "floor", "trunc", "sqrt", "sign", "round", "max", "min", "pow",
];

const MATH_CONSTANTS: [&str; 8] = [
    "PI", "E", "LN2", "LN10", "LOG2E", "LOG10E", "SQRT2", "SQRT1_2",
];

/// What the module proves, one statement at a time
struct Proof<'s> {
    scoping: &'s Scoping,
    /// Where the package's style APIs are bound, by the API they bind
    apis: FxHashMap<SymbolId, String>,
    namespaces: FxHashSet<SymbolId>,
    /// Bindings of plain data or of what the styles create, declared so far:
    /// reading one declared later reads it before it is initialized
    values: FxHashMap<SymbolId, Value>,
}

impl Proof<'_> {
    fn symbol(&self, identifier: &IdentifierReference<'_>) -> Option<SymbolId> {
        self.scoping
            .get_reference(identifier.reference_id.get()?)
            .symbol_id()
    }

    /// Whether `expression` names the global `name`, which nothing declares
    fn is_global(&self, expression: &Expression<'_>, name: &str) -> bool {
        matches!(expression, Expression::Identifier(identifier)
            if identifier.name == name && self.symbol(identifier).is_none())
    }

    fn api<'a>(&'a self, expression: &'a Expression<'_>) -> Option<&'a str> {
        match unwrap_syntax_only(expression) {
            Expression::Identifier(identifier) => {
                self.apis.get(&self.symbol(identifier)?).map(String::as_str)
            }
            Expression::StaticMemberExpression(member) => {
                let Expression::Identifier(identifier) = &member.object else {
                    return None;
                };
                let name = member.property.name.as_str();
                (self.namespaces.contains(&self.symbol(identifier)?)
                    && (name == "styled" || UtilType::from_str_opt(name).is_some()))
                .then_some(name)
            }
            _ => None,
        }
    }

    /// `styled`, `styled.div`, `styled(Link)`, `styled.div.attrs({})`
    fn styled(&self, expression: &Expression<'_>) -> bool {
        if self.api(expression) == Some("styled") {
            return true;
        }
        match unwrap_syntax_only(expression) {
            Expression::StaticMemberExpression(member) => self.styled(&member.object),
            Expression::CallExpression(call) => {
                self.styled(&call.callee) && self.arguments(&call.arguments)
            }
            callee => self.api(callee) == Some("styled"),
        }
    }

    /// Whether `callee` is a style API the build compiles
    fn creates(&self, callee: &Expression<'_>) -> bool {
        let callee = unwrap_syntax_only(callee);
        self.api(callee).is_some() || self.styled(callee)
    }

    fn arguments(&self, arguments: &[Argument<'_>]) -> bool {
        arguments.iter().all(|argument| match argument {
            Argument::SpreadElement(spread) => self
                .value(&spread.argument)
                .is_some_and(|value| value.iterable()),
            argument => self.pure(argument.to_expression()),
        })
    }

    /// A style API called, or its template written, over plain data
    fn creation(&self, expression: &Expression<'_>) -> bool {
        match unwrap_syntax_only(expression) {
            Expression::CallExpression(call) => {
                self.creates(&call.callee) && self.arguments(&call.arguments)
            }
            Expression::TaggedTemplateExpression(tagged) => {
                self.creates(&tagged.tag)
                    && tagged
                        .quasi
                        .expressions
                        .iter()
                        .all(|expression| self.pure(expression))
            }
            _ => false,
        }
    }

    /// A `Math` function over plain data, which the build folds
    fn folds(&self, call: &CallExpression<'_>) -> bool {
        matches!(unwrap_syntax_only(&call.callee), Expression::StaticMemberExpression(member)
            if self.is_global(&member.object, "Math")
                && MATH_FOLDED.contains(&member.property.name.as_str()))
            && self.arguments(&call.arguments)
    }

    /// Whether reading `expression` has no effect and gives the same on every
    /// build: plain data, what the module declared before it, and what the
    /// build folds. A function, an accessor, a call of anything else or a
    /// global is not.
    fn pure(&self, expression: &Expression<'_>) -> bool {
        self.value(expression).is_some_and(|value| value.data())
    }

    /// Binds what `import` gives, when it gives only what the build knows: the
    /// package's style APIs, types, and constants of plain data
    fn import(
        &mut self,
        import: &ImportDeclaration<'_>,
        packages: [&str; 2],
        known: &dyn Fn(&str) -> Option<Value>,
    ) -> bool {
        if import.import_kind.is_type() {
            return true;
        }
        let specifiers = import.specifiers.as_deref().map_or(&[][..], |s| s);
        let of_package = packages.contains(&import.source.value.as_str());
        !specifiers.is_empty()
            && specifiers.iter().all(|specifier| match specifier {
                ImportDeclarationSpecifier::ImportSpecifier(specifier)
                    if specifier.import_kind.is_type() =>
                {
                    true
                }
                ImportDeclarationSpecifier::ImportSpecifier(specifier) if of_package => {
                    let name = specifier.imported.name();
                    let name = name.as_str();
                    (name == "styled" || UtilType::from_str_opt(name).is_some())
                        && specifier.local.symbol_id.get().is_some_and(|symbol| {
                            self.apis.insert(symbol, name.to_string());
                            true
                        })
                }
                ImportDeclarationSpecifier::ImportNamespaceSpecifier(specifier) if of_package => {
                    specifier.local.symbol_id.get().is_some_and(|symbol| {
                        self.namespaces.insert(symbol);
                        true
                    })
                }
                specifier => {
                    !of_package
                        && known(specifier.local().name.as_str()).is_some()
                        && specifier.local().symbol_id.get().is_some_and(|symbol| {
                            if let Some(value) = known(specifier.local().name.as_str()) {
                                self.values.insert(symbol, value);
                            }
                            true
                        })
                }
            })
    }

    /// Declares the constants of `declaration`, each of what the build reads
    fn constants(&mut self, declaration: &VariableDeclaration<'_>) -> bool {
        if declaration.declare {
            return true;
        }
        declaration.kind == VariableDeclarationKind::Const
            && declaration.declarations.iter().all(|declarator| {
                let (BindingPattern::BindingIdentifier(identifier), Some(init)) =
                    (&declarator.id, &declarator.init)
                else {
                    return false;
                };
                match (self.value(init), identifier.symbol_id.get()) {
                    (Some(value), Some(symbol)) => {
                        self.values.insert(symbol, value);
                        true
                    }
                    _ => false,
                }
            })
    }

    fn declaration(&mut self, declaration: &Declaration<'_>) -> bool {
        match declaration {
            Declaration::VariableDeclaration(declaration) => self.constants(declaration),
            Declaration::FunctionDeclaration(function) => self.function(function),
            Declaration::TSTypeAliasDeclaration(_) | Declaration::TSInterfaceDeclaration(_) => true,
            _ => false,
        }
    }

    fn function(&mut self, function: &oxc_ast::ast::Function<'_>) -> bool {
        if let Some(symbol) = function.id.as_ref().and_then(|id| id.symbol_id.get()) {
            self.values.insert(symbol, Value::Function);
        }
        true
    }

    fn export(&self, specifier: &ExportSpecifier<'_>) -> bool {
        specifier.export_kind.is_type()
            || matches!(&specifier.local, ModuleExportName::IdentifierReference(identifier)
                if self.symbol(identifier).is_some_and(|symbol| self.values.contains_key(&symbol)))
    }

    /// Whether running `statement` has no effect but the ones the build compiles
    fn statement(&mut self, statement: &Statement<'_>) -> bool {
        match statement {
            Statement::EmptyStatement(_)
            | Statement::ImportDeclaration(_)
            | Statement::TSTypeAliasDeclaration(_)
            | Statement::TSInterfaceDeclaration(_) => true,
            Statement::ExpressionStatement(statement) => self.creation(&statement.expression),
            Statement::VariableDeclaration(declaration) => self.constants(declaration),
            Statement::FunctionDeclaration(function) => self.function(function),
            Statement::ExportDeclaration(export) => self.declaration(&export.declaration),
            Statement::ExportNamedDeclaration(export) => {
                export.export_kind.is_type()
                    || export
                        .specifiers
                        .iter()
                        .all(|specifier| self.export(specifier))
            }
            Statement::ExportDefaultDeclaration(export) => match &export.declaration {
                ExportDefaultDeclarationKind::TSInterfaceDeclaration(_) => true,
                ExportDefaultDeclarationKind::FunctionDeclaration(function) => {
                    self.function(function)
                }
                ExportDefaultDeclarationKind::ClassDeclaration(_) => false,
                declaration => self.pure(declaration.to_expression()),
            },
            _ => false,
        }
    }
}

#[cfg(test)]
mod coverage_tests;
