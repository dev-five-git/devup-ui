use super::counter_fixture_support::{fixture, produced, state};
use css::{
    allocation_input::{
        AllocationContext, CapturedNameConfig, LegacyDeclaration, LegacyInput, LegacyVariable,
        NameMode,
    },
    counter_names::{AllocatedName, NameAddress},
};
use extractor::extract_style::ExtractDynamicStyle;
use std::collections::HashMap;

#[rstest::rstest]
#[case(NameMode::Debug)]
#[case(NameMode::AtomHoist)]
#[serial_test::serial]
fn baseline_no_site_produces_exact_names_without_slots_when_mode_is_not_counter(
    #[case] mode: NameMode,
) {
    // Given: a genuine Counter constructor in a baseline naming mode with preserved map state.
    let _state = state();
    match mode {
        NameMode::Debug => css::debug::set_debug(true),
        NameMode::AtomHoist => css::atom_hoist::set_atom_hoist(Some(2)),
        NameMode::Counter => panic!("baseline fixture requires debug or atom"),
    }
    let before = HashMap::from([
        ("empty".into(), HashMap::new()),
        (String::new(), HashMap::from([("seed".into(), 9)])),
    ]);
    css::class_map::set_class_map(before.clone());
    let style = fixture("a", || ExtractDynamicStyle::new("color", 0, "tone", None));
    let (selector, value, class_name, variable_name) = match mode {
        NameMode::Debug => (None, None, "pcolor-0---255", "--pcolor-0-"),
        NameMode::AtomHoist => (
            Some("n-n".into()),
            Some("var(--pv1-636f6c6f72-0-6e2d6e)".into()),
            concat!(
                "pa1-g-636f6c6f72-0-s-",
                "766172282d2d7076312d363336663663366637322d302d36653264366529",
                "-6e2d6e-255"
            ),
            "--pv1-636f6c6f72-0-6e2d6e",
        ),
        NameMode::Counter => panic!("baseline fixture requires debug or atom"),
    };
    let context = AllocationContext {
        config: CapturedNameConfig {
            prefix: "p".into(),
            mode,
        },
        file: None,
        delivery: None,
    };
    let input = LegacyInput::Declaration(LegacyDeclaration {
        property: "color".into(),
        level: 0,
        value,
        selector: selector.clone(),
        order: None,
    });
    let variable_input = LegacyInput::Variable(LegacyVariable {
        property: "color".into(),
        level: 0,
        selector,
    });
    // When: the real no-site adapter uses the selected baseline projection.
    let receipt = produced(style.counter_produce(None));
    // Then: exact names and full baseline envelopes match independent inputs; maps stay unchanged.
    assert_eq!(
        receipt.class.allocation,
        AllocatedName {
            name: class_name.into(),
            address: NameAddress::Baseline {
                mode,
                input: input.clone(),
                context: context.clone()
            }
        }
    );
    assert_eq!(receipt.class.input, input);
    assert_eq!(receipt.class.context, context);
    assert_eq!(receipt.variable, variable_name);
    let variable = receipt
        .variable_allocation
        .unwrap_or_else(|| panic!("baseline variable receipt"));
    assert_eq!(variable.input, variable_input);
    assert_eq!(
        variable.allocation,
        AllocatedName {
            name: variable_name.into(),
            address: NameAddress::Baseline {
                mode,
                input: variable_input,
                context
            }
        }
    );
    assert_eq!(css::class_map::get_class_map(), before);
}
