use super::modules;
use rstest::rstest;
use serial_test::serial;

#[path = "imported_stage_identity_fixtures.rs"]
mod fixtures;

#[rstest]
#[serial]
fn imported_stages_preserve_effective_default_when_target_getters_are_observable(
    #[values("origin", "retarget", "caller", "repeat", "throw")] mode: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given: private lazy target getters in different modules and a captured callback default.
    let _state = crate::test_state::TestStateGuard::new();
    let (files, observer) = fixtures::targets(mode);
    let borrowed: Vec<_> = files
        .iter()
        .map(|(name, source)| (name.as_str(), source.as_str()))
        .collect();
    let outputs = modules::compile(&borrowed, single_css);
    // When: the complete ESM graph resolves the actual effective target and executes its renderer.
    let actual = modules::execute(&outputs, &observer);
    // Then: only the selected default runs, before callbacks; caller as skips defaults entirely.
    assert_eq!(actual["before"], serde_json::json!([]));
    let (target, trace) = match mode {
        "origin" => ("button", serde_json::json!(["origin-default", "callback"])),
        "retarget" => ("a", serde_json::json!(["first-default", "callback"])),
        "caller" => ("nav", serde_json::json!(["callback"])),
        "repeat" => ("section", serde_json::json!(["last-default", "callback"])),
        "throw" => {
            assert_eq!(actual["error"], "RangeError");
            assert_eq!(actual["trace"], serde_json::json!(["origin-default"]));
            return;
        }
        _ => panic!("known target mode"),
    };
    assert_eq!(actual["target"], target);
    assert_eq!(actual["trace"], trace);
}

#[rstest]
#[serial]
fn imported_stages_share_callback_props_when_an_inherited_default_mutates_them(
    #[values(false, true)] single_css: bool,
) {
    // Given: admitted callback defaults with an inherited mutation and a later read.
    let _state = crate::test_state::TestStateGuard::new();
    let (files, observer) = fixtures::callback_frame();
    let borrowed: Vec<_> = files
        .iter()
        .map(|(name, source)| (name.as_str(), source.as_str()))
        .collect();
    let outputs = modules::compile(&borrowed, single_css);
    // When: origin and extension callbacks execute in their own lexical modules.
    let actual = modules::execute(&outputs, &observer);
    // Then: they receive the same callback props frame, as in local flattened composition.
    assert_eq!(
        actual["trace"],
        serde_json::json!(["origin", "own", "origin"])
    );
}

#[rstest]
#[case::import_plain((false, false))]
#[case::import_hybrid((false, true))]
#[case::local_plain((true, false))]
#[case::local_hybrid((true, true))]
#[serial]
fn imported_stages_preserve_style_identity_when_attrs_use_the_local_merge_mode(
    #[case] site: (bool, bool),
    #[values(false, true)] provided: bool,
    #[values(false, true)] single_css: bool,
) {
    // Given: plain prefix attrs, optionally followed by a callback, and a caller style getter.
    let _state = crate::test_state::TestStateGuard::new();
    let (local, hybrid) = site;
    let files = fixtures::files(local, hybrid);
    let borrowed: Vec<_> = files
        .iter()
        .map(|(name, source)| (name.as_str(), source.as_str()))
        .collect();
    let outputs = modules::compile(&borrowed, single_css);
    // When: all attrs and the no-variable styled callbacks render through complete ESM.
    let actual = modules::execute(&outputs, &fixtures::observation(local, provided));
    // Then: style identity, absence and getter timing retain the original local attrs contract.
    assert_eq!(actual["sameStyle"], !hybrid);
    assert_eq!(actual["absentStyle"], !hybrid && !provided);
    assert_eq!(
        actual["trace"],
        match (hybrid, provided) {
            (false, _) => serde_json::json!([]),
            (true, false) => serde_json::json!(["attrs", false]),
            (true, true) => serde_json::json!(["style", "attrs", false]),
        }
    );
}

#[rstest]
#[serial]
fn imported_stages_preserve_absent_props_when_plain_attrs_have_no_styles(
    #[values(false, true)] single_css: bool,
) {
    // Given: a plain-attrs renderer with no class source or variable source.
    let _state = crate::test_state::TestStateGuard::new();
    let (files, observer) = fixtures::empty();
    let borrowed: Vec<_> = files
        .iter()
        .map(|(name, source)| (name.as_str(), source.as_str()))
        .collect();
    let outputs = modules::compile(&borrowed, single_css);
    // When: the generated renderer calls an arbitrary component target.
    let actual = modules::execute(&outputs, &observer);
    // Then: private stages do not invent empty public className/style values.
    assert_eq!(actual["classType"], "undefined");
    assert_eq!(actual["styleType"], "undefined");
    assert_eq!(actual["target"], "custom");
}
