use css::{class_map::reset_class_map, file_map::reset_file_map};
use serial_test::serial;

use super::{IMPORT_CSS, extracted, visit};

#[test]
#[serial]
fn a_read_of_a_removed_alias_is_reported_where_it_is_read() {
    let visited = visit(&format!(
        "{IMPORT_CSS}\
         export function f() {{ const c = css; return [c]; }}\n\
         export function g(c) {{ return [c]; }}\n\
         export const keep = css;"
    ));
    assert_eq!(
        visited.errors,
        vec![
            "`c` is read at runtime, where it does not exist: the build compiles it only where it is called or rendered"
                .to_string(),
            "`css` is read at runtime, where it does not exist: the build compiles it only where it is called or rendered"
                .to_string(),
        ]
    );
}

#[test]
#[serial]
fn shadowed_imports_stay_code_through_the_whole_extraction() {
    reset_class_map();
    reset_file_map();
    let output = crate::extract(
        "test.tsx",
        "import { css, Box } from '@devup-ui/react';\n\
         export function f(css) { return css({ color: 'red' }); }\n\
         export const g = ({ Box }) => <Box p={1} />;\n\
         export const real = css({ color: 'blue' });\n\
         export const view = <Box p={2} />;",
        crate::ExtractOption {
            import_main_css: false,
            ..crate::ExtractOption::default()
        },
    )
    .map(|output| output.code.split_whitespace().collect::<Vec<_>>().join(" "));
    assert_eq!(
        output.map_err(|error| error.to_string()),
        Ok("import \"@devup-ui/react/devup-ui-0.css\"; \
            export function f(css) { return css({ color: \"red\" }); } \
            export const g = ({ Box }) => <Box p={1} />; \
            export const real = \"a-a\"; \
            export const view = <div className=\"a-b\" />;"
            .to_string())
    );
}

#[test]
#[serial]
fn changed_top_level_binding_is_reported_but_a_local_of_the_same_name_is_not() {
    let own = extracted(
        "import { css, Box } from '@devup-ui/react';\n\
         const base = { p: 4 };\nbase.p = 8;\n\
         export const a = css(base);",
    );
    assert!(own.is_err_and(|error| error.contains("`css()` cannot use `base`")));

    let element = extracted(
        "import { Box } from '@devup-ui/react';\n\
         const base = { p: 4 };\nbase.p = 8;\n\
         export const a = <Box {...base} />;",
    );
    assert!(element.is_err_and(|error| error.contains("`<Box>` cannot use `base`")));

    let shadowed = extracted(
        "import { css, Box } from '@devup-ui/react';\n\
         const base = { p: 4 };\nbase.p = 8;\n\
         export function f(base) { return [css(base), <Box {...base} />]; }\n\
         export const real = css({ color: 'red' });",
    );
    assert!(
        shadowed
            .as_ref()
            .is_ok_and(|code| code.contains("export const real = \"a-a\"")
                && code.contains("<div {...__devupSpread0} className=")
                && code.contains("...base")),
        "{shadowed:?}"
    );
}

#[test]
#[serial]
fn unknown_top_level_binding_is_reported_but_a_local_of_the_same_name_is_not() {
    let own = extracted(
        "import { css } from '@devup-ui/react';\n\
         const rules = compute();\n\
         export const a = css(rules, { color: 'red' });",
    );
    assert!(own.is_err_and(|error| error.contains("rules")));

    let shadowed = extracted(
        "import { css } from '@devup-ui/react';\n\
         const rules = compute();\n\
         export function f(rules) { return css(rules, { color: 'red' }); }\n\
         export const real = css({ color: 'blue' });",
    );
    assert!(shadowed.is_ok(), "{shadowed:?}");
}

#[test]
#[serial]
fn css_prop_reads_of_changed_and_unknown_bindings_follow_their_scope() {
    let compile = |code: &str| {
        reset_class_map();
        reset_file_map();
        crate::extract(
            "test.tsx",
            &format!("import {{ Box }} from '@devup-ui/react';\n{code}"),
            crate::ExtractOption {
                import_main_css: false,
                import_aliases: std::collections::HashMap::from([(
                    "@emotion/react".to_string(),
                    crate::ImportAlias::NamedToNamed,
                )]),
                ..crate::ExtractOption::default()
            },
        )
        .map(|output| output.code)
        .map_err(|error| error.to_string())
    };
    for (own, shadowed) in [
        (
            "const rules = { color: 'red' };\nrules.color = 'blue';\nexport const a = <Box css={rules} />;",
            "const rules = { color: 'red' };\nrules.color = 'blue';\nexport const a = (rules) => <Box css={rules} />;",
        ),
        (
            "const rules = compute();\nexport const a = <Box css={[rules]} />;",
            "const rules = compute();\nexport const a = (rules) => <Box css={[rules]} />;",
        ),
    ] {
        assert!(
            compile(own).is_err_and(|error| error.contains("cannot use `rules`")),
            "{own}"
        );
        assert!(
            compile(shadowed).is_ok_and(|code| code.contains("<div className={rules} />")),
            "{shadowed}"
        );
    }
}

#[test]
#[serial]
fn a_global_read_is_not_a_removed_alias_of_the_same_name() {
    for (import, label) in [
        ("import { css } from '@devup-ui/react';", "import"),
        ("const { css } = require('@devup-ui/react');", "require"),
    ] {
        let visited = visit(&format!(
            "{import}\n\
             export function f() {{ const a = css; return a({{ color: 'red' }}); }}\n\
             export function g() {{ return a(); }}"
        ));
        assert_eq!(visited.errors, Vec::<String>::new(), "{label}");
        assert!(
            visited.code.contains("return a();"),
            "{label}: {}",
            visited.code
        );
        assert!(
            !visited.code.contains("const a = css"),
            "{label}: {}",
            visited.code
        );
    }
    let raw = visit(&format!(
        "{IMPORT_CSS}export function f() {{ const a = css; return a({{ color: 'red' }}); }}\n\
         export const raw = [css];"
    ));
    assert_eq!(raw.errors.len(), 1, "{:?}", raw.errors);
    assert!(raw.errors[0].starts_with("`css` is read at runtime"));
}
