use super::execute_in_mode;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case::surviving("", "orange")]
#[case::covered("border-color:black;", "black")]
#[serial]
fn scalar_callbacks_interleave_once_when_captured_mixin_separates_them(
    #[case] later: &str,
    #[case] border: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given: the first scalar is covered, but its default and getter still belong to this render.
    let source = format!(
        "const trace=[];const data={{get before(){{trace.push('before');return 'red';}},get mixin(){{trace.push('mixin');return 'blue';}},get inactive(){{trace.push('inactive');return 'wrong';}},get after(){{trace.push('after');return 'orange';}}}};const Card=styled.div`color:${{(p,unused=(trace.push('beforeDefault'),0))=>p.data.before}};${{(p,unused=(trace.push('mixinDefault'),0))=>p.on?{{color:p.data.mixin}}:{{color:p.data.inactive}}}};border-color:${{(p,unused=(trace.push('afterDefault'),0))=>p.data.after}};color:green;{later}`;const defined=trace.slice();"
    );
    // When: the complete generated renderer selects one mixin branch.
    let observed = execute_in_mode(
        "test.ts",
        &source,
        "const element=render({target:Card,props:{on:true,data}});JSON.stringify([defined,trace,values(element,'color'),values(element,'border-color')]);",
        single_css,
    );
    // Then: authored interpolation order survives D1 coverage and no inactive getter runs.
    assert_eq!(
        observed,
        serde_json::json!([
            [],
            [
                "beforeDefault",
                "before",
                "mixinDefault",
                "mixin",
                "afterDefault",
                "after"
            ],
            ["green"],
            [border]
        ])
    );
}

#[rstest]
#[case::selector(("&:hover {", "&:hover", None))]
#[case::media(("@media(min-width:700px) {", "@media(min-width:700px)", None))]
#[case::layer(("@layer theme {", "", Some("theme")))]
#[serial]
fn known_mixin_choices_keep_context_when_callback_capture_selects_them(
    #[case] context: (&str, &str, Option<&str>),
    #[values(false, true)] single_css: bool,
) {
    let (opening, selector, layer) = context;
    // Given: empty CSS is falsy, a written empty object is truthy, and omissions Keep earlier atoms.
    let source = format!(
        "const trace=[];const mixin=css({{color:'red',padding:2}});const empty=css({{}});const Card=styled.div`{opening}color:blue;${{(p,unused=(trace.push('default'),0))=>p.on?mixin:null}};${{p=>empty||{{margin:'3px'}}}};${{p=>({{}})||{{color:'wrong'}}}};}}`;const defined=trace.slice();"
    );
    // When: two real renders select the known mixin and the omitted branch.
    let observation = format!(
        "const first=render({{target:Card,props:{{on:true}}}});const second=render({{target:Card,props:{{on:false}}}});function selected(e){{return atoms(e).filter(r=>r[3]==={selector}&&r[4]==={layer}).map(r=>[r[0],r[1]]).sort();}}JSON.stringify([defined,trace,selected(first),selected(second)]);",
        selector = serde_json::json!(selector),
        layer = serde_json::json!(layer)
    );
    let observed = execute_in_mode("test.ts", &source, &observation, single_css);
    // Then: context and library units remain intact, with no replacement for a missing property.
    assert_eq!(
        observed,
        serde_json::json!([
            [],
            ["default", "default"],
            [["color", "red"], ["margin", "3px"], ["padding", "8px"]],
            [["color", "blue"], ["margin", "3px"]]
        ])
    );
}

#[rstest]
#[serial]
fn inherited_template_branches_rebind_when_attrs_and_retargeted_extensions_share_names(
    #[values(false, true)] single_css: bool,
) {
    // Given: inherited and own captures reserve colliding authored names and retain attrs.
    let source = "const trace=[];function make(__devupRecord0){const Base=styled.div.attrs({role:'button'})`${(p,unused=(trace.push(__devupRecord0),0))=>p.on?{color:p.tone,margin:p.margin}:{color:'black'}};`;const Child=styled(Base)`${(p,__devupRecord1=(trace.push('child'),0))=>p.on?{opacity:p.opacity}:null};`;return Child.withComponent('aside');}const Card=make('base');const defined=trace.slice();const App=ref=>jsx(Card,{on:true,tone:'blue',margin:3,opacity:0.5,ref,children:'child',className:'caller',style:{color:'olive'},forwardedAs:'header'});";
    // When: the retargeted renderer evaluates both inherited and own original callbacks.
    let observed = execute_in_mode(
        "test.ts",
        source,
        "const element=render(App(7));JSON.stringify([defined,trace,element.target,values(element,'color'),values(element,'margin'),values(element,'opacity'),element.props.role,element.props.ref,element.props.children,element.props.as,element.props.style.color,element.props.className.split(/\\s+/).includes('caller'),'tone' in element.props,slots(element)]);",
        single_css,
    );
    // Then: bindings and reads stay paired through attrs, retargeting and forwarding.
    assert_eq!(
        observed,
        serde_json::json!([
            [],
            ["base", "child"],
            "aside",
            ["blue"],
            ["3px"],
            [0.5],
            "button",
            7,
            "child",
            "header",
            "olive",
            true,
            false,
            []
        ])
    );
}

