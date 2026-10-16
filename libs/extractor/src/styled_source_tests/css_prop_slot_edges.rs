use super::css_prop_execution::{CAPTURED, css_props, element, execute};
use super::*;

#[rstest]
#[case("const __devupCss0='authored';", "__devupCss0")]
#[case(
    "function reserve(__devupCss0){return __devupCss0;}const authored='authored';",
    "authored"
)]
#[case("const authored={__devupCss0:'authored'};", "authored.__devupCss0")]
#[case(
    "const authored={'__devupCss0':'authored'};",
    "authored['__devupCss0']"
)]
#[case(
    "const authored={['__devupCss0']:'authored'};",
    "authored['__devupCss0']"
)]
#[case(
    "const authored=<aside __devupCss0='authored'/>;",
    "authored.props.__devupCss0"
)]
#[serial]
fn css_prop_slot_avoids_authored_names_when_reserving_before_traversal(
    #[case] collision: &str,
    #[case] authored: &str,
) {
    // Given: original bindings, JSX names and static/string keys occupy the first key.
    let source = format!("{collision}{CAPTURED}const App=()=> <Card css={{{{padding:'4px'}}}}/>;");
    // When: the extracted JSX executes, including authored records unrelated to styling.
    let result = execute(
        "test.tsx",
        &source,
        &format!(
            "const raw=App();const keys=slots(raw);const final=render(raw);JSON.stringify([raw.target===Card,keys.length,keys.includes('__devupCss0'),{authored},slots(final),values(final,'color'),trace]);"
        ),
    );
    // Then: the private protocol never reuses an original name or forwards its own key.
    assert_eq!(
        result,
        serde_json::json!([
            true,
            1,
            false,
            "authored",
            [],
            ["green"],
            ["controller", "leaf"]
        ])
    );
}

#[rstest]
#[case("test.tsx")]
#[case("test.ts")]
#[case("test.cts")]
#[serial]
fn css_prop_slot_composes_inherited_fixed_styles_when_retargeted_then_extended(
    #[case] filename: &str,
) {
    // Given: inherited captures remain deferred through native retargeting and extension.
    let app = element(filename, &css_props(filename, "{color:'blue'}", ""));
    let base = CAPTURED.replace("const Card=", "const Base=");
    let source = format!(
        "{base}const Mid=Base.withComponent('section');const Card=styled(Mid)({{backgroundColor:'black'}});const App=()=>{app};"
    );
    // When: only the final component renders, not its unused siblings.
    let result = execute(
        filename,
        &source,
        "const before=trace.slice();const raw=App();const final=render(raw);JSON.stringify([before,raw.target===Card,final.target,values(final,'color'),values(final,'background-color'),slots(final),trace]);",
    );
    // Then: recomputed capability retains the right closure and all winning fixed atoms.
    assert_eq!(
        result,
        serde_json::json!([
            [],
            true,
            "section",
            ["blue"],
            ["black"],
            [],
            ["controller", "leaf"]
        ])
    );
}

#[rstest]
#[case("const Card=Base.withComponent(Target);")]
#[case("const Card=styled(Base).attrs({id:'card'})({});")]
#[case("const Card=styled(Base)`background-color:${p=>p.tone};`;")]
#[case("const Card=styled(Base)('opaque',{});")]
#[serial]
fn css_prop_slot_is_absent_when_extension_loses_fixed_native_capability(#[case] extension: &str) {
    // Given: each extension invalidates a different eligibility condition.
    let base = CAPTURED.replace("const Card=", "const Base=");
    let source = format!(
        "{base}const Target='aside';{extension}const App=()=>jsx(Card,{{css:{{padding:'4px'}},tone:'black'}});"
    );
    // When: the original non-overlapping retained-target lowering executes.
    let result = execute(
        "test.ts",
        &source,
        "const raw=App();const final=render(raw);JSON.stringify([raw.target===Card,slots(raw),slots(final),values(final,'padding'),trace]);",
    );
    // Then: no stale inherited capability installs a slot on an ineligible renderer.
    assert_eq!(
        result,
        serde_json::json!([true, [], [], ["4px"], ["controller", "leaf"]])
    );
}

#[rstest]
#[case("test.tsx")]
#[case("test.ts")]
#[serial]
fn css_prop_slot_orders_selector_breakpoint_atoms_when_later_parts_overlap(#[case] filename: &str) {
    // Given: fixed responsive/selector styles overlap only in the same composition key.
    let base = CAPTURED.replace(
        "color:'green'",
        "color:['green','yellow'],_hover:{color:'orange'}",
    );
    let app = element(filename, &css_props(filename, "responsive", ""));
    let source = format!(
        "{base}const responsive=css({{color:['blue','pink'],_hover:{{color:'purple'}}}});const App=()=>{app};"
    );
    // When: the compiled call renders its composed class list.
    let result = execute(
        filename,
        &source,
        "const raw=App();const final=render(raw);JSON.stringify([raw.target===Card,values(final,'color'),values(final,'color',1),values(final,'color',0,'&:hover'),slots(final),trace]);",
    );
    // Then: every key has exactly its later atom, never a losing own class.
    assert_eq!(
        result,
        serde_json::json!([
            true,
            ["blue"],
            ["pink"],
            ["purple"],
            [],
            ["controller", "leaf"]
        ])
    );
}

#[rstest]
#[case("test.tsx")]
#[case("test.ts")]
#[serial]
fn css_prop_slot_retains_responsive_base_when_raw_arrays_are_fallbacks(#[case] filename: &str) {
    // Given: raw Emotion fallback arrays overlap a captured responsive Devup base.
    let base = CAPTURED.replace(
        "color:'green'",
        "color:['green','yellow'],_hover:{color:'orange'}",
    );
    let app = element(
        filename,
        &css_props(
            filename,
            "{color:['blue','pink'],_hover:{color:'purple'}}",
            "",
        ),
    );
    let source = format!("{base}const App=()=>{app};");
    // When: the compiled call renders its composed class list.
    let result = execute(
        filename,
        &source,
        "const raw=App();const final=render(raw);JSON.stringify([raw.target===Card,values(final,'color'),values(final,'color',1),values(final,'color',0,'&:hover'),slots(final),trace]);",
    );
    // Then: fallback declarations replace level zero only, preserving captures and level one.
    assert_eq!(
        result,
        serde_json::json!([
            true,
            ["blue;color:pink"],
            ["yellow"],
            ["purple"],
            [],
            ["controller", "leaf"]
        ])
    );
}
