use rstest::rstest;
use serial_test::serial;

#[path = "css_boundary_fixtures.rs"]
mod fixtures;
use super::namespace_css_slots::modules;
use fixtures::{Case, Site};

#[test]
#[serial]
fn compiled_forwarding_when_props_have_no_spread_inlines_and_filters_the_target() {
    // Given: without an outer spread the same disjoint css rules admit native inlining.
    let source = "const Card=styled.article({color:'red'});const App=()=>jsx(Card,{id:'inline',css:{padding:1},$private:'secret'});";
    // When: the public extractor lowers compiled JSX and the resulting element renders.
    let actual = super::execute(
        "forwarding.ts",
        source,
        "const raw=App();const final=render(raw);JSON.stringify([typeof raw.target==='string',final.target,final.props.id,Object.hasOwn(final.props,'$private'),values(final,'color'),values(final,'padding')]);",
    );
    // Then: the native target receives public props and both rules, but no private prop.
    assert_eq!(
        actual,
        serde_json::json!([true, "article", "inline", false, ["red"], ["1px"]])
    );
}

#[test]
#[serial]
fn compiled_forwarding_when_props_include_a_spread_retains_the_renderer() {
    // Given: a spread prevents proof of the rendered tag even with disjoint css rules.
    let source = "const Card=styled.article({color:'red'});const App=()=>jsx(Card,{...{id:'spread'},css:{padding:1},$private:'secret'});";
    // When: the public extractor lowers compiled JSX and the emitted renderer runs.
    let actual = super::execute(
        "forwarding.ts",
        source,
        "const raw=App();const final=render(raw);JSON.stringify([typeof raw.target==='string',final.target,final.props.id,Object.hasOwn(final.props,'$private'),values(final,'color'),values(final,'padding')]);",
    );
    // Then: the retained styled boundary filters private props and keeps both sets of rules.
    assert_eq!(
        actual,
        serde_json::json!([false, "article", "spread", false, ["red"], ["1px"]])
    );
}

fn observe(site: Site, case: Case, single_css: bool) -> serde_json::Value {
    let fixture = fixtures::fixture(site, case);
    let files: Vec<_> = fixture
        .files
        .iter()
        .map(|(name, code)| (name.as_str(), code.as_str()))
        .collect();
    let outputs = modules::compile(&files, single_css);
    modules::execute(&outputs, &fixture.observation)
}

#[rstest]
#[serial]
fn styled_forwarding_survives_when_css_replaces_renderer(
    #[values(
        Site::LocalJsx,
        Site::LocalCompiled,
        Site::NamespaceJsx,
        Site::NamespaceCompiled,
        Site::NamespaceComputed
    )]
    site: Site,
    #[values(
        Case::Inline,
        Case::Configured,
        Case::Slot,
        Case::Retained,
        Case::Primitive,
        Case::Generic
    )]
    case: Case,
    #[values(false, true)] single_css: bool,
) {
    // Given: styled and non-styled controls with identical authored props.
    let _state = crate::test_state::TestStateGuard::new();
    // When: the complete defining/entry modules render through their real bindings.
    let actual = observe(site, case, single_css);
    // Then: only the styled contract filters private/configured props, preserving public inputs.
    assert_eq!(
        actual["private"],
        matches!(case, Case::Primitive | Case::Generic)
    );
    assert_eq!(
        actual["rawNative"],
        matches!(case, Case::Inline | Case::Configured | Case::Primitive)
    );
    assert_eq!(actual["rawSlots"], usize::from(matches!(case, Case::Slot)));
    if matches!(case, Case::Configured | Case::Retained) {
        assert_eq!(actual["blocked"], false);
    }
    assert_eq!(
        actual["public"],
        serde_json::json!([
            "article", "first", "last", "label", true, 7, "child", "black", true
        ])
    );
    assert_eq!(
        actual["baseTrace"],
        if matches!(case, Case::Slot) {
            serde_json::json!(["base"])
        } else {
            serde_json::json!([])
        }
    );
}

#[rstest]
#[serial]
fn denied_values_evaluate_once_when_inline_forwarding_filters_them(
    #[values(
        Site::LocalJsx,
        Site::LocalCompiled,
        Site::NamespaceJsx,
        Site::NamespaceCompiled,
        Site::NamespaceComputed
    )]
    site: Site,
    #[values(false, true)] single_css: bool,
) {
    // Given: denied values contain a getter and an original normal function/default.
    let _state = crate::test_state::TestStateGuard::new();
    // When: authored props construct and the resolved styled target renders once.
    let actual = observe(site, Case::Effects, single_css);
    // Then: filtering preserves authored order, default/receiver/arguments and public props.
    assert_eq!(
        actual["trace"],
        serde_json::json!([
            "before", "getter", "default", "this", 0, "ref", "children", "after"
        ])
    );
    assert_eq!(actual["private"], false);
    assert_eq!(actual["blocked"], false);
    assert_eq!(
        actual["public"],
        serde_json::json!([
            "article", "first", "last", "label", true, 7, "child", "black", true
        ])
    );
}

#[rstest]
#[serial]
fn original_getter_receiver_survives_when_forwarding_omits_its_property(
    #[values(Site::LocalCompiled, Site::NamespaceCompiled, Site::NamespaceComputed)] site: Site,
    #[values(Case::Accessors, Case::RetainedAccessors)] case: Case,
    #[values(false, true)] single_css: bool,
) {
    // Given: own accessor properties read other original config properties through this.
    let _state = crate::test_state::TestStateGuard::new();
    // When: the emitted complete module constructs the element and renders once.
    let actual = observe(site, case, single_css);
    // Then: both getters run once on their original receiver before either value is omitted.
    assert_eq!(
        actual["trace"],
        serde_json::json!(["private", "first", "blocked", "first"])
    );
    assert_eq!(actual["private"], false);
    assert_eq!(actual["blocked"], false);
}
