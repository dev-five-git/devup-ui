use super::css_prop_execution::{CAPTURED, css_props, element, execute};
use super::*;

#[rstest]
#[case::read(("seen=props['__devupCss0'];", "authored"))]
#[case::write(("props['__devupCss0']='authored';", "unread"))]
#[case::optional_read(("seen=props?.['__devupCss0'];", "authored"))]
#[case::template_read(("seen=props[`__devupCss0`];", "authored"))]
#[serial]
fn css_prop_slot_reserves_bracket_names_when_plain_and_css_callers_render(
    #[case] access: (&str, &str),
    #[values("test.tsx", "test.jsx", "test.ts", "test.cts", "test.cjs")] filename: &str,
    #[values(false, true)] selected: bool,
) {
    // Given: only a computed-member access in the source names the authored prop.
    let plain = if filename.ends_with('x') {
        "<Card {...props}/>"
    } else {
        "jsx(Card,props)"
    };
    let extra = if filename.ends_with('x') {
        "ref={ref} children='css'"
    } else {
        "ref,children:'css'"
    };
    let app = element(filename, &css_props(filename, "{color:'blue'}", extra));
    let source = format!(
        "{CAPTURED}selected={selected};let seen='unread';function Plain(props){{{}return {plain};}}const App=ref=>{app};",
        access.0,
    );

    // When: plain and CSS calls execute the same captured component with distinct refs.
    let result = execute(
        filename,
        &source,
        "const before=trace.slice();const props={ref:7,children:'plain'};props['__devupCss0']='authored';const rawPlain=Plain(props);const first=render(rawPlain);const rawCss=App(8);const keys=slots(rawCss);const second=render(rawCss);JSON.stringify([before,rawPlain.target===Card,rawCss.target===Card,values(first,'color'),values(second,'color'),keys.length,keys.includes('__devupCss0'),first.props['__devupCss0'],[first,second].map(e=>[e.target,e.props.ref,e.props.children,keys.some(key=>Object.hasOwn(e.props,key))]),seen,trace]);",
    );

    // Then: authored0 survives while the different private key is consumed, never forwarded.
    let trace = if selected {
        vec!["controller", "leaf", "controller", "leaf"]
    } else {
        vec!["controller", "controller"]
    };
    assert_eq!(
        result,
        serde_json::json!([
            [],
            true,
            true,
            ["green"],
            ["blue"],
            1,
            false,
            "authored",
            [["article", 7, "plain", false], ["article", 8, "css", false]],
            access.1,
            trace
        ])
    );
}
