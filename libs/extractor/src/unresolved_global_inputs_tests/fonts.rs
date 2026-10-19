use super::rejects_at;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("{fontFamily: unknownFamily, src: 'face.woff2'}", "unknownFamily")]
#[case("{fontFamily: tick(), src: 'face.woff2'}", "tick()")]
#[case("{fontFamily: 'Face', src: unknownSrc}", "unknownSrc")]
#[case("{fontFamily: 'Face', src: tick()}", "tick()")]
#[case(
    "{fontFamily: 'Face', src: 'face.woff2', fontWeight: unknownWeight}",
    "unknownWeight"
)]
#[case("{fontFamily: 'Face', src: 'face.woff2', fontStyle: tick()}", "tick()")]
#[case(
    "{...unknownDescriptors, fontFamily: 'Face', src: 'face.woff2'}",
    "unknownDescriptors"
)]
#[case("{fontFamily: 'Face', ...tick(), src: 'face.woff2'}", "tick()")]
#[case(
    "{fontFamily: 'Face', src: 'face.woff2', fontStyle: void tick()}",
    "void tick()"
)]
#[serial]
fn rejects_unresolved_font_descriptors_when_present(
    #[case] face: &str,
    #[case] marker: &str,
    #[values(false, true)] single_css: bool,
) {
    let source = format!(
        "import {{ globalCss }} from '@devup-ui/react';\nglobalCss({{ fontFaces: [{face}] }});"
    );
    rejects_at(&source, marker, single_css);
}

#[rstest]
#[case("font-family: Face; src: ${unknownSrc};", "unknownSrc")]
#[case("font-family: Face; src: ${tick()};", "tick()")]
#[case("font-family: Face; src: url(${unknownSrc});", "unknownSrc")]
#[case("font-family: Face; src: url(${tick()});", "tick()")]
#[case("font-family: Face; ${unknownProperty}: normal;", "unknownProperty")]
#[case("font-family: Face; ${tick()}: normal;", "tick()")]
#[case(
    "font-family: Face; ${unknownStatement}; src: face.woff2;",
    "unknownStatement"
)]
#[case("font-family: Face; ${tick()}; src: face.woff2;", "tick()")]
#[case("${unknownStatement}", "unknownStatement")]
#[case("${tick()}", "tick()")]
#[case("font-family: Face; src: ${void tick()};", "void tick()")]
#[serial]
fn rejects_unresolved_font_templates_before_css_to_style_discards_them(
    #[case] text: &str,
    #[case] marker: &str,
    #[values(false, true)] single_css: bool,
) {
    let source = format!(
        "import {{ globalCss }} from '@devup-ui/react';\nglobalCss({{ fontFaces: [`{text}`] }});"
    );
    rejects_at(&source, marker, single_css);
}

#[rstest]
#[case("{fontFamily: 'Face', src: 'face.woff2', fontWeight: 400}", ("font-weight", "400"))]
#[case("{fontFamily: `Fa${'ce'}`, src: 'face.woff2', fontStyle: `nor${'mal'}`}", ("font-style", "normal"))]
#[case("{fontFamily: 'Face', src: 'face.woff2', fontStyle: false}", ("font-style", "false"))]
#[case("`font-family: Face; src: face.woff2; font-style: normal;`", ("font-style", "normal"))]
#[case("`font-family: ${`Fa${'ce'}`}; src: face.woff2;`", ("font-family", "Face"))]
#[case("`font-family: ${p => p.theme.fonts.body}; src: face.woff2;`", ("font-family", "var(--fonts-body)"))]
#[case("`${'font-family'}: Face; src: face.woff2;`", ("font-family", "Face"))]
#[case("`${'font-family: Face;'} src: face.woff2;`", ("font-family", "Face"))]
#[serial]
fn readable_font_faces_keep_literal_pure_and_theme_descriptors(
    #[case] face: &str,
    #[case] descriptor: (&str, &str),
    #[values(false, true)] single_css: bool,
) {
    let (property, expected) = descriptor;
    let source = format!(
        "import {{ globalCss }} from '@devup-ui/react';\nglobalCss({{ fontFaces: [{face}] }});"
    );
    let output =
        super::extract_fixture(&source, single_css).unwrap_or_else(|error| panic!("{error}"));
    let faces: Vec<_> = output
        .styles
        .iter()
        .map(|style| match style {
            crate::ExtractStyleValue::FontFace(face) => {
                face.properties.get(property).map(String::as_str)
            }
            other => panic!("expected font face, got {other:?}"),
        })
        .collect();
    assert_eq!(faces, vec![Some(expected)]);
}

#[rstest]
#[serial]
fn absent_font_descriptors_remain_absent(
    #[values("undefined", "null")] absent: &str,
    #[values(false, true)] single_css: bool,
) {
    let source = format!(
        "import {{ globalCss }} from '@devup-ui/react';\nglobalCss({{fontFaces: [{{fontFamily: {absent}, src: 'face.woff2', fontStyle: {absent}}}]}});"
    );
    let output =
        super::extract_fixture(&source, single_css).unwrap_or_else(|error| panic!("{error}"));
    let faces: Vec<_> = output
        .styles
        .iter()
        .map(|style| match style {
            crate::ExtractStyleValue::FontFace(face) => &face.properties,
            other => panic!("expected font face, got {other:?}"),
        })
        .collect();
    assert_eq!(
        faces,
        vec![&std::collections::BTreeMap::from([(
            "src".to_string(),
            "url(face.woff2)".to_string(),
        )])]
    );
}

#[rstest]
#[case("${p => p.theme.fonts.body}: Face; src: face.woff2;")]
#[case("${p => p.theme.fonts.body}; font-family: Face; src: face.woff2;")]
#[serial]
fn theme_references_are_not_font_descriptor_names_or_statements(
    #[case] text: &str,
    #[values(false, true)] single_css: bool,
) {
    let statement = format!("globalCss({{fontFaces:[`{text}`]}});");
    let column = crate::dead_properties_test_utils::position(&statement, "p =>") + 1;
    let source = format!("import {{globalCss}} from '@devup-ui/react';\n{statement}");
    let error =
        crate::dead_properties_test_utils::failure(super::extract_fixture(&source, single_css));
    let (location, _) = error
        .split_once(": ")
        .unwrap_or_else(|| panic!("unlocated error: {error}"));
    assert_eq!(location, format!("structural.tsx:2:{column}"));
    assert!(error.contains("p.theme.fonts.body"));
    assert!(error.contains(" at build time:"));
}

#[rstest]
#[case("${p => p.theme.fonts.body}: Face; src: url(face.woff2);")]
#[case("${p => p.theme.fonts.body}; font-family: Face; src: url(face.woff2);")]
#[serial]
fn stylesheet_font_placement_errors_keep_the_original_interpolation(
    #[case] text: &str,
    #[values(false, true)] single_css: bool,
) {
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    let statement = format!("globalCss({{fontFaces:[`{text}`]}});");
    let column = crate::dead_properties_test_utils::position(&statement, "p =>") + 1;
    let source = format!("import {{globalCss}} from '@devup-ui/react';\n{statement}");
    let error = crate::dead_properties_test_utils::failure(crate::extract(
        "font-structural.css.ts",
        &source,
        crate::ExtractOption {
            single_css,
            ..crate::ExtractOption::default()
        },
    ));
    let (location, _) = error
        .split_once(": ")
        .unwrap_or_else(|| panic!("unlocated error: {error}"));
    assert_eq!(location, format!("font-structural.css.ts:2:{column}"));
    assert!(error.contains("p.theme.fonts.body"));
}
