use crate::{
    StyleSheetProperty,
    counter_evidence::{EmissionWitness, Materialization, RecordFootprint},
    emission_seed::*,
};
use css::style_selector::StyleSelector;
use std::fmt::Debug;

pub(crate) fn require_ok<T, E: Debug>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("expected Ok, got Err({error:?})"),
    }
}

pub(crate) fn require_some<T>(option: Option<T>) -> T {
    match option {
        Some(value) => value,
        None => panic!("expected Some, got None"),
    }
}

#[test]
#[should_panic(expected = "Err(Expansion)")]
fn require_ok_rejects_an_error_instead_of_accepting_the_fixture() {
    require_ok::<(), _>(Err(crate::counter_evidence::ReplayError::Expansion));
}

#[test]
#[should_panic(expected = "got None")]
fn require_some_rejects_an_absent_fixture() {
    require_some::<()>(None);
}

pub(crate) fn declaration(property: &str, value: &str) -> DeclarationSeed {
    DeclarationSeed {
        property: property.into(),
        value: value.into(),
        level: 0,
        selector: None,
        style_order: None,
        layer: None,
        resolution: Resolution::CssVariable,
        first_value: None,
        preset: None,
    }
}

pub(crate) fn seed(body: EmissionInput) -> EmissionSeed {
    EmissionSeed {
        placement: EmissionContext {
            source_file: "raw.tsx".into(),
            bucket: "delivery.tsx".into(),
            single_css: false,
            hoisted: false,
        },
        body,
    }
}

pub(crate) fn preset() -> FrozenPreset {
    FrozenPreset {
        name: "heading".into(),
        frames: vec![
            Some(FrozenTypography {
                font_size: Some("14px".into()),
                font_weight: Some("400".into()),
                ..FrozenTypography::default()
            }),
            None,
            Some(FrozenTypography {
                font_size: Some("24px".into()),
                font_family: Some(" $fonts.heading ".into()),
                ..FrozenTypography::default()
            }),
        ],
    }
}

pub(crate) fn property(value: &str) -> RecordFootprint {
    RecordFootprint::Property {
        bucket: "delivery.tsx".into(),
        order: 255,
        level: 0,
        record: StyleSheetProperty {
            class_name: "a".into(),
            property: "font-size".into(),
            value: value.into(),
            selector: None,
            layer: None,
            typography: false,
            hoisted: false,
            owner_reset: false,
        },
    }
}

pub(crate) fn witness(seed: EmissionSeed) -> EmissionWitness {
    EmissionWitness {
        expansion: require_ok(seed.replay("a")),
        seed,
        name: "a".into(),
        materialization: Materialization::Complete,
    }
}

pub(crate) fn global_dynamic() -> EmissionWitness {
    let mut declaration = declaration("padding", "independent");
    declaration.selector = Some(StyleSelector::Global("body".into(), "raw.tsx".into()));
    witness(seed(EmissionInput::Dynamic {
        declaration,
        variable: "--v".into(),
        site: None,
        important: false,
    }))
}
