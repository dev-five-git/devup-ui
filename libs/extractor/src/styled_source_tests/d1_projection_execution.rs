use super::execute_in_mode;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[serial]
fn d1_projection_selects_later_lower_value_when_written_parts_compete(
    #[values(false, true)] single_css: bool,
) {
    // Given: sheet value sorting would otherwise put the earlier 8px declaration last.
    let source = "const Card=styled.div({padding:'8px'},{p:'4px'});";
    // When: the complete generated renderer selects the composed declaration.
    let observed = execute_in_mode(
        "multipart-priority.ts",
        source,
        "JSON.stringify(values(render({target:Card,props:{}}),'padding'));",
        single_css,
    );
    // Then: only the later 4px class is active, independently of sheet sorting.
    assert_eq!(observed, serde_json::json!(["4px"]));
}

#[rstest]
#[case::selector_chunk("_hover:{p:4,padding:3}", "&:hover", "12px")]
#[case::root_chunks(
    "selectors:{'&:hover':{ padding: 3 }},_hover:{ p: 4 }",
    "&:hover",
    "16px"
)]
#[case::selector_reverse("_hover:{padding:3,p:4}", "&:hover", "16px")]
#[case::containers_reverse(
    "_hover:{ p: 4 },selectors:{'&:hover':{ padding: 3 }}",
    "&:hover",
    "12px"
)]
#[serial]
fn d1_projection_preserves_origin_when_flat_raw_vectors_are_identical(
    #[case] rules: &str,
    #[case] selector: &str,
    #[case] expected: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given: an inherited declaration forces the semantic composition boundary.
    let source = format!(
        "const Base=styled.div({{color:'black'}});const Card=styled(Base)(p=>({{{rules}}}));"
    );
    // When: the original callback renders through the complete generated module.
    let observed = execute_in_mode(
        "origin.ts",
        &source,
        &format!(
            "JSON.stringify(values(render({{target:Card,props:{{}}}}),'padding',0,'{selector}'));"
        ),
        single_css,
    );
    // Then: equal flattened raw vectors can have different authored winners.
    assert_eq!(observed, serde_json::json!([expected]));
}

#[rstest]
#[case::empty("const Middle=styled(Base)({});const Card=styled(Middle)({ margin: 1 });")]
#[case::empty_callback(
    "const Middle=styled(Base)(p=>p.on?{ margin: 1 }:{});const Card=styled(Middle)({ width: 2 });"
)]
#[case::with_component(
    "const Middle=Base.withComponent('section');const Card=styled(Middle)({ margin: 1 });"
)]
#[case::css_prop(
    "const Middle=styled(Base)({});const Card=styled(Middle)({});const App=()=>jsx(Card,{css:{margin:1}});"
)]
#[serial]
fn d1_projection_keeps_stored_base_alias_when_extended_or_retargeted(
    #[case] extension: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given: a raw base retains both aliases but publishes a normalized private summary.
    let source = format!("const Base=styled.div({{padding:3,p:4}});{extension}");
    let element = if extension.contains("const App=") {
        "App()"
    } else {
        "{target:Card,props:{ on: false }}"
    };
    // When: an empty extension and another semantic consumer reuse that summary.
    let observed = execute_in_mode(
        "inheritance.ts",
        &source,
        &format!("JSON.stringify(values(render({element}),'padding'));"),
        single_css,
    );
    // Then: the later authored base alias survives without converting composed data twice.
    assert_eq!(observed, serde_json::json!(["16px"]));
}

