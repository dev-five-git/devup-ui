use rstest::rstest;
use serial_test::serial;

pub(super) use super::super::namespace_module_execution as modules;

#[path = "namespace_fixtures.rs"]
mod fixtures;
use fixtures::{fixture, observation};

#[path = "imported_retarget_execution.rs"]
mod imported_retarget_execution;
#[path = "imported_stage_execution.rs"]
mod imported_stage_execution;

#[rstest]
#[case::inline("inline", false, 0)]
#[case::slot("slot", true, 1)]
#[case::retained("retained", true, 0)]
#[case::inherited("inherited", true, 1)]
#[case::retarget("retarget", true, 1)]
#[case::forwarding("forwarding", true, 1)]
#[case::as_target("as", true, 0)]
#[case::choice("choice", true, 0)]
#[serial]
fn namespace_css_when_complete_modules_render_preserves_lowering_and_captures(
    #[case] kind: &str,
    #[case] identity: bool,
    #[case] slots: usize,
    #[values("jsx", "dot", "computed")] surface: &str,
    #[values(false, true)] selected: bool,
    #[values(false, true)] single_css: bool,
) {
    // Given: an imported selected component, a real resolver, and shadowed caller bindings.
    let _state = crate::test_state::TestStateGuard::new();
    let files = fixture(kind, surface, selected);
    let borrowed: Vec<_> = files
        .iter()
        .map(|(name, code)| (name.as_str(), code.as_str()))
        .collect();
    let outputs = modules::compile(&borrowed, single_css);
    // When: both emitted modules execute as ESM, including the defining lexical scope.
    let actual = modules::execute(
        &outputs,
        &observation(
            files[1]
                .0
                .rsplit('/')
                .next()
                .unwrap_or_else(|| panic!("entry filename")),
        ),
    );
    // Then: Inline/Slot/Retained retain identity, authored effects and private-slot consumption.
    assert_eq!(actual["before"], serde_json::json!([]));
    assert_eq!(actual["identity"], true);
    assert_eq!(actual["rawIdentity"], identity);
    assert_eq!(actual["rawSlots"], slots);
    let trace = if matches!(
        kind,
        "slot" | "inherited" | "retarget" | "forwarding" | "as"
    ) {
        if selected {
            vec!["controller", "leaf", "controller"]
        } else {
            vec!["controller", "controller", "leaf"]
        }
    } else {
        vec![]
    };
    assert_eq!(
        actual["trace"],
        if matches!(kind, "retained" | "choice") {
            serde_json::json!(["default", true, 1, "default", true, 1])
        } else {
            serde_json::json!(trace)
        }
    );
    for (index, element) in actual["elements"]
        .as_array()
        .unwrap_or_else(|| panic!("rendered elements array"))
        .iter()
        .enumerate()
    {
        assert_eq!(
            element["target"],
            match kind {
                "retarget" => "section",
                "as" => "nav",
                _ => "article",
            }
        );
        assert_eq!(element["ref"], 7 + index);
        assert_eq!(element["children"], "child");
        assert_eq!(element["slots"], serde_json::json!([]));
        assert_eq!(element["transient"], false);
        assert_eq!(element["blocked"], kind != "forwarding");
        assert!(
            element["classes"]
                .as_array()
                .unwrap_or_else(|| panic!("rendered classes array"))
                .contains(&serde_json::json!("external"))
        );
    }
}
