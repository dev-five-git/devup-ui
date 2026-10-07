use oxc_allocator::Allocator;
use oxc_ast::ast::{CallExpression, Expression, ImportDeclarationSpecifier, Statement};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_semantic::{Scoping, SemanticBuilder};
use oxc_span::SourceType;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;

mod dispatch;
mod edits;
mod emission;
pub(crate) mod execution;
pub(crate) mod lowering;
mod prepare;
mod readback;
mod rewrite;
pub(crate) mod selection;
pub(crate) use prepare::{Prepared, prepare};

pub(crate) const APIS: [&str; 16] = [
    "style",
    "globalStyle",
    "styleVariants",
    "keyframes",
    "fontFace",
    "globalFontFace",
    "createVar",
    "fallbackVar",
    "createContainer",
    "layer",
    "globalLayer",
    "createThemeContract",
    "createGlobalThemeContract",
    "assignVars",
    "createTheme",
    "createGlobalTheme",
];

pub(crate) fn is_module(filename: &str, code: &str) -> bool {
    let allocator = Allocator::default();
    let parsed = Parser::new(
        &allocator,
        code,
        SourceType::from_path(filename).unwrap_or_default(),
    )
    .parse();
    if !parsed.diagnostics.is_empty() {
        return false;
    }
    let semantic = SemanticBuilder::new().build(&parsed.program).semantic;
    let scoping = semantic.scoping();
    if scoping.root_unresolved_references().keys().any(|name| {
        !matches!(
            name.as_str(),
            "undefined"
                | "NaN"
                | "Infinity"
                | "Math"
                | "String"
                | "Number"
                | "Boolean"
                | "Array"
                | "Object"
                | "JSON"
                | "parseInt"
                | "parseFloat"
                | "isNaN"
                | "isFinite"
                | "encodeURIComponent"
                | "decodeURIComponent"
                | "encodeURI"
                | "decodeURI"
        )
    }) {
        return false;
    }
    let mut apis = FxHashSet::default();
    let mut namespaces = FxHashSet::default();
    for statement in &parsed.program.body {
        if let Statement::ImportDeclaration(import) = statement {
            if import.import_kind.is_type() {
                continue;
            }
            if import.source.value == "@vanilla-extract/css" {
                for specifier in import.specifiers.iter().flatten() {
                    match specifier {
                        ImportDeclarationSpecifier::ImportSpecifier(specifier)
                            if !specifier.import_kind.is_type()
                                && APIS.contains(&specifier.imported.name().as_str()) =>
                        {
                            apis.extend(specifier.local.symbol_id.get());
                        }
                        ImportDeclarationSpecifier::ImportNamespaceSpecifier(specifier) => {
                            namespaces.extend(specifier.local.symbol_id.get());
                        }
                        _ => {}
                    }
                }
            } else if !import.source.value.starts_with('.')
                && !import.source.value.starts_with('/')
                && !import.source.value.starts_with("@/")
            {
                return false;
            }
        }
    }
    if apis.is_empty() && namespaces.is_empty() {
        return false;
    }
    let mut reader = Reader {
        scoping,
        apis,
        namespaces,
        jsx: false,
        call: false,
        api_depth: 0,
    };
    for statement in &parsed.program.body {
        let declaration = match statement {
            Statement::VariableDeclaration(declaration) => Some(declaration.as_ref()),
            Statement::ExportDeclaration(export) => match &export.declaration {
                oxc_ast::ast::Declaration::VariableDeclaration(declaration) => {
                    Some(declaration.as_ref())
                }
                oxc_ast::ast::Declaration::FunctionDeclaration(_)
                | oxc_ast::ast::Declaration::ClassDeclaration(_) => return false,
                _ => None,
            },
            _ => None,
        };
        if declaration.is_some_and(|declaration| {
            declaration.declarations.iter().any(|declarator| {
                matches!(
                    declarator.init.as_ref(),
                    Some(
                        Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_)
                    )
                )
            })
        }) {
            return false;
        }
        match statement {
            Statement::ImportDeclaration(_)
            | Statement::VariableDeclaration(_)
            | Statement::ExportDeclaration(_)
            | Statement::ExportNamedDeclaration(_)
            | Statement::ExportFromDeclaration(_)
            | Statement::ExportAllDeclaration(_)
            | Statement::ExportDefaultDeclaration(_)
            | Statement::TSTypeAliasDeclaration(_)
            | Statement::TSInterfaceDeclaration(_)
            | Statement::EmptyStatement(_) => {}
            Statement::ExpressionStatement(statement) => {
                if !matches!(&statement.expression, Expression::CallExpression(call) if reader.is_api(&call.callee))
                {
                    return false;
                }
            }
            _ => return false,
        }
    }
    reader.visit_program(&parsed.program);
    reader.call && !reader.jsx
}

#[cfg(test)]
mod tests;

struct Reader<'s> {
    scoping: &'s Scoping,
    apis: FxHashSet<SymbolId>,
    namespaces: FxHashSet<SymbolId>,
    jsx: bool,
    call: bool,
    api_depth: usize,
}

impl Reader<'_> {
    fn symbol(&self, expression: &Expression<'_>) -> Option<SymbolId> {
        let Expression::Identifier(identifier) = expression else {
            return None;
        };
        self.scoping
            .get_reference(identifier.reference_id.get()?)
            .symbol_id()
    }

    fn is_api(&self, expression: &Expression<'_>) -> bool {
        match expression {
            Expression::Identifier(_) => self
                .symbol(expression)
                .is_some_and(|symbol| self.apis.contains(&symbol)),
            Expression::StaticMemberExpression(member) => {
                self.symbol(&member.object)
                    .is_some_and(|symbol| self.namespaces.contains(&symbol))
                    && APIS.contains(&member.property.name.as_str())
            }
            _ => false,
        }
    }
}

impl<'a> Visit<'a> for Reader<'_> {
    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        let api = self.is_api(&call.callee);
        self.call |= api;
        self.api_depth += usize::from(api);
        walk::walk_call_expression(self, call);
        self.api_depth -= usize::from(api);
    }

    fn visit_arrow_function_expression(
        &mut self,
        function: &oxc_ast::ast::ArrowFunctionExpression<'a>,
    ) {
        if self.api_depth == 0 {
            self.jsx = true;
        } else {
            walk::walk_arrow_function_expression(self, function);
        }
    }

    fn visit_function(
        &mut self,
        function: &oxc_ast::ast::Function<'a>,
        flags: oxc_syntax::scope::ScopeFlags,
    ) {
        if self.api_depth == 0 {
            self.jsx = true;
        } else {
            walk::walk_function(self, function, flags);
        }
    }

    fn visit_jsx_element(&mut self, _: &oxc_ast::ast::JSXElement<'a>) {
        self.jsx = true;
    }

    fn visit_jsx_fragment(&mut self, _: &oxc_ast::ast::JSXFragment<'a>) {
        self.jsx = true;
    }
}
