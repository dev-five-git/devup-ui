use std::rc::Rc;

use oxc_allocator::Allocator;
use oxc_ast::ast::Expression;
use oxc_ast::builder::AstBuilder;
use oxc_codegen::Codegen;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rustc_hash::FxHashMap;

use super::{Change, ChangeSite, Changed, Inlined, Unknown, inline_constants};
use crate::ExtractOption;
use crate::css_prop::CssProp;

fn inline(code: &str, css_prop: CssProp) -> (String, Inlined) {
    let allocator = Allocator::default();
    let mut program = Parser::new(&allocator, code, SourceType::tsx())
        .parse()
        .program;
    let inlined = inline_constants(
        &AstBuilder::new(&allocator),
        &mut program,
        "a.tsx",
        &ExtractOption::default(),
        None,
        css_prop,
    );
    (Codegen::new().build(&program).code, inlined)
}

fn expression<'a>(allocator: &'a Allocator, code: &'a str) -> Expression<'a> {
    Parser::new(allocator, code, SourceType::tsx())
        .parse_expression()
        .unwrap_or_else(|_| panic!("{code}"))
}

#[test]
fn a_local_named_like_a_style_function_reads_no_styles() {
    let (code, _) = inline(
        "import { css } from '@devup-ui/react';
import * as ui from '@devup-ui/react';
const SIZE = 4;
export const own = css({ w: SIZE });
export const member = ui.css({ w: SIZE });
export function shadowed(css, ui) { return [css({ w: SIZE }), ui.css({ w: SIZE }), parseInt('1')]; }",
        CssProp::Off,
    );
    assert_eq!(code.matches("w: 4").count(), 2, "{code}");
    assert_eq!(code.matches("w: SIZE").count(), 2, "{code}");
}

#[test]
fn a_local_named_like_a_style_component_reads_no_styles() {
    let (code, _) = inline(
        "import { Box } from '@devup-ui/react';
import * as ui from '@devup-ui/react';
const COLOR = 'red';
export const own = <><Box color={COLOR} /><ui.Box color={COLOR} /></>;
export function shadowed(Box, ui) { return <><Box color={COLOR} /><ui.Box color={COLOR} /></>; }",
        CssProp::Off,
    );
    assert_eq!(code.matches("{COLOR}").count(), 2, "{code}");
}

#[test]
fn a_local_named_like_a_class_names_binding_reads_no_styles() {
    let (code, _) = inline(
        "import { ClassNames } from '@emotion/react';
const SIZE = 4;
export const own = <ClassNames>{({ css }) => css({ width: SIZE })}</ClassNames>;
export const nested = <ClassNames>{({ css }) => ((css) => css({ width: SIZE }))(css)}</ClassNames>;
export const tagged = <ClassNames>{({ cx }) => cx`width: ${SIZE}px`}</ClassNames>;
export function shadowed(ClassNames) { return <ClassNames>{({ css }) => css({ width: SIZE })}</ClassNames>; }",
        CssProp::Elements,
    );
    assert_eq!(code.matches("width: SIZE").count(), 2, "{code}");
    assert_eq!(code.matches("width: \"4px\"").count(), 1, "{code}");
    assert_eq!(code.matches("${SIZE}").count(), 0, "{code}");
}

#[test]
fn a_local_named_like_the_jsx_function_takes_no_css_prop() {
    let (code, _) = inline(
        "import { jsx } from '@emotion/react';
import { Box } from '@devup-ui/react';
const SIZE = 4;
export const tag = jsx('div', { css: { width: SIZE } });
export const component = jsx(Box, { css: { width: SIZE } });
export function shadowedJsx(jsx) { return jsx('div', { css: { width: SIZE } }); }
export function shadowedBox(Box) { return jsx(Box, { css: { width: SIZE } }); }",
        CssProp::Elements,
    );
    assert_eq!(code.matches("width: \"4px\"").count(), 2, "{code}");
    assert_eq!(code.matches("width: SIZE").count(), 2, "{code}");
}

#[test]
fn a_local_named_like_a_style_component_takes_no_css_attribute() {
    let (code, _) = inline(
        "import { Box } from '@devup-ui/react';
const SIZE = 4;
export const own = <Box css={{ width: SIZE }} />;
export const tag = <div css={{ width: SIZE }} />;
export const custom = <Custom css={{ width: SIZE }} />;
export function shadowed(Box) { return <Box css={{ width: SIZE }} />; }",
        CssProp::Elements,
    );
    assert_eq!(code.matches("width: \"4px\"").count(), 2, "{code}");
    assert_eq!(code.matches("width: SIZE").count(), 2, "{code}");
}

