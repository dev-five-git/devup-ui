use super::*;
use boa_engine::{Context, Source};
use oxc_codegen::Codegen;

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn authored_captures_survive_when_css_prop_renders_fixed_prepared_styled(
    #[case] css_prop: bool,
    #[values(false, true)] selected: bool,
) {
    let _state = crate::test_state::TestStateGuard::new();
    // Given: retained controller/leaf effects whose style is overwritten by a literal.
    let css = if css_prop { "css:{padding:4}," } else { "" };
    let source = format!(
        "import {{styled}} from '@devup-ui/react';import {{jsx}} from '@emotion/react';const trace=[];const selected={selected};const Card=styled.article({{...((trace.push('controller'),selected)?{{color:(trace.push('leaf'),'red')}}:{{}}),color:'green'}});const App=ref=>jsx(Card,{{{css}ref,children:'child'}});"
    );
    reset_class_map();
    reset_file_map();
    let output = extract(
        "test.ts",
        &source,
        ExtractOption {
            single_css: true,
            import_aliases: std::collections::HashMap::from([(
                "@emotion/react".to_string(),
                crate::ImportAlias::NamedToNamed,
            )]),
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("capture extraction failed: {error}"));
    let arena = Allocator::default();
    let mut program = parse(&arena, "test.ts", &output.code);
    let calls = renders(&program);
    let helper = &calls[0].helper;
    program
        .body
        .retain(|statement| !matches!(statement, Statement::ImportDeclaration(_)));
    let code = Codegen::new().build(&program).code;

    // When: emitted CSS-prop calls render twice through the real generated callback.
    let script = format!(
        "const __devupForwardRef=render=>({{render}});function jsx(target,props){{return typeof target==='string'?{{target,props}}:target.render(props,props.ref);}}const {helper}=jsx;{code}\nconst before=trace.slice();const first=App(7);const second=App(8);JSON.stringify([before,first.target,second.target,first.props.ref,second.props.ref,first.props.children,second.props.children,trace]);"
    );
    let mut context = Context::default();
    let result = context
        .eval(Source::from_bytes(&script))
        .unwrap_or_else(|error| panic!("CSS-prop capture execution failed: {error}\n{code}"));
    let result = result
        .as_string()
        .unwrap_or_else(|| panic!("capture observation must be JSON"))
        .to_std_string_escaped();

    // Then: unused style captures still execute once per render in authored order.
    let trace = if selected {
        vec!["controller", "leaf", "controller", "leaf"]
    } else {
        vec!["controller", "controller"]
    };
    let expected = serde_json::json!([[], "article", "article", 7, 8, "child", "child", trace]);
    assert_eq!(result, expected.to_string(), "css_prop={css_prop}\n{code}");
}

#[rstest]
#[case("test.jsx")]
#[case("test.ts")]
#[serial]
fn selected_component_keeps_identity_when_css_prop_composes(
    #[case] filename: &str,
    #[values(false, true)] captured: bool,
    #[values(false, true)] selected: bool,
) {
    use super::css_prop_execution::{CAPTURED, css_props, element, execute};

    // Given: selected Card with fixed styles, plus captured and capture-free controls.
    let card = if captured {
        CAPTURED
    } else {
        "const trace=[];let selected=true;const Card=styled.article({color:'green'});"
    };
    let extra = if filename.ends_with('x') {
        "ref={ref} children='child' className='external'"
    } else {
        "ref,children:'child',className:'external'"
    };
    let plain = element(filename, extra);
    let composed = element(
        filename,
        &css_props(filename, "{color:'blue',padding:'4px'}", extra),
    );
    let source = format!(
        "{card}selected={selected};const Parent=styled.section({{[Card]:{{outlineColor:'orange'}}}});const Plain=(trace,selected,ref)=>{plain};const App=(trace,selected,ref)=>{composed};"
    );

    // When: mixed callers render through emitted code, with caller-scope shadowing.
    let result = execute(
        filename,
        &source,
        "const selector=Object.values(rules).find(rule=>rule[0]==='outline-color'&&rule[1]==='orange')[3];const marker=selector.slice(selector.lastIndexOf('.')+1);const before=trace.slice();const first=render(Plain([],false,7));const raw=App([],false,8);const second=render(raw);selected=!selected;const third=render(Plain([],false,9));JSON.stringify([before,raw.target===Card,slots(raw).length,[first,second,third].map(e=>[e.target,e.props.ref,e.props.children,slots(e),e.props.className.split(/\\s+/).includes(marker),e.props.className.includes('external')]),values(first,'color'),values(second,'color'),values(second,'padding'),values(third,'color'),trace]);",
    );

    // Then: selected identity survives both Inline and Slot, without losing later-wins or effects.
    let mut trace = Vec::new();
    if captured {
        for active in [selected, selected, !selected] {
            trace.push("controller");
            if active {
                trace.push("leaf");
            }
        }
    }
    assert_eq!(
        result,
        serde_json::json!([
            [],
            captured,
            usize::from(captured),
            [
                ["article", 7, "child", [], true, true],
                ["article", 8, "child", [], true, true],
                ["article", 9, "child", [], true, true]
            ],
            ["green"],
            ["blue"],
            ["4px"],
            ["green"],
            trace
        ])
    );
}

#[rstest]
#[case("test.jsx")]
#[case("test.ts")]
#[serial]
fn css_prop_composes_extension_when_base_is_selected(
    #[case] filename: &str,
    #[values(false, true)] selected_base: bool,
) {
    use super::css_prop_execution::{CAPTURED, css_props, element, execute};

    // Given: selecting a base must not change an extension's fixed native styles or captures.
    let base = CAPTURED.replace("Card", "Base");
    let selector = if selected_base {
        "const Parent=styled.section({[Base]:{outlineColor:'orange'}});"
    } else {
        "const Parent=styled.section({outlineColor:'orange'});"
    };
    let extra = if filename.ends_with('x') {
        "ref={7} children='child'"
    } else {
        "ref:7,children:'child'"
    };
    let app = element(filename, &css_props(filename, "{color:'blue'}", extra));
    let source =
        format!("{base}const Card=styled(Base)({{padding:'2px'}});{selector}const App=()=>{app};");

    // When: the extension composes an overlapping CSS prop through its original renderer.
    let result = execute(
        filename,
        &source,
        "const raw=App();const final=render(raw);JSON.stringify([raw.target===Card,final.target,final.props.ref,final.props.children,values(final,'color'),values(final,'padding'),slots(final),trace]);",
    );

    // Then: adding selector identity preserves parent composition capability and authored effects.
    assert_eq!(
        result,
        serde_json::json!([
            true,
            "article",
            7,
            "child",
            ["blue"],
            ["2px"],
            [],
            ["controller", "leaf"]
        ])
    );
}
