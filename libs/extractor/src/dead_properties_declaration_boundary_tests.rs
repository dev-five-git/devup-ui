use crate::dead_properties_test_utils::{failure, position};
use crate::utils::{RESPONSIVE_ARRAY, build_time_error};
use crate::{ExtractOption, ExtractStyleValue, ImportAlias, extract};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("box-align", "boxAlign")]
#[case("box-pack", "boxPack")]
#[case("box-flex", "boxFlex")]
#[case("box-flex-group", "boxFlexGroup")]
#[case("box-orient", "boxOrient")]
#[case("box-ordinal-group", "boxOrdinalGroup")]
#[case("box-direction", "boxDirection")]
#[case("box-lines", "boxLines")]
#[case("flex-order", "flexOrder")]
#[case("flex-positive", "flexPositive")]
#[case("flex-negative", "flexNegative")]
#[case("flex-preferred-size", "flexPreferredSize")]
#[case("scroll-snap-points-x", "scrollSnapPointsX")]
#[case("scroll-snap-points-y", "scrollSnapPointsY")]
#[case("scroll-snap-coordinate", "scrollSnapCoordinate")]
#[case("scroll-snap-destination", "scrollSnapDestination")]
#[case("scroll-snap-type-x", "scrollSnapTypeX")]
#[case("scroll-snap-type-y", "scrollSnapTypeY")]
#[serial]
fn rejects_dead_declarations_when_entering_public_expression_apis(
    #[case] canonical: &str,
    #[case] camel: &str,
    #[values(
        "globalString",
        "globalTemplate",
        "keyframesString",
        "hover",
        "valueObject",
        "positionTry",
        "viewTransitionClass"
    )]
    boundary: &str,
) {
    // Given declaration bodies, rather than namespace names or token values.
    reset_class_map();
    reset_file_map();
    let statement = match boundary {
        "globalString" => format!("globalCss('body {{ color:red; {canonical}:1 }}');"),
        "globalTemplate" => format!("globalCss(`body {{ color:red; {canonical}:1 }}`);"),
        "keyframesString" => format!("const k = keyframes('from {{ {canonical}:1 }}');"),
        "hover" => format!("const e = <Box _hover={{{{ {camel}:'1' }}}} />;"),
        "valueObject" => format!("const e = <Box color={{{{ {camel}:'1' }}}} />;"),
        "positionTry" => format!("const p = stylex.positionTry({{ {camel}:'1' }});"),
        "viewTransitionClass" => {
            format!("const v = stylex.viewTransitionClass({{ {camel}:'1' }});")
        }
        _ => unreachable!(),
    };
    let source = format!(
        "import {{ Box, globalCss, keyframes }} from '@devup-ui/react';\nimport * as stylex from '@stylexjs/stylex';\n{statement}"
    );
    let name = if statement.contains(camel) {
        camel
    } else {
        canonical
    };
    let column = position(&statement, name) + 1;

    // When the public extractor reaches the expression or declaration boundary.
    let error = failure(extract("boundary.tsx", &source, ExtractOption::default()));

    // Then the authored key owns the error, including the original API name.
    let api = match boundary {
        "globalString" | "globalTemplate" => "`globalCss()`",
        "keyframesString" => "`keyframes()`",
        "hover" | "valueObject" => "`<Box>`",
        "positionTry" => "`stylex.positionTry()`",
        "viewTransitionClass" => "`stylex.viewTransitionClass()`",
        _ => unreachable!(),
    };
    assert!(
        error.starts_with(&format!("boundary.tsx:3:{column}:")),
        "{error}"
    );
    assert!(error.contains(api), "{error}");
    assert!(error.contains(canonical), "{error}");
    let requirement = match crate::dead_properties::requirement(canonical) {
        Some(requirement) => requirement,
        None => panic!("fixture has no policy for {canonical}"),
    };
    assert!(error.ends_with(requirement), "{error}");
}

#[rstest]
#[case("'@media print'", "@media print")]
#[case("_print", "_print")]
#[serial]
fn rejects_global_declarations_when_authored_inside_at_rule_maps(
    #[case] key: &str,
    #[case] path: &str,
) {
    // Given a stylesheet with an authored global selector map inside an at-rule.
    reset_class_map();
    reset_file_map();
    let statement = format!("globalCss({{ {key}: {{ body: {{ boxAlign:'center' }} }} }});");
    let source = format!("import {{ globalCss }} from '@devup-ui/react';\n{statement}");
    let column = position(&statement, "boxAlign") + 1;

    // When authored stylesheet calls are validated before their evaluation.
    let error = failure(extract("global.css.ts", &source, ExtractOption::default()));

    // Then recursion keeps the selector path and the key's source location.
    assert_eq!(
        error,
        format!(
            "global.css.ts:2:{column}: {}",
            build_time_error(
                "globalCss",
                &format!("{path} -> body -> boxAlign"),
                "unprefixed box-align has no browser implementation; use display: flex and align-items",
            )
        )
    );
}

