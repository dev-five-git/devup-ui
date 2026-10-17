use serial_test::serial;

use super::{visit, visit_with};
use crate::css_prop::CssProp;

#[test]
#[serial]
fn components_follow_the_binding_their_element_reads() {
    let visited = visit(
        "import { Box, Flex as flex } from '@devup-ui/react';\n\
         export const a = ({ Box }) => <Box p={1} />;\n\
         export const b = <Box p={2} />;\n\
         export function c(Box) { return <Box p={3} />; }\n\
         export const d = <flex p={4} />;\n\
         export const e = <Box.X p={5} />;",
    );
    assert_eq!(
        visited.errors,
        vec![
            "`Box` is read at runtime, where it does not exist: the build compiles it only where it is called or rendered"
                .to_string()
        ]
    );
    assert_eq!(visited.styles, 1);
    for local in [
        "<Box p={1} />",
        "<Box p={3} />",
        "<flex p={4} />",
        "<Box.X p={5} />",
    ] {
        assert!(visited.code.contains(local), "{local}: {}", visited.code);
    }
    assert!(
        visited.code.contains("<div className=\"a\" />"),
        "{}",
        visited.code
    );
}

#[test]
#[serial]
fn global_component_follows_its_binding() {
    let visited = visit(
        "import { Global } from '@devup-ui/react/compat';\n\
         import * as D from '@devup-ui/react';\n\
         export const a = <Global styles={{ body: { margin: 0 } }} />;\n\
         export const b = ({ Global }) => <Global styles={{ body: { margin: 1 } }} />;\n\
         export const c = <D.Global styles={{ body: { margin: 2 } }} />;\n\
         export const d = (D) => <D.Global styles={{ body: { margin: 3 } }} />;",
    );
    assert_eq!(visited.errors, Vec::<String>::new());
    assert_eq!(visited.styles, 2);
    assert!(
        visited.code.contains("export const a = <Global />;"),
        "{}",
        visited.code
    );
    assert!(
        visited.code.contains("export const c = <D.Global />;"),
        "{}",
        visited.code
    );
    for local in [
        "<Global styles={{ body: { margin: 1 } }} />",
        "<D.Global styles={{ body: { margin: 3 } }} />",
    ] {
        assert!(visited.code.contains(local), "{local}: {}", visited.code);
    }
}

#[test]
#[serial]
fn styled_follows_the_binding_it_is_read_through() {
    let visited = visit(
        "import { styled, Box } from '@devup-ui/react';\n\
         import * as D from '@devup-ui/react';\n\
         export const A = styled.div({ color: 'red' });\n\
         export const B = D.styled.div({ color: 'blue' });\n\
         export const C = styled('div', { color: 'green' });\n\
         export const E = D.styled('div', { color: 'orange' });\n\
         export const F = styled(Box)({ color: 'pink' });\n\
         export const G = styled(D.Box)({ color: 'gray' });\n\
         export const H = styled.div.attrs({ role: 'note' })({ color: 'teal' });\n\
         export const I = D.styled.div.attrs({ role: 'note' })({ color: 'navy' });\n\
         export function f(styled, D) {\n\
         \treturn [styled.div({ color: 'red' }), styled('div', { color: 'red' }), styled.div.attrs({ role: 'note' })({ color: 'red' }), D.styled.div({ color: 'red' }), D.styled('div', { color: 'red' })];\n\
         }",
    );
    assert_eq!(visited.errors, Vec::<String>::new());
    assert_eq!(visited.styles, 8);
    assert!(
        visited.code.contains(
            "return [ styled.div({ color: \"red\" }), styled(\"div\", { color: \"red\" }), styled.div.attrs({ role: \"note\" })({ color: \"red\" }), D.styled.div({ color: \"red\" }), D.styled(\"div\", { color: \"red\" }) ];"
        ),
        "{}",
        visited.code
    );
    assert!(
        !visited.code.contains("export const A = styled")
            && !visited.code.contains("export const I = D.styled"),
        "{}",
        visited.code
    );
}

#[test]
#[serial]
fn imported_jsx_function_is_read_through_its_binding() {
    let visited = visit(
        "import { jsx } from 'react/jsx-runtime';\n\
         import { Box } from '@devup-ui/react';\n\
         export const a = jsx(Box, { p: 1 });\n\
         export function f(jsx) { return jsx('div', { p: 2 }); }",
    );
    assert_eq!(visited.errors, Vec::<String>::new());
    assert_eq!(visited.styles, 1);
    assert!(
        visited
            .code
            .contains("export const a = jsx(\"div\", { className: \"a\" });"),
        "{}",
        visited.code
    );
    assert!(
        visited.code.contains("return jsx(\"div\", { p: 2 });"),
        "{}",
        visited.code
    );
}

#[test]
#[serial]
fn css_prop_reaches_only_the_components_the_file_imports() {
    let visited = visit_with(
        "import { Box } from '@devup-ui/react';\n\
         export const a = <Box css={{ color: 'red' }} />;\n\
         export const b = ({ Box }) => <Box css={{ color: 'blue' }} />;",
        |visitor| visitor.takes_css_prop(CssProp::Elements),
    );
    assert_eq!(visited.errors, Vec::<String>::new());
    assert_eq!(visited.styles, 1);
    assert!(
        visited
            .code
            .contains("export const a = <div className=\"a\" />;"),
        "{}",
        visited.code
    );
    assert!(
        visited.code.contains("<Box css={{ color: \"blue\" }} />"),
        "{}",
        visited.code
    );
}

#[test]
#[serial]
fn calls_read_once_are_walked_again_with_the_bindings_they_read() {
    let visited = visit(
        "import { jsx } from 'react/jsx-runtime';\n\
         import { Box, css } from '@devup-ui/react';\n\
         export const a = jsx(Box, { ...props(), p: 1, className: css({ color: 'red' }) });\n\
         export function f(css) { return jsx(Box, { ...props(), p: 2, className: css({ color: 'blue' }) }); }",
    );
    assert_eq!(visited.errors, Vec::<String>::new(), "{}", visited.code);
    assert_eq!(visited.styles, 3);
    assert!(
        !visited.code.contains("css({ color: \"red\" })"),
        "{}",
        visited.code
    );
    assert!(
        visited.code.contains("css({ color: \"blue\" })"),
        "{}",
        visited.code
    );
}
