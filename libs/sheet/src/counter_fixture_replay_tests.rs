use super::counter_fixture_support::{address, fixture, produced, state};
use crate::{
    StyleSheetProperty,
    counter_evidence::{Expansion, RecordFootprint},
    emission_seed::{
        DeclarationSeed, EmissionContext, EmissionInput, EmissionSeed, NumericSite, Resolution,
    },
};
use extractor::extract_style::ExtractDynamicStyle;
use std::collections::HashMap;

#[rstest::rstest]
#[case(false)]
#[case(true)]
#[serial_test::serial]
fn sheet_replay_adds_consumer_and_reset_without_reservations_when_producer_is_authentic(
    #[case] numeric: bool,
) {
    // Given: independent source expectations describe the actual public constructor.
    let _state = state();
    let (variable, site, class_key) = if numeric {
        (
            "---pSa-c-d",
            Some(NumericSite {
                source: 0,
                at: 2,
                role: 3,
            }),
            "color-1-var(---pSa-c-d)--255",
        )
    } else {
        ("--pb", None, "color-1---255")
    };
    let style = fixture("a", || {
        let style = ExtractDynamicStyle::new("color", 1, "tone", None);
        if numeric { style.at_role(2, 3) } else { style }
    });
    let seed = EmissionSeed {
        placement: EmissionContext {
            source_file: "a".into(),
            bucket: "root".into(),
            single_css: true,
            hoisted: false,
        },
        body: EmissionInput::Dynamic {
            declaration: DeclarationSeed {
                property: "color".into(),
                value: String::new(),
                level: 1,
                selector: None,
                style_order: None,
                layer: None,
                resolution: Resolution::CssVariable,
                first_value: None,
                preset: None,
            },
            variable: variable.into(),
            site: site.clone(),
            important: false,
        },
    };
    let receipt = produced(style.counter_produce(None));
    let before = if numeric {
        HashMap::from([(String::new(), HashMap::from([(class_key.into(), 0)]))])
    } else {
        HashMap::from([(
            String::new(),
            HashMap::from([(class_key.into(), 0), ("color-1-".into(), 1)]),
        )])
    };
    assert_eq!(css::class_map::get_class_map(), before);
    address(&receipt.class, ("", class_key, 0, "pa"));
    assert_eq!(receipt.variable, variable);
    // When: the existing sheet replay creates an expansion with the independent expected class.
    let expansion = seed
        .replay("pa")
        .unwrap_or_else(|error| panic!("sheet replay: {error:?}"));
    // Then: consumer/reset bind the same class and variable, without extra counter slots.
    let consumer = RecordFootprint::Property {
        bucket: "root".into(),
        order: 255,
        level: 1,
        record: StyleSheetProperty {
            class_name: "pa".into(),
            property: "color".into(),
            value: format!("var({variable})"),
            selector: None,
            layer: None,
            typography: false,
            hoisted: false,
            owner_reset: false,
        },
    };
    let reset = RecordFootprint::Property {
        bucket: "root".into(),
        order: 255,
        level: 0,
        record: StyleSheetProperty {
            class_name: "pa".into(),
            property: variable.into(),
            value: "initial".into(),
            selector: None,
            layer: None,
            typography: false,
            hoisted: false,
            owner_reset: true,
        },
    };
    assert_eq!(
        expansion,
        Expansion::Dynamic {
            variable: variable.into(),
            site,
            important: false,
            consumer: Box::new(consumer),
            reset: Box::new(reset)
        }
    );
    assert_eq!(css::class_map::get_class_map(), before);
}
