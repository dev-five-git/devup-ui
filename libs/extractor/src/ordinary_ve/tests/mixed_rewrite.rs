use serial_test::serial;

use super::mixed_canonical::assert_preserved;
use super::mixed_support::{assert_consumed, extract, has_static};

#[test]
#[serial]
fn native_definitions_are_hygienic_when_excluded_bindings_name_the_utilities()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = "import {style,keyframes as frames} from '@vanilla-extract/css';const css=window.name,globalCss=document.title,keyframes=window.location,_ve0=window.document,__ve_capture_0__=window.innerWidth;export const box=(()=>({nested:[style({color:'red'}),frames({from:{opacity:0}})]}))();";
    // When
    let output = extract("ts", source)?;
    // Then
    assert_consumed(&output);
    assert!(has_static(&output, "color", "red"));
    assert_preserved(
        &output.code,
        "const css=window.name,globalCss=document.title,keyframes=window.location,_ve0=window.document,__ve_capture_0__=window.innerWidth;",
    );
    Ok(())
}

#[test]
#[serial]
fn defaults_and_default_exports_compile_when_a_private_native_helper_returns_nested_values()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = "import {createVar,style} from '@vanilla-extract/css';function size(){return 4}function make(){const value=createVar();return {nested:{value},box:style({margin:value})}}export const {nested:{value},box,missing=createVar('default')}=make();export default style([box,{padding:size(),borderWidth:missing}]);const browser=window.document;";
    // When
    let output = extract("ts", source)?;
    // Then
    assert_consumed(&output);
    assert!(has_static(&output, "margin", "var(--var-0-0)"));
    assert!(has_static(&output, "border-width", "var(--default-0-1)"));
    assert!(has_static(&output, "padding", "4px"));
    assert_preserved(
        &output.code,
        "function size(){return 4}const browser=window.document;",
    );
    assert!(!output.code.contains("make()"));
    assert!(output.code.contains("export default"));
    Ok(())
}

#[test]
#[serial]
fn authored_css_retains_its_merge_mode_when_generated_native_operands_are_ordered()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let authored = concat!(
        "import {css} from '@devup-ui/react';",
        "export const authored=css({p:1},{p:2});"
    );
    let control = extract("ts", authored)?;
    let source = format!(
        "import {{style}} from '@vanilla-extract/css';const native=style([{{margin:8}},{{margin:12}}]);{authored}const browser=window.document;"
    );
    // When
    let output = extract("ts", &source)?;
    // Then
    assert_consumed(&output);
    assert!(has_static(&output, "margin", "12px"));
    assert!(!has_static(&output, "margin", "8px"));
    let padding = |output: &crate::ExtractOutput| {
        let mut values: Vec<_> = output
            .styles
            .iter()
            .filter(|value| {
                matches!(value,
            crate::ExtractStyleValue::Static(style) if style.property == "padding")
            })
            .cloned()
            .collect();
        values.sort();
        values
    };
    assert_eq!(padding(&output), padding(&control));
    Ok(())
}

#[test]
#[serial]
fn client_directives_remain_directives_when_native_definitions_are_inserted()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = "'use client';import {style} from '@vanilla-extract/css';const box=style({color:'red'});const browser=window.document;export const view=<div className={box}>{browser.title}</div>;";
    // When
    let output = extract("tsx", source)?;
    // Then
    assert_consumed(&output);
    let allocator = oxc_allocator::Allocator::default();
    let parsed =
        oxc_parser::Parser::new(&allocator, &output.code, oxc_span::SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    assert_eq!(
        parsed
            .program
            .directives
            .first()
            .map(|directive| directive.expression.value.as_str()),
        Some("use client")
    );
    assert_preserved(&output.code, "const browser=window.document;");
    Ok(())
}
