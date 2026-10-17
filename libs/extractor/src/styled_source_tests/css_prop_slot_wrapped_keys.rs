use super::css_prop_execution::{CAPTURED, css_props, element, execute};
use super::*;

#[derive(Clone, Copy, Debug)]
enum KeySite {
    Read,
    Write,
    OptionalRead,
    Object,
}

#[rstest]
#[case::paren_tsx(("(KEY)", "test.tsx"))]
#[case::paren_jsx(("(KEY)", "test.jsx"))]
#[case::paren_ts(("(KEY)", "test.ts"))]
#[case::paren_cts(("(KEY)", "test.cts"))]
#[case::paren_cjs(("(KEY)", "test.cjs"))]
#[case::as_tsx(("(KEY as const)", "test.tsx"))]
#[case::as_ts(("(KEY as const)", "test.ts"))]
#[case::as_cts(("(KEY as const)", "test.cts"))]
#[case::nonnull_tsx(("KEY!", "test.tsx"))]
#[case::nonnull_ts(("KEY!", "test.ts"))]
#[case::nonnull_cts(("KEY!", "test.cts"))]
#[case::satisfies_tsx(("(KEY satisfies string)", "test.tsx"))]
#[case::satisfies_ts(("(KEY satisfies string)", "test.ts"))]
#[case::satisfies_cts(("(KEY satisfies string)", "test.cts"))]
#[case::typeassert_ts(("<string>KEY", "test.ts"))]
#[case::typeassert_paren_ts(("<string>(KEY)", "test.ts"))]
#[case::nested_tsx(("(((KEY as const)!) satisfies string)", "test.tsx"))]
#[case::nested_ts(("(((<string>(KEY as const))!) satisfies string)", "test.ts"))]
#[case::nested_cts(("((((KEY as const)!) satisfies string))", "test.cts"))]
#[serial]
fn css_prop_slot_reserves_wrapped_keys_when_original_syntax_is_erased(
    #[case] fixture: (&str, &str),
    #[values("'__devupCss0'", "`__devupCss0`", "`\\u005f_devupCss0`")] literal: &str,
    #[values(KeySite::Read, KeySite::Write, KeySite::OptionalRead, KeySite::Object)] site: KeySite,
) {
    // Given: only a wrapped literal or cooked static-template key occupies authored0.
    let (wrapper, filename) = fixture;
    let key = wrapper.replace("KEY", literal);
    let (body, passed, seed, seen) = match site {
        KeySite::Read => (
            format!("seen=props[{key}];"),
            "props",
            "props['__devupCss0']='authored';",
            "authored",
        ),
        KeySite::Write => (format!("props[{key}]='authored';"), "props", "", "unread"),
        KeySite::OptionalRead => (
            format!("seen=props?.[{key}];"),
            "props",
            "props['__devupCss0']='authored';",
            "authored",
        ),
        KeySite::Object => (
            format!("const given={{...props,[{key}]:'authored'}};"),
            "given",
            "",
            "unread",
        ),
    };
    let plain = if filename.ends_with('x') {
        format!("<Card {{...{passed}}}/>")
    } else {
        format!("jsx(Card,{passed})")
    };
    let extra = if filename.ends_with('x') {
        "ref={ref} children='css'"
    } else {
        "ref,children:'css'"
    };
    let app = element(filename, &css_props(filename, "{color:'blue'}", extra));
    let source = format!(
        "{CAPTURED}let seen='unread';function Plain(props){{{body}return {plain};}}const App=ref=>{app};"
    );

    // When: actual generated callbacks render Plain and its CSS sibling with distinct refs.
    let observation = format!(
        "const before=trace.slice();const props={{ref:7,children:'plain'}};{seed}const rawPlain=Plain(props);const first=render(rawPlain);const rawCss=App(8);const keys=slots(rawCss);const second=render(rawCss);JSON.stringify([before,rawPlain.target===Card,rawCss.target===Card,values(first,'color'),values(second,'color'),keys.length,keys.includes('__devupCss0'),first.props['__devupCss0'],[first,second].map(e=>[e.target,e.props.ref,e.props.children,keys.some(key=>Object.hasOwn(e.props,key))]),seen,trace]);"
    );
    let result = execute(filename, &source, &observation);

    // Then: only the fresh private key is consumed; authored0, own green and later blue survive.
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
            seen,
            ["controller", "leaf", "controller", "leaf"]
        ]),
        "{filename}: {key}, {site:?}"
    );
}
