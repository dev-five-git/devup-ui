#[path = "retained/capability.rs"]
mod capability;
mod retained;

use retained::{BASE, run};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn fixed_css_composes_when_imported_or_extended(
    #[case] extension: bool,
    #[values(false, true)] jsx: bool,
    #[values(false, true)] overlap: bool,
) {
    // Given: the direct/extended caller shadows both defining capture names.
    let filename = if jsx { "/src/app.tsx" } else { "/src/app.ts" };
    let definition = if extension {
        "const Card=styled(Base)({padding:'2px'});"
    } else {
        ""
    };
    let imported = if extension { "Base" } else { "Base as Card" };
    let css = if overlap {
        "{color:'blue',margin:'3px'}"
    } else {
        "{margin:'3px'}"
    };
    let element = if jsx {
        format!("<Card css={{{css}}} ref={{ref}} children='child' className='external' />")
    } else {
        format!("jsx(Card,{{css:{css},ref,children:'child',className:'external'}})")
    };
    let app = format!(
        "import {{styled}} from '@devup-ui/react';import {{jsx}} from '@emotion/react';import {{{imported}}} from './base';const trace=[];const selected=false;{definition}const App=(trace,selected,ref)=>{element};export {{Card,App,trace}};"
    );
    // When: generated module callbacks render twice through the real compiled caller.
    let result = run(
        &[("/src/base.ts", BASE), (filename, &app)],
        &format!(
            "const app=modules[{filename:?}];const raw=app.App([],false,7);const first=render(raw);const second=render(app.App([],false,8));JSON.stringify([raw.target===app.Card,[first,second].map(e=>[e.target,e.props.ref,e.props.children,slots(e),values(e,'color'),values(e,'padding'),values(e,'margin'),e.props.className.includes('external')]),modules['/src/base.ts'].trace,app.trace,identities(first).length,new Set(identities(first)).size]);"
        ),
    );
    // Then: only the winning atoms survive, identities are unique, and A owns its effects.
    let padding = if extension { vec!["2px"] } else { vec![] };
    assert_eq!(
        result,
        serde_json::json!([
            true,
            [
                [
                    "article",
                    7,
                    "child",
                    [],
                    [if overlap { "blue" } else { "green" }],
                    padding,
                    ["3px"],
                    true
                ],
                [
                    "article",
                    8,
                    "child",
                    [],
                    [if overlap { "blue" } else { "green" }],
                    padding,
                    ["3px"],
                    true
                ]
            ],
            ["controller", "leaf", "controller", "leaf"],
            [],
            if extension { 2 } else { 1 },
            if extension { 2 } else { 1 }
        ])
    );
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn complete_class_replacement_stays_empty_when_registered_caller_covers_styles(
    #[case] extension: bool,
) {
    // Given: registered purple covers both the fixed color and the CSS-prop color.
    let definition = if extension {
        "const Card=styled(Base)({});"
    } else {
        ""
    };
    let imported = if extension { "Base" } else { "Base as Card" };
    let app = format!(
        "import {{styled,css}} from '@devup-ui/react';import {{jsx}} from '@emotion/react';import {{{imported}}} from './base';{definition}const override=css({{color:'purple'}});const App=()=>jsx(Card,{{css:{{color:'blue'}},className:override+' external'}});export {{Card,App}};"
    );
    // When: a complete empty replacement travels through equal-spelled local/upstream slots.
    let result = run(
        &[("/src/base.ts", BASE), ("/src/app.ts", &app)],
        "const app=modules['/src/app.ts'];const raw=app.App();const final=render(raw);JSON.stringify([slots(raw).map(k=>raw.props[k]),values(final,'color'),slots(final),modules['/src/base.ts'].trace]);",
    );
    // Then: an empty string suppresses the original green instead of restoring it.
    assert_eq!(
        result,
        serde_json::json!([[""], ["purple"], [], ["controller", "leaf"]])
    );
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn dynamic_css_variables_keep_inline_precedence_when_fixed_import_is_extended(#[case] jsx: bool) {
    // Given: CSS variables are caller-owned and inline styles may replace them.
    let filename = if jsx { "/src/app.tsx" } else { "/src/app.ts" };
    let element = if jsx {
        "<Card css={{color:value}} style={style} ref={7} children='child'/>"
    } else {
        "jsx(Card,{css:{color:value},style,ref:7,children:'child'})"
    };
    let app = format!(
        "import {{styled}} from '@devup-ui/react';import {{jsx}} from '@emotion/react';import {{Base}} from './base';const Card=styled(Base)({{padding:'2px'}});const App=(value,style)=>{element};export {{App}};"
    );
    // When: one caller overrides a property, then another overrides the generated variable.
    let result = run(
        &[("/src/base.ts", BASE), (filename, &app)],
        &format!(
            "const app=modules[{filename:?}];const first=render(app.App('blue',{{color:'pink'}}));const variable=Object.keys(first.props.style).find(k=>k.startsWith('--'));const second=render(app.App('teal',{{[variable]:'orange',opacity:0.5}}));JSON.stringify([values(first,'color'),values(second,'color'),first.props.style.color,second.props.style.opacity,values(first,'padding'),first.props.ref,first.props.children,slots(first),slots(second),modules['/src/base.ts'].trace]);"
        ),
    );
    // Then: inline precedence, captures, and final ordinary props match the local contract.
    assert_eq!(
        result,
        serde_json::json!([
            ["blue"],
            ["orange"],
            "pink",
            0.5,
            ["2px"],
            7,
            "child",
            [],
            [],
            ["controller", "leaf", "controller", "leaf"]
        ])
    );
}
