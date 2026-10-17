use crate::{ExtractOption, ExtractOutput, ExtractStyleValue, extract_without_source_map};
use serial_test::serial;

#[test]
#[serial]
fn coverage_contract_rejects_dead_font_face_descriptor_when_media_query_is_imports()
-> Result<(), String> {
    // Given
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    let source = "import{globalCss}from'@devup-ui/react';globalCss({'@media':{imports:{fontFaces:[{fontFamily:'Example',boxAlign:'center'}]}}});";
    let column = source.find("boxAlign").ok_or("missing authored key")? + 1;
    // When
    let error =
        extract_without_source_map("/coverage-contract.tsx", source, ExtractOption::default())
            .err()
            .ok_or("dead font-face descriptor accepted")?
            .to_string();
    // Then
    assert!(
        error.starts_with(&format!("/coverage-contract.tsx:1:{column}:")),
        "{error}"
    );
    assert!(error.contains("`boxAlign`"), "{error}");
    assert!(error.contains("box-align"), "{error}");
    Ok(())
}

fn compile(body: &str) -> Result<ExtractOutput, String> {
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    extract_without_source_map(
        "/coverage-contract.css.ts",
        &format!("import{{css}}from'@devup-ui/react';{body}"),
        ExtractOption::default(),
    )
    .map_err(|error| error.to_string())
}

#[rstest::rstest]
#[case("css(base,{props:data,styleOrder:3,p:2})")]
#[case("css('external',{props:data,p:2})")]
#[case("css(['external',null,false,undefined,{props:data,p:2}])")]
#[case("css({props:data,_hover:{as:'section',p:2}})")]
#[case("css({props:data,_nthChild:{params:[{},[],'2n'],p:2}})")]
#[case(concat!("css({props:data,':hover':{", "p:2},'[data-on]':{", "w:3},'> a':{", "h:4}})"))]
#[case(concat!("css({props:data,'@media (min-width:1px)':{'@media not print and (color)':{", "p:2}}})"))]
#[serial]
fn coverage_contract_bridge_keeps_shallow_rules(#[case] expression: &str) -> Result<(), String> {
    // Given: independent shallow extraction and an ignored, heap-built payload.
    let prefix = "const base=css({color:'red'});";
    let shallow = expression.replace("props:data", "props:0");
    let expected = compile(&format!("{prefix}export const card={shallow};"))?;
    assert!(expected.styles.iter().any(|style| matches!(style,
        ExtractStyleValue::Static(style) if style.property() == "padding" && style.value() == "8px"
    )));
    let source = format!(
        "{prefix}let data=0;for(let i=0;i<16;i++)data={{payload:data}};export const card={expression};"
    );
    // When
    let actual = compile(&source)?;
    // Then
    assert_eq!(actual.code, expected.code);
    assert_eq!(actual.styles, expected.styles);
    Ok(())
}

#[rstest::rstest]
#[case("selectors:{invalidSelector:{p:2}}")]
#[case(concat!("_invalidSelector:{", "p:2}"))]
#[serial]
fn coverage_contract_bridge_rejects_invalid_selectors(#[case] rules: &str) -> Result<(), String> {
    // Given
    let source = format!(
        "let data=0;for(let i=0;i<16;i++)data={{payload:data}};export const card=css({{props:data,{rules}}});"
    );
    // When
    let error = compile(&source).err().ok_or("invalid selector accepted")?;
    // Then
    assert!(error.starts_with("/coverage-contract.css.ts:1:"), "{error}");
    assert!(error.contains(crate::utils::SELECTOR_NAME), "{error}");
    Ok(())
}

#[test]
#[serial]
fn coverage_contract_bridge_template_keeps_scalar_declarations() -> Result<(), String> {
    // Given: a structured statement forces the bridge alongside literal CSS values.
    let source = concat!(
        "let data=0;for(let i=0;i<16;i++)data={",
        "payload:data};export const card=css`color:${'red'};${{props:data,p:2}}`;"
    );
    // When
    let output = compile(source)?;
    // Then
    assert!(output.styles.iter().any(|style| matches!(style,
        ExtractStyleValue::Static(style) if style.property() == "color" && style.value() == "red"
    )));
    assert!(output.styles.iter().any(|style| matches!(style,
        ExtractStyleValue::Static(style) if style.property() == "padding" && style.value() == "8px"
    )));
    Ok(())
}

#[test]
#[serial]
fn coverage_contract_bridge_template_rejects_unplaced_scalar() -> Result<(), String> {
    // Given: the text interpolation is not a declaration or a standalone statement.
    let source = concat!(
        "let data=0;for(let i=0;i<16;i++)data={",
        "payload:data};export const card=css`color:red;${{props:data,p:2}} ${'orphan'} {",
        "color:blue}`;"
    );
    // When
    let error = compile(source).err().ok_or("unplaced scalar accepted")?;
    // Then
    assert!(error.starts_with("/coverage-contract.css.ts:1:"), "{error}");
    assert!(error.contains("css"), "{error}");
    Ok(())
}

#[test]
#[serial]
fn coverage_contract_bridge_rejects_responsive_arrays_in_data_slots() -> Result<(), String> {
    // Given: capture preserves vars as data; lowering still checks its responsive shape.
    let source = concat!(
        "let data=0;for(let i=0;i<16;i++)data={",
        "payload:data};export const card=css({props:data,vars:[[1,2]]});"
    );
    // When
    let error = compile(source)
        .err()
        .ok_or("nested responsive vars accepted")?;
    // Then
    assert!(error.starts_with("/coverage-contract.css.ts:1:"), "{error}");
    assert!(error.contains(crate::utils::RESPONSIVE_ARRAY), "{error}");
    Ok(())
}
