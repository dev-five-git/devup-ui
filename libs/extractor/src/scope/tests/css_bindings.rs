use serial_test::serial;

use super::{IMPORT_CSS, extracted, visit};

#[test]
#[serial]
fn style_function_shadowed_by_any_binding_stays_a_call() {
    for declaration in [
        "function f(css) { return css({ color: 'red' }); }",
        "const f = (css) => css({ color: 'red' });",
        "function f() { const css = (x) => x; return css({ color: 'red' }); }",
        "function f() { let css = (x) => x; return css({ color: 'red' }); }",
        "function f() { var css = (x) => x; return css({ color: 'red' }); }",
        "function f() { function css(x) { return x; } return css({ color: 'red' }); }",
        "function f() { class css {} return css({ color: 'red' }); }",
        "function f() { try { g(); } catch (css) { return css({ color: 'red' }); } }",
        "function f() { { const css = (x) => x; return css({ color: 'red' }); } }",
        "function f() { return [1].map((css) => css({ color: 'red' })); }",
    ] {
        let visited = visit(&format!(
            "{IMPORT_CSS}export {declaration}\nexport const real = css({{ color: 'blue' }});"
        ));
        assert_eq!(visited.errors, Vec::<String>::new(), "{declaration}");
        assert_eq!(visited.styles, 1, "{declaration}");
        assert!(
            visited.code.contains("css({ color: \"red\" })"),
            "{declaration}: {}",
            visited.code
        );
        assert!(
            visited.code.ends_with("export const real = \"a\";"),
            "{declaration}: {}",
            visited.code
        );
    }
}

#[test]
#[serial]
fn minified_import_alias_shadowed_by_a_parameter() {
    let visited = visit(
        "import{css as a}from'@devup-ui/react';export const x=a({color:'red'});export const f=a=>a({color:'blue'});",
    );
    assert_eq!(visited.errors, Vec::<String>::new());
    assert_eq!(visited.styles, 1);
    assert_eq!(
        visited.code,
        "export const x = \"a\"; export const f = (a) => a({ color: \"blue\" });"
    );
}

#[test]
#[serial]
fn keyframes_global_css_and_templates_follow_their_bindings() {
    let visited = visit(
        "import { css, keyframes, globalCss } from '@devup-ui/react';\n\
         export function a() { class keyframes {} return keyframes({ from: { color: 'red' } }); }\n\
         export function b() { { const globalCss = (x) => x; return globalCss({ body: { margin: 0 } }); } }\n\
         export function c() { function css(x) { return x; } return css`color: red;`; }\n\
         export function d(keyframes) { return keyframes`from { color: red; }`; }\n\
         export function e(globalCss) { return globalCss`body { margin: 0; }`; }\n\
         export const real = css`color: blue;`;\n\
         export const frames = keyframes({ from: { color: 'red' } });\n\
         globalCss({ body: { margin: 0 } });",
    );
    assert_eq!(visited.errors, Vec::<String>::new());
    assert_eq!(visited.styles, 3);
    for local in [
        "keyframes({ from: { color: \"red\" } })",
        "globalCss({ body: { margin: 0 } })",
        "css`color: red;`",
        "keyframes`from { color: red; }`",
        "globalCss`body { margin: 0; }`",
    ] {
        assert!(visited.code.contains(local), "{local}: {}", visited.code);
    }
    assert!(
        visited.code.contains("export const real = \"a\";"),
        "{}",
        visited.code
    );
}

#[test]
#[serial]
fn aliases_follow_the_binding_wherever_they_are_declared() {
    let visited = visit(&format!(
        "{IMPORT_CSS}\
         export function f() {{ const a = css; {{ const b = a; return b({{ color: 'red' }}); }} }}\n\
         export function g(css) {{ const c = css; return c({{ color: 'blue' }}); }}\n\
         export function h() {{ const keep = 1, d = css; return d({{ color: 'green' }}) + keep; }}"
    ));
    assert_eq!(visited.errors, Vec::<String>::new());
    assert_eq!(visited.styles, 2);
    assert_eq!(
        visited.code,
        "export function f() { { return \"a\"; } } \
         export function g(css) { const c = css; return c({ color: \"blue\" }); } \
         export function h() { const keep = 1; return \"b\" + keep; }"
    );
}

#[test]
#[serial]
fn minified_alias_called_with_runtime_arguments_stays_a_call() {
    let code = "import{css as a}from'@devup-ui/react';\
        export const x=a({color:'red'});\
        export function f(a,getValue){return a(getValue())}\
        export const g=(a)=>a('ordinary runtime argument');";
    let visited = visit(code);
    assert_eq!(visited.errors, Vec::<String>::new());
    assert_eq!(visited.styles, 1);
    for expected in [
        "export const x = \"a\";",
        "return a(getValue());",
        "a(\"ordinary runtime argument\")",
    ] {
        assert!(
            visited.code.contains(expected),
            "{expected}: {}",
            visited.code
        );
    }
    assert!(extracted(code).is_ok());
}
