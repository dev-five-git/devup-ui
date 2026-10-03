use serial_test::serial;

use super::visit;

#[test]
#[serial]
fn package_imported_whole_is_read_through_its_binding() {
    for import in [
        "import * as D from '@devup-ui/react';",
        "import D from '@devup-ui/react';",
    ] {
        let visited = visit(&format!(
            "{import}\n\
             export const a = D.css({{ color: 'red' }});\n\
             export function b(D) {{ return D.css({{ color: 'blue' }}); }}\n\
             export const c = <D.Box p={{1}} />;\n\
             export const d = (D) => <D.Box p={{2}} />;\n\
             export const e = D.unknown({{ color: 'green' }});\n\
             export const f = <D.Box.X p={{3}} />;\n\
             export const g = <D.Nothing />;\n\
             export const h = <this.Box />;\n\
             export const i = <svg:rect />;"
        ));
        assert_eq!(visited.errors, Vec::<String>::new(), "{import}");
        assert_eq!(visited.styles, 2, "{import}");
        for expected in [
            "export const a = \"a\";",
            "D.css({ color: \"blue\" })",
            "<div className=\"b\" />",
            "(D) => <D.Box p={2} />",
            "D.unknown({ color: \"green\" })",
            "<D.Box.X p={3} />",
            "<D.Nothing />",
            "<this.Box />",
            "<svg:rect />",
        ] {
            assert!(
                visited.code.contains(expected),
                "{import} {expected}: {}",
                visited.code
            );
        }
    }
}

#[test]
#[serial]
fn required_package_is_read_through_the_bindings_that_hold_it() {
    let visited = visit(
        "const { Box: B, css, ['styled']: s, ...rest } = require('@devup-ui/react');\n\
         const [x] = require('@devup-ui/react');\n\
         const { a: { b } } = require('@devup-ui/react');\n\
         const { jsx: j } = require('react/jsx-runtime');\n\
         const runtime = require('react/jsx-runtime'), D = require('@devup-ui/react');\n\
         export const t = j(B, { p: 1 });\n\
         export const u = css({ color: 'red' });\n\
         export function f(css) { return css({ color: 'blue' }); }\n\
         export const v = runtime.jsx(D.Box, { p: 2 });\n\
         export const w = D.css({ color: 'green' });\n\
         export function g(runtime, D) { return [runtime.jsx(D.Box, { p: 3 }), D.css({ color: 'teal' })]; }\n\
         export function h(require) {\n\
         \tconst { css: d } = require('@devup-ui/react');\n\
         \tconst E = require('@devup-ui/react');\n\
         \tconst r = require('react/jsx-runtime');\n\
         \treturn [d({ color: 'pink' }), E.css({ color: 'gray' }), r.jsx(E.Box, { p: 4 })];\n\
         }",
    );
    assert_eq!(visited.errors, Vec::<String>::new());
    assert_eq!(visited.styles, 4);
    for expected in [
        "export const t = j(\"div\", { className: \"a\" });",
        "export const u = \"b\";",
        "css({ color: \"blue\" })",
        "export const v = runtime.jsx(\"div\", { className: \"c\" });",
        "export const w = \"d\";",
        "return [runtime.jsx(D.Box, { p: 3 }), D.css({ color: \"teal\" })];",
        "return [ d({ color: \"pink\" }), E.css({ color: \"gray\" }), r.jsx(E.Box, { p: 4 }) ];",
    ] {
        assert!(
            visited.code.contains(expected),
            "{expected}: {}",
            visited.code
        );
    }
}
