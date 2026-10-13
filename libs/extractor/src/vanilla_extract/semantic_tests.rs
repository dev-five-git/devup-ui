use rstest::rstest;
use serial_test::serial;

use super::{CollectedStyles, Stylesheet, execute_located};
use crate::ExtractOption;

fn evaluate(body: &str) -> Result<CollectedStyles, String> {
    css::file_map::reset_file_map();
    let code = format!(
        "import {{ assignVars, createTheme, createGlobalTheme, createThemeContract, createGlobalThemeContract, fallbackVar }} from '@devup-ui/react';\n{body}"
    );
    execute_located(
        Stylesheet {
            filename: "semantics.css.ts",
            code: &code,
            source: &code,
            edits: &[],
        },
        &ExtractOption::default(),
        None,
    )
    .map(|(collected, _)| collected)
}

#[rstest]
#[case("assignVars(contract, {color:{brand:'red'}})")]
#[case("assignVars(contract, {color:{brand:'red',text:'white',extra:'x'}})")]
#[case("assignVars(contract, {color:'red'})")]
#[case("assignVars(contract, {})")]
#[case("createTheme(contract, {color:{brand:'red'}})")]
#[case("createTheme(contract, {color:{brand:'red',text:'white',extra:'x'}})")]
#[case("createGlobalTheme(':root', contract, {color:{text:'white'}})")]
#[case("createGlobalTheme(':root', contract, {color:'red'})")]
#[case("assignVars(undefined, {a:'b'})")]
#[case("assignVars({empty:{}}, {})")]
#[case("assignVars({}, {empty:{}})")]
#[case("assignVars({a:'var(--a)'}, {a:{}})")]
#[serial]
fn contract_assignment_rejects_when_normalized_shapes_differ(
    #[case] call: &str,
) -> Result<(), String> {
    // Given
    let body =
        format!("const contract=createThemeContract({{color:{{brand:null,text:null}}}});\n{call};");
    // When
    let error = evaluate(&body)
        .err()
        .ok_or("mismatched full contract accepted")?;
    // Then
    assert!(error.starts_with("semantics.css.ts:3:"), "{error}");
    assert!(error.contains("Tokens don't match contract"), "{error}");
    Ok(())
}

#[rstest]
#[case("'red'", "\"red\"")]
#[case("'var(--a)'", "\"var(--a)\"")]
#[case("'var(--a)','var(--b)','red'", "\"var(--a, var(--b, red))\"")]
#[case("'var(--a, blue)','red'", "\"var(--a, blue, red)\"")]
#[case("'var(--a)','rgb(1, 2, 3)'", "\"var(--a, rgb(1, 2, 3))\"")]
#[serial]
fn fallback_chains_when_nonfinal_values_are_variable_references(
    #[case] arguments: &str,
    #[case] expected: &str,
) -> Result<(), String> {
    // Given
    let body = format!("export const result=fallbackVar({arguments});");
    // When
    let collected = evaluate(&body)?;
    // Then
    assert_eq!(
        collected.constant_exports,
        [("result".into(), expected.into())]
    );
    Ok(())
}

#[rstest]
#[case("'red','blue'")]
#[case("'rgb(1,2,3)','red'")]
#[case("'var(--a)','blue','red'")]
#[case("7,'red'")]
#[case("'var(--a)\\n','red'")]
#[case("'var(--a\\n)','red'")]
#[serial]
fn fallback_rejects_when_a_nonfinal_value_is_not_a_variable(
    #[case] arguments: &str,
) -> Result<(), String> {
    // Given
    let body = format!("export const result=fallbackVar({arguments});");
    // When
    let error = evaluate(&body)
        .err()
        .ok_or("invalid fallback variable accepted")?;
    // Then
    assert!(error.starts_with("semantics.css.ts:2:"), "{error}");
    assert!(error.contains("Invalid variable name"), "{error}");
    Ok(())
}

#[rstest]
#[case("{nested:{name:null}}")]
#[case("{nested:{name:7}}")]
#[case("{nested:{name:'a b'}}")]
#[case("{nested:{name:'9name'}}")]
#[case("{nested:{name:'-9name'}}")]
#[case("{nested:{name:'----name'}}")]
#[case("{nested:{name:'é'}}")]
#[case("{nested:{name:'a\\\\b'}}")]
#[case("{nested:{name:null}}, ()=>42")]
#[case("{nested:{name:null}}, ()=>undefined")]
#[case("{nested:{name:null}}, ()=>({toString:()=> 'valid'})")]
#[serial]
fn global_contract_rejects_when_a_name_is_not_an_unescaped_identifier(
    #[case] arguments: &str,
) -> Result<(), String> {
    // Given
    let body = format!("export const result=createGlobalThemeContract({arguments});");
    // When
    let error = evaluate(&body)
        .err()
        .ok_or("invalid global variable name accepted")?;
    // Then
    assert!(error.starts_with("semantics.css.ts:2:"), "{error}");
    assert!(
        error.contains("Invalid variable name for \"nested.name\""),
        "{error}"
    );
    Ok(())
}

#[rstest]
#[case(
    "createGlobalThemeContract({name:'--brand-color'})",
    r#"{ "name": "var(--brand-color)" }"#
)]
#[case(
    "createGlobalThemeContract({ name: null }, (value,path)=>value===null?path.join('-'):'wrong')",
    r#"{ "name": "var(--name)" }"#
)]
#[case("createGlobalThemeContract({name:''})", r#"{ "name": "var(--)" }"#)]
#[case("createGlobalThemeContract({name:'-'})", r#"{ "name": "var(---)" }"#)]
#[serial]
fn global_contract_preserves_when_names_match_upstream_escaping(
    #[case] expression: &str,
    #[case] expected: &str,
) -> Result<(), String> {
    // Given
    let body = format!("export const result={expression};");
    // When
    let collected = evaluate(&body)?;
    // Then
    assert_eq!(
        collected.constant_exports,
        [("result".into(), expected.into())]
    );
    Ok(())
}

