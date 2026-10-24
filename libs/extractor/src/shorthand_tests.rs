use std::collections::BTreeMap;

use css::{class_map::reset_class_map, file_map::reset_file_map, set_custom_shorthands};
use serial_test::serial;

use crate::{
    ExtractOption, ExtractOutput, extract, extract_style::extract_style_value::ExtractStyleValue,
};

fn extract_element(element: &str) -> Result<ExtractOutput, Box<dyn std::error::Error>> {
    reset_class_map();
    reset_file_map();
    extract(
        "shorthand.tsx",
        &format!("import {{ Box }} from '@devup-ui/react'; export const element = {element};"),
        ExtractOption {
            package: "@devup-ui/react".into(),
            single_css: true,
            import_main_css: false,
            ..ExtractOption::default()
        },
    )
}

fn static_properties(output: &ExtractOutput) -> Vec<(String, String, u8)> {
    let mut styles: Vec<_> = output
        .styles
        .iter()
        .map(|value| {
            let ExtractStyleValue::Static(style) = value else {
                panic!("expected static shorthand styles")
            };
            (style.property.clone(), style.value.clone(), style.level)
        })
        .collect();
    styles.sort();
    styles
}

#[test]
#[serial]
fn shorthand_expands_builtin_and_camel_targets() -> Result<(), Box<dyn std::error::Error>> {
    set_custom_shorthands(BTreeMap::from([(
        "insetX".into(),
        vec!["left".into(), "marginRight".into(), "py".into()],
    )]))?;
    let output = extract_element("<Box insetX=\"7px\" />")?;
    assert_eq!(
        static_properties(&output),
        vec![
            ("left".into(), "7px".into(), 0),
            ("margin-right".into(), "7px".into(), 0),
            ("padding-bottom".into(), "7px".into(), 0),
            ("padding-top".into(), "7px".into(), 0),
        ]
    );
    assert!(!output.code.contains("insetX="));
    assert!(output.code.contains("className="));
    set_custom_shorthands(BTreeMap::new())?;
    Ok(())
}

#[test]
#[serial]
fn shorthand_preserves_responsive_levels_and_hover() -> Result<(), Box<dyn std::error::Error>> {
    set_custom_shorthands(BTreeMap::from([(
        "insetX".into(),
        vec!["left".into(), "right".into()],
    )]))?;
    let output = extract_element("<Box _hover={{ insetX: ['1px', null, '3px'] }} />")?;
    assert_eq!(
        static_properties(&output),
        vec![
            ("left".into(), "1px".into(), 0),
            ("left".into(), "3px".into(), 2),
            ("right".into(), "1px".into(), 0),
            ("right".into(), "3px".into(), 2),
        ]
    );
    for value in &output.styles {
        let ExtractStyleValue::Static(style) = value else {
            panic!("expected static hover styles")
        };
        assert_eq!(
            style
                .selector
                .as_ref()
                .map(|selector| selector.as_class_str().into_owned()),
            Some("&:hover".into())
        );
    }
    set_custom_shorthands(BTreeMap::new())?;
    Ok(())
}

#[test]
#[serial]
fn shorthand_yields_to_later_explicit_target() -> Result<(), Box<dyn std::error::Error>> {
    set_custom_shorthands(BTreeMap::from([(
        "insetX".into(),
        vec!["left".into(), "right".into()],
    )]))?;
    let output = extract_element("<Box insetX=\"1px\" left=\"9px\" />")?;
    assert_eq!(
        static_properties(&output),
        vec![
            ("left".into(), "9px".into(), 0),
            ("right".into(), "1px".into(), 0)
        ]
    );
    set_custom_shorthands(BTreeMap::new())?;
    Ok(())
}

#[test]
#[serial]
fn shorthand_dynamic_values_stay_on_the_element() -> Result<(), Box<dyn std::error::Error>> {
    set_custom_shorthands(BTreeMap::from([(
        "insetX".into(),
        vec!["left".into(), "right".into()],
    )]))?;
    let output = extract_element("<Box insetX={position} />")?;
    let mut properties: Vec<_> = output
        .styles
        .iter()
        .map(|value| {
            let ExtractStyleValue::Dynamic(style) = value else {
                panic!("expected dynamic shorthand styles")
            };
            assert_eq!(style.identifier(), "position");
            style.property().to_string()
        })
        .collect();
    properties.sort();
    assert_eq!(properties, ["left", "right"]);
    assert!(output.code.contains("style="));
    assert!(!output.code.contains("insetX="));
    set_custom_shorthands(BTreeMap::new())?;
    Ok(())
}

#[test]
#[serial]
fn rejected_registration_cannot_change_extraction() -> Result<(), Box<dyn std::error::Error>> {
    set_custom_shorthands(BTreeMap::from([(
        "insetX".into(),
        vec!["left".into(), "right".into()],
    )]))?;
    let error = match set_custom_shorthands(BTreeMap::from([(
        "insetX".into(),
        vec!["width".into(), "widht".into()],
    )])) {
        Err(error) => error,
        Ok(()) => panic!("expected invalid shorthand registration to fail"),
    };
    assert_eq!(error.alias, "insetX");
    assert_eq!(error.target, "widht");
    assert_eq!(error.index, 1);
    let output = extract_element("<Box insetX=\"4px\" />")?;
    assert_eq!(
        static_properties(&output),
        vec![
            ("left".into(), "4px".into(), 0),
            ("right".into(), "4px".into(), 0)
        ]
    );
    set_custom_shorthands(BTreeMap::new())?;
    Ok(())
}
