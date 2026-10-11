use super::*;
use rstest::rstest;

const ROUTES: &[(&str, &str)] = &[
    (
        "import {css} from '@devup-ui/react';",
        "const a=css({styleOrder:VALUE,color:'red'});",
    ),
    (
        "import {styled} from '@devup-ui/react';",
        "const a=styled('div',{styleOrder:VALUE,color:'red'});",
    ),
    (
        "import {globalCss} from '@devup-ui/react';",
        "globalCss({body:{styleOrder:VALUE,color:'red'}});",
    ),
    (
        "import {css} from '@devup-ui/react';",
        "const a=css`style-order:${VALUE};color:red`;",
    ),
    (
        "import {styled} from '@devup-ui/react';",
        "const a=styled('div')`style-order:${VALUE};color:red`;",
    ),
    (
        "import {globalCss} from '@devup-ui/react';",
        "globalCss`body{style-order:${VALUE};color:red}`;",
    ),
    (
        "import {css} from '@emotion/css';",
        "const a=css({styleOrder:VALUE,color:'red'});",
    ),
    (
        "import {css} from '@emotion/css';",
        "const a=css`style-order:${VALUE};color:red`;",
    ),
    (
        "import styled from '@emotion/styled';",
        "const a=styled.div({styleOrder:VALUE,color:'red'});",
    ),
    (
        "import styled from '@emotion/styled';",
        "const a=styled.div`style-order:${VALUE};color:red`;",
    ),
    (
        "import styled from 'styled-components';",
        "const a=styled.div({styleOrder:VALUE,color:'red'});",
    ),
    (
        "import styled from 'styled-components';",
        "const a=styled.div`style-order:${VALUE};color:red`;",
    ),
    (
        "import {css} from 'styled-components';",
        "const a=css({styleOrder:VALUE,color:'red'});",
    ),
    (
        "import {css} from 'styled-components';",
        "const a=css`style-order:${VALUE};color:red`;",
    ),
    (
        "import {createGlobalStyle} from 'styled-components';",
        "const a=createGlobalStyle`body{style-order:${VALUE};color:red}`;",
    ),
    (
        "import {Box} from '@devup-ui/react';",
        "const a=<Box styleOrder={VALUE} color='red'/>;",
    ),
    (
        "import {Box} from '@devup-ui/react';import {jsx} from 'react/jsx-runtime';",
        "const a=jsx(Box,{styleOrder:VALUE,color:'red'});",
    ),
];

fn sibling_source(route: usize, value: &str) -> String {
    let (import, body) = ROUTES[route];
    format!(
        "{import}\nconst guard=true;const compute=()=>2;{}",
        body.replace("VALUE", value)
    )
}

fn sibling_compile(source: &str) -> Result<ExtractOutput, String> {
    compile_with(
        source,
        ExtractOption {
            import_aliases: HashMap::from([
                ("@emotion/css".to_string(), ImportAlias::NamedToNamed),
                (
                    "styled-components".to_string(),
                    ImportAlias::DefaultToNamed("styled".to_string()),
                ),
                (
                    "@emotion/styled".to_string(),
                    ImportAlias::DefaultToNamed("styled".to_string()),
                ),
            ]),
            ..ExtractOption::default()
        },
    )
}

#[rstest]
#[serial]
fn siblings_reject_when_an_explicit_literal_is_invalid(
    #[values(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16)] route: usize,
    #[values(
        "true?2:'01'",
        "false&&'01'",
        "false",
        "guard?2:'01'",
        "true?compute():'01'"
    )]
    value: &str,
) {
    // Given: recognized direct metadata with an invalid explicit branch.
    let source = sibling_source(route, value);
    let invalid = if value == "false" { "false" } else { "'01'" };
    let column = source
        .lines()
        .nth(1)
        .required("fixture line")
        .find(invalid)
        .required("invalid literal")
        + 1;
    // When: the public extractor processes the authored source.
    let actual = sibling_compile(&source).required_err("invalid metadata must reject");
    // Then: exactly one error points to the invalid literal, not the guard.
    assert_eq!(actual.lines().count(), 1, "{source}: {actual}");
    assert!(
        actual.starts_with(&format!("a.tsx:2:{column}:")),
        "{source}: {actual}"
    );
}

#[rstest]
#[serial]
fn siblings_keep_default_order_when_a_literal_and_is_false(
    #[values(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16)] route: usize,
    #[values(false, true)] fallback: bool,
) {
    // Given: an order whose && synthesizes absence.
    let mut source = sibling_source(route, "false&&2");
    if fallback {
        source.push_str("import {css as fallbackCss} from '@devup-ui/react';function make(n){return n+'px'};const b=fallbackCss({w:make(2)});");
    }
    // When: public extraction consumes the metadata.
    let actual = sibling_compile(&source).required("absent order must compile");
    // Then: the red rule keeps the route's default order, with no order 2.
    let orders: Vec<_> = actual
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) if style.property() == "color" => {
                Some((style.value(), style.style_order()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(orders.len(), 1, "{source}");
    assert_eq!(orders[0].0, "red");
    assert!(
        matches!(orders[0].1, None | Some(0 | 255)),
        "{source}: {orders:?}"
    );
}

#[test]
#[serial]
fn metadata_guards_stay_lazy_when_an_ordinary_value_needs_fallback() {
    // Given: nested effectful guards and an independently computable width.
    let source = "import {css} from '@devup-ui/react';const make=()=> '2px';const guard=name=>(trace.push(name),false);const a=css({styleOrder:guard('outer')&&(guard('inner')?2:3),w:make()});";
    // When: the generated code runs through the real element driver.
    let actual = whole::evaluate(source, "a.trim()");
    // Then: only the outer guard runs, and the rule has default order.
    assert_eq!(actual.trace, serde_json::json!(["outer"]));
    assert_eq!(actual.element, "width-0-2px--255-a");
}

#[rstest]
#[case(
    "function make(n){return n+'px'};const a=css({w:make(2)});",
    "width",
    "2px",
    None
)]
#[case(
    "const make=()=>2;const a=css({styleOrder:make(),color:'red'});",
    "color",
    "red",
    Some(2)
)]
#[case(
    "function make(n){return n+'px'};const a=css({styleOrder:false&&2,w:make(2)});",
    "width",
    "2px",
    None
)]
#[case(
    "function make(n){return n+'px'};const a=css`style-order:${false&&2};width:${make(2)}`;",
    "width",
    "2px",
    None
)]
#[serial]
fn computable_values_keep_boa_fallback_when_static_extraction_needs_evaluation(
    #[case] body: &str,
    #[case] property: &str,
    #[case] value: &str,
    #[case] order: Option<u8>,
) {
    // Given: a function result that constant inlining cannot compute.
    let source = format!("import {{css}} from '@devup-ui/react';{body}");
    // When: the public fallback evaluates the exact dependency closure.
    let actual = output(&source);
    // Then: the computed rule is emitted rather than rejecting or becoming a runtime style.
    assert!(actual.styles.iter().any(|style| matches!(style, ExtractStyleValue::Static(style) if style.property() == property && style.value() == value && style.style_order() == order)), "{source}");
}