#[rstest]
#[case::local(
    "const mixin=css({padding:3,p:4});const Card=styled.div(mixin,{margin:1});",
    "16px"
)]
#[case::alias(
    "const original=css({padding:3,p:4});const mixin=original;const Card=styled.div(mixin,{margin:1});",
    "16px"
)]
#[case::inline("const Card=styled.div(css({padding:3,p:4}),{margin:1});", "16px")]
#[case::overridden(
    "const mixin=css({padding:3,p:4});const Card=styled.div(mixin,{p:1});",
    "4px"
)]
#[case::template_before("const Card=styled.div`padding:20px;${{padding:3,p:4}}`;", "16px")]
#[case::template_after("const Card=styled.div`${{padding:3,p:4}}padding:4px;`;", "4px")]
#[case::known_template(
    "const mixin=css({padding:3,p:4});const Card=styled.div`${mixin}padding:4px;`;",
    "4px"
)]
#[case::text_identity("const Card=styled.div`padding:16px;padding:4px;`;", "4px")]
#[serial]
fn d1_projection_preserves_known_metadata_when_rules_or_text_compose(
    #[case] source: &str,
    #[case] expected: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given: metadata producers or template text enter an authored composition.
    // When: the real renderer selects the resulting classes.
    let observed = execute_in_mode(
        "metadata.ts",
        source,
        "JSON.stringify(values(render({target:Card,props:{}}),'padding'));",
        single_css,
    );
    // Then: known rules remain semantic and source CSS text is never reversed.
    assert_eq!(observed, serde_json::json!([expected]));
}

#[rstest]
#[case::levels("const Base=styled.div({p:[1,null,2,null,3]});const Card=styled(Base)({p:[null,null,4]});", "{}", serde_json::json!([["4px"],["16px"],["12px"]]))]
#[case::level_keep("const Base=styled.div({p:[1,null,2,null,3]});const Card=styled(Base)(p=>p.on?{p:[null,null,4]}:{});", "{on:false}", serde_json::json!([["4px"],["8px"],["12px"]]))]
#[case::layer_root("const Base=styled.div({ p: 1 });const Card=styled(Base)({'@layer':{base:{ padding: 3,p:4 },'base.inner':{ p: 2 }}});", "{}", serde_json::json!([["base","16px"],["base.inner","8px"],[null,"4px"]]))]
#[case::layer_selector("const Base=styled.div({color:'black'});const Card=styled(Base)({'@layer':{base:{_hover:{p:4,padding:3}}}});", "{}", serde_json::json!([["base","12px"]]))]
#[case::same_layer("const Base=styled.div({'@layer':{base:{p:2}}});const Card=styled(Base)({'@layer':{base:{padding:3,p:4}}});", "{}", serde_json::json!([["base","16px"]]))]
#[serial]
fn d1_projection_keeps_context_when_aliases_and_missing_levels_compete(
    #[case] source: &str,
    #[case] props: &str,
    #[case] expected: serde_json::Value,
    #[values(false, true)] single_css: bool,
) {
    // Given: keys differ by breakpoint, selector or layer rather than property spelling.
    let observe = if source.contains("'@layer'") {
        "atoms(element).filter(r=>r[0]==='padding').map(r=>[r[4],r[1]]).sort((a,b)=>String(a[0]).localeCompare(String(b[0])))"
    } else {
        "[values(element,'padding',0),values(element,'padding',2),values(element,'padding',4)]"
    };
    // When: the selected callback's missing branch must Keep each earlier matching key.
    let observed = execute_in_mode(
        "context.ts",
        source,
        &format!("const element=render({{target:Card,props:{props}}});JSON.stringify({observe});"),
        single_css,
    );
    // Then: only identical context keys compete.
    assert_eq!(observed, expected);
}

