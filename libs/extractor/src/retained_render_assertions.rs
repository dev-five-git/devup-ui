use crate::class_selection_support::{compile, observe, option};
use crate::retained_render_support::{RenderRecords, declarations, props};
use serde_json::json;

pub fn opaque(scope: &str, value: &str) -> RenderRecords {
    let expected = match scope {
        "const scaled = n => n * 4; const size = runtimeSize;"
        | "const scaled = n => { externalEffect(); return n * 4; }; const size = 2;" => 8,
        "const scaled = n => n * 4; let size = 2; size = 3;"
        | "const box = { size: 2 }; box.size = 3; const scaled = n => n * 4;" => 12,
        "const scaled = n => n * 4; const size = Math.random();" => 2,
        _ => panic!("original opaque fixture"),
    };
    let mut records = Vec::new();
    for single_css in [false, true] {
        let base = format!(
            "import './environment.js'; import {{styled}} from '@devup-ui/core'; {scope} export const Rules = styled.div({{ padding: {value} }});"
        );
        for surface in ["base", "reexport"] {
            let app = format!(
                "import {{styled}} from '@devup-ui/core'; import {{Rules}} from './{surface}.ts'; export const A = styled(Rules)`color: blue;`;"
            );
            let outputs = compile(
                &[
                    ("/namespace/base.ts", &base),
                    ("/namespace/reexport.ts", "export {Rules} from './base.ts';"),
                    ("/namespace/app.tsx", &app),
                ],
                option(single_css, "@devup-ui/core"),
            );
            let consumer = &outputs[2].1;
            assert_eq!(consumer.styles.len(), 1);
            assert_eq!(
                consumer.dependencies,
                match surface {
                    "base" => vec!["/namespace/base.ts"],
                    "reexport" => vec!["/namespace/base.ts", "/namespace/reexport.ts"],
                    _ => unreachable!(),
                }
            );
            for private in ["scaled", "runtimeSize", "externalEffect", "box.size"] {
                assert!(!consumer.code.contains(private), "{}", consumer.code);
            }
            // Given: authored free globals execute before the original defining module.
            let actual = observe(
                &outputs,
                concat!(
                    "import {",
                    "A",
                    "} from './app.tsx'; const input={ref:7,children:'child',id:'kept',className:'caller',style:{opacity:0.5}}; const first=render(jsx(A,input)); const effects=globalThis.effects; const second=render(jsx(A,input)); globalThis.observed={first,second,effects,total:globalThis.effects};"
                ),
                "globalThis.runtimeSize=2; Math.random=()=>0.5; globalThis.effects=0; globalThis.externalEffect=()=>{globalThis.effects++};",
            );
            // Then: each actual render preserves origin computation and exactly one effect.
            let effects = usize::from(scope.contains("externalEffect"));
            assert_eq!(actual["effects"], effects);
            assert_eq!(actual["total"], effects * 2);
            for element in [&actual["first"], &actual["second"]] {
                assert_eq!(element["target"], "div");
                props(element, json!({"ref":7,"children":"child","id":"kept"}));
                assert_eq!(
                    declarations(&outputs, element, single_css),
                    vec![
                        ("color".into(), "blue".into()),
                        ("padding".into(), expected.into())
                    ]
                );
            }
            records.push((
                outputs,
                vec![actual["first"].clone(), actual["second"].clone()],
                single_css,
            ));
        }
    }
    records
}