#[test]
#[serial]
fn keeps_import_metadata_when_validating_global_selector_maps()
-> Result<(), Box<dyn std::error::Error>> {
    // Given import metadata in a deferred global call alongside an emitted style.
    reset_class_map();
    reset_file_map();
    let source = "import { style, globalCss } from '@devup-ui/react';\nfunction loadFonts() { globalCss({ imports:[{url:'box-align.css',boxAlign:'metadata'}] }); }\nexport const card=style({color:'red'});";

    // When the stylesheet's authored global map and evaluated style are checked.
    let output = extract("imports.css.ts", source, ExtractOption::default())?;

    // Then metadata is not mistaken for a declaration, and the style still emits.
    assert!(output.styles.iter().any(|style| matches!(
        style,
        ExtractStyleValue::Static(style) if style.property == "color" && style.value == "red"
    )));
    Ok(())
}

#[test]
#[serial]
fn keeps_font_face_literals_when_a_descriptor_object_has_an_unreadable_spread()
-> Result<(), Box<dyn std::error::Error>> {
    // Given a spread that has no statically readable font-face descriptors.
    reset_class_map();
    reset_file_map();
    let source = "import { globalCss } from '@devup-ui/react';\nglobalCss({ fontFaces:[{...external, fontFamily:'BoundaryFont', src:'local(BoundaryFont)'}] });";

    // When the global font-face declaration API extracts literal descriptors.
    let output = extract("font.tsx", source, ExtractOption::default())?;

    // Then the existing spread-ignore behavior preserves the authored literals.
    let faces: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::FontFace(face) => Some(face.properties.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        faces,
        vec![std::collections::BTreeMap::from([
            ("font-family".to_string(), "BoundaryFont".to_string()),
            ("src".to_string(), "local(BoundaryFont)".to_string()),
        ])]
    );
    Ok(())
}

#[rstest]
#[case("{_hover:[{...([1,2] as const)},null]}")]
#[case("{selectors:{'&:hover':[{...[1,2]},null]}}")]
#[case("{'@media':{'(min-width:600px)':[{...[1,2]},null]}}")]
#[serial]
fn rejects_nested_responsive_arrays_when_spread_into_selector_entries(#[case] argument: &str) {
    // Given an array spread nested inside a responsive selector entry.
    reset_class_map();
    reset_file_map();
    let statement = format!("export const card=style({argument});");
    let source = format!("import {{style}} from '@vanilla-extract/css';\n{statement}");
    let column = position(&statement, "[1,2]") + 1;
    let option = ExtractOption {
        import_aliases: std::collections::HashMap::from([(
            "@vanilla-extract/css".to_string(),
            ImportAlias::NamedToNamed,
        )]),
        ..ExtractOption::default()
    };

    // When the authored selector walk follows the object spread.
    let error = failure(extract("spread.css.ts", &source, option));

    // Then the nested array, not its containing object or call, owns the error.
    assert_eq!(
        error,
        format!(
            "spread.css.ts:2:{column}: {}",
            build_time_error("style", "[1, 2]", RESPONSIVE_ARRAY)
        )
    );
}

#[rstest]
#[case("{boxAlign:'center',color:'red'}")]
#[case("{...{boxAlign:'center'},boxAlign:'end',color:'red'}")]
#[serial]
fn rejects_root_dead_keys_when_literal_spreads_are_overridden(#[case] argument: &str) {
    // Given a root declaration, including a spread key shadowed by a later key.
    reset_class_map();
    reset_file_map();
    let statement = format!("const c=css({argument});");
    let source = format!("import {{css}} from '@devup-ui/react';\n{statement}");
    let column = position(&statement, "boxAlign") + 1;

    // When root validation runs before flattening or consuming properties.
    let error = failure(extract("root.tsx", &source, ExtractOption::default()));

    // Then the original authored key still owns the policy failure.
    assert!(
        error.starts_with(&format!("root.tsx:2:{column}:")),
        "{error}"
    );
    assert!(error.contains("box-align"), "{error}");
    assert!(
        error.contains("use display: flex and align-items"),
        "{error}"
    );
}
