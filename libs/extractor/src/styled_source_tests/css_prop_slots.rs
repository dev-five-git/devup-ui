use super::css_prop_execution::{CAPTURED, css_props, element, execute};
use super::*;

#[rstest]
#[case("test.tsx")]
#[case("test.jsx")]
#[case("test.ts")]
#[case("test.cts")]
#[case("test.cjs")]
#[serial]
fn css_prop_slot_preserves_closure_when_mixed_renders_compose(
    #[case] filename: &str,
    #[values(false, true)] overlap: bool,
    #[values(false, true)] selected: bool,
) {
    // Given: CSS and no-CSS callers shadow both captured source names.
    let extra = if filename.ends_with('x') {
        "ref={ref} children='child' className='external'"
    } else {
        "ref,children:'child',className:'external'"
    };
    let own = element(filename, extra);
    let css = if overlap {
        "{color:'blue',padding:'4px'}"
    } else {
        "{padding:'4px'}"
    };
    let composed = element(filename, &css_props(filename, css, extra));
    let source = format!(
        "{CAPTURED}selected={selected};const Plain=(trace,selected,ref)=>{own};const App=(trace,selected,ref)=>{composed};"
    );
    // When: alternating callers execute the original forwardRef callback three times.
    let result = execute(
        filename,
        &source,
        "const before=trace.slice();const first=render(Plain([],false,7));const raw=App([],false,8);const second=render(raw);selected=!selected;const third=render(Plain([],false,9));JSON.stringify([before,raw.target===Card,[first,second,third].map(e=>[e.target,e.props.ref,e.props.children,slots(e),e.props.className.includes('external')]),values(first,'color'),values(second,'color'),values(second,'padding'),values(third,'color'),trace]);",
    );
    // Then: D1 removes losing atoms; closure/refs/children survive and the slot never leaks.
    let mut trace = Vec::new();
    for active in [selected, selected, !selected] {
        trace.push("controller");
        if active {
            trace.push("leaf");
        }
    }
    assert_eq!(
        result,
        serde_json::json!([
            [],
            true,
            [
                ["article", 7, "child", [], true],
                ["article", 8, "child", [], true],
                ["article", 9, "child", [], true]
            ],
            ["green"],
            [if overlap { "blue" } else { "green" }],
            ["4px"],
            ["green"],
            trace
        ])
    );
}

#[rstest]
#[case("test.tsx")]
#[case("test.ts")]
#[case("test.cts")]
#[case("test.cjs")]
#[serial]
fn css_prop_slot_replaces_empty_own_classes_when_registered_caller_covers_atoms(
    #[case] filename: &str,
    #[values(false, true)] empty: bool,
) {
    // Given: a registered caller class wins over both fixed own styles and CSS-prop styles.
    let css = if empty {
        "{color:'blue'}"
    } else {
        "{color:'blue',padding:'4px'}"
    };
    let extra = if filename.ends_with('x') {
        "className={override+' external'}"
    } else {
        "className:override+' external'"
    };
    let app = element(filename, &css_props(filename, css, extra));
    let source = format!("{CAPTURED}const override=css({{color:'purple'}});const App=()=>{app};");
    // When: the compiled CSS caller renders through Card, even with no replacement atom.
    let result = execute(
        filename,
        &source,
        "const raw=App();const keys=slots(raw);const final=render(raw);JSON.stringify([raw.target===Card,keys.length,keys.length===1&&raw.props[keys[0]]==='',raw.props.className===override+' external',values(final,'color'),values(final,'padding'),slots(final),trace]);",
    );
    // Then: explicit empty transport suppresses defaults; the caller class stays className.
    assert_eq!(
        result,
        serde_json::json!([
            true,
            1,
            empty,
            true,
            ["purple"],
            if empty { vec![] } else { vec!["4px"] },
            [],
            ["controller", "leaf"]
        ])
    );
}

#[rstest]
#[case("test.tsx")]
#[case("test.ts")]
#[case("test.cts")]
#[case("test.cjs")]
#[serial]
fn css_prop_slot_keeps_vars_beneath_inline_style_when_dynamic_values_render(
    #[case] filename: &str,
) {
    // Given: generated CSS variables and inline style overrides with distinct values.
    let extra = if filename.ends_with('x') {
        "style={style} ref={7} children='child'"
    } else {
        "style,ref:7,children:'child'"
    };
    let app = element(
        filename,
        &css_props(filename, "{color:value,padding:'4px'}", extra),
    );
    let source = format!("{CAPTURED}const App=(value,style)=>{app};");
    // When: the caller overrides first a CSS property, then the generated variable itself.
    let result = execute(
        filename,
        &source,
        "const raw=App('blue',{color:'pink'});const first=render(raw);const variable=Object.keys(first.props.style).find(key=>key.startsWith('--'));const second=render(App('teal',{[variable]:'orange',opacity:0.5}));JSON.stringify([raw.target===Card,values(first,'color'),values(second,'color'),first.props.style.color,second.props.style.opacity,values(first,'padding'),first.props.ref,first.props.children,slots(first),slots(second),trace]);",
    );
    // Then: vars are real caller-scope values, inline style wins, and captures still execute.
    assert_eq!(
        result,
        serde_json::json!([
            true,
            ["blue"],
            ["orange"],
            "pink",
            0.5,
            ["4px"],
            7,
            "child",
            [],
            [],
            ["controller", "leaf", "controller", "leaf"]
        ])
    );
}
