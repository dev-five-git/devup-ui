use serial_test::serial;

use super::{extracted, visit};

const IMPORTS: &str = "import { Box, css, styled } from '@devup-ui/react';";

#[test]
#[serial]
fn nested_factory_components_compile_when_the_factory_binding_is_supported() {
    for factory in ["jsx", "jsxs", "jsxDEV"] {
        let code = format!(
            "{IMPORTS} import {{ {factory} }} from 'react/jsx-runtime';
             export const a = {factory}(Box, {{children: {factory}(Box, {{p: 1}})}});"
        );
        let output = extracted(&code).unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(
            output.matches(&format!("{factory}(\"div\"")).count(),
            2,
            "{output}"
        );
    }
}

#[test]
#[serial]
fn raw_reads_are_rejected_when_nested_factories_keep_them() {
    for nested in [
        "jsx(Box, {id: css})",
        "jsx(Box, {p: 1}, css)",
        "jsx(Box, {p: 1}, ...[css])",
        "jsx.call(null, Box, {p: 1})",
        "jsx.apply(null, [Box, {p: 1}])",
        "jsx.random(Box, {p: 1})",
        "((jsx) => jsx(Box, {p: 1}))(other)",
        "jsx(Box)",
        "jsx.random(css, {p: 1})",
    ] {
        let code = format!(
            "{IMPORTS} import {{ jsx }} from 'react/jsx-runtime';
             export const a = jsx(Box, {{children: {nested}}});"
        );
        let Err(error) = extracted(&code) else {
            panic!("raw factory read compiled: {nested}");
        };
        assert!(error.contains("is read at runtime"), "{nested}: {error}");
    }
}

#[test]
#[serial]
fn style_function_members_are_rejected_when_the_visitor_does_not_compile_them() {
    for value in [
        "css.call(null, {color: 'blue'})",
        "css.apply(null, [{color: 'blue'}])",
        "css.random({color: 'blue'})",
        "css.attrs({id: 'x'})",
        "css.withConfig({displayName: 'x'})",
        "css.random`color: blue;`",
        "styled.div.random({color: 'blue'})",
    ] {
        let code = format!("{IMPORTS} export const a = <Box p={{{value}}} />;");
        let Err(error) = extracted(&code) else {
            panic!("unsupported member compiled: {value}");
        };
        assert!(error.contains("is read at runtime"), "{value}: {error}");
    }
}

#[test]
#[serial]
fn namespace_jsx_factories_compile_when_their_members_are_supported() {
    let output = extracted(&format!(
        "{IMPORTS} const React = require('react/jsx-runtime');
         export const a = React.jsx(Box, {{children: React.jsxs(Box, {{p: 1}})}});"
    ))
    .unwrap_or_else(|error| panic!("{error}"));
    assert!(output.contains("React.jsxs(\"div\""), "{output}");
}

#[test]
#[serial]
fn raw_attrs_reads_are_rejected_when_the_import_would_be_removed() {
    for attrs in [
        "{id: css}",
        "{className: css}",
        "{style: {color: css}}",
        "() => ({id: css})",
        "() => ({className: css})",
        "() => ({style: {color: css}})",
        "{id: css.call(null, {color: 'blue'})}",
        "{id: css.apply(null, [{color: 'blue'}])}",
        "{id: css.random({color: 'blue'})}",
    ] {
        let code =
            format!("{IMPORTS} export const A = styled.div.attrs({attrs})({{color: 'red'}});");
        let Err(error) = extracted(&code) else {
            panic!("raw attrs read compiled: {attrs}");
        };
        assert!(
            error.contains("`css` is read at runtime"),
            "{attrs}: {error}"
        );
    }
}

