mod retained;

use retained::{BASE, run};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case(("", "aside", "caller"))]
#[case((".withComponent('main')", "main", "missing"))]
#[case((".withComponent(Target)", "nav", "caller"))]
#[serial]
fn retained_chain_uses_declared_target_filter_when_intermediate_has_no_bindings(
    #[case] target: (&str, &str, &str),
    #[values(false, true)] native_middle: bool,
) {
    let (retarget, expected_tag, tone) = target;
    // Given: A owns captures, B has zero local bindings, and C may retarget B again.
    let middle_target = if native_middle { "'section'" } else { "Target" };
    let middle = format!(
        "import {{styled}} from '@devup-ui/react';import {{forwardRef}} from 'react';import {{jsx}} from 'react/jsx-runtime';import {{Base}} from './base';const trace=[];const Target=forwardRef((p,ref)=>jsx('aside',{{...p,'data-tone':p.theme?.tone??'missing',ref}}));const Mid=Base.withComponent({middle_target});export {{Mid,trace}};"
    );
    let definition = if retarget.is_empty() {
        "const Card=styled(Mid)({color:'blue',padding:'2px'});".to_string()
    } else {
        format!(
            "const Extended=styled(Mid)({{color:'blue',padding:'2px'}});const Card=Extended{retarget};"
        )
    };
    let app = format!(
        "import {{styled}} from '@devup-ui/react';import {{forwardRef}} from 'react';import {{jsx}} from 'react/jsx-runtime';import {{Mid}} from './mid';const trace=[];const Target=forwardRef((p,ref)=>jsx('nav',{{...p,'data-tone':p.theme?.tone??'missing',ref}}));{definition}export {{Card,trace}};"
    );
    // When: the actual generated three-module chain renders twice with target channels.
    let result = run(
        &[
            ("/src/base.ts", BASE),
            ("/src/mid.ts", &middle),
            ("/src/app.ts", &app),
        ],
        "const app=modules['/src/app.ts'];const before=modules['/src/base.ts'].trace.slice();const first=render(jsx(app.Card,{theme:{tone:'caller'},customProp:'custom',ref:7,children:'child',forwardedAs:'em',style:{opacity:0.5}}));const second=render(jsx(app.Card,{theme:{tone:'caller'},ref:8,children:'next'}));JSON.stringify([before,[first,second].map(e=>[e.target,e.props.ref,e.props.children,slots(e),values(e,'color'),values(e,'padding')]),first.props['data-tone']??'missing',first.props.as,first.props.customProp,first.props.style.opacity,modules['/src/base.ts'].trace,modules['/src/mid.ts'].trace,app.trace,identities(first).length,new Set(identities(first)).size]);",
    );
    // Then: incoming forwarding bypasses obsolete filters and the original A renderer remains.
    let tag = if retarget.is_empty() && native_middle {
        "section"
    } else {
        expected_tag
    };
    let tone = if retarget.is_empty() {
        if native_middle { "missing" } else { "caller" }
    } else {
        tone
    };
    assert_eq!(
        result,
        serde_json::json!([
            [],
            [
                [tag, 7, "child", [], ["blue"], ["2px"]],
                [tag, 8, "next", [], ["blue"], ["2px"]]
            ],
            tone,
            "em",
            "custom",
            0.5,
            ["controller", "leaf", "controller", "leaf"],
            [],
            [],
            3,
            3
        ])
    );
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn exact_slot_pairs_are_translated_when_source_and_importer_reservations_differ(
    #[case] collide: bool,
    #[values("/src/app.ts", "/src/app.tsx")] filename: &str,
) {
    // Given: wrapped authored class/forwarded keys force independent upstream reservations.
    let authored = if collide {
        "const authored={[(('__devupCss0') as string)]:'class',['__devupForwarded0' as const]:'forwarded'};"
    } else {
        "const authored={value:'control'};"
    };
    let base = format!("{authored}{BASE}");
    let app = "import {styled} from '@devup-ui/react';import * as UI from './base';const trace=[];const Card=styled(UI.Base)({color:'blue'});export {Card,trace};";
    let outer = "import {styled} from '@devup-ui/react';import {Card} from './app';const Outer=styled(Card)({padding:'2px'});export {Outer};";
    // When: equal or distinct exact keys are consumed, then empty object/string overrides render.
    let result = run(
        &[
            ("/src/base.ts", &base),
            (filename, app),
            ("/src/outer.ts", outer),
        ],
        &format!(
            "render(jsx(modules['/src/outer.ts'].Outer,{{ref:7,children:'child',theme:{{tone:'value'}}}}));const cardProps=frames[0].props;const baseProps=frames[1].props;const classKey=slots({{props:cardProps}}).find(k=>k.startsWith('__devupCss'));const forwardedKey=slots({{props:cardProps}}).find(k=>k.startsWith('__devupForwarded'));const same=slots({{props:cardProps}}).sort().join()===slots({{props:baseProps}}).sort().join();frames.length=0;const final=render(jsx(modules[{filename:?}].Card,{{[classKey]:'',[forwardedKey]:{{}},ref:9,children:'removed',theme:{{tone:'removed'}},customProp:'removed',style:{{opacity:0.75}}}}));JSON.stringify([same,slots(final),values(final,'color'),final.props.ref??null,final.props.children??null,final.props.theme??null,final.props.customProp??null,final.props.style.opacity,modules['/src/base.ts'].trace,modules[{filename:?}].trace]);"
        ),
    );
    // Then: neither empty override restores defaults and no private transport reaches the leaf.
    assert_eq!(
        result,
        serde_json::json!([
            !collide,
            [],
            [],
            null,
            null,
            null,
            null,
            0.75,
            ["controller", "leaf", "controller", "leaf"],
            []
        ])
    );
}

#[test]
#[serial]
fn combined_known_filters_use_raw_props_when_custom_chain_retargets_native_root() {
    // Given: two canonical named predicates and a custom target that reads theme/ref/children.
    let base = BASE.replace(
        "styled.article(",
        "styled.article.withConfig({shouldForwardProp:p=>p!=='dropA'})(",
    );
    let middle = "import {styled} from '@devup-ui/react';import {Base} from './base';const Mid=styled(Base).withConfig({shouldForwardProp:p=>p!=='dropB'})({padding:'2px'});export {Mid};";
    let app = "import {styled} from '@devup-ui/react';import {forwardRef} from 'react';import {jsx} from 'react/jsx-runtime';import {Mid} from './mid';const Target=forwardRef((p,ref)=>jsx('nav',{...p,ref}));const Card=Mid.withComponent(Target);const App=()=> <Card theme={{tone:'caller'}} dropA='A' dropB='B' $secret='secret' customProp='custom' ref={7} children='child'/>;export {Card,App};";
    // When: JSX pruning and the compiled renderer use the same inherited filter metadata.
    let result = run(
        &[
            ("/src/base.ts", &base),
            ("/src/mid.ts", middle),
            ("/src/app.tsx", app),
        ],
        "const raw=modules['/src/app.tsx'].App();const final=render(raw);JSON.stringify([raw.props.dropA??null,raw.props.dropB??null,raw.props.$secret??null,final.props.dropA??null,final.props.dropB??null,final.props.$secret??null,final.props.theme.tone,final.props.customProp,final.props.ref,final.props.children,slots(final),values(final,'padding'),modules['/src/base.ts'].trace]);",
    );
    // Then: canonical filtering, not a new runtime predicate, removes only its known omissions.
    assert_eq!(
        result,
        serde_json::json!([
            null,
            null,
            null,
            null,
            null,
            null,
            "caller",
            "custom",
            7,
            "child",
            [],
            ["2px"],
            ["controller", "leaf"]
        ])
    );
}

#[test]
#[serial]
fn local_attrs_reads_keep_inputs_when_fixed_import_filters_forwarding() {
    // Given: local own attrs and a CSS read need props that canonical native forwarding omits.
    let app = "import {styled} from '@devup-ui/react';import {Base} from './base';const Card=styled(Base).attrs(p=>({title:p.source,style:{opacity:(0.5)}}))`color:${p=>p.tone};padding:2px;`;export {Card};";
    // When: only this local pipeline runs before the original fixed definer callback.
    let result = run(
        &[("/src/base.ts", BASE), ("/src/app.ts", app)],
        "const final=render(jsx(modules['/src/app.ts'].Card,{source:'title',tone:'blue',style:{color:'pink'},ref:7,children:'child'}));JSON.stringify([final.props.title,final.props.tone??null,values(final,'color'),values(final,'padding'),final.props.style.color,final.props.style.opacity,final.props.ref,final.props.children,slots(final),modules['/src/base.ts'].trace]);",
    );
    // Then: evaluation uses raw props separately from the complete forwarded-rest object.
    assert_eq!(
        result,
        serde_json::json!([
            "title",
            null,
            ["blue"],
            ["2px"],
            "pink",
            0.5,
            7,
            "child",
            [],
            ["controller", "leaf"]
        ])
    );
}
