use crate::dead_properties_test_utils::{failure, position};
use crate::{ExtractOption, extract};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("boxAlign")]
#[case("boxPack")]
#[case("boxFlex")]
#[case("boxFlexGroup")]
#[case("boxOrient")]
#[case("boxOrdinalGroup")]
#[case("boxDirection")]
#[case("boxLines")]
#[case("flexOrder")]
#[case("flexPositive")]
#[case("flexNegative")]
#[case("flexPreferredSize")]
#[case("scrollSnapPointsX")]
#[case("scrollSnapPointsY")]
#[case("scrollSnapCoordinate")]
#[case("scrollSnapDestination")]
#[case("scrollSnapTypeX")]
#[case("scrollSnapTypeY")]
#[serial]
fn stylex_property_boundaries_reject_dead_names(
    #[case] property: &str,
    #[values("static", "pseudo", "dynamic", "keyframes")] shape: &str,
) {
    reset_class_map();
    reset_file_map();
    let statement = match shape {
        "static" => format!("const c = stylex.create({{x:{{{property}:'center'}}}});"),
        "pseudo" => format!("const c = stylex.create({{x:{{':hover':{{{property}:'center'}}}}}});"),
        "dynamic" => format!("const c = stylex.create({{x:(value)=>({{{property}:value}})}});"),
        "keyframes" => format!("const c = stylex.keyframes({{from:{{{property}:'center'}}}});"),
        _ => unreachable!(),
    };
    let source = format!("import * as stylex from '@stylexjs/stylex';\n{statement}");
    let error = failure(extract(
        "stylex-dead.tsx",
        &source,
        ExtractOption::default(),
    ));
    let column = position(&statement, property) + 1;
    assert!(
        error.starts_with(&format!("stylex-dead.tsx:2:{column}:")),
        "{error}"
    );
    assert!(error.contains("use "), "{error}");
}

#[test]
#[serial]
fn stylex_namespace_and_token_names_are_not_declarations() -> Result<(), Box<dyn std::error::Error>>
{
    reset_class_map();
    reset_file_map();
    let source = "import * as stylex from '@stylexjs/stylex';\nconst vars = stylex.defineVars({boxAlign:'red'}); const c = stylex.create({boxAlign:{color:vars.boxAlign,strokeColor:'red',imeMode:'active',WebkitBoxOrient:'vertical',futureDraft:'on'}});";
    let output = extract("stylex-control.tsx", source, ExtractOption::default())?;
    assert_ne!(output.styles.len(), 0);
    Ok(())
}