pub fn forwarding(source: &str) -> RenderRecords {
    // The full extraction/refusal owner stays intact; these original definitions execute verbatim.
    let definitions = source
        .lines()
        .filter(|line| {
            ["const B =", "const L =", "const F ="]
                .iter()
                .any(|prefix| line.starts_with(prefix))
        })
        .collect::<Vec<_>>()
        .join("\n");
    let source = format!(
        "import './environment.js'; import {{styled}} from '@devup-ui/core';\n{definitions}\nexport {{B,L,F}};"
    );
    let mut records = Vec::new();
    for single_css in [false, true] {
        let outputs = compile(
            &[("/namespace/forwarding.tsx", &source)],
            option(single_css, "@devup-ui/core"),
        );
        let actual = observe(
            &outputs,
            "import {B,L,F} from './forwarding.tsx'; const common={ref:7,children:'child',id:'kept',className:'caller',style:{opacity:0.5},theme:{brand:'theme'},$secret:'private'}; const elements=[render(jsx(B,{...common,$c:'red',tone:'blue'})),render(jsx(L,{...common,level:'green',tone:'ordinary',other:'kept',distance:'3px'})),render(jsx(F,{...common,$c:'purple',tone:'ordinary',forwardedAs:'b'}))]; globalThis.observed={elements,calls:globalThis.calls};",
            "globalThis.k='distance'; globalThis.config={}; globalThis.calls=[]; globalThis.Link=(props)=>{calls.push(['Link',props.ref,props.as]);return jsx('a',props)};",
        );
        assert_eq!(actual["calls"], json!([["Link", 7, "b"]]));
        for (i, expected) in [json!({"ref":7,"children":"child","id":"kept"}), json!({"ref":7,"children":"child","id":"kept","tone":"ordinary","other":"kept","distance":"3px"}), json!({"ref":7,"children":"child","id":"kept","tone":"ordinary","theme":{"brand":"theme"},"as":"b"})].into_iter().enumerate() {
            assert_eq!(actual["elements"][i]["target"], if i == 2 { "a" } else { "div" });
            props(&actual["elements"][i], expected);
        }
        assert_eq!(
            declarations(&outputs, &actual["elements"][0], single_css),
            vec![
                ("background".into(), "blue".into()),
                ("color".into(), "red".into())
            ]
        );
        assert_eq!(
            declarations(&outputs, &actual["elements"][1], single_css),
            vec![
                ("color".into(), "green".into()),
                ("margin".into(), "3px".into())
            ]
        );
        assert_eq!(
            declarations(&outputs, &actual["elements"][2], single_css),
            vec![("color".into(), "purple".into())]
        );
        records.push((
            outputs,
            actual["elements"]
                .as_array()
                .unwrap_or_else(|| panic!("elements"))
                .clone(),
            single_css,
        ));
    }
    records
}

pub fn retarget(source: &str) -> RenderRecords {
    // Unknown make()/other operations remain extraction-only with their original assertions.
    let definitions = source
        .lines()
        .filter(|line| {
            !line.contains("make()")
                && !line.contains("other.withComponent")
                && !line.starts_with("export const a =")
        })
        .collect::<Vec<_>>()
        .join("\n");
    let source = format!("import './environment.js';\n{definitions}");
    let mut records = Vec::new();
    for single_css in [false, true] {
        let outputs = compile(
            &[("/namespace/retarget.tsx", &source)],
            option(single_css, "@devup-ui/core"),
        );
        let actual = observe(
            &outputs,
            "import {Aside,Linked,Nested,Again} from './retarget.tsx'; const common={children:'child',className:'caller',style:{opacity:0.5}}; const linked=render(jsx(Linked,{...common,ref:7,forwardedAs:'b'})); const nested=render(jsx(Nested,{...common,ref:9})); const overridden=render(jsx(Linked,{...common,as:'nav',ref:11})); const aside=render(jsx(Aside,{...common,ref:13})); const again=render(jsx(Again,{...common,ref:15})); globalThis.observed={elements:[linked,nested,overridden,aside,again],calls:globalThis.calls};",
            "globalThis.calls=[]; globalThis.Link=(props)=>{calls.push(['Link',props.ref]);return jsx('a',props)}; globalThis.motion={div:(props)=>{calls.push(['motion.div',props.ref]);return jsx('main',props)}};",
        );
        assert_eq!(actual["calls"], json!([["Link", 7], ["motion.div", 9]]));
        for (i, (target, reference)) in [
            ("a", 7),
            ("main", 9),
            ("nav", 11),
            ("aside", 13),
            ("nav", 15),
        ]
        .into_iter()
        .enumerate()
        {
            let element = &actual["elements"][i];
            assert_eq!(element["target"], target);
            let mut expected = json!({"ref":reference,"children":"child","role":"region"});
            if i == 0 {
                expected["as"] = json!("b");
            }
            props(element, expected);
            assert_eq!(
                declarations(&outputs, element, single_css),
                vec![("color".into(), "red".into())]
            );
        }
        records.push((
            outputs,
            actual["elements"]
                .as_array()
                .unwrap_or_else(|| panic!("elements"))
                .clone(),
            single_css,
        ));
    }
    records
}
