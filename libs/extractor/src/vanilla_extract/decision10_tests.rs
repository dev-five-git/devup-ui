use rstest::rstest;
use serial_test::serial;

use super::{execute_vanilla_extract, json_string};

#[rstest]
#[case("{src:'local(Inter)',fontWeight:400}", vec![serde_json::json!({"src":"local(Inter)","fontWeight":400,"fontFamily":"\"Body_Font-0-0\""})])]
#[case("[{src:'local(Inter)',fontWeight:400},{src:'local(Inter Bold)',fontWeight:700}]", vec![serde_json::json!({"src":"local(Inter)","fontWeight":400,"fontFamily":"\"Body_Font-0-0\""}),serde_json::json!({"src":"local(Inter Bold)","fontWeight":700,"fontFamily":"\"Body_Font-0-0\""})])]
#[serial]
fn decision10_local_font_returns_the_quoted_family_used_by_every_rule(
    #[case] rule: &str,
    #[case] expected: Vec<serde_json::Value>,
    #[values("tsx", "css.ts")] extension: &str,
) -> Result<(), String> {
    // Given
    css::file_map::reset_file_map();
    let code = format!(
        "import {{fontFace,createVar}} from '@devup-ui/react';\nexport const family=fontFace({rule},'Body Font');\nexport const next=createVar();"
    );
    // When
    let collected =
        execute_vanilla_extract(&code, "@devup-ui/react", &format!("/font.{extension}"))?;
    // Then
    assert_eq!(
        collected.constant_exports,
        [
            ("family".into(), json_string("\"Body_Font-0-0\"")),
            ("next".into(), json_string("var(--var-0-1)"))
        ]
    );
    let faces = collected
        .font_faces
        .iter()
        .map(|rule| {
            serde_json::from_str::<serde_json::Value>(rule).map_err(|error| error.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(faces, expected);
    Ok(())
}

#[rstest]
#[case("styleVariants(data)")]
#[case("styleVariants(data,(value,key)=>({color:value.color}))")]
#[serial]
fn decision10_variants_allocate_styles_in_for_in_order_for_both_forms(
    #[case] call: &str,
    #[values("tsx", "css.ts")] extension: &str,
) -> Result<(), String> {
    // Given
    css::file_map::reset_file_map();
    let code = format!(
        "import {{styleVariants}} from '@devup-ui/react';\nconst data=Object.create({{'1':{{color:'red'}},inherited:{{color:'purple'}},shadowed:{{color:'orange'}},blocked:{{color:'pink'}}}});\ndata.z={{color:'navy'}};data['10']={{color:'blue'}};data['2']={{color:'teal'}};data.shadowed={{color:'green'}};\nObject.defineProperty(data,'hidden',{{value:{{color:'yellow'}}}});Object.defineProperty(data,'blocked',{{value:{{color:'brown'}}}});\nexport const variants={call};"
    );
    // When
    let collected =
        execute_vanilla_extract(&code, "@devup-ui/react", &format!("/variants.{extension}"))?;
    // Then
    assert_eq!(collected.constant_exports, [("variants".into(), r#"{ "1": _ve4, "2": _ve0, "10": _ve1, "z": _ve2, "shadowed": _ve3, "inherited": _ve5 }"#.into())]);
    assert_eq!(collected.styles.len(), 6);
    for (index, color) in ["teal", "blue", "navy", "green", "red", "purple"]
        .into_iter()
        .enumerate()
    {
        let entry = collected
            .styles
            .get(&format!("_ve{index}"))
            .ok_or("variant style missing")?;
        match entry.operands.as_slice() {
            [super::operands::StyleOperand::Rules(json)] => assert_eq!(
                serde_json::from_str::<serde_json::Value>(json)
                    .map_err(|error| error.to_string())?,
                serde_json::json!({"color":color})
            ),
            operands => panic!("unexpected variant operands: {operands:?}"),
        }
    }
    Ok(())
}

#[rstest]
#[case("{src:'url(example.woff2)',fontFamily:'Authored'}", vec![serde_json::json!({"src":"url(example.woff2)","fontFamily":"Given Family"})])]
#[case("[{src:'local(Regular)',fontFamily:'Wrong',fontWeight:400},{src:'local(Bold)',fontFamily:'Also Wrong',fontWeight:700}]", vec![serde_json::json!({"src":"local(Regular)","fontWeight":400,"fontFamily":"Given Family"}),serde_json::json!({"src":"local(Bold)","fontWeight":700,"fontFamily":"Given Family"})])]
#[serial]
fn decision10_global_font_returns_void_and_overrides_each_authored_family(
    #[case] rule: &str,
    #[case] expected: Vec<serde_json::Value>,
    #[values("tsx", "css.ts")] extension: &str,
) -> Result<(), String> {
    // Given
    css::file_map::reset_file_map();
    let code = format!(
        "import {{globalFontFace,createVar}} from '@devup-ui/react';\nexport const result=globalFontFace('Given Family',{rule});\nexport const next=createVar();"
    );
    // When
    let collected =
        execute_vanilla_extract(&code, "@devup-ui/react", &format!("/font.{extension}"))?;
    // Then
    assert_eq!(
        collected.constant_exports,
        [
            ("result".into(), "undefined".into()),
            ("next".into(), json_string("var(--var-0-0)"))
        ]
    );
    let faces = collected
        .font_faces
        .iter()
        .map(|rule| {
            serde_json::from_str::<serde_json::Value>(rule).map_err(|error| error.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(faces, expected);
    Ok(())
}
