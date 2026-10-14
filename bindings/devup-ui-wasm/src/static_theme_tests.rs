use super::*;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("px='$gutter'", vec![0, 2])]
#[case("px={'$gutter'}", vec![0, 2])]
#[case("px={['$gutter']}", vec![0])]
#[case("_hover={{px:['$gutter']}}", vec![0])]
#[case("selectors={{'&:focus':{px:['$gutter']}}}", vec![0])]
#[serial]
fn theme_arrays_remain_static_when_array_indices_define_the_responsive_levels(
    #[case] props: &str,
    #[case] expected: Vec<u8>,
) {
    // Given: a responsive token expanded only by the established root-token path.
    reset_build_state_internal();
    register_theme_internal(
        serde_json::from_value(
            serde_json::json!({"length":{"default":{"gutter":["8px",null,"16px"]}}}),
        )
        .unwrap_or_else(|error| panic!("{error}")),
    );
    let source = format!("import {{Box}} from '@devup-ui/react'; export const x=<Box {props}/>;");
    // When
    let output = code_extract_internal(
        "theme.tsx",
        &source,
        "@devup-ui/react",
        "df".into(),
        true,
        false,
        false,
        HashMap::new(),
    )
    .unwrap_or_else(|error| panic!("{error}"));
    // Then: class-only tokens have no tuple/inline spreads, and array tokens stay at index zero.
    for marker in [
        "__devupAssignment",
        "__devupLevel",
        "__devupValue",
        "style=",
    ] {
        assert!(!output.code().contains(marker), "{}", output.code());
    }
    with_style_sheet(|sheet| {
        let levels = sheet.properties[""][&255]
            .iter()
            .filter(|(_, properties)| {
                properties
                    .iter()
                    .any(|property| property.property == "padding-left")
            })
            .map(|(level, _)| *level)
            .collect::<Vec<_>>();
        assert_eq!(levels, expected);
    });
    reset_build_state_internal();
    register_theme_internal(sheet::theme::Theme::default());
}
