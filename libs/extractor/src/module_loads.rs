//! Where a program loads a module: an import, a re-export, `require()`,
//! `import x = require()` or `import()`, each naming it by a string literal

use oxc_ast::ast::{
    Argument, CallExpression, ExportAllDeclaration, ExportFromDeclaration, Expression,
    ImportDeclaration, ImportExpression, Program, StringLiteral, TSImportEqualsDeclaration,
    TSModuleReference,
};
use oxc_ast_visit::{Visit, walk};

/// Where `program` loads one of the `packages`, in source order: the offset
/// of the statement or call loading it, with the package. Type-only imports
/// and re-exports are erased, so they load nothing.
pub(crate) fn module_loads(
    program: &Program<'_>,
    packages: &[&'static str],
) -> Vec<(u32, &'static str)> {
    let mut loads = Loads {
        packages,
        found: Vec::new(),
    };
    loads.visit_program(program);
    loads.found.sort_unstable();
    loads.found
}

struct Loads<'p> {
    packages: &'p [&'static str],
    found: Vec<(u32, &'static str)>,
}

impl Loads<'_> {
    fn push(&mut self, offset: u32, source: &StringLiteral<'_>) {
        if let Some(package) = self
            .packages
            .iter()
            .find(|package| source.value == **package)
        {
            self.found.push((offset, *package));
        }
    }
}

impl<'a> Visit<'a> for Loads<'_> {
    fn visit_import_declaration(&mut self, it: &ImportDeclaration<'a>) {
        if !it.import_kind.is_type() {
            self.push(it.span.start, &it.source);
        }
    }

    fn visit_export_from_declaration(&mut self, it: &ExportFromDeclaration<'a>) {
        if !it.export_kind.is_type() {
            self.push(it.span.start, &it.source);
        }
    }

    fn visit_export_all_declaration(&mut self, it: &ExportAllDeclaration<'a>) {
        if !it.export_kind.is_type() {
            self.push(it.span.start, &it.source);
        }
    }

    fn visit_ts_import_equals_declaration(&mut self, it: &TSImportEqualsDeclaration<'a>) {
        if let TSModuleReference::ExternalModuleReference(reference) = &it.module_reference
            && !it.import_kind.is_type()
        {
            self.push(it.span.start, &reference.expression);
        }
    }

    fn visit_import_expression(&mut self, it: &ImportExpression<'a>) {
        if let Expression::StringLiteral(source) = &it.source {
            self.push(it.span.start, source);
        }
        walk::walk_import_expression(self, it);
    }

    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if it.callee.is_specific_id("require")
            && let [Argument::StringLiteral(source)] = it.arguments.as_slice()
        {
            self.push(it.span.start, source);
        }
        walk::walk_call_expression(self, it);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;
    use rstest::rstest;

    fn loads(code: &str) -> Vec<(u32, &'static str)> {
        let allocator = Allocator::default();
        let program = Parser::new(&allocator, code, SourceType::ts())
            .parse()
            .program;
        module_loads(&program, &["a", "b"])
    }

    #[rstest]
    #[case("import x from 'a';", vec![(0, "a")])]
    #[case("import type x from 'a'; import 'c';", vec![])]
    #[case("export { x } from 'b';", vec![(0, "b")])]
    #[case("export type { x } from 'b'; export const y = 1;", vec![])]
    #[case("export const y = require('a');", vec![(17, "a")])]
    #[case("export * from 'a'; export type * from 'b';", vec![(0, "a")])]
    #[case("import x = require('b'); import type y = require('a'); import z = y.w;", vec![(0, "b")])]
    #[case("f(import('a'), import(name), require('b', 1), require(name), req('a'));", vec![(2, "a")])]
    #[case("const x = () => require('b');", vec![(16, "b")])]
    fn finds_every_way_a_module_loads(#[case] code: &str, #[case] expected: Vec<(u32, &str)>) {
        assert_eq!(loads(code), expected);
    }
}
