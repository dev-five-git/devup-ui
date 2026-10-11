use serial_test::serial;

use super::{IMPORT_CSS, extracted, visit};

const READ: &str = "`css` is read at runtime, where it does not exist: the build compiles it only where it is called or rendered";

const IMPORT_BOX_CSS: &str = "import { Box, css } from '@devup-ui/react';\n";

fn reads(code: &str) -> Vec<String> {
    visit(&format!("{IMPORT_BOX_CSS}{code}")).errors
}

#[test]
#[serial]
fn a_read_of_the_import_lowered_into_styles_is_reported() {
    for code in [
        "export const a = <Box p={css} />;",
        "export const a = <Box p={css ? 1 : 2} />;",
        "export const a = <Box className={css} />;",
        "export const a = <Box style={{ color: css }} />;",
        "export const a = <Box {...{ p: css }} />;",
        "export const a = <Box p={[1, css]} />;",
    ] {
        assert_eq!(reads(code), vec![READ.to_string()], "{code}");
    }
}

#[test]
#[serial]
fn a_read_of_the_import_in_the_arguments_of_a_compiled_call_is_reported() {
    for code in [
        "export const a = <Box p={css({ color: css })} />;",
        "export const a = css({ color: css });",
        "export const a = css`color: ${css};`;",
        "export const a = css({ color: 'red' }, css);",
    ] {
        let errors = reads(code);
        assert_eq!(
            errors.first(),
            Some(&READ.to_string()),
            "{code}: {errors:?}"
        );
    }
}

#[test]
#[serial]
fn a_read_of_the_import_in_a_jsx_call_is_reported() {
    let visited = visit(
        "import { jsx } from 'react/jsx-runtime';\n\
         import { Box, css } from '@devup-ui/react';\n\
         export const a = jsx(Box, { p: css });",
    );
    assert_eq!(visited.errors, vec![READ.to_string()]);
}

#[test]
#[serial]
fn a_read_of_an_alias_lowered_into_styles_is_reported() {
    let visited = visit(&format!(
        "{IMPORT_BOX_CSS}const alias = css;\nexport const a = <Box p={{alias}} />;"
    ));
    assert_eq!(
        visited.errors,
        vec![READ.replace("`css`", "`alias`")],
        "{}",
        visited.code
    );
}

#[test]
#[serial]
fn a_local_or_a_global_sharing_the_name_is_a_value() {
    for code in [
        "export const a = (css) => <Box p={css} />;",
        "export const a = ({ css }) => <Box className={css} style={{ color: css }} />;",
        "export function f() { const css = 1; return <Box p={css ? 1 : 2} />; }",
    ] {
        assert_eq!(reads(code), Vec::<String>::new(), "{code}");
    }
    let global = visit("import { Box } from '@devup-ui/react';\nexport const a = <Box p={css} />;");
    assert_eq!(global.errors, Vec::<String>::new());
    assert!(global.code.contains("css"), "{}", global.code);
}

#[test]
#[serial]
fn nested_compiled_calls_and_elements_are_not_reads() {
    for code in [
        "export const a = <Box className={css({ color: 'red' })} />;",
        "export const a = <Box p={css({ color: 'red' })} />;",
        "export const a = <Box p={[1, css({ color: 'red' })]} />;",
        "export const a = css({ color: 'red' }, css({ color: 'blue' }));",
    ] {
        let visited = visit(&format!("{IMPORT_BOX_CSS}{code}"));
        assert_eq!(visited.errors, Vec::<String>::new(), "{code}");
        assert!(visited.styles > 0, "{code}");
    }
    let styled = visit(
        "import { Box, styled } from '@devup-ui/react';\n\
         export const A = styled(Box)({ color: 'red' });\n\
         export const B = styled.div({ color: 'blue' });\n\
         export const C = styled(Box).attrs({ id: 'c' })({ color: 'green' });",
    );
    assert_eq!(styled.errors, Vec::<String>::new());
}

#[test]
#[serial]
fn the_whole_extraction_rejects_the_read_with_where_it_is() {
    let rejected = extracted(&format!(
        "{IMPORT_CSS}export const a = <div className={{css}} />;"
    ));
    assert!(rejected.is_err_and(|error| error.contains("`css` is read at runtime")));
    let lowered =
        extracted("import { Box, css } from '@devup-ui/react';\nexport const a = <Box p={css} />;");
    assert!(lowered.is_err_and(|error| error.contains("`css` is read at runtime")));
    let shadowed = extracted(
        "import { Box, css } from '@devup-ui/react';\n\
         export const a = (css) => <Box p={css} />;\n\
         export const b = css({ color: 'red' });",
    );
    assert!(shadowed.is_ok(), "{shadowed:?}");
}
