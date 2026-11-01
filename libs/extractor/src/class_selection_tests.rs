use crate::{class_selection_expectations as intent, class_selection_support as support};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case(1)]
#[case(2)]
#[case(3)]
#[case(4)]
#[case(5)]
#[case(6)]
#[case(7)]
#[case(8)]
#[case(9)]
#[case(10)]
#[case(11)]
#[case(12)]
#[case(13)]
#[case(14)]
#[case(15)]
#[case(16)]
#[case(17)]
#[case(18)]
#[case(19)]
#[case(20)]
#[case(21)]
#[case(22)]
#[case(23)]
#[case(24)]
#[case(25)]
#[case(26)]
#[case(27)]
#[case(28)]
#[case(29)]
#[case(30)]
#[case(31)]
#[case(32)]
#[case(33)]
#[case(34)]
#[serial]
fn test_css_composes_known_classes(#[case] case: usize, #[values(false, true)] single_css: bool) {
    // Given: the original complete module and independently varied authored globals.
    let _state = crate::test_state::TestStateGuard::new();
    let outputs = support::known(case, single_css);
    for state in intent::states(case) {
        // When: ESM executes the entire emitted module, including all compiler captures.
        let tokens = support::observe_known(&outputs, state);
        // Then: exact metadata keys/values and opaque tokens match authored intent.
        assert_eq!(
            support::meanings(&outputs, &tokens, single_css),
            intent::expected(case, state, single_css),
            "case{case} {state:?} single_css={single_css}\n{outputs:?}\n{tokens}"
        );
    }
}

pub(crate) fn imported_b(single_css: bool) {
    let outputs = support::compile(
        &[
            (
                "/namespace/app.ts",
                crate::class_selection_fixtures::IMPORTED_APP,
            ),
            (
                "/namespace/styles.ts",
                crate::class_selection_fixtures::IMPORTED_BASE,
            ),
        ],
        support::option(single_css, "@devup-ui/react"),
    );
    assert_eq!(outputs[0].1.dependencies, vec!["/namespace/styles.ts"]);
    let observed = support::observe(
        &outputs,
        "import {a,b} from './app.ts';globalThis.observed=[a,b];",
        "",
    );
    for (index, color) in [(0, "crimson"), (1, "teal")] {
        let mut expected = vec![
            intent::declaration("color", color),
            intent::Meaning::Declaration(intent::Declaration {
                property: "color".into(),
                value: "red".into(),
                level: 0,
                selector: Some("&:hover".into()),
                layer: None,
                order: 255,
            }),
        ];
        if index == 1 {
            expected.push(intent::declaration("margin", "4px"));
        }
        expected.sort();
        assert_eq!(
            support::meanings(
                &outputs,
                observed[index]
                    .as_str()
                    .unwrap_or_else(|| panic!("imported class result")),
                single_css
            ),
            expected
        );
    }
}

#[rstest]
#[serial]
fn imported_class_b_selects_original_keys(#[values(false, true)] single_css: bool) {
    // Given: the original base/danger declarations and b call across real module boundaries.
    let _state = crate::test_state::TestStateGuard::new();
    // When/Then: execute both complete modules and check exact selected declarations.
    imported_b(single_css);
}

#[rstest]
#[case::inline(
    crate::class_selection_fixtures::INLINE,
    "[a(false),a(true),a('opaque')]"
)]
#[case::text(crate::class_selection_fixtures::TEXT, "[a]")]
#[serial]
fn snapshot_fixture_selects_original_meaning(
    #[case] source: &str,
    #[case] call: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given: the byte-identical second tuple from each original snapshot owner.
    let _state = crate::test_state::TestStateGuard::new();
    let outputs = support::compile(
        &[("/namespace/fixture.ts", source)],
        support::option(single_css, "@devup-ui/react"),
    );
    // When: the complete emitted ESM executes, not a detached class expression.
    let observed = support::observe(
        &outputs,
        &format!("import {{a}} from './fixture.ts';globalThis.observed={call};"),
        "",
    );
    // Then: independent exact winners exclude the covered red/green declaration respectively.
    let mut expected = match call {
        "[a]" => vec![
            intent::declaration("color", "red"),
            intent::declaration("margin", "1px"),
            intent::declaration("padding", "2px"),
        ],
        _ => vec![
            intent::declaration("color", "green"),
            intent::declaration("margin", "4px"),
        ],
    };
    expected.sort();
    for tokens in observed
        .as_array()
        .unwrap_or_else(|| panic!("snapshot fixture observations"))
    {
        assert_eq!(
            support::meanings(
                &outputs,
                tokens
                    .as_str()
                    .unwrap_or_else(|| panic!("snapshot fixture class result")),
                single_css
            ),
            expected
        );
    }
}
