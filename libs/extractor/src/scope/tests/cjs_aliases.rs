use serial_test::serial;

use super::extracted;

fn compiled(code: &str) -> String {
    extracted(code).unwrap_or_else(|error| format!("error: {error}"))
}

#[test]
#[serial]
fn comma_separated_declarators_alias_what_the_require_before_them_binds() {
    for code in [
        "const { css: a } = require('@devup-ui/react'), b = a;\nexport const x = b({ color: 'red' });",
        "const { css } = require('@devup-ui/react'), a = css, b = a;\nexport const x = b({ color: 'red' });",
        "const { css: a } = require('@devup-ui/react');\nconst b = a;\nexport const x = b({ color: 'red' });",
        "import { css as a } from '@devup-ui/react';\nconst b = a, c = b;\nexport const x = c({ color: 'red' });",
    ] {
        let output = compiled(code);
        assert!(
            output.contains("export const x = \"a-a\""),
            "{code}: {output}"
        );
        assert!(
            !output.contains("b(") && !output.contains("c(") && !output.contains(" = a"),
            "{code}: {output}"
        );
    }
}

#[test]
#[serial]
fn an_alias_is_read_by_the_functions_declared_before_it() {
    let output = compiled(
        "const { css } = require('@devup-ui/react'), f = () => b({ color: 'red' }), b = css;\n\
         export const x = f();",
    );
    assert!(!output.contains("error"), "{output}");
    assert!(!output.contains("b ="), "{output}");
}

#[test]
#[serial]
fn a_declarator_not_required_from_the_package_stays_code() {
    let output = compiled(
        "const { css: a } = require('other'), b = a;\nexport const x = b({ color: 'red' });",
    );
    assert!(
        output.contains("b = a") && output.contains("b({ color: 'red' })"),
        "{output}"
    );
}