#[test]
#[serial]
fn attrs_compile_nested_calls_when_their_original_bindings_are_available() {
    for attrs in [
        "{id: css({color: 'blue'})}",
        "{className: css({color: 'blue'})}",
        "() => ({className: css({color: 'blue'})})",
        "{children: <Box p={1} />}",
        "{children: jsx(Box, {p: 1})}",
    ] {
        let code = format!(
            "{IMPORTS} import {{jsx}} from 'react/jsx-runtime';
             export const A = styled.div.attrs({attrs})({{color: 'red'}});
             export const B = styled(A)({{color: 'green'}});"
        );
        let visited = visit(&code);
        assert_eq!(visited.errors, Vec::<String>::new(), "{attrs}");
        assert!(visited.styles >= 2, "{attrs}: {}", visited.code);
        assert!(!visited.code.contains("css("), "{}", visited.code);
        assert!(!visited.code.contains("<Box"), "{}", visited.code);
        assert!(!visited.code.contains("jsx(Box"), "{}", visited.code);
    }
}

#[test]
#[serial]
fn attrs_keep_local_reads_when_the_name_shadows_the_import() {
    for attrs in [
        "css => ({id: css})",
        "() => { const css = 'local'; return {id: css}; }",
        "css => ({id: css({color: 'blue'})})",
    ] {
        let code =
            format!("{IMPORTS} export const A = styled.div.attrs({attrs})({{color: 'red'}});");
        let output = extracted(&code).unwrap_or_else(|error| panic!("{error}"));
        assert!(output.contains("css"), "{output}");
    }
}

#[test]
#[serial]
fn ignored_config_metadata_is_not_a_runtime_read() {
    let output = extracted(&format!(
        "{IMPORTS} export const A = styled.div.withConfig({{displayName: css}})({{color: 'red'}});"
    ))
    .unwrap_or_else(|error| panic!("{error}"));
    assert!(!output.contains("displayName"), "{output}");
    let nested = extracted(&format!(
        "{IMPORTS} export const a = <Box children={{styled.div.withConfig({{displayName: css}})({{color: 'red'}})}} />;"
    )).unwrap_or_else(|error| panic!("{error}"));
    assert!(!nested.contains("displayName"), "{nested}");
}

#[test]
#[serial]
fn namespace_factories_reject_raw_reads_when_styles_or_attrs_keep_them() {
    for factory in [
        "D.styled.div({color: css})",
        "D.styled.div`color: ${css};`",
        "D.styled.div.attrs({id: css})({color: 'red'})",
        "D.styled.div.attrs({className: css})`color: red;`",
        "D.styled(Box)({color: css})",
        "D.styled(Box).attrs(() => ({id: css}))({color: 'red'})",
    ] {
        let code =
            format!("{IMPORTS} const D = require('@devup-ui/react'); export const A = {factory};");
        let Err(error) = extracted(&code) else {
            panic!("raw namespace read compiled: {factory}");
        };
        assert!(
            error.contains("`css` is read at runtime"),
            "{factory}: {error}"
        );
    }
}

#[test]
#[serial]
fn namespace_bases_compile_when_the_component_binding_is_rendered() {
    for factory in [
        "D.styled(Box)({color: 'red'})",
        "D.styled(D.Box)({color: 'red'})",
        "D.styled(Box).attrs({id: css({color: 'blue'})})`color: red;`",
    ] {
        let code =
            format!("{IMPORTS} const D = require('@devup-ui/react'); export const A = {factory};");
        let output = extracted(&code).unwrap_or_else(|error| panic!("{error}"));
        assert!(!output.contains("D.styled"), "{output}");
    }
}

#[test]
#[serial]
fn namespace_members_stay_ordinary_when_the_namespace_is_shadowed() {
    let output = extracted(&format!(
        "{IMPORTS} const D = require('@devup-ui/react');
         export const f = (D, css) => D.styled.div.attrs({{id: css}})({{color: css}});"
    ))
    .unwrap_or_else(|error| panic!("{error}"));
    assert!(output.contains("D.styled.div.attrs"), "{output}");
}
