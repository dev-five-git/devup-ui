use std::collections::BTreeSet;

use crate::{
    ExtractOption, ExtractStyleValue,
    css_utils::{css_to_style, keyframes_to_keyframes_style, optimize_css_block},
    extract,
};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use rstest::rstest;
use serial_test::serial;

#[test]
fn quoted_boundaries_select_only_selector_interpolations() {
    // Given quoted value punctuation and selector strings spanning quasis.
    let allocator = oxc_allocator::Allocator::default();
    let source = concat!(
        "css`content:'a;${",
        "value}${",
        "other}{'; &[title='${",
        "selector}'] {",
        "color:red}`"
    );
    let parsed = oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::tsx()).parse();
    let mut selected = Vec::new();
    // When selector dependencies are visited before extraction.
    crate::style_values::selectors(&parsed.program, &mut |expression| {
        selected.push(crate::utils::readable_code(expression));
    });
    // Then value expressions are not mistaken for selector dependencies.
    assert_eq!(selected, vec!["selector"]);
}

fn static_values(source: &str) -> Result<BTreeSet<(String, String)>, Box<dyn std::error::Error>> {
    reset_class_map();
    reset_file_map();
    let output = extract("quoted.tsx", source, ExtractOption::default())?;
    Ok(output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) => {
                Some((style.property().to_string(), style.value().to_string()))
            }
            _ => None,
        })
        .collect())
}

fn boxed(props: &str) -> String {
    format!("import {{ Box }} from '@devup-ui/react';\nexport const View = () => <Box {props} />;")
}

#[rstest]
#[case::number(r#"content='"value:0.5"'"#, "content", r#""value:0.5""#)]
#[case::single_quoted(r#"content="'0.5'""#, "content", "'0.5'")]
#[case::signed_unit(r#"content='"-0.12px"'"#, "content", r#""-0.12px""#)]
#[case::color(r#"content="'#ff0000'""#, "content", "'#ff0000'")]
#[case::color_function(
    r#"content='"rgba(255,0,0,0.5)"'"#,
    "content",
    r#""rgba(255,0,0,0.5)""#
)]
#[case::list(r#"content='"a, b"'"#, "content", r#""a, b""#)]
#[case::semicolon(r#"content='"a;"'"#, "content", r#""a;""#)]
#[case::open_paren(r#"content='"("'"#, "content", r#""(""#)]
#[case::close_paren(r#"content="')'""#, "content", "')'")]
#[case::spaces(r#"content='"white  space"'"#, "content", r#""white  space""#)]
#[case::mixed_css(
    r#"content='"color: #ff0000; gap: 0.50px"'"#,
    "content",
    r#""color: #ff0000; gap: 0.50px""#
)]
#[case::escaped_quote(r#"content={"'it\\'s 0.5'"}"#, "content", r"'it\'s 0.5'")]
#[case::around_string(r#"content="'a' 0.50 'b'""#, "content", "'a' .50 'b'")]
#[case::unquoted_number("opacity={0.5}", "opacity", ".5")]
#[case::unquoted_color(r##"color="#ff0000""##, "color", "#F00")]
#[serial]
fn box_props_keep_quoted_strings(
    #[case] props: &str,
    #[case] property: &str,
    #[case] expected: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let values = static_values(&boxed(props))?;
    assert_eq!(
        values,
        BTreeSet::from([(property.to_string(), expected.to_string())])
    );
    Ok(())
}

#[rstest]
#[case::number("content: '0.5'", "content", "'0.5'")]
#[case::color("content: \"#ff0000\"", "content", "\"#ff0000\"")]
#[case::list("content: 'a, b'", "content", "'a, b'")]
#[case::paren("content: '('", "content", "'('")]
#[case::family_list("font-family: 'A, B', sans-serif", "font-family", "'A, B',sans-serif")]
#[case::family_quote("font-family: 'say \"hi\"', b", "font-family", "'say \"hi\"',b")]
#[case::family_escape(r"font-family: 'It\'s two', b", "font-family", r"'It\'s two',b")]
#[case::around_string("margin: 0px 'x' 0px", "margin", "0 'x' 0")]
fn templates_keep_quoted_strings(
    #[case] css: &str,
    #[case] property: &str,
    #[case] expected: &str,
) {
    let styles = css_to_style(css, 0, &None);
    assert_eq!(
        styles
            .iter()
            .map(|style| (style.property(), style.value()))
            .collect::<Vec<_>>(),
        vec![(property, expected)]
    );
}