#[test]
#[serial]
fn assignment_succeeds_when_a_matching_subcontract_is_supplied() -> Result<(), String> {
    // Given
    let body = "const contract=createThemeContract({color:{brand:null,text:null}});\nexport const result=assignVars({brand:contract.color.brand},{brand:'red'});";
    // When
    let collected = evaluate(body)?;
    // Then
    assert_eq!(
        collected.constant_exports,
        [(
            "result".into(),
            r#"{ "var(--color-brand-0-0)": "red" }"#.into()
        )]
    );
    Ok(())
}

#[test]
#[serial]
fn contract_walk_uses_inherited_enumerable_keys_when_own_keys_are_hidden() -> Result<(), String> {
    // Given
    let body = "const shape=Object.create({inherited:null,shadowed:null});\nObject.defineProperty(shape,'shadowed',{value:null});\nObject.defineProperty(shape,'hidden',{value:null});\nshape.visible=null;\nexport const result=createThemeContract(shape);";
    // When
    let collected = evaluate(body)?;
    // Then
    assert_eq!(
        collected.constant_exports,
        [(
            "result".into(),
            r#"{ "visible": "var(--visible-0-0)", "inherited": "var(--inherited-0-1)" }"#.into()
        )]
    );
    Ok(())
}

#[test]
#[serial]
fn assignment_preserves_values_when_leaf_types_are_normalized() -> Result<(), String> {
    // Given
    let body = "const contract=createGlobalThemeContract({s:'s',n:'n',nil:'nil',absent:'absent'});\nexport const result=assignVars(contract,{absent:undefined,nil:null,n:4,s:'red',skip:[1],flag:true,fn:()=>1});";
    // When
    let collected = evaluate(body)?;
    // Then
    assert_eq!(collected.constant_exports, [("result".into(), r#"{ "var(--absent)": "undefined", "var(--nil)": "null", "var(--n)": "4", "var(--s)": "red" }"#.into())]);
    Ok(())
}

#[test]
#[serial]
fn assignment_preserves_keys_when_contract_references_contain_fallbacks() -> Result<(), String> {
    // Given
    let body = "export const result=assignVars({a:'var(--a, red)'},{a:'blue'});";
    // When
    let collected = evaluate(body)?;
    // Then
    assert_eq!(
        collected.constant_exports,
        [("result".into(), r#"{ "var(--a, red)": "blue" }"#.into())]
    );
    Ok(())
}

#[test]
#[serial]
fn theme_assignment_uses_inherited_tokens_when_no_layer_is_supplied() -> Result<(), String> {
    // Given
    let body = "const contract=createGlobalThemeContract({inherited:'inherited',visible:'visible'});\nconst tokens=Object.create({inherited:'red'});\ntokens.visible='blue';\ncreateGlobalTheme(':root',contract,tokens);";
    // When
    let collected = evaluate(body)?;
    // Then
    assert_eq!(
        collected.global_styles,
        [(
            ":root".into(),
            r#"{"--visible":"blue","--inherited":"red"}"#.into()
        )]
    );
    Ok(())
}

#[test]
#[serial]
fn contract_walk_skips_functions_when_they_have_own_properties() -> Result<(), String> {
    // Given
    let body = "const fn=()=>1;fn.token=null;\nexport const result=createThemeContract({fn,arr:[null],flag:true,empty:{},number:4,missing:undefined});";
    // When
    let collected = evaluate(body)?;
    // Then
    assert_eq!(
        collected.constant_exports,
        [(
            "result".into(),
            r#"{ "empty": {}, "number": "var(--number-0-0)", "missing": "var(--missing-0-1)" }"#
                .into()
        )]
    );
    Ok(())
}

#[test]
#[serial]
fn global_contract_callback_receives_value_and_path_when_mapping_a_nested_leaf()
-> Result<(), String> {
    // Given
    let body = "export const result=createGlobalThemeContract({color:{brand:'original'}},(value,path)=>`${value}-${path.join('-')}`);";
    // When
    let collected = evaluate(body)?;
    // Then
    assert_eq!(
        collected.constant_exports,
        [(
            "result".into(),
            r#"{ "color": { "brand": "var(--original-color-brand)" } }"#.into()
        )]
    );
    Ok(())
}

#[test]
#[serial]
fn theme_tokens_use_object_rest_when_a_layer_is_inherited() -> Result<(), String> {
    // Given
    let body = "const tokens=Object.create({'@layer':'theme',inherited:'red'});\ntokens.visible='blue';\nObject.defineProperty(tokens,'hidden',{value:'green'});\nexport const result=createGlobalTheme(':root',tokens);";
    // When
    let collected = evaluate(body)?;
    // Then
    assert_eq!(
        collected.global_styles,
        [(
            ":root".into(),
            r#"{"@layer":"theme","--visible-0-0":"blue"}"#.into()
        )]
    );
    Ok(())
}

#[test]
#[serial]
fn theme_omits_layer_when_optional_layer_is_undefined() -> Result<(), String> {
    // Given
    let body = "export const result=createGlobalTheme(':root',{'@layer':undefined,color:'red'});";
    // When
    let collected = evaluate(body)?;
    // Then
    assert_eq!(
        collected.global_styles,
        [(":root".into(), r#"{"--color-0-0":"red"}"#.into())]
    );
    Ok(())
}
