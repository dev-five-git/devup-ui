use super::modules;
use rstest::rstest;
use serial_test::serial;

#[path = "imported_stage_controls.rs"]
mod controls;
#[path = "imported_stage_identity.rs"]
mod identity;

#[rstest]
#[serial]
fn imported_stages_match_local_attrs_when_the_same_pipeline_executes(
    #[values(false, true)] jsx: bool,
    #[values(false, true)] selected: bool,
    #[values(false, true)] single_css: bool,
) {
    // Given: the identical supported attrs/callback pipeline, local and across module owners.
    let _state = crate::test_state::TestStateGuard::new();
    let files = fixtures::local(jsx);
    let borrowed: Vec<_> = files
        .iter()
        .map(|(name, source)| (name.as_str(), source.as_str()))
        .collect();
    let outputs = modules::compile(&borrowed, single_css);
    let local = modules::execute(&outputs, &fixtures::local_observation(jsx, selected));
    let files = fixtures::files("named", "native", jsx);
    let borrowed: Vec<_> = files
        .iter()
        .map(|(name, source)| (name.as_str(), source.as_str()))
        .collect();
    let outputs = modules::compile(&borrowed, single_css);
    // When: both concrete render pipelines execute.
    let imported = modules::execute(&outputs, &fixtures::observation(jsx, selected, "native"));
    // Then: imported stages preserve the local authored trace, target and merged attrs.
    assert_eq!(imported["trace"], local["trace"]);
    assert_eq!(imported["target"], local["target"]);
    assert_eq!(imported["props"]["style"], local["props"]["style"]);
    for actual in [&imported, &local] {
        assert!(
            actual["props"]["className"]
                .as_str()
                .unwrap_or_else(|| panic!("classes"))
                .ends_with("baseAttr ownAttr lastAttr external")
        );
    }
}

#[path = "imported_stage_fixtures.rs"]
pub(super) mod fixtures;

#[rstest]
#[serial]
fn imported_stages_preserve_order_when_extended_across_modules(
    #[values("named", "reexport", "dot", "computed", "reserved")] surface: &str,
    #[values("native", "component", "as")] target: &str,
    #[values(false, true)] jsx: bool,
    #[values(false, true)] selected: bool,
    #[values(false, true)] single_css: bool,
) {
    // Given: origin-owned callbacks and three attrs stages across real ESM boundaries.
    let _state = crate::test_state::TestStateGuard::new();
    let files = fixtures::files(surface, target, jsx);
    let borrowed: Vec<_> = files
        .iter()
        .map(|(name, source)| (name.as_str(), source.as_str()))
        .collect();
    let outputs = modules::compile(&borrowed, single_css);
    // When: imported extension, repeated retarget and the concrete renderer execute.
    let actual = modules::execute(&outputs, &fixtures::observation(jsx, selected, target));
    // Then: all attrs precede callbacks, with lexical receiver/defaults and exact inputs intact.
    let mut trace = serde_json::json!([
        "base-attrs",
        "own-attrs",
        "last-attrs",
        "base-default",
        true,
        1,
        "base-callback",
        "last",
        "own-default",
        true,
        1,
        "own-callback",
        "last",
        "last-callback",
        "last",
    ]);
    if target == "component" {
        trace
            .as_array_mut()
            .unwrap_or_else(|| panic!("trace array"))
            .extend([serde_json::json!("target"), serde_json::json!("em")]);
    }
    assert_eq!(actual["trace"], trace);
    assert_eq!(
        actual["target"],
        match target {
            "native" => "section",
            "component" => "custom",
            "as" => "nav",
            _ => panic!("known target"),
        }
    );
    assert_eq!(actual["props"]["ref"], 7);
    assert_eq!(actual["props"]["children"], "child");
    assert_eq!(actual["props"]["as"], "em");
    assert_eq!(
        actual["props"]["style"],
        serde_json::json!({"baseStyle":1,"ownStyle":2,"caller":3})
    );
    let classes = actual["props"]["className"]
        .as_str()
        .unwrap_or_else(|| panic!("classes"));
    assert!(
        classes.ends_with("baseAttr ownAttr lastAttr external"),
        "{classes}"
    );
    assert_eq!(actual["props"].get("$private"), None);
    assert_eq!(
        actual["props"]
            .as_object()
            .unwrap_or_else(|| panic!("props"))
            .keys()
            .filter(|name| name.starts_with("__devup"))
            .count(),
        0
    );
}

#[rstest]
#[serial]
fn imported_stages_retain_execution_when_the_export_was_locally_retargeted(
    #[values(false, true)] single_css: bool,
) {
    // Given: a local captured retarget exported before the imported extension chain.
    let _state = crate::test_state::TestStateGuard::new();
    let files = fixtures::exported_retarget();
    let borrowed: Vec<_> = files
        .iter()
        .map(|(name, source)| (name.as_str(), source.as_str()))
        .collect();
    let outputs = modules::compile(&borrowed, single_css);
    // When: the final imported retarget renders.
    let actual = modules::execute(&outputs, &fixtures::observation(false, true, "native"));
    // Then: origin and extension callbacks still run exactly once, after every attrs stage.
    assert_eq!(actual["target"], "section");
    assert_eq!(
        actual["trace"],
        serde_json::json!([
            "base-attrs",
            "own-attrs",
            "last-attrs",
            "base-default",
            true,
            1,
            "base-callback",
            "last",
            "own-default",
            true,
            1,
            "own-callback",
            "last",
            "last-callback",
            "last",
        ])
    );
}

#[rstest]
#[serial]
fn imported_stages_keep_callback_scopes_when_an_extension_covers_a_getter(
    #[values(false, true)] local: bool,
    #[values(false, true)] single_css: bool,
) {
    // Given: normal/lexical callbacks and receiver-sensitive getters, also compiled locally.
    let _state = crate::test_state::TestStateGuard::new();
    let files = fixtures::callbacks(local);
    let borrowed: Vec<_> = files
        .iter()
        .map(|(name, source)| (name.as_str(), source.as_str()))
        .collect();
    let outputs = modules::compile(&borrowed, single_css);
    // When: a static extension covers the original dynamic color declaration.
    let actual = modules::execute(&outputs, &fixtures::callback_observation(local));
    // Then: covered work still executes once with its original receiver and callback scope.
    assert_eq!(
        actual["trace"],
        serde_json::json!([
            "normal-default",
            true,
            1,
            "color",
            true,
            "lexical-default",
            true,
            "margin",
            true,
        ])
    );
    assert_eq!(actual["target"], "a");
    assert_eq!(actual["props"]["ref"], 7);
    assert_eq!(actual["props"]["children"], "child");
}
