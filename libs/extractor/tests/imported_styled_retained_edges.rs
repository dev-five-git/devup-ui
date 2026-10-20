mod retained;

use retained::{BASE, run};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn retained_effect_order_matches_winning_styles_when_both_modules_capture(#[case] overlap: bool) {
    // Given: overwritten effects in each module have independent trace owners.
    let base = BASE.replace(
        "trace.push('controller')",
        "(trace.push('controller'),order.push('A'))",
    );
    let color = if overlap { ",color:'blue'" } else { "" };
    let app = format!(
        "import {{styled}} from '@devup-ui/react';import {{Base}} from './base';const trace=[];const Card=styled(Base)({{...((trace.push('controller'),order.push('B'),true)?{{margin:(trace.push('leaf'),'1px')}}:{{}}),margin:'2px',padding:'2px'{color}}});export {{Card,trace}};"
    );
    // When: two renders execute the generated callbacks and expose their actual atoms.
    let result = run(
        &[("/src/base.ts", &base), ("/src/app.ts", &app)],
        "const app=modules['/src/app.ts'];const before=[modules['/src/base.ts'].trace.slice(),app.trace.slice()];const first=render(jsx(app.Card,{ref:7,children:'first'}));const second=render(jsx(app.Card,{ref:8,children:'second'}));JSON.stringify([before,[first,second].map(e=>[e.target,e.props.ref,e.props.children,values(e,'color'),values(e,'margin'),values(e,'padding'),slots(e)]),modules['/src/base.ts'].trace,app.trace,order]);",
    );
    // Then: B precedes A per render while D1 removes only the losing style, not its effects.
    assert_eq!(
        result,
        serde_json::json!([
            [[], []],
            [
                [
                    "article",
                    7,
                    "first",
                    [if overlap { "blue" } else { "green" }],
                    ["2px"],
                    ["2px"],
                    []
                ],
                [
                    "article",
                    8,
                    "second",
                    [if overlap { "blue" } else { "green" }],
                    ["2px"],
                    ["2px"],
                    []
                ]
            ],
            ["controller", "leaf", "controller", "leaf"],
            ["controller", "leaf", "controller", "leaf"],
            ["B", "A", "B", "A"]
        ])
    );
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn default_target_reads_stay_in_definer_when_retained_child_renders(#[case] retarget: bool) {
    // Given: the middle module alone owns an observable default-target getter.
    let middle = "import {styled} from '@devup-ui/react';import {forwardRef} from 'react';import {jsx} from 'react/jsx-runtime';import {Base} from './base';const trace=[];const Target=forwardRef((p,ref)=>jsx('aside',{...p,ref}));const targets={get Target(){trace.push('target');return Target;}};const Mid=Base.withComponent(targets.Target);export {Mid,trace};";
    let definition = if retarget {
        "Mid.withComponent('main')"
    } else {
        "styled(Mid)({})"
    };
    let app = format!(
        "import {{styled}} from '@devup-ui/react';import {{Mid}} from './mid';const Card={definition};export {{Card}};"
    );
    // When: two default renders and one runtime-as override traverse the original middle callback.
    let result = run(
        &[
            ("/src/base.ts", BASE),
            ("/src/mid.ts", middle),
            ("/src/app.ts", &app),
        ],
        "const mid=modules['/src/mid.ts'];const before=mid.trace.slice();const Card=modules['/src/app.ts'].Card;const first=render(jsx(Card,{ref:7,children:'first'}));const second=render(jsx(Card,{ref:8,children:'second'}));const third=render(jsx(Card,{as:'nav',forwardedAs:'em',ref:9,children:'third'}));JSON.stringify([before,[first,second,third].map(e=>[e.target,e.props.ref,e.props.children,e.props.as??null,slots(e)]),mid.trace,modules['/src/base.ts'].trace]);",
    );
    // Then: no eager or importer getter read is added; explicit targets skip obsolete defaults.
    let tag = if retarget { "main" } else { "aside" };
    let reads = if retarget {
        vec![]
    } else {
        vec!["target", "target"]
    };
    assert_eq!(
        result,
        serde_json::json!([
            [],
            [
                [tag, 7, "first", null, []],
                [tag, 8, "second", null, []],
                ["nav", 9, "third", "em", []]
            ],
            reads,
            [
                "controller",
                "leaf",
                "controller",
                "leaf",
                "controller",
                "leaf"
            ]
        ])
    );
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn custom_export_advertises_its_own_slot_when_imported_css_replaces_styles(#[case] jsx: bool) {
    // Given: a custom fixed-summary descendant has no local captures or portable Target AST.
    let middle = "import {styled} from '@devup-ui/react';import {forwardRef} from 'react';import { jsx } from 'react/jsx-runtime';import {Base} from './base';const Target=forwardRef((p,ref)=>jsx('aside',{...p,'data-tone':p.theme?.tone,ref}));const Mid=Base.withComponent(Target);export {Mid};";
    let filename = if jsx { "/src/app.tsx" } else { "/src/app.ts" };
    let element = if jsx {
        "<Mid css={{color:'blue',padding:'4px'}} theme={{tone:'caller'}} ref={7} children='child'/>"
    } else {
        "jsx(Mid,{css:{color:'blue',padding:'4px'},theme:{tone:'caller'},ref:7,children:'child'})"
    };
    let app = format!(
        "import {{jsx}} from '@emotion/react';import {{Mid}} from './mid';const App=()=>{element};export {{App}};"
    );
    // When: the last importer composes styles directly against the middle module's real pair.
    let result = run(
        &[
            ("/src/base.ts", BASE),
            ("/src/mid.ts", middle),
            (filename, &app),
        ],
        &format!(
            "const raw=modules[{filename:?}].App();const final=render(raw);JSON.stringify([slots(raw).length,final.target,final.props['data-tone'],final.props.ref,final.props.children,values(final,'color'),values(final,'padding'),slots(final),identities(final).length,new Set(identities(final)).size,modules['/src/base.ts'].trace]);"
        ),
    );
    // Then: the custom renderer and A closure remain while later fixed CSS wins exactly once.
    assert_eq!(
        result,
        serde_json::json!([
            1,
            "aside",
            "caller",
            7,
            "child",
            ["blue"],
            ["4px"],
            [],
            2,
            2,
            ["controller", "leaf"]
        ])
    );
}

#[rstest]
#[case("/src/app.ts")]
#[case("/src/app.tsx")]
#[serial]
fn bound_import_keeps_original_value_when_lowercase_name_requires_eager_binding(
    #[case] filename: &str,
) {
    // Given: a lowercase imported value uses the canonical eager bound-target path.
    let app = "import {styled} from '@devup-ui/react';import {Base as base} from './base';const Card=styled(base)({color:'blue'});export {Card};";
    // When: real emitted code renders the retained lowercase binding twice.
    let result = run(
        &[("/src/base.ts", BASE), (filename, app)],
        &format!(
            "const Card=modules[{filename:?}].Card;const before=modules['/src/base.ts'].trace.slice();const first=render(jsx(Card,{{ref:7,children:'first'}}));const second=render(jsx(Card,{{ref:8,children:'second'}}));JSON.stringify([before,[first,second].map(e=>[e.target,e.props.ref,e.props.children,values(e,'color'),slots(e)]),modules['/src/base.ts'].trace]);"
        ),
    );
    // Then: the renderer is a value, never a lowercase native tag or an invalid JSX target.
    assert_eq!(
        result,
        serde_json::json!([
            [],
            [
                ["article", 7, "first", ["blue"], []],
                ["article", 8, "second", ["blue"], []]
            ],
            ["controller", "leaf", "controller", "leaf"]
        ])
    );
}
