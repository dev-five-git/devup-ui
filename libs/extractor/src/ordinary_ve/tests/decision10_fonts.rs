use rstest::rstest;
use serial_test::serial;

use super::consumer_support::located_failure;
use super::mixed_canonical::assert_preserved;
use super::mixed_support::{assert_consumed, extract, has_static};
use crate::ExtractStyleValue;

#[rstest]
#[case("{src:'url(example.woff2)'}", vec![("url(example.woff2)", None)])]
#[case("[{src:'local(Regular)',fontWeight:400},{src:'local(Bold)',fontWeight:700}]", vec![("local(Regular)", Some("400")), ("local(Bold)", Some("700"))])]
#[serial]
fn decision10_font_control_preserves_exact_return_and_font_face_rules(
    #[case] rule: &str,
    #[case] expected: Vec<(&str, Option<&str>)>,
    #[values("tsx", "css.ts")] extension: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = format!(
        "import {{fontFace,globalFontFace,style}} from '@vanilla-extract/css';\nexport const family=fontFace({rule});\nexport const nothing=globalFontFace('Example',{rule});\nexport const box=style({{fontFamily:family}});"
    );
    // When
    let output = extract(extension, &source)?;
    // Then
    assert_consumed(&output);
    assert_preserved(&output.code, "export const family='\"font-0-0\"';");
    assert_preserved(&output.code, "export const nothing=undefined;");
    assert!(
        has_static(&output, "font-family", "\"font-0-0\""),
        "{:?}",
        output.styles
    );
    let mut faces: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::FontFace(face) => Some(face.properties.clone()),
            _ => None,
        })
        .collect();
    faces.sort();
    let mut expected: Vec<_> = ["font-0-0", "Example"]
        .into_iter()
        .flat_map(|family| {
            expected.iter().map(move |(src, weight)| {
                let mut properties = std::collections::BTreeMap::from([
                    ("font-family".into(), family.into()),
                    ("src".into(), (*src).into()),
                ]);
                if let Some(weight) = weight {
                    properties.insert("font-weight".into(), (*weight).into());
                }
                properties
            })
        })
        .collect();
    expected.sort();
    assert_eq!(faces, expected);
    Ok(())
}

#[rstest]
#[case("{src:'local(Inter)',fontFamily:'Authored'}")]
#[case("{src:'local(Inter)',fontFamily:undefined}")]
#[case("Object.create({fontFamily:'Authored'},{src:{value:'local(Inter)',enumerable:true}})")]
#[case("Object.defineProperty({src:'local(Inter)'},'fontFamily',{value:'Authored'})")]
#[serial]
fn decision10_local_font_rejects_font_family_members_in_single_and_array_rules(
    #[case] invalid: &str,
    #[values("single", "first", "last")] position: &str,
    #[values("tsx", "css.ts")] extension: &str,
) {
    // Given
    let rule = match position {
        "single" => invalid.to_string(),
        "first" => format!("[{invalid},{{src:'local(Valid)'}}]"),
        "last" => format!("[{{src:'local(Valid)'}},{invalid}]"),
        _ => unreachable!("fixture position"),
    };
    let source = format!(
        "import {{fontFace}} from '@vanilla-extract/css';\nexport const family=fontFace({rule});"
    );
    // When / Then
    let result = extract(extension, &source);
    let error = match &result {
        Ok(output) => panic!("local fontFamily accepted: {}", output.code),
        Err(error) => error.to_string(),
    };
    assert!(error.contains("globalFontFace"), "{error}");
    located_failure(result, &format!("/mixed.{extension}:2:"), "fontFamily");
}

#[rstest]
#[case("{src:'local(Inter)',fontFamily:'Authored'}")]
#[case("Object.create({fontFamily:'Authored'},{src:{value:'local(Inter)',enumerable:true}})")]
#[case("Object.defineProperty({src:'local(Inter)'},'fontFamily',{value:'Authored'})")]
#[serial]
fn decision10_global_font_accepts_authored_families_when_local_font_would_reject(
    #[case] rule: &str,
    #[values(false, true)] array: bool,
    #[values("tsx", "css.ts")] extension: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let rule = if array {
        format!("[{rule}]")
    } else {
        rule.to_string()
    };
    let source = format!(
        "import {{globalFontFace}} from '@vanilla-extract/css';\nexport const result=globalFontFace('Given Family',{rule});"
    );
    // When
    let output = extract(extension, &source)?;
    // Then
    assert_consumed(&output);
    assert_preserved(&output.code, "export const result=undefined;");
    let faces: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::FontFace(face) => Some(&face.properties),
            _ => None,
        })
        .collect();
    assert_eq!(
        faces,
        [&std::collections::BTreeMap::from([
            ("font-family".into(), "\"Given Family\"".into()),
            ("src".into(), "local(Inter)".into())
        ])]
    );
    Ok(())
}
