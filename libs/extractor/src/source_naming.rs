//! Source-only import dependencies, before any constants are replaced.

use css::Naming;
use oxc_ast::ast::{Expression, Program};
use oxc_ast_visit::{Visit, walk};
use oxc_semantic::Scoping;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::ExtractOption;

#[cfg(test)]
#[path = "source_naming_tests.rs"]
mod tests;

fn is_package_source(source: &str, package: &str) -> bool {
    source == package
        || source
            .strip_prefix(package)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

#[derive(Default)]
struct Reads {
    names: FxHashSet<SymbolId>,
    external: bool,
}

struct Reader<'s> {
    scoping: &'s Scoping,
    option: &'s ExtractOption,
    reads: Reads,
}

impl<'a> Visit<'a> for Reader<'_> {
    fn visit_identifier_reference(&mut self, id: &oxc_ast::ast::IdentifierReference<'a>) {
        if let Some(symbol) = id
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
        {
            self.reads.names.insert(symbol);
        }
    }

    fn visit_call_expression(&mut self, call: &oxc_ast::ast::CallExpression<'a>) {
        self.reads.external |= external_call(call, self.option);
        walk::walk_call_expression(self, call);
    }

    fn visit_import_expression(&mut self, import: &oxc_ast::ast::ImportExpression<'a>) {
        self.reads.external = true;
        walk::walk_import_expression(self, import);
    }
}

struct Bindings<'s> {
    scoping: &'s Scoping,
    option: &'s ExtractOption,
    imports: FxHashSet<SymbolId>,
    dependencies: FxHashMap<SymbolId, Reads>,
    external: bool,
}

impl<'a> Visit<'a> for Bindings<'_> {
    fn visit_import_declaration(&mut self, import: &oxc_ast::ast::ImportDeclaration<'a>) {
        let source = import.source.value.as_str();
        if !is_package_source(source, &self.option.package)
            && source != crate::STYLEX_PACKAGE
            && !self.option.import_aliases.contains_key(source)
            && !import.import_kind.is_type()
        {
            self.external = true;
            for specifier in import.specifiers.iter().flatten() {
                self.imports.extend(specifier.local().symbol_id.get());
            }
        }
    }

    fn visit_variable_declarator(&mut self, declaration: &oxc_ast::ast::VariableDeclarator<'a>) {
        if let Some(init) = &declaration.init {
            for id in declaration.id.get_binding_identifiers() {
                if let Some(symbol) = id.symbol_id.get() {
                    let mut reader = Reader {
                        scoping: self.scoping,
                        option: self.option,
                        reads: Reads::default(),
                    };
                    reader.visit_expression(init);
                    self.external |= reader.reads.external;
                    self.dependencies.insert(symbol, reader.reads);
                }
            }
        }
        walk::walk_variable_declarator(self, declaration);
    }

    fn visit_function(
        &mut self,
        function: &oxc_ast::ast::Function<'a>,
        flags: oxc_syntax::scope::ScopeFlags,
    ) {
        if let Some(symbol) = function.id.as_ref().and_then(|id| id.symbol_id.get()) {
            let mut reader = Reader {
                scoping: self.scoping,
                option: self.option,
                reads: Reads::default(),
            };
            reader.visit_function(function, flags);
            self.external |= reader.reads.external;
            self.dependencies.insert(symbol, reader.reads);
        }
        walk::walk_function(self, function, flags);
    }

    fn visit_call_expression(&mut self, call: &oxc_ast::ast::CallExpression<'a>) {
        self.external |= external_call(call, self.option);
        walk::walk_call_expression(self, call);
    }

    fn visit_import_expression(&mut self, import: &oxc_ast::ast::ImportExpression<'a>) {
        self.external = true;
        walk::walk_import_expression(self, import);
    }
}

fn external_call(call: &oxc_ast::ast::CallExpression<'_>, option: &ExtractOption) -> bool {
    matches!(&call.callee, Expression::Identifier(id) if id.name == "require")
        && !matches!(call.arguments.first(), Some(oxc_ast::ast::Argument::StringLiteral(source))
            if is_package_source(source.value.as_str(), &option.package)
                || source.value == crate::STYLEX_PACKAGE
                || option.import_aliases.contains_key(source.value.as_str()))
}

/// The transitive dependency graph is built before inlining erases reads.
pub(crate) fn symbols(
    program: &Program<'_>,
    scoping: &Scoping,
    option: &ExtractOption,
) -> FxHashMap<SymbolId, Naming> {
    let mut bindings = Bindings {
        scoping,
        option,
        imports: FxHashSet::default(),
        dependencies: FxHashMap::default(),
        external: false,
    };
    bindings.visit_program(program);
    let mut risky = bindings.imports;
    loop {
        let count = risky.len();
        for (symbol, reads) in &bindings.dependencies {
            if reads.external || reads.names.iter().any(|name| risky.contains(name)) {
                risky.insert(*symbol);
            }
        }
        if count == risky.len() {
            return risky
                .into_iter()
                .map(|symbol| (symbol, Naming::Risky))
                .collect();
        }
    }
}

/// A stylesheet executes arbitrary local code; any external module read can
/// affect the shape or presence of its collected rules.
pub(crate) fn stylesheet(code: &str, filename: &str, option: &ExtractOption) -> Naming {
    let allocator = oxc_allocator::Allocator::default();
    let source_type = oxc_span::SourceType::from_path(filename).unwrap_or_default();
    let program = oxc_parser::Parser::new(&allocator, code, source_type)
        .parse()
        .program;
    let scoping = oxc_semantic::SemanticBuilder::new()
        .build(&program)
        .semantic
        .into_scoping();
    let mut bindings = Bindings {
        scoping: &scoping,
        option,
        imports: FxHashSet::default(),
        dependencies: FxHashMap::default(),
        external: false,
    };
    bindings.visit_program(&program);
    if bindings.external {
        Naming::Risky
    } else {
        Naming::Own
    }
}