#[rstest]
#[serial]
fn stylex_keeps_quoted_strings() -> Result<(), Box<dyn std::error::Error>> {
    let values = static_values(
        "import stylex from '@stylexjs/stylex';\nconst styles = stylex.create({ a: { content: \"'0.5'\", opacity: '0.5' } });",
    )?;
    assert_eq!(
        values,
        BTreeSet::from([
            ("content".to_string(), "'0.5'".to_string()),
            ("opacity".to_string(), ".5".to_string()),
        ])
    );
    Ok(())
}

#[rstest]
#[serial]
fn font_face_names_keep_quoted_strings() -> Result<(), Box<dyn std::error::Error>> {
    reset_class_map();
    reset_file_map();
    let output = extract(
        "quoted.tsx",
        "import { globalCss } from '@devup-ui/react';\nglobalCss({ fontFaces: [{ fontFamily: \"'A, B'\", src: 'a.woff2' }] });",
        ExtractOption::default(),
    )?;
    let faces: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::FontFace(face) => {
                Some(face.properties["font-family"].as_str().to_string())
            }
            _ => None,
        })
        .collect();
    assert_eq!(faces, vec!["'A, B'".to_string()]);
    Ok(())
}

#[rstest]
#[case::semicolon("content: 'a;b'; color: red", &[("color", "red"), ("content", "'a;b'")])]
#[case::semicolon_space("content: 'a; b'", &[("content", "'a; b'")])]
#[case::open_brace("content: '{'; color: red", &[("color", "red"), ("content", "'{'")])]
#[case::close_brace("content: '}'; color: red", &[("color", "red"), ("content", "'}'")])]
#[case::semicolon_brace("content: ';}'", &[("content", "';}'")])]
#[case::colon("content: \"a:b\"; color: red", &[("color", "red"), ("content", "\"a:b\"")])]
#[case::nested(
    "&:hover { content: 'a;b'; color: red } content: '}'",
    &[("color", "red"), ("content", "'a;b'"), ("content", "'}'")]
)]
#[case::comment("content: '/* a */'; /* b */ color: red", &[("color", "red"), ("content", "'/* a */'")])]
#[case::spaces("content: 'a   b'; color:   red", &[("color", "red"), ("content", "'a   b'")])]
fn template_blocks_keep_quoted_strings(#[case] css: &str, #[case] expected: &[(&str, &str)]) {
    let mut styles: Vec<_> = css_to_style(css, 0, &None)
        .iter()
        .map(|style| (style.property().to_string(), style.value().to_string()))
        .collect();
    styles.sort();
    let expected: Vec<_> = expected
        .iter()
        .map(|(property, value)| (property.to_string(), value.to_string()))
        .collect();
    assert_eq!(styles, expected);
}

#[rstest]
#[case::semicolon("content: 'a; b'", "content:'a; b'")]
#[case::both("content: 'a;b'; color: red", "content:'a;b';color:red")]
#[case::open_brace("content: '{'; color: red", "content:'{';color:red")]
#[case::semicolon_brace(concat!("a{", "content:';}';color:red;}"), concat!("a{", "content:';}';color:red}"))]
#[case::nested(
    concat!("a { content: '}' ; b ", "{ color: red ; } }"),
    concat!("a{content:'}';b", "{color:red}}")
)]
#[case::family(
    "a{font-family: 'A, B', 'Open Sans'; }",
    "a{font-family:'A, B',\"Open Sans\"}"
)]
fn optimized_blocks_keep_quoted_strings(#[case] css: &str, #[case] expected: &str) {
    assert_eq!(optimize_css_block(css), expected);
}

#[test]
fn keyframes_keep_quoted_strings() {
    let frames = keyframes_to_keyframes_style(
        "from { content: '}'; color: red } to { content: 'a;b'; opacity: 0.5 }",
    );
    let frames: Vec<_> = frames
        .iter()
        .map(|(key, styles)| {
            (
                key.as_str(),
                styles
                    .iter()
                    .map(|style| (style.property(), style.value()))
                    .collect::<Vec<_>>(),
            )
        })
        .collect();
    assert_eq!(
        frames,
        vec![
            ("from", vec![("color", "red"), ("content", "'}'")]),
            ("to", vec![("content", "'a;b'"), ("opacity", ".5")]),
        ]
    );
}