#[rstest]
#[case::object_true("p.input.on?{padding:3,p:4}:null", true, "16px")]
#[case::object_false("p.input.on?{padding:3,p:4}:null", false, "8px")]
#[case::known_true("p.input.on?mixin:{padding:3,p:4}", true, "4px")]
#[case::known_false("p.input.on?mixin:{padding:3,p:4}", false, "16px")]
#[case::sequential_keep("p.input.on?{padding:3,p:4}:{padding:3}", false, "12px")]
#[case::sequential_replace("p.input.on?{padding:3,p:4}:{padding:3}", true, "16px")]
#[case::layer_keep(
    "p.input.on?{'@layer':{base:{padding:3,p:4}}}:{'@layer':{base:{padding:3}}}",
    false,
    "12px"
)]
#[serial]
fn d1_projection_resolves_keep_when_an_authored_chunk_follows_an_atom(
    #[case] rules: &str,
    #[case] on: bool,
    #[case] expected: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given: an original callback default and getter make duplicate execution observable.
    let source = format!(
        "const trace=[];const mixin=css({{p:1}});const Base=styled.div({{p:2}});const Card=styled(Base)((p,unused=(trace.push('default'),0))=>{rules});const input={{get on(){{trace.push('on');return {on};}}}};"
    );
    let layer = if rules.contains("'@layer'") {
        "&&r[4]==='base'"
    } else {
        "&&r[4]===null"
    };
    // When: one complete render evaluates one authored branch selection.
    let observed = execute_in_mode(
        "keep.ts",
        &source,
        &format!(
            "const element=render({{target:Card,props:{{input}}}});JSON.stringify([trace,atoms(element).filter(r=>r[0]==='padding'{layer}).map(r=>r[1])]);"
        ),
        single_css,
    );
    // Then: Keep sees the preceding chunk, and the callback/default/controller run once.
    assert_eq!(observed, serde_json::json!([["default", "on"], [expected]]));
}

#[rstest]
#[serial]
fn d1_projection_rebinds_summary_when_capture_base_is_covered_and_extended(
    #[values(false, true)] single_css: bool,
) {
    // Given: both raw emission and its summary refer to the same original captures.
    let source = "const trace=[];const Base=styled.div((p,unused=(trace.push('default'),0))=>({padding:3,p:4,color:p.input.color}));const Middle=Base.withComponent('section');const Empty=styled(Middle)({});const Card=styled(Empty)({color:'green'});const input={get color(){trace.push('color');return 'red';}};";
    // When: a covered dynamic leaf still executes in the original callback once.
    let observed = execute_in_mode(
        "rebind.ts",
        source,
        "const element=render({target:Card,props:{input}});JSON.stringify([trace,element.target,values(element,'padding'),values(element,'color')]);",
        single_css,
    );
    // Then: cloning metadata never introduces a second callback or stale capture name.
    assert_eq!(
        observed,
        serde_json::json!([["default", "color"], "section", ["16px"], ["green"]])
    );
}

#[rstest]
#[case::record(
    "import {css} from '@devup-ui/react';const rules={padding:3,p:4};export const mixin=css(rules);"
)]
#[case::style("import {css} from '@devup-ui/react';export const mixin=css({padding:3,p:4});")]
#[serial]
fn d1_projection_keeps_imported_metadata_when_record_or_style_is_reused(
    #[case] module: &'static str,
    #[values(false, true)] single_css: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given: the real module resolver supplies native Record or completed Style metadata.
    let _state = crate::test_state::TestStateGuard::new();
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    let resolver = move |source: &str, _: &str| {
        (source == "./mixin").then(|| {
            crate::ModuleResolution::Resolved(crate::ResolvedModule {
                path: "/mixin.ts".to_string(),
                code: module.to_string(),
                source_type: None,
            })
        })
    };
    // When: the complete generated renderer composes the imported semantic operand.
    let output = crate::extract_with_modules(
        "/consumer.ts",
        "import {styled} from '@devup-ui/react';import {mixin} from './mixin';const Card=styled.div(mixin,{ margin: 1 });",
        crate::ExtractOption {
            single_css,
            ..crate::ExtractOption::default()
        },
        false,
        &resolver,
    )?;
    let bucket = (!single_css).then(|| css::file_map::canonical("/consumer.ts"));
    let observed = super::execute_extracted(
        ("/consumer.ts", bucket.as_deref()),
        &output,
        "JSON.stringify(values(render({target:Card,props:{}}),'padding')); ",
    );
    // Then: the imported producer publishes the later alias, not reversed raw emission.
    assert_eq!(observed, serde_json::json!(["16px"]));
    Ok(())
}
