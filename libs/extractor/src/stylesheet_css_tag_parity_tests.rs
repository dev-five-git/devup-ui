use crate::{ExtractOption, ExtractOutput, extract_without_source_map};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use rstest::rstest;
use serial_test::serial;

fn compile(body: &str) -> Result<ExtractOutput, String> {
    reset_class_map();
    reset_file_map();
    let source = format!("import {{css}} from '@devup-ui/react'; const forceRun=(()=>1)(); {body}");
    extract_without_source_map("/parity.css.ts", &source, ExtractOption::default())
        .map_err(|error| error.to_string())
}

#[rstest]
#[case(
    "css`padding: ${2}px; color: ${'red'};`",
    "css('padding: 2px; color: red;')"
)]
#[case(
    r#"css`content: "\\41"; color: red;`"#,
    r#"css('content: "\\\\41"; color: red;')"#
)]
#[case("css``", "css('')")]
#[case("css`&:hover { color: ${'red'}; }`", "css('&:hover { color: red; }')")]
#[serial]
fn css_tag_terminal_preserves_classes_and_styles_when_tag_is_valid(
    #[case] tag: &str,
    #[case] call: &str,
) -> Result<(), String> {
    // Given
    let expected = compile(&format!("export const card={call};"))?;
    // When
    let actual = compile(&format!("export const card={tag};"))?;
    // Then
    assert_eq!(actual.code, expected.code);
    assert_eq!(actual.styles, expected.styles);
    Ok(())
}

#[test]
#[serial]
fn css_tag_terminal_preserves_nested_getter_effects_when_rules_are_interpolated()
-> Result<(), String> {
    // Given
    let expected =
        compile("export const card=css('padding: 8px; color: red;'); export const reads=1;")?;
    // When
    let actual = compile(
        "let count=0; const rules={get p(){count++;return 2;},color:'red'}; export const card=css`padding: ${rules.p * 4}px; color: ${rules.color};`; export const reads=count;",
    )?;
    // Then
    assert_eq!(actual.code, expected.code);
    assert_eq!(actual.styles, expected.styles);
    Ok(())
}
