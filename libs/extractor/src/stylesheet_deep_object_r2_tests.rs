use crate::{ExtractOption, ExtractOutput, ExtractStyleValue, extract_without_source_map};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use serial_test::serial;

fn compile(body: &str) -> Result<ExtractOutput, String> {
    reset_class_map();
    reset_file_map();
    let source = format!("import{{css,keyframes}}from'@devup-ui/react';\n{body}");
    extract_without_source_map("/template-plan.css.ts", &source, ExtractOption::default())
        .map_err(|error| error.to_string())
}

#[rstest::rstest]
#[case("css`color:blue; ${ {props:data,p:2} }`", "padding", "8px")]
#[case("css`color:blue; ${ [base,{props:data,p:[3]}] }`", "padding", "12px")]
#[case(
    "css`color:blue; ${ {props:data,selectors:{[`${base} &`]:{animation:`${fade} 1s`,p:2}}} }`",
    "padding",
    "8px"
)]
#[case(
    "css`color:blue; ${ {props:data,'@supports (display:grid)':{_hover:{p:2}}} }`",
    "padding",
    "8px"
)]
#[serial]
fn deep_object_r2_template_statements_match_shallow_static_rules(
    #[case] expression: &str,
    #[case] property: &str,
    #[case] value: &str,
) -> Result<(), String> {
    // Given: the same evaluated rule at shallow and deep capture depths.
    let prefix =
        "const base=css({p:[1,2]});const fade=keyframes({from:{opacity:0},to:{opacity:1}});";
    let expected = compile(&format!(
        "{prefix}const data=(()=>1)();export const card={expression};"
    ))?;
    let body = format!(
        "{prefix}let data=1;for(let i=0;i<128;i++)data={{payload:data}};export const card={expression};"
    );
    // When
    let output = compile(&body)?;
    // Then
    assert!(output.styles.iter().any(|style| matches!(style, ExtractStyleValue::Static(style) if style.property() == property && style.value() == value)), "{output:?}");
    let binding = |code: &str| {
        code.lines()
            .find(|line| line.starts_with("export const card ="))
            .map(str::to_string)
    };
    let card = binding(&output.code).ok_or("card binding must remain")?;
    assert!(card.contains(" = \""), "{card}");
    assert_eq!(binding(&expected.code), Some(card));
    assert_eq!(output.styles, expected.styles);
    Ok(())
}

#[rstest::rstest]
#[case("opacity:${data}")]
#[case(concat!("${", "data} {color:red}"))]
#[case("color:${data};")]
#[serial]
fn deep_object_r2_invalid_template_keeps_inner_authored_origin(
    #[case] text: &str,
) -> Result<(), String> {
    // Given: an authored inner tag, a caught call and a later unrelated style.
    let body = format!(
        "let data={{p:2}};for(let i=0;i<128;i++)data={{_hover:data}};\nfunction make(){{\n return css`{text}`;\n}}\ntry{{make()}}catch(e){{e.message='erased'}}\nexport const card=css({{p:2}});"
    );
    // When
    let error = compile(&body)
        .err()
        .ok_or("invalid placement must reject")?;
    // Then
    assert!(error.starts_with("/template-plan.css.ts:4:9:"), "{error}");
    assert!(error.contains("css"), "{error}");
    Ok(())
}

#[test]
#[serial]
fn deep_object_r2_reentrant_getter_retains_inner_template_origin() -> Result<(), String> {
    // Given
    let body = concat!(
        "let data={",
        "p:2};for(let i=0;i<128;i++)data={_hover:data};\nconst rules={get p(){\n css`opacity:${data}`;\n return 2;\n}};\nexport const card=css(rules);"
    );
    // When
    let error = compile(body)
        .err()
        .ok_or("inner invalid tag must remain terminal")?;
    // Then
    assert!(error.starts_with("/template-plan.css.ts:4:2:"), "{error}");
    Ok(())
}

#[test]
#[serial]
fn deep_object_r2_parameterized_context_matches_flat_selector() -> Result<(), String> {
    // Given
    let selector = format!("&{}", ":nth-child(2n):hover".repeat(4096));
    let expected = compile(&format!(
        "export const card=css({{selectors:{{'{selector}':{{p:2}}}}}});"
    ))?;
    let body = concat!(
        "let rules={",
        "p:2};for(let i=0;i<4096;i++)rules={_nthChild:{params:['2n'],_hover:rules}};export const card=css(rules);"
    );
    // When
    let output = compile(body)?;
    // Then
    assert_eq!(output.code, expected.code);
    assert_eq!(output.styles, expected.styles);
    Ok(())
}

#[rstest::rstest]
#[case("{props:data,_hover:{boxAlign:'start'}}")]
#[case("[{props:data,_hover:{boxAlign:'start'}}]")]
#[serial]
fn deep_object_r2_statement_declarations_remain_terminal(
    #[case] rules: &str,
) -> Result<(), String> {
    // Given
    let body = format!(
        "let data=1;for(let i=0;i<128;i++)data={{payload:data}};\nfunction make(){{\n return css`color:blue; ${{{rules}}}`;\n}}\ntry{{make()}}catch(e){{e.message='erased'}}\nexport const card=css({{p:2}});"
    );
    // When
    let error = compile(&body)
        .err()
        .ok_or("dead declarations must reject")?;
    // Then
    assert!(error.starts_with("/template-plan.css.ts:4:9:"), "{error}");
    assert!(error.contains("box-align"), "{error}");
    Ok(())
}