#[test]
fn names_only_locals_read_are_not_reported_unknown_or_changed() {
    let (_, shadowed) = inline(
        "import { css } from '@devup-ui/react';
const unknown = compute();
const table = { a: 1 };
table.a = 2;
export function f(unknown, table) { return css({ w: unknown, h: table.a }); }",
        CssProp::Off,
    );
    assert!(shadowed.unknown.is_empty());
    assert!(shadowed.changed.is_empty());

    let (_, read) = inline(
        "import { css } from '@devup-ui/react';
const unknown = compute();
const table = { a: 1 };
table.a = 2;
export const a = css({ w: unknown, h: table.a });",
        CssProp::Off,
    );
    assert!(!read.unknown.is_empty());
    assert!(!read.changed.is_empty());
}

#[test]
fn the_scoping_built_is_kept_on_every_return_after_it() {
    for (code, css_prop, kept) in [
        ("const SIZE = 4;\nexport const a = 1;", CssProp::Off, false),
        (
            "import { css } from '@devup-ui/react';\nconst SIZE = 4;",
            CssProp::Off,
            true,
        ),
        (
            "import { css } from '@devup-ui/react';\nconst SIZE = 4;\ncss({ w: 1 });",
            CssProp::Off,
            true,
        ),
        (
            "import { css } from '@devup-ui/react';\nconst SIZE = 4;\ncss({ w: Math.max(1, 2) });",
            CssProp::Off,
            true,
        ),
        (
            "import { css } from '@devup-ui/react';\nconst SIZE = 4;\ncss({ w: SIZE });",
            CssProp::Off,
            true,
        ),
        (
            "const SIZE = 4;\nexport const a = <div css={{ width: SIZE }} />;",
            CssProp::Elements,
            true,
        ),
    ] {
        let (_, inlined) = inline(code, css_prop);
        assert_eq!(inlined.scoping.is_some(), kept, "{code}");
        if let Some(scoping) = &inlined.scoping {
            assert!(scoping.get_root_binding("SIZE".into()).is_some(), "{code}");
        }
    }
}

struct Properties<'s> {
    inlined: &'s Inlined,
    scoping: &'s oxc_semantic::Scoping,
    found: Vec<(bool, bool)>,
}

impl<'a> oxc_ast_visit::Visit<'a> for Properties<'_> {
    fn visit_object_property(&mut self, property: &oxc_ast::ast::ObjectProperty<'a>) {
        let reads = |identifier: &oxc_ast::ast::IdentifierReference<'_>| {
            crate::css_prop::reads_top_level(self.scoping, identifier)
        };
        self.found.push((
            self.inlined.unknown.read_by_in(&property.value, &reads),
            self.inlined.changed.read_by_in(&property.value, &reads),
        ));
    }
}

#[test]
fn the_scoping_kept_guards_reads_of_unknown_and_changed_bindings() {
    let allocator = Allocator::default();
    let mut program = Parser::new(
        &allocator,
        "import { css } from '@devup-ui/react';
const unknown = compute();
const table = { a: 1 };
table.a = 2;
export const top = css({ w: unknown, h: table.a });
export function shadowed(unknown, table) { return css({ w: unknown, h: table.a }); }",
        SourceType::tsx(),
    )
    .parse()
    .program;
    let inlined = inline_constants(
        &AstBuilder::new(&allocator),
        &mut program,
        "a.tsx",
        &ExtractOption::default(),
        None,
        CssProp::Off,
    );
    let Some(scoping) = inlined.scoping.clone() else {
        panic!("no scoping kept");
    };
    let mut properties = Properties {
        inlined: &inlined,
        scoping: &scoping,
        found: Vec::new(),
    };
    oxc_ast_visit::Visit::visit_program(&mut properties, &program);
    assert_eq!(
        properties.found,
        vec![
            (false, false),
            (true, false),
            (false, true),
            (false, false),
            (false, false)
        ]
    );
}

#[test]
fn unknown_counts_only_the_identifiers_accepted() {
    let allocator = Allocator::default();
    let unknown = Unknown {
        names: std::iter::once("x".to_string()).collect(),
        partial: FxHashMap::default(),
    };
    for code in ["x", "x()", "x[k]", "x.y[k]", "x.y()"] {
        let expression = expression(&allocator, code);
        assert!(unknown.read_by_in(&expression, &|_| true), "{code}");
        assert!(!unknown.read_by_in(&expression, &|_| false), "{code}");
    }
    let expression = expression(&allocator, "y[x]");
    assert!(!unknown.read_by_in(&expression, &|_| true));
}

#[test]
fn changed_counts_only_the_identifiers_accepted() {
    let allocator = Allocator::default();
    let change = Rc::new(Change {
        name: "x".to_string(),
        site: ChangeSite::Here(0),
        handed: false,
    });
    let changed = Changed {
        whole: FxHashMap::from_iter([("x".to_string(), change)]),
        holding: FxHashMap::default(),
    };
    for code in ["x", "x.y", "x[k]"] {
        let expression = expression(&allocator, code);
        assert!(changed.read_by_in(&expression, &|_| true), "{code}");
        assert!(!changed.read_by_in(&expression, &|_| false), "{code}");
    }
}
