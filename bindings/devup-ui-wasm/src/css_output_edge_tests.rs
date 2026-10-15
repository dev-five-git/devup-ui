use super::{
    ImportAlias, Output, code_extract_without_source_map_internal, set_debug, with_style_sheet_mut,
};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use rstest::rstest;
use serial_test::serial;
use sheet::StyleSheet;
use std::collections::BTreeMap;
use std::collections::HashMap;

fn compile(source: &str) -> Result<Output, String> {
    compile_file("css-edges.tsx", source)
}

fn compile_file(filename: &str, source: &str) -> Result<Output, String> {
    with_style_sheet_mut(|sheet| *sheet = StyleSheet::default());
    reset_class_map();
    reset_file_map();
    set_debug(true);
    let output = code_extract_without_source_map_internal(
        filename,
        source,
        "@devup-ui/react",
        "@devup-ui/react".to_string(),
        true,
        false,
        false,
        HashMap::from([(
            "@vanilla-extract/css".to_string(),
            ImportAlias::NamedToNamed,
        )]),
    );
    set_debug(false);
    output
}

#[rstest]
#[case("flex", "flex", "3")]
#[case("flexGrow", "flex-grow", "3")]
#[case("flexShrink", "flex-shrink", "3")]
#[case("order", "order", "3")]
#[case("msFlex", "-ms-flex", "3")]
#[case("msFlexOrder", "-ms-flex-order", "3")]
#[case("msFlexPositive", "-ms-flex-positive", "3")]
#[case("msFlexNegative", "-ms-flex-negative", "3")]
#[case("WebkitBoxFlex", "-webkit-box-flex", "3")]
#[case("WebkitBoxOrdinalGroup", "-webkit-box-ordinal-group", "3")]
#[case("WebkitLineClamp", "-webkit-line-clamp", "3")]
#[case("MozTabSize", "-moz-tab-size", "3")]
#[case("msGridColumnSpan", "-ms-grid-column-span", "3")]
#[case("WebkitAnimationDuration", "-webkit-animation-duration", "3ms")]
#[case("WebkitTransitionDelay", "-webkit-transition-delay", "3ms")]
#[case("MozAnimationDelay", "-moz-animation-delay", "3ms")]
#[case("msTransitionDuration", "-ms-transition-duration", "3ms")]
#[case("OAnimationDuration", "-o-animation-duration", "3ms")]
#[case("OTransitionDelay", "-o-transition-delay", "3ms")]
#[case("p", "padding", "12px")]
#[case("WebkitBorderRadius", "-webkit-border-radius", "12px")]
#[serial]
fn numeric_edges_emit_css(
    #[case] key: &str,
    #[case] property: &str,
    #[case] value: &str,
) -> Result<(), String> {
    let source =
        format!("import {{ Box }} from '@devup-ui/react';\nconst e = <Box {key}={{3}} />;");
    let output = compile(&source)?;
    let css = output.css().ok_or("expected emitted CSS")?;
    assert!(css.contains(&format!("{{{property}:{value}}}")), "{css}");
    Ok(())
}

#[test]
#[serial]
fn flat_responsive_values_emit_css_after_base() -> Result<(), String> {
    let source = "import { Box } from '@devup-ui/react';\nconst e = <Box p={[1, null, 2]} />;";
    let output = compile(source)?;
    let css = output.css().ok_or("expected emitted CSS")?;
    let expected = format!(
        ".padding-0-4px--255{{padding:{base}}}@media(min-width:768px){{.padding-2-8px--255{{padding:{responsive}}}}}",
        base = "4px",
        responsive = "8px"
    );
    assert!(css.ends_with(&expected), "{css}");
    Ok(())
}

#[test]
#[serial]
fn selector_escape_hatches_emit_scoped_css() -> Result<(), String> {
    let source = "import { Box } from '@devup-ui/react';\nconst e = <Box selectors={{ ':hover': { color: 'red' }, '& > p': { color: 'blue' } }} />;";
    let output = compile(source)?;
    let css = output.css().ok_or("expected emitted CSS")?;
    let hover = format!(":hover{{color:{color}}}", color = "red");
    let child = format!(" > p{{color:{color}}}", color = "blue");
    assert!(css.contains(&hover), "{css}");
    assert!(css.contains(&child), "{css}");
    assert!(!css.contains(":body{"), "{css}");
    Ok(())
}

#[test]
#[serial]
fn supported_experimental_and_unknown_properties_still_emit_css() -> Result<(), String> {
    let source = "import { Box } from '@devup-ui/react';\nconst e = <Box strokeColor='red' imeMode='active' futureProperty='custom' WebkitBoxOrient='vertical' p={2} />;";
    let output = compile(source)?;
    let css = output.css().ok_or("expected emitted CSS")?;
    for declaration in [
        "stroke-color:red",
        "ime-mode:active",
        "future-property:custom",
        "-webkit-box-orient:vertical",
        "padding:8px",
    ] {
        assert!(css.contains(&format!("{{{declaration}}}")), "{css}");
    }
    Ok(())
}

#[rstest]
#[case("import { Box } from '@devup-ui/react';\nconst e = <Box boxAlign='center' />;")]
#[case("import { css } from '@devup-ui/react';\nconst c = css({ boxAlign: 'center' });")]
#[case("import { styled } from '@devup-ui/react';\nconst S = styled.div`box-align: center;`;")]
#[case(
    "import { globalCss } from '@devup-ui/react';\nglobalCss({ body: { boxAlign: 'center' } });"
)]
#[case(
    "import { keyframes } from '@devup-ui/react';\nconst k = keyframes({ from: { boxAlign: 'center' } });"
)]
#[serial]
fn dead_properties_fail_before_css_emission(#[case] source: &str) -> Result<(), String> {
    let error = compile(source)
        .err()
        .ok_or("expected a declaration error")?;
    assert!(error.starts_with("css-edges.tsx:2:"), "{error}");
    assert!(error.contains("align-items"), "{error}");
    Ok(())
}

