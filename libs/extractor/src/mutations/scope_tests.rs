use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;

use super::{Use, uses};
use crate::css_prop::CssProp;

const COMPAT: &str = "@devup-ui/react/compat";

/// The uses of the bindings of `code`, one line for each, with `styles` the
/// names the module imports from the style packages
fn describe(code: &str, styles: &[&str], css: Option<(CssProp, &str)>) -> Vec<String> {
    let allocator = Allocator::default();
    let program = Parser::new(&allocator, code, SourceType::tsx())
        .parse()
        .program;
    let line = |at: u32| {
        code[at as usize..]
            .lines()
            .next()
            .unwrap_or_default()
            .to_string()
    };
    let mut found: Vec<String> = uses(&program, &|name| styles.contains(&name), css)
        .into_iter()
        .filter(|(name, _)| name == "a")
        .flat_map(|(name, uses)| {
            uses.into_iter().map(move |found| match found {
                Use::Changes { at, depth } => format!("{name} changes {depth}: {}", line(at)),
                Use::Escapes { at, path, into } => {
                    format!("{name} escapes {path:?} into {into:?}: {}", line(at))
                }
                Use::Calls { at, path } => format!("{name} calls {path:?}: {}", line(at)),
            })
        })
        .collect();
    found.sort();
    found
}

#[test]
fn a_local_named_like_a_style_api_is_not_one() {
    let styles = ["css", "Box", "Devup"];
    let imports = "import { css, Box } from '@devup-ui/react';\nimport * as Devup from '@devup-ui/react';\nconst a = {};\n";
    for (code, escapes) in [
        ("css(a);", 0),
        ("css.x(a);", 0),
        ("<Box p={f(a)} />;", 0),
        ("<Devup.Box p={f(a)} />;", 0),
        ("function f(css) { css(a); }", 1),
        ("function f() { const css = g; css(a); }", 1),
        ("function f(css) { css.x(a); }", 1),
        ("function f(Box) { return <Box p={f(a)} />; }", 1),
        ("function f(Devup) { return <Devup.Box p={f(a)} />; }", 1),
    ] {
        let code = format!("{imports}{code}");
        assert_eq!(describe(&code, &styles, None).len(), escapes, "{code}");
    }
}

#[test]
fn a_global_named_like_a_style_api_is_not_one() {
    let styles = ["css", "Box", "Devup"];
    for code in [
        "css(a);",
        "css.x(a);",
        "<Box p={f(a)} />;",
        "<Devup.Box p={f(a)} />;",
    ] {
        let code = format!("const a = {{}};\n{code}");
        assert_eq!(describe(&code, &styles, None).len(), 1, "{code}");
    }
}

#[test]
fn a_local_named_like_a_global_is_not_one() {
    let a = "const a = {};\n";
    for (code, expected) in [
        ("Object.assign(a, {});", vec!["a changes 1: a, {});"]),
        (
            "function f(Object) { Object.assign(a, {}); }",
            vec!["a escapes [] into None: a, {}); }"],
        ),
        ("String(a);", vec![]),
        (
            "function f(String) { String(a); }",
            vec!["a escapes [] into None: a); }"],
        ),
        ("JSON.stringify(a);", vec![]),
        (
            "function f(JSON) { JSON.stringify(a); }",
            vec!["a escapes [] into None: a); }"],
        ),
    ] {
        assert_eq!(
            describe(&format!("{a}{code}"), &[], None),
            expected,
            "{code}"
        );
    }
}

#[test]
fn a_local_named_like_a_class_names_binding_is_not_one() {
    let prelude = "import { ClassNames } from '@emotion/react';\nconst a = {};\n";
    for (code, escapes) in [
        (
            "const x = <ClassNames>{({ css }) => css(f(a))}</ClassNames>;",
            0,
        ),
        (
            "const x = <ClassNames>{({ cx }) => cx`${a}`}</ClassNames>;",
            0,
        ),
        (
            "const x = <ClassNames>{({ css }) => ((css) => css(f(a)))(css)}</ClassNames>;",
            1,
        ),
        (
            "const x = <ClassNames>{({ cx }) => ((cx) => cx`${a}`)(cx)}</ClassNames>;",
            1,
        ),
        (
            "function h(ClassNames) { return <ClassNames>{({ css }) => css(f(a))}</ClassNames>; }",
            1,
        ),
    ] {
        let code = format!("{prelude}{code}");
        let css = Some((CssProp::Elements, COMPAT));
        assert_eq!(describe(&code, &[], css).len(), escapes, "{code}");
    }
}

#[test]
fn a_local_named_like_an_element_builder_takes_no_css_prop() {
    let prelude = "import { jsx } from '@emotion/react';\nimport { Box } from '@devup-ui/react';\nconst a = {};\n";
    for (css_prop, code, escapes) in [
        (CssProp::Elements, "jsx('div', { css: f(a) });", 0),
        (CssProp::Elements, "jsx(Box, { css: f(a) });", 0),
        (CssProp::Elements, "<Box css={f(a)} />;", 0),
        (CssProp::Elements, "<div css={f(a)} />;", 0),
        (CssProp::Elements, "<Custom css={f(a)} />;", 1),
        (CssProp::Everywhere, "<Custom css={f(a)} />;", 0),
        (
            CssProp::Elements,
            "function g(jsx) { jsx('div', { css: f(a) }); }",
            1,
        ),
        (
            CssProp::Elements,
            "function g(Box) { jsx(Box, { css: f(a) }); }",
            1,
        ),
        (
            CssProp::Elements,
            "function g(Box) { return <Box css={f(a)} />; }",
            1,
        ),
    ] {
        let code = format!("{prelude}{code}");
        let css = Some((css_prop, COMPAT));
        assert_eq!(describe(&code, &["Box"], css).len(), escapes, "{code}");
    }
}
