use super::super::{KernelError, authentic_support::*, state_live};
use crate::{StyleSheet, StyleSheetProperty};
use extractor::extract_style::{ExtractStyleProperty, style_property::StyleProperty};

fn valid_sheet() -> StyleSheet {
    let mut sheet = StyleSheet::default();
    update(&mut sheet, &styles([])).required("independent empty Counter state");
    state_live::capture_owned(&sheet).required("valid BASE before Current");
    sheet
}

fn rejects_unproved(sheet: &mut StyleSheet) {
    let owned = sheet.counter_state.as_ref().required("owned state");
    assert_eq!(owned.authored, vec![]);
    assert_eq!(owned.rejection, None);
    assert!(matches!(
        state_live::capture_owned(sheet),
        Err(KernelError::Coverage)
    ));
    assert_eq!(
        update(sheet, &styles([])),
        Err(UpdateError::Kernel(KernelError::Coverage))
    );
}

#[test]
#[serial_test::serial]
fn current_static_stays_unproved_when_valid_counter_base_receives_only_static() {
    // Given
    let _state = state();
    let mut sheet = valid_sheet();
    let incoming = ExtractStaticStyle::new("opacity", "0", 1, None);
    let (StyleProperty::ClassName(class_name) | StyleProperty::Variable { class_name, .. }) =
        incoming.extract(Some("a"));
    // When
    let effects = sheet
        .update_styles(&styles([ExtractStyleValue::Static(incoming)]), "a", false)
        .required("Current Static");
    // Then
    assert_eq!(effects, (true, false));
    assert_eq!(
        sheet.properties["a"][&255][&1],
        std::iter::once(StyleSheetProperty {
            class_name,
            property: "opacity".into(),
            value: "0".into(),
            selector: None,
            layer: None,
            typography: false,
            hoisted: false,
            owner_reset: false,
        })
        .collect()
    );
    rejects_unproved(&mut sheet);
}

#[test]
#[serial_test::serial]
fn current_typography_stays_unproved_when_populated_preset_materializes_members() {
    // Given
    let _state = state();
    let _presets = Presets::save();
    let mut sheet = valid_sheet();
    css::content_typography::set(BTreeMap::from([(
        "repair".into(),
        vec![
            (0, "font-size".into(), "18px".into()),
            (0, "font-weight".into(), "700".into()),
        ],
    )]));
    let incoming = ExtractStaticStyle::new("typography", "repair", 0, None);
    let (StyleProperty::ClassName(class_name) | StyleProperty::Variable { class_name, .. }) =
        incoming.extract(Some("a"));
    // When
    let effects = sheet
        .update_styles(&styles([ExtractStyleValue::Static(incoming)]), "a", false)
        .required("Current populated Typography");
    // Then
    assert_eq!(effects, (true, false));
    assert_eq!(
        sheet.properties["a"][&255][&0],
        [("font-size", "18px"), ("font-weight", "700"),]
            .into_iter()
            .map(|(property, value)| StyleSheetProperty {
                class_name: class_name.clone(),
                property: property.into(),
                value: value.into(),
                selector: None,
                layer: Some("t".into()),
                typography: true,
                hoisted: false,
                owner_reset: false,
            })
            .collect()
    );
    rejects_unproved(&mut sheet);
}

#[test]
#[serial_test::serial]
fn current_dynamic_stays_unproved_when_only_dynamic_and_reset_materialize() {
    // Given
    let _state = state();
    let mut sheet = valid_sheet();
    let incoming = ExtractDynamicStyle::new("padding", 1, "space", None);
    let StyleProperty::Variable {
        class_name,
        variable_name,
        ..
    } = incoming.extract(Some("a"))
    else {
        panic!("dynamic fixture must extract a variable")
    };
    // When
    let effects = sheet
        .update_styles(&styles([ExtractStyleValue::Dynamic(incoming)]), "a", false)
        .required("Current Dynamic/reset");
    // Then
    assert_eq!(effects, (true, false));
    assert_eq!(
        sheet.properties["a"][&255][&1],
        std::iter::once(StyleSheetProperty {
            class_name: class_name.clone(),
            property: "padding".into(),
            value: format!("var({variable_name})"),
            selector: None,
            layer: None,
            typography: false,
            hoisted: false,
            owner_reset: false,
        })
        .collect()
    );
    assert_eq!(
        sheet.properties["a"][&255][&0],
        std::iter::once(StyleSheetProperty {
            class_name,
            property: variable_name,
            value: "initial".into(),
            selector: None,
            layer: None,
            typography: false,
            hoisted: false,
            owner_reset: true,
        })
        .collect()
    );
    rejects_unproved(&mut sheet);
}

#[test]
#[serial_test::serial]
fn current_keyframes_stay_unproved_when_valid_counter_base_receives_only_keyframes() {
    // Given
    let _state = state();
    let mut sheet = valid_sheet();
    let mut incoming = ExtractKeyframes::default();
    incoming.keyframes.insert(
        "from".into(),
        vec![
            ExtractStaticStyle::new("opacity", "0", 0, None),
            ExtractStaticStyle::new("opacity", "0", 0, None),
        ],
    );
    let name = incoming.content_name().name("p");
    // When
    let effects = sheet
        .update_styles(
            &styles([ExtractStyleValue::Keyframes(incoming)]),
            "a",
            false,
        )
        .required("Current Keyframes");
    // Then
    assert_eq!(effects, (true, false));
    assert_eq!(
        sheet.keyframes,
        BTreeMap::from([(
            "a".into(),
            BTreeMap::from([(
                name,
                BTreeMap::from([(
                    "from".into(),
                    vec![
                        ("opacity".into(), "0".into()),
                        ("opacity".into(), "0".into()),
                    ]
                )])
            ),])
        )])
    );
    rejects_unproved(&mut sheet);
}