#[rstest]
#[case::compiled("test.ts")]
#[case::syntax("test.tsx")]
#[serial]
fn covered_template_branches_keep_private_css_slot_when_inherited_renderer_is_retargeted(
    #[case] filename: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given: fully covered branch leaves still require the renderer even with a fixed style summary.
    let app = if filename.ends_with('x') {
        "ref=><Card css={{color:'blue'}} ref={ref} className='caller'>child</Card>"
    } else {
        "ref=>jsx(Card,{css:{color:'blue'},ref,children:'child',className:'caller'})"
    };
    let source = format!(
        "const trace=[];const state={{get on(){{trace.push('on');return true;}}}};const data={{get color(){{trace.push('leaf');return 'red';}}}};const Base=styled.div`${{(p,__devupRecord0=(trace.push('base'),0))=>state.on?{{color:data.color}}:null}};color:green;`;const Child=styled(Base)`padding:1px;`;const Card=Child.withComponent('section');const defined=trace.slice();const App={app};"
    );
    // When: authored CSS-prop lowering transports a replacement through the real captured renderer.
    let observed = execute_in_mode(
        filename,
        &source,
        "const raw=App(7);const element=render(raw);JSON.stringify([defined,trace,raw.target===Card,slots(raw).length,values(element,'color'),values(element,'padding'),element.target,element.props.ref,element.props.children,slots(element),element.props.className.split(/\\s+/).includes('caller')]);",
        single_css,
    );
    // Then: one private slot preserves execution and is consumed before the native element.
    assert_eq!(
        observed,
        serde_json::json!([
            [],
            ["base", "on", "leaf"],
            true,
            1,
            ["blue"],
            ["1px"],
            "section",
            7,
            "child",
            [],
            true
        ])
    );
}

#[rstest]
#[serial]
fn scalar_theme_paths_stay_static_when_captured_rules_use_distinct_numeric_roles(
    #[values(false, true)] single_css: bool,
) {
    // Given: there is no runtime theme object and property-function numbers differ from rule leaves.
    let source = "const trace=[];const Card=styled.div`color:${p=>p.theme.colors.primary};height:${(p,unused=(trace.push('scalar'),0))=>p.height};${(p,unused=(trace.push('rules'),0))=>p.on?{width:p.width,margin:q=>q.margin}:null};`;const defined=trace.slice();";
    // When: the original callbacks execute with numeric props and no theme.
    let observed = execute_in_mode(
        "test.ts",
        source,
        "const element=render({target:Card,props:{on:true,width:2,margin:2,height:3}});JSON.stringify([defined,trace,values(element,'width'),values(element,'margin'),values(element,'height'),atoms(element).filter(r=>r[0]==='color').map(r=>r[1])]);",
        single_css,
    );
    // Then: mapping requires no runtime theme read while each numeric role retains its units.
    assert_eq!(
        observed,
        serde_json::json!([
            [],
            ["scalar", "rules"],
            ["2px"],
            [2],
            [3],
            ["var(--colors-primary)"]
        ])
    );
}

#[rstest]
#[serial]
fn opaque_statement_callbacks_keep_order_when_scalar_callbacks_surround_them(
    #[values(false, true)] single_css: bool,
) {
    // Given: an unknown top-level class is not interpreted as a known mixin.
    let source = "const trace=[];const Card=styled.div`height:${(p,unused=(trace.push('before'),0))=>p.height};${(p,unused=(trace.push('opaque'),0))=>p.extra};width:${(p,unused=(trace.push('after'),0))=>p.width};`;const defined=trace.slice();";
    // When: the original opaque callback executes between two scalar callbacks.
    let observed = execute_in_mode(
        "test.ts",
        source,
        "const element=render({target:Card,props:{height:2,width:3,extra:'opaque'}});JSON.stringify([defined,trace,values(element,'height'),values(element,'width'),element.props.className.split(/\\s+/).includes('opaque')]);",
        single_css,
    );
    // Then: the opaque class path remains intact without moving its invocation.
    assert_eq!(
        observed,
        serde_json::json!([[], ["before", "opaque", "after"], [2], [3], true])
    );
}

#[rstest]
#[serial]
fn mapped_branch_theme_leaves_stay_static_when_contextual_mixins_capture_lengths(
    #[values(false, true)] single_css: bool,
) {
    // Given: a selected contextual rule callback maps theme leaves without receiving a runtime theme.
    let source = "const trace=[];const Card=styled.div`@layer theme { &:hover { ${(p,unused=(trace.push('default'),0))=>p.on&&{padding:p.padding,color:p.theme.colors.primary}}; } }`;const defined=trace.slice();";
    // When: opposite branches render through the complete contextual callback capture.
    let observed = execute_in_mode(
        "test.ts",
        source,
        "const first=render({target:Card,props:{on:true,padding:2}});const second=render({target:Card,props:{on:false,padding:3}});JSON.stringify([defined,trace,values(first,'padding',0,'&:hover'),atoms(first).filter(r=>r[0]==='color').map(r=>[r[1],r[3],r[4]]),atoms(second)]);",
        single_css,
    );
    // Then: branch mapping retains selector/layer metadata and omission, with one default per render.
    assert_eq!(
        observed,
        serde_json::json!([
            [],
            ["default", "default"],
            ["2px"],
            [["var(--colors-primary)", "&:hover", "theme"]],
            []
        ])
    );
}
