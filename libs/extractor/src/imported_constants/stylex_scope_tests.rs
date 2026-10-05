use oxc_allocator::Allocator;
use oxc_ast::ast::{CallExpression, Expression, Statement};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_span::SourceType;

use super::ModuleScope;
use crate::stylex::StylexFunction;

/// The `StyleX` API the callee of each call reads, and the one its member
/// object reads, in source order
struct Callees<'s, 'p, 'a> {
    scope: &'s ModuleScope<'p, 'a>,
    found: Vec<(Option<StylexFunction>, Option<StylexFunction>)>,
}

impl<'a> Visit<'a> for Callees<'_, '_, 'a> {
    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        let object = match &call.callee {
            Expression::StaticMemberExpression(member) => {
                self.scope.stylex_function(&member.object)
            }
            _ => None,
        };
        self.found
            .push((self.scope.stylex_function(&call.callee), object));
        walk::walk_call_expression(self, call);
    }
}

fn read(code: &str) -> Vec<(Option<StylexFunction>, Option<StylexFunction>)> {
    let allocator = Allocator::default();
    let program = Parser::new(&allocator, code, SourceType::tsx())
        .parse()
        .program;
    let mut scope = ModuleScope::new("a.tsx", &program, None);
    for statement in &program.body {
        if let Statement::ImportDeclaration(import) = statement {
            scope.import(import);
        }
    }
    let mut callees = Callees {
        scope: &scope,
        found: Vec::new(),
    };
    callees.visit_program(&program);
    callees.found
}

#[test]
fn a_helper_is_the_import_its_callee_reads_and_not_its_spelling() {
    let found = read(
        "import * as sx from '@stylexjs/stylex';
import { firstThatWorks as ftw, types as t, include } from '@stylexjs/stylex';
import { include as other } from 'elsewhere';
sx.firstThatWorks(a);
ftw(a);
include(a);
other(a);
t.color(a);
sx.types.color(a);
function shadowed(sx, ftw, include, t) {
  sx.firstThatWorks(a);
  ftw(a);
  include(a);
  t.color(a);
  sx.types.color(a);
}",
    );
    assert_eq!(
        found,
        vec![
            (Some(StylexFunction::FirstThatWorks), None),
            (Some(StylexFunction::FirstThatWorks), None),
            (Some(StylexFunction::Include), None),
            (None, None),
            (None, Some(StylexFunction::Types)),
            (None, Some(StylexFunction::Types)),
            (None, None),
            (None, None),
            (None, None),
            (None, None),
            (None, None),
        ]
    );
}

fn extracted(code: &str) -> Result<crate::ExtractOutput, String> {
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    crate::extract(
        "test.tsx",
        code,
        crate::ExtractOption {
            import_main_css: false,
            ..crate::ExtractOption::default()
        },
    )
    .map_err(|error| error.to_string())
}

fn declarations(output: &crate::ExtractOutput) -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            crate::ExtractStyleValue::Static(style) => {
                Some((style.property.clone(), style.value.clone()))
            }
            _ => None,
        })
        .collect();
    found.sort();
    found
}

fn error_of(code: &str) -> String {
    extracted(code).err().unwrap_or_default()
}

const SX: &str = "import * as sx from '@stylexjs/stylex';\n";

#[test]
#[serial_test::serial]
fn genuine_helpers_compile_through_every_import_spelling() {
    let output = extracted(
        "import * as sx from '@stylexjs/stylex';
import { create, defineVars, firstThatWorks as ftw, include as inc, types as t } from '@stylexjs/stylex';
const base = create({ a: { color: 'red' } });
const vars = defineVars({ size: t.length('1px'), tone: sx.types.color({ default: 'blue' }) });
const styles = sx.create({
  b: { backgroundColor: ftw('red', 'blue'), width: vars.size, borderColor: vars.tone },
  c: { ...inc(base.a), height: sx.firstThatWorks('1px', '2px') },
});
export const A = () => <div {...sx.props(styles.b, styles.c)} />;",
    )
    .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(
        declarations(&output),
        vec![
            ("background-color".to_string(), "blue".to_string()),
            ("background-color".to_string(), "red".to_string()),
            (
                "border-color".to_string(),
                "var(--sxvar-746573742e747378-746f6e65-)".to_string()
            ),
            ("color".to_string(), "red".to_string()),
            ("height".to_string(), "1px".to_string()),
            ("height".to_string(), "2px".to_string()),
            (
                "width".to_string(),
                "var(--sxvar-746573742e747378-73697a65-)".to_string()
            ),
        ]
    );
}

#[test]
#[serial_test::serial]
fn shadowed_and_unrelated_helpers_are_reported_at_build_time() {
    for (code, expected) in [
        (
            "export function f(firstThatWorks) { return sx.create({ a: { color: firstThatWorks('red', 'blue') } }); }",
            "`stylex.create()` cannot use `firstThatWorks(",
        ),
        (
            "export function f(o) { return sx.create({ a: { color: o.firstThatWorks('red') } }); }",
            "`stylex.create()` cannot use `o.firstThatWorks(",
        ),
        (
            "const base = sx.create({ a: { color: 'red' } });\nexport function f(include) { return sx.create({ b: { ...include(base.a) } }); }",
            "`stylex.create()` cannot use `...include(",
        ),
        (
            "const base = sx.create({ a: { color: 'red' } });\nexport function f(o) { return sx.create({ b: { ...o.include(base.a) } }); }",
            "`stylex.create()` cannot use `...o.include(",
        ),
        (
            "export function f(types) { return sx.defineVars({ c: types.color('red') }); }",
            "`stylex.defineVars()` cannot use `types.color(",
        ),
        (
            "export function f(o) { return sx.defineVars({ c: o.types.color('red') }); }",
            "`stylex.defineVars()` cannot use `o.types.color(",
        ),
        (
            "export const p = sx.positionTry({ top: sx.firstThatWorks('1px') });",
            "`stylex.positionTry()` cannot use `sx.firstThatWorks(",
        ),
        (
            "export function f(firstThatWorks) { return sx.positionTry({ top: firstThatWorks('1px') }); }",
            "`stylex.positionTry()` cannot use `firstThatWorks(",
        ),
    ] {
        let message = error_of(&format!("{SX}{code}"));
        assert!(message.contains(expected), "{expected}: {message}");
    }
}

#[test]
#[serial_test::serial]
fn aliased_types_read_in_an_imported_module_still_publish_their_variables() {
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    let resolver = |specifier: &str, _: &str| {
        (specifier == "./vars").then(|| crate::ResolvedModule {
            path: "/src/vars.ts".to_string(),
            code: "import { defineVars, types as t } from '@stylexjs/stylex';
export const vars = defineVars({ tone: t.color('red'), size: t.length(unknown()) });"
                .to_string(),
        })
    };
    let output = crate::extract_with_modules(
        "/src/App.tsx",
        "import * as sx from '@stylexjs/stylex';
import { vars } from './vars';
const s = sx.create({ a: { color: vars.tone } });
export const A = () => <div {...sx.props(s.a)} />;",
        crate::ExtractOption {
            import_main_css: false,
            ..crate::ExtractOption::default()
        },
        false,
        &resolver,
    )
    .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(
        declarations(&output),
        vec![(
            "color".to_string(),
            "var(--sxvar-2f7372632f766172732e7473-746f6e65-)".to_string()
        )]
    );
}
