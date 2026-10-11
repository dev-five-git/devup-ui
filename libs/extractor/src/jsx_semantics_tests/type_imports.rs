use super::*;
use serial_test::serial;

#[test]
#[serial]
fn a_type_only_import_declaration_stays_with_what_it_names() {
    let rendered = code(
        "import type { Box, css } from '@devup-ui/react';\nexport type A = typeof Box;\nexport type B = typeof css;",
    );

    assert!(
        rendered.contains("import type { Box, css } from \"@devup-ui/react\""),
        "{rendered}"
    );
}

#[test]
#[serial]
fn a_type_only_default_and_namespace_import_stay() {
    let rendered = code(
        "import type Devup from '@devup-ui/react';\nimport type * as ns from '@devup-ui/react';\nexport type D = typeof Devup;\nexport type N = typeof ns.Box;",
    );

    assert!(rendered.contains("import type Devup from"), "{rendered}");
    assert!(rendered.contains("import type * as ns from"), "{rendered}");
}

#[test]
#[serial]
fn an_inline_type_specifier_stays_beside_the_compiled_ones() {
    let rendered = code(
        "import { Box, type Flex } from '@devup-ui/react';\nexport type F = typeof Flex;\nexport const a = <Box color=\"red\" />;",
    );

    assert!(
        rendered.contains("import { type Flex } from \"@devup-ui/react\""),
        "{rendered}"
    );
    assert!(rendered.contains("<div className="), "{rendered}");
}

#[test]
#[serial]
fn only_inline_type_specifiers_keep_the_import() {
    let rendered =
        code("import { type Flex } from '@devup-ui/react';\nexport type F = typeof Flex;");

    assert!(
        rendered.contains("import { type Flex } from \"@devup-ui/react\""),
        "{rendered}"
    );
}

#[test]
#[serial]
fn a_type_import_never_registers_a_compiled_binding() {
    let rendered = code(
        "import type { Box } from '@devup-ui/react';\nimport { css } from '@devup-ui/react';\nexport const a = (Box) => <Box color=\"red\" />;\nexport const b = css({ color: 'blue' });",
    );

    assert!(rendered.contains("<Box color=\"red\" />"), "{rendered}");
    assert!(rendered.contains("import type { Box }"), "{rendered}");
}

#[test]
#[serial]
fn type_specifiers_of_the_runtime_and_stylex_register_nothing() {
    let rendered = code(&format!(
        "{BOX}import {{ jsx, type jsxs }} from 'react/jsx-runtime';\nimport {{ type create }} from '@stylexjs/stylex';\nimport type {{ StyleXStyles }} from '@stylexjs/stylex';\nexport const a = jsx(Box, {{ color: 'red' }});\nexport type S = StyleXStyles | typeof create | typeof jsxs;"
    ));

    assert!(rendered.contains("jsx(\"div\""), "{rendered}");
    assert!(rendered.contains("type jsxs"), "{rendered}");
    assert!(rendered.contains("type create"), "{rendered}");
}
