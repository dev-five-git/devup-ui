use super::*;

#[test]
#[serial]
fn mixed_nonrendering_macros_keep_values_when_styled_generates_react_imports() -> TestResult {
    // Given
    reset();
    let source = "import {styled,css,globalCss,style} from '@devup-ui/react';const loop={};loop.self=loop;export {loop};export const Card=styled('section',{p:2});globalCss({body:{m:0}});const base=css({w:3});export const box=style([base,{color:'sienna'}]);";
    // When
    let output =
        extract_without_source_map("/mixed-rendering.css.ts", source, ExtractOption::default())?;
    // Then
    assert!(output.code.contains("from \"react\""), "{}", output.code);
    assert!(
        !output.code.contains("__devupDeferredRendering"),
        "{}",
        output.code
    );
    assert!(output.styles.iter().any(|value| matches!(value, ExtractStyleValue::Static(style) if style.property() == "width" && style.value() == "12px")), "{:?}", output.styles);
    assert!(output.styles.iter().any(|value| matches!(value, ExtractStyleValue::Static(style) if style.property() == "margin" && style.value() == "0" && matches!(style.selector(), Some(css::style_selector::StyleSelector::Global(selector, _)) if selector == "body"))), "{:?}", output.styles);
    Ok(())
}

#[test]
#[serial]
fn destructured_styled_result_fails_when_authored_execution_reads_rendering() -> TestResult {
    // Given
    reset();
    let source = "import {styled,style} from '@devup-ui/react';\nconst loop={};loop.self=loop;export {loop};\nconst {displayName}=styled('section',{p:2});\nexport const box=style({color:'sienna'});";
    // When
    let error = extract_without_source_map(
        "/destructured-rendering.css.ts",
        source,
        ExtractOption::default(),
    )
    .err()
    .ok_or("rendering destructure was accepted")?
    .to_string();
    // Then
    assert!(
        error.starts_with("/destructured-rendering.css.ts:3:"),
        "{error}"
    );
    assert!(error.contains("rendering scaffold"), "{error}");
    assert!(error.contains("Fix:"), "{error}");
    Ok(())
}