#[test]
#[serial]
fn nested_responsive_arrays_fail_before_css_emission() -> Result<(), String> {
    let source = "import { Box } from '@devup-ui/react';\nconst e = <Box color={[[\"red\", \"blue\"], \"green\"]} />;";
    let error = compile(source)
        .err()
        .ok_or("expected a responsive array error")?;
    assert!(error.starts_with("css-edges.tsx:2:24:"), "{error}");
    assert!(error.contains("responsive arrays must be flat"), "{error}");
    Ok(())
}

#[rstest]
#[case(r"const c = css('color:red;\nbox-align:center');")]
#[case(r"const c = css('box\u002dalign:center');")]
#[serial]
fn escaped_string_css_dead_names_fail_before_emission(
    #[case] statement: &str,
) -> Result<(), String> {
    let source = format!("import {{ css }} from '@devup-ui/react';\n{statement}");
    let error = compile(&source)
        .err()
        .ok_or("expected a declaration error")?;
    assert!(error.starts_with("css-edges.tsx:2:"), "{error}");
    assert!(error.contains("align-items"), "{error}");
    Ok(())
}

#[test]
#[serial]
fn configured_shorthand_names_keep_their_valid_targets() -> Result<(), String> {
    css::set_custom_shorthands(BTreeMap::from([(
        "boxAlign".to_string(),
        vec!["align-items".to_string()],
    )]));
    let source = "import { Box } from '@devup-ui/react';\nconst e = <Box boxAlign='center' />;";
    let output = compile(source);
    css::set_custom_shorthands(BTreeMap::new());
    let css = output?.css().ok_or("expected emitted CSS")?;
    assert!(css.contains("{align-items:center}"), "{css}");
    assert!(!css.contains("{box-align:"), "{css}");
    Ok(())
}

#[rstest]
#[case(
    "import { style } from '@vanilla-extract/css';\nconst makeStyle = style;\nexport const invalid = makeStyle({ selectors: { '&:hover': { boxOrient: 'vertical' } } });",
    3
)]
#[case(
    "import * as ve from '@vanilla-extract/css';\nexport const invalid = ve['style']({ boxAlign: 'center' });",
    2
)]
#[case(
    "import { style } from '@vanilla-extract/css';\nconst property = 'boxAlign';\nexport const invalid = style({ [property]: 'center' });",
    3
)]
#[serial]
fn evaluated_declarations_report_authored_calls(
    #[case] source: &str,
    #[case] line: usize,
) -> Result<(), String> {
    let error = compile_file("css-edges.css.ts", source)
        .err()
        .ok_or("expected a declaration error")?;
    assert!(
        error.starts_with(&format!("css-edges.css.ts:{line}:")),
        "{error}"
    );
    assert!(
        error.contains("boxOrient") || error.contains("boxAlign"),
        "{error}"
    );
    Ok(())
}

#[rstest]
#[case(
    "import { style } from '@vanilla-extract/css'; const p = 'boxAlign'; export const invalid = style({ [p]: 'center' });",
    "style({"
)]
#[case(
    "import { style } from '@vanilla-extract/css'; export const invalid = style({ boxAlign: 'center' });",
    "boxAlign:"
)]
#[serial]
fn same_line_import_aliases_keep_authored_columns(
    #[case] source: &str,
    #[case] token: &str,
) -> Result<(), String> {
    let column = source.find(token).ok_or("fixture has no expected token")? + 1;
    let error = compile_file("css-edges.css.ts", source)
        .err()
        .ok_or("expected a declaration error")?;
    assert!(
        error.starts_with(&format!("css-edges.css.ts:1:{column}:")),
        "{error}"
    );
    Ok(())
}

#[test]
#[serial]
fn computed_css_text_reports_its_authored_call() -> Result<(), String> {
    let source = "import { css } from '@devup-ui/react';\nconst c = css('box-' + 'align:center');";
    let error = compile(source)
        .err()
        .ok_or("expected a declaration error")?;
    assert!(error.starts_with("css-edges.tsx:2:11:"), "{error}");
    assert!(error.contains("align-items"), "{error}");
    Ok(())
}

#[test]
#[serial]
fn stylex_dead_declarations_fail_before_css_emission() -> Result<(), String> {
    let statement = "export const c = stylex.create({ x: { boxAlign: 'center' } });";
    let source = format!("import * as stylex from '@stylexjs/stylex';\n{statement}");
    let column = statement.find("boxAlign").ok_or("fixture lacks property")? + 1;
    let error = compile(&source)
        .err()
        .ok_or("expected a declaration error")?;
    assert!(
        error.starts_with(&format!("css-edges.tsx:2:{column}:")),
        "{error}"
    );
    assert!(error.contains("align-items"), "{error}");
    Ok(())
}

#[rstest]
#[case(
    "import { style } from '@vanilla-extract/css';\nconst values = [[1,2],3];\n\n\nexport const bad = style({ p: values });",
    "css-edges.css.ts:5:20:"
)]
#[case(
    "import { style } from '@vanilla-extract/css';\n\n\nexport const bad = style({ p: [[1,2],3] });",
    "css-edges.css.ts:4:32:"
)]
#[serial]
fn stylesheet_nested_arrays_report_authored_locations(
    #[case] source: &str,
    #[case] location: &str,
) -> Result<(), String> {
    let error = compile_file("css-edges.css.ts", source)
        .err()
        .ok_or("expected a responsive array error")?;
    assert!(error.starts_with(location), "{error}");
    assert!(error.contains("responsive arrays must be flat"), "{error}");
    Ok(())
}
