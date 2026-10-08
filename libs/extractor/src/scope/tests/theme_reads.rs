use serial_test::serial;

use super::{visit, visit_with};
use crate::css_prop::CssProp;

const IMPORT: &str = "import { ClassNames } from '@devup-ui/react/compat';\n";

fn class_names(rendered: &str) -> super::Visited {
    visit(&format!(
        "{IMPORT}export const a = <ClassNames>{{({{ css, cx, theme }}) => {rendered}}}</ClassNames>;"
    ))
}

#[test]
#[serial]
fn a_local_sharing_the_name_of_the_theme_is_not_the_theme() {
    for rendered in [
        "<div className={cx({ on: ((theme) => theme.on)(state) })} />",
        "<div className={cx({ on: (function (theme) { return theme.on; })(state) })} />",
        "<div className={cx({ on: (() => { const theme = state; return theme.on; })() })} />",
        "<div className={cx({ on: (() => { { const theme = state; return theme.on; } })() })} />",
        "<div className={cx({ on: [1].map((theme) => theme.on) })} />",
        "<div className={cx({ on: ((x, { theme }) => theme.on)(1, state) })} />",
    ] {
        let visited = class_names(rendered);
        assert_eq!(visited.errors, Vec::<String>::new(), "{rendered}");
    }
}

#[test]
#[serial]
fn the_theme_the_child_function_takes_becomes_css_variables() {
    let visited = class_names(
        "<div className={cx(css({ color: theme.colors.primary }), { on: ((theme) => theme.m)(1) })} />",
    );
    assert_eq!(visited.errors, Vec::<String>::new());
    assert_eq!(visited.styles, 1);
    assert!(
        visited.code.contains("((theme) => theme.m)(1)"),
        "{}",
        visited.code
    );
    assert!(
        !visited.code.contains("var(--m)"),
        "the parameter is not the theme: {}",
        visited.code
    );
}

#[test]
#[serial]
fn a_direct_read_of_the_theme_it_cannot_write_is_still_reported() {
    for rendered in [
        "<div className={cx({ on: theme.on })} />",
        "<div className={cx({ on: ((x) => theme.on)(state) })} />",
        "<div className={cx({ on: ((theme) => theme.on)(theme.on) })} />",
    ] {
        let visited = class_names(rendered);
        assert_eq!(visited.errors.len(), 1, "{rendered}: {:?}", visited.errors);
        assert!(
            visited.errors[0].contains("`<ClassNames>` cannot use `theme.on`"),
            "{rendered}: {:?}",
            visited.errors
        );
    }
}

#[test]
#[serial]
fn a_function_of_the_theme_in_a_css_prop_reads_only_its_own_parameter() {
    let compile = |code: &str| {
        visit_with(
            &format!("import {{ Box }} from '@devup-ui/react';\n{code}"),
            |visitor| visitor.takes_css_prop(CssProp::Elements),
        )
    };
    let own =
        compile("export const a = <Box css={(theme) => ({ color: theme.colors.primary })} />;");
    assert_eq!(own.errors, Vec::<String>::new());
    assert!(own.code.contains("className"), "{}", own.code);

    let local =
        compile("export const a = <Box css={(theme) => ({ color: ((theme) => theme.a)(1) })} />;");
    assert!(
        !local.errors.iter().any(|error| error.contains("theme.a")),
        "{:?}",
        local.errors
    );

    let global = compile("export const a = <Box css={() => ({ color: theme.a })} />;");
    assert!(
        !global
            .errors
            .iter()
            .any(|error| error.contains("`theme.a`")),
        "a free `theme` is not a parameter: {:?}",
        global.errors
    );
}
