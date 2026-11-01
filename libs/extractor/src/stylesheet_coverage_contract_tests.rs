use crate::{ExtractOption, ExtractOutput, ExtractStyleValue, extract_without_source_map};
use serial_test::serial;

#[test]
#[serial]
fn coverage_contract_rejects_dead_font_face_descriptor_when_media_query_is_imports()
-> Result<(), String> {
    // Given
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    let source = "import{globalCss}from'@devup-ui/react';globalCss({'@media':{imports:{fontFaces:[{fontFamily:'Example',boxAlign:'center'}]}}});";
    let column = source.find("boxAlign").ok_or("missing authored key")? + 1;
    // When
    let error =
        extract_without_source_map("/coverage-contract.tsx", source, ExtractOption::default())
            .err()
            .ok_or("dead font-face descriptor accepted")?
            .to_string();
    // Then
    assert!(
        error.starts_with(&format!("/coverage-contract.tsx:1:{column}:")),
        "{error}"
    );
    assert!(error.contains("`boxAlign`"), "{error}");
    assert!(error.contains("box-align"), "{error}");
    Ok(())
}

fn compile(body: &str) -> Result<ExtractOutput, String> {
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    extract_without_source_map(
        "/coverage-contract.css.ts",
        &format!("import{{css}}from'@devup-ui/react';{body}"),
        ExtractOption::default(),
    )
    .map_err(|error| error.to_string())
}

#[test]
#[serial]
fn w82g_serialization_output_matrix() {
    // Given
    let _state = crate::test_state::TestStateGuard::new();
    let contexts = [
        (
            "jsx-responsive",
            "/matrix.tsx",
            "export const view=<Box p={values()}/>;",
            false,
        ),
        (
            "jsx-call",
            "/matrix.tsx",
            "export const view=jsx(Box,{p:values()});",
            false,
        ),
        (
            "css-object",
            "/matrix.tsx",
            "export const card=css({p:values()});",
            false,
        ),
        (
            "styled",
            "/matrix.tsx",
            "export const Card=styled('div',{p:values()});",
            false,
        ),
        (
            "keyframes",
            "/matrix.tsx",
            "export const animation=keyframes({from:{p:values()}});",
            false,
        ),
        (
            "stylesheet-values",
            "/matrix.css.ts",
            "export const card=css({p:values()});",
            false,
        ),
        (
            "stylesheet-export",
            "/matrix.css.ts",
            concat!(
                "export const value=values();export const card=css({",
                "p:2});"
            ),
            false,
        ),
        (
            "stylesheet-params",
            "/matrix.css.ts",
            "export const card=css({_is:{params:values(),p:2}});",
            true,
        ),
        (
            "ordinary-params",
            "/matrix.tsx",
            "export const card=css({_is:{params:values(),p:2}});",
            true,
        ),
    ];
    for (name, filename, body, params) in contexts {
        for (kind, middle) in [("hole", ""), ("undefined", "undefined"), ("null", "null")] {
            for single_css in [false, true] {
                css::class_map::reset_class_map();
                css::file_map::reset_file_map();
                let array = if params {
                    format!("['a',{middle},'b']")
                } else {
                    format!("[1,{middle},3]")
                };
                let source = format!(
                    "import{{Box,css,styled,keyframes}}from'@devup-ui/react';import{{jsx}}from'react/jsx-runtime';function values(){{return {array}}}{body}"
                );
                let option = crate::ExtractOption {
                    single_css,
                    ..crate::ExtractOption::default()
                };
                let reaches_literal = crate::build_time_values::has_build_time_values(
                    filename, &source, &option, None,
                );
                // When
                let result = extract_without_source_map(filename, &source, option);
                // Then: every recorded row is also a permanent behavioral regression.
                if params && (kind != "hole" || filename == "/matrix.tsx") {
                    let error = result
                        .as_ref()
                        .err()
                        .unwrap_or_else(|| panic!("{name}/{kind}: invalid entry accepted"));
                    assert!(
                        error
                            .to_string()
                            .contains(crate::extractor::selector_params::SELECTOR_PARAMS),
                        "{error}"
                    );
                } else {
                    let output = result
                        .as_ref()
                        .unwrap_or_else(|error| panic!("{name}/{kind}: {error}"));
                    assert!(!output.styles.is_empty(), "{name}/{kind}");
                    if params {
                        assert!(output.styles.iter().any(|style| matches!(style,
                            ExtractStyleValue::Static(style) if style.selector.as_ref().is_some_and(|selector| selector.to_string() == "&:is(a,b)")
                        )));
                    }
                }
                let outcome = match result {
                    Ok(output) => {
                        serde_json::json!({"code": output.code, "styles": format!("{:?}", output.styles)})
                    }
                    Err(error) => serde_json::json!({"error": error.to_string()}),
                };
                eprintln!(
                    "W82G_MATRIX {}",
                    serde_json::json!({"context": name, "kind": kind, "singleCss": single_css, "params": params, "source": source, "filename": filename, "reachesLiteral": reaches_literal, "outcome": outcome})
                );
            }
        }
    }
}

#[rstest::rstest]
#[case("css(base,{props:data,styleOrder:3,p:2})", false)]
#[case("css('external',{props:data,p:2})", false)]
#[case("css(['external',null,false,undefined,{props:data,p:2}])", false)]
#[case("css({props:data,_hover:{as:'section',p:2}})", false)]
#[case("css({props:data,_hover:{params:[],p:2}})", false)]
#[case("css({props:data,_nthChild:{params:['2n'],p:2}})", false)]
#[case("css({props:data,_host:{params:['','x'],p:2}})", false)]
#[case("css({props:data,_hover:{params:['x'],p:2}})", false)]
#[case("css({props:data,_hover:{params:null,p:2}})", false)]
#[case("css({props:data,_hover:{params:undefined,p:2}})", false)]
#[case("css({props:data,_hover:{params:void 0,p:2}})", false)]
#[case(concat!("css({props:data,_nthChild:{", "p:2}})"), false)]
#[case("css({props:data,_is:{params:[base],p:2}})", false)]
#[case("css({props:data,_is:{params:[`.${base}`],p:2}})", false)]
#[case(
    "css({props:data,_nthChild:{params:(()=>['2n'])(),_hover:{params:[],p:2}}})",
    false
)]
#[case("css({props:data,_is:{params:(()=>['a',,'b'])(),p:2}})", true)]
#[case(
    "css({props:data,_groupNthChild:{params:['2n'],_hover:{params:[],p:2}}})",
    false
)]
#[case(
    "css({props:data,selectors:{\"&[data-note=':is']:hover\":{params:[],p:2}}})",
    false
)]
#[case(
    "css({props:data,selectors:{\"&[data-note='x']:nth-child\":{params:['2n'],p:2}}})",
    false
)]
#[case(concat!("css({props:data,':hover':{", "p:2},'[data-on]':{", "w:3},'> a':{", "h:4}})"), false)]
#[case(concat!("css({props:data,'@media (min-width:1px)':{'@media not print and (color)':{", "p:2}}})"), false)]
#[case("css({props:data,_is:{params:(()=>['a',null,'b'])(),p:2}})", true)]
#[case("css({props:data,_is:{params:(()=>['a',undefined,'b'])(),p:2}})", true)]
#[serial]
fn coverage_contract_bridge_keeps_shallow_rules(
    #[case] expression: &str,
    #[case] refused: bool,
    #[values(false, true)] single_css: bool,
) -> Result<(), String> {
    // Given: independent shallow extraction and an ignored, heap-built payload.
    let prefix = "const base=css({color:'red'});";
    let compile = |body: &str| {
        css::class_map::reset_class_map();
        css::file_map::reset_file_map();
        extract_without_source_map(
            "/coverage-contract.css.ts",
            &format!("import{{css}}from'@devup-ui/react';{body}"),
            ExtractOption {
                single_css,
                ..ExtractOption::default()
            },
        )
        .map_err(|error| error.to_string())
    };
    let shallow = expression.replace("props:data", "props:0");
    if refused {
        for (body, owner) in [
            (format!("{prefix}export const card={shallow};"), "(()=>"),
            (
                format!(
                    "{prefix}let data=0;for(let i=0;i<16;i++)data={{payload:data}};export const card={expression};"
                ),
                "css({props:data",
            ),
        ] {
            let source = format!("import{{css}}from'@devup-ui/react';{body}");
            let column = source.find(owner).ok_or("missing supplying helper call")? + 1;
            let error = compile(&body)
                .err()
                .ok_or("lossy computed params accepted")?;
            assert_eq!(
                error,
                format!(
                    "/coverage-contract.css.ts:1:{column}: `css()` cannot use `null` at build time: {}",
                    crate::extractor::selector_params::SELECTOR_PARAMS
                )
            );
        }
        return Ok(());
    }
    let expected = compile(&format!("{prefix}export const card={shallow};"))?;
    assert!(expected.styles.iter().any(|style| matches!(style,
        ExtractStyleValue::Static(style) if style.property() == "padding" && style.value() == "8px"
    )));
    let source = format!(
        "{prefix}let data=0;for(let i=0;i<16;i++)data={{payload:data}};export const card={expression};"
    );
    // When
    let actual = compile(&source)?;
    // Then
    assert_eq!(actual.code, expected.code);
    assert_eq!(actual.styles, expected.styles);
    Ok(())
}

#[rstest::rstest]
#[case("[{},[],'2n']", "{}")]
#[case("[[],'2n']", "[]")]
#[case("['2n',{}]", "{}")]
#[case("['2n',[]]", "[]")]
#[serial]
fn coverage_contract_bridge_rejects_unreadable_selector_params(
    #[case] params: &str,
    #[case] marker: &str,
) -> Result<(), String> {
    // Given an unreadable params entry alongside the same ignored deep payload.
    let body = format!(
        "let data=0;for(let i=0;i<16;i++)data={{payload:data}};export const card=css({{props:data,_nthChild:{{params:{params},p:2}}}});"
    );
    let source = format!("import{{css}}from'@devup-ui/react';{body}");
    let column = source.find(marker).ok_or("missing params entry")? + 1;
    // When the bridge validates the actual authored params rather than discarding them.
    let error = compile(&body).err().ok_or("unreadable params accepted")?;
    // Then the original entry receives the same located refusal as shallow extraction.
    assert!(
        error.starts_with(&format!("/coverage-contract.css.ts:1:{column}:")),
        "{error}"
    );
    assert!(
        error.contains(crate::extractor::selector_params::SELECTOR_PARAMS),
        "{error}"
    );
    Ok(())
}

#[rstest::rstest]
#[case("selectors:{invalidSelector:{p:2}}")]
#[case(concat!("_invalidSelector:{", "p:2}"))]
#[serial]
fn coverage_contract_bridge_rejects_invalid_selectors(#[case] rules: &str) -> Result<(), String> {
    // Given
    let source = format!(
        "let data=0;for(let i=0;i<16;i++)data={{payload:data}};export const card=css({{props:data,{rules}}});"
    );
    // When
    let error = compile(&source).err().ok_or("invalid selector accepted")?;
    // Then
    assert!(error.starts_with("/coverage-contract.css.ts:1:"), "{error}");
    assert!(error.contains(crate::utils::SELECTOR_NAME), "{error}");
    Ok(())
}

#[rstest::rstest]
#[case("ordinary", "['a',,'b']", true)]
#[case("ordinary", "['a',undefined,'b']", false)]
#[case("shallow", "['a',,'b']", true)]
#[case("shallow", "['a',undefined,'b']", false)]
#[case("deep", "['a',,'b']", true)]
#[case("deep", "['a',undefined,'b']", false)]
#[serial]
fn coverage_contract_sparse_params_follow_the_same_contract(
    #[case] mode: &str,
    #[case] params: &str,
    #[case] accepted: bool,
) -> Result<(), String> {
    let _state = crate::test_state::TestStateGuard::new();
    let prefix = if mode == "deep" {
        concat!("let data=0;for(let i=0;i<16;i++)data={", "payload:data};")
    } else {
        "const data=0;"
    };
    let source = format!(
        "import{{css}}from'@devup-ui/react';{prefix}export const card=css({{props:data,_is:{{params:{params},color:'red'}}}});"
    );
    let filename = if mode == "ordinary" {
        "/coverage-contract.tsx"
    } else {
        "/coverage-contract.css.ts"
    };
    let result = extract_without_source_map(filename, &source, ExtractOption::default())
        .map_err(|error| error.to_string());
    if accepted {
        let output = result?;
        assert!(output.styles.iter().any(|style| matches!(style,
            ExtractStyleValue::Static(style) if style.selector.as_ref().is_some_and(|selector| selector.to_string().contains(":is(a,b)"))
        )));
    } else {
        let error = result.err().ok_or("present undefined params accepted")?;
        let column = source.find("undefined").ok_or("missing undefined entry")? + 1;
        assert!(
            error.starts_with(&format!("{filename}:1:{column}:")),
            "{error}"
        );
        assert!(
            error.contains(crate::extractor::selector_params::SELECTOR_PARAMS),
            "{error}"
        );
    }
    Ok(())
}

#[test]
#[serial]
fn coverage_contract_bridge_template_keeps_scalar_declarations() -> Result<(), String> {
    // Given: a structured statement forces the bridge alongside literal CSS values.
    let source = concat!(
        "let data=0;for(let i=0;i<16;i++)data={",
        "payload:data};export const card=css`color:${'red'};${{props:data,p:2}}`;"
    );
    // When
    let output = compile(source)?;
    // Then
    assert!(output.styles.iter().any(|style| matches!(style,
        ExtractStyleValue::Static(style) if style.property() == "color" && style.value() == "red"
    )));
    assert!(output.styles.iter().any(|style| matches!(style,
        ExtractStyleValue::Static(style) if style.property() == "padding" && style.value() == "8px"
    )));
    Ok(())
}

#[test]
#[serial]
fn coverage_contract_bridge_template_rejects_unplaced_scalar() -> Result<(), String> {
    // Given: the text interpolation is not a declaration or a standalone statement.
    let source = concat!(
        "let data=0;for(let i=0;i<16;i++)data={",
        "payload:data};export const card=css`color:red;${{props:data,p:2}} ${'orphan'} {",
        "color:blue}`;"
    );
    // When
    let error = compile(source).err().ok_or("unplaced scalar accepted")?;
    // Then
    assert!(error.starts_with("/coverage-contract.css.ts:1:"), "{error}");
    assert!(error.contains("css"), "{error}");
    Ok(())
}

#[test]
#[serial]
fn coverage_contract_bridge_rejects_responsive_arrays_in_data_slots() -> Result<(), String> {
    // Given: capture preserves vars as data; lowering still checks its responsive shape.
    let source = concat!(
        "let data=0;for(let i=0;i<16;i++)data={",
        "payload:data};export const card=css({props:data,vars:[[1,2]]});"
    );
    // When
    let error = compile(source)
        .err()
        .ok_or("nested responsive vars accepted")?;
    // Then
    assert!(error.starts_with("/coverage-contract.css.ts:1:"), "{error}");
    assert!(error.contains(crate::utils::RESPONSIVE_ARRAY), "{error}");
    Ok(())
}

#[test]
#[serial]
fn coverage_contract_namespace_tag_composes_constant_rules() -> Result<(), String> {
    // Given: an ordinary call independently defines the expected declarations.
    let expected = compile("export const card=css({p:2,color:'tomato'});")?;
    let source = "import * as UI from '@devup-ui/react';const rules={p:2,color:'tomato'};export const card=UI.css`${rules}`;";
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    // When
    let output = extract_without_source_map(
        "/coverage-contract.css.ts",
        source,
        ExtractOption::default(),
    )
    .map_err(|error| error.to_string())?;
    // Then
    let card = output
        .code
        .lines()
        .find(|line| line.starts_with("export const card = "))
        .ok_or("missing namespace card export")?;
    let expected_card = expected
        .code
        .lines()
        .find(|line| line.starts_with("export const card = "))
        .ok_or("missing ordinary card export")?;
    assert_eq!(card, expected_card);
    assert_eq!(output.styles, expected.styles);
    Ok(())
}

#[test]
#[serial]
fn coverage_contract_bridge_template_keeps_registered_scalar_on_card() -> Result<(), String> {
    // Given: a fresh ordinary call supplies the base class independently of the template.
    let base = compile(concat!("export const card=css({", "p:1});"))?;
    let base = base
        .code
        .split_once("export const card = \"")
        .and_then(|(_, tail)| tail.split_once('"'))
        .ok_or("missing ordinary base class")?
        .0;
    let source = concat!(
        "const base=css({p:1});let data=0;for(let i=0;i<128;i++)data={",
        "payload:data};export const card=css`color:blue; ${base} ${{props:data,p:2}}`;"
    );
    // When
    let output = compile(source)?;
    // Then: the base must be selected by card, not merely present in the stylesheet.
    let card = output
        .code
        .split_once("export const card = \"")
        .and_then(|(_, tail)| tail.split_once('"'))
        .ok_or("missing compiled card classes")?
        .0;
    assert!(
        card.split_whitespace().any(|class| class == base),
        "{output:?}"
    );
    for (property, value) in [("color", "blue"), ("padding", "8px")] {
        assert!(
            output.styles.iter().any(|style| {
                matches!(style, ExtractStyleValue::Static(style)
                if style.property() == property && style.value() == value)
                    && matches!(style.extract(Some("/coverage-contract.css.ts")),
                    Some(crate::extract_style::style_property::StyleProperty::ClassName(name))
                    if card.split_whitespace().any(|class| class == name))
            }),
            "missing selected {property}:{value}: {output:?}"
        );
    }
    Ok(())
}

#[rstest::rstest]
#[case(concat!("css({", "p:1},{props:data,_is:{params:['a',{}],p:2}})"))]
#[case("css({...extra,props:data,_is:{params:['a',{}],p:2}})")]
#[case("css({[key]:1,props:data,_is:{params:['a',{}],p:2}})")]
#[case("css({p:1,p:3,props:data,_is:{params:['a',{}],p:2}})")]
#[case("css({props:data,_is:{params:[computed(),{}],p:2}})")]
#[case("css({...{_is:{params:['a',{}],p:2}},props:data})")]
#[case("css(...[{props:data,_is:{params:['a',{}],p:2}}])")]
#[case("css({props:data,_is:{params:['a',{}],p:2},p:1,p:3})")]
#[case("css({props:data,_hover:{...extra,_nthChild:{params:[],p:2}}})")]
#[serial]
fn coverage_contract_preflight_keeps_definitely_winning_bad_leaves(
    #[case] expression: &str,
    #[values(false, true)] single_css: bool,
) -> Result<(), String> {
    // Given: unknown metadata precedes the definite winning authored container.
    let _state = crate::test_state::TestStateGuard::new();
    let rule = format!("export const card={expression};");
    let source = format!(
        "import{{css}}from'@devup-ui/react';\nlet data=0;for(let i=0;i<16;i++)data={{payload:data}};const extra={{color:'red'}};const key='color';function computed(){{return 'a'}}\n{rule}"
    );
    let column = rule
        .find("{}")
        .or_else(|| rule.find("[]"))
        .ok_or("missing authored params witness")?
        + 1;
    // When
    let error = extract_without_source_map(
        "/winning-params.css.ts",
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    )
    .err()
    .ok_or("winning bad params accepted")?
    .to_string();
    // Then
    assert!(
        error.starts_with(&format!("/winning-params.css.ts:3:{column}:")),
        "{error}"
    );
    assert!(
        error.contains(crate::extractor::selector_params::SELECTOR_PARAMS),
        "{error}"
    );
    Ok(())
}

#[rstest::rstest]
#[case("css({props:data,_is:{params:[{}],params:['a'],p:2}})")]
#[case("css({props:data,_is:{params:[{}],p:9},_is:{params:['a'],p:2}})")]
#[case("css({props:data,_is:{params:[{}],p:9},...winner})")]
#[case("css({props:data,_is:{...oldParams,params:['a'],p:2}})")]
#[case("css({props:data,_is:{params:[{}],...paramsWinner,p:2}})")]
#[case("css({props:data,_is:{params:[{}],p:9},[key]:{params:['a'],p:2}})")]
#[serial]
fn coverage_contract_preflight_defers_overridden_bad_metadata(
    #[case] expression: &str,
    #[values(false, true)] single_css: bool,
) -> Result<(), String> {
    // Given: the runtime winner differs from the overwritten invalid metadata.
    let _state = crate::test_state::TestStateGuard::new();
    let expected = compile("export const card=css({_is:{params:['a'],p:2}});")?;
    let source = format!(
        "import{{css}}from'@devup-ui/react';let data=0;for(let i=0;i<16;i++)data={{payload:data}};const winner={{_is:{{params:['a'],p:2}}}};const oldParams={{params:[{{}}]}};const paramsWinner={{params:['a']}};const key='_is';export const card={expression};"
    );
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    // When
    let output = extract_without_source_map(
        "/winning-params.css.ts",
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    )
    .map_err(|error| error.to_string())?;
    // Then
    assert_eq!(output.styles, expected.styles);
    Ok(())
}

#[rstest::rstest]
#[case("css`${{_is:{params:[],p:2}}}`", false)]
#[case("css`color:${{_is:{params:[],p:2}}};`", true)]
fn coverage_contract_authored_preflight_only_owns_template_statements(
    #[case] expression: &str,
    #[case] accepted: bool,
) -> Result<(), String> {
    // Given: identical object data placed at the two real interpolation boundaries.
    let source = format!("import{{css}}from'@devup-ui/react';export const card={expression};");
    // When
    let result =
        crate::dead_properties::instrument(&source, "/template-params.css.ts", "@devup-ui/react");
    // Then
    if accepted {
        result.map_err(|error| format!("value data received a params diagnostic: {error}"))?;
    } else {
        let error = result.err().ok_or("statement empty params accepted")?;
        let column = source.find("[]").ok_or("missing authored empty list")? + 1;
        assert!(
            error.starts_with(&format!("/template-params.css.ts:1:{column}:")),
            "{error}"
        );
        assert!(
            error.contains(crate::extractor::selector_params::SELECTOR_PARAMS),
            "{error}"
        );
    }
    Ok(())
}

#[rstest::rstest]
#[serial]
fn coverage_contract_computed_invalid_selector_keeps_dead_key_ownership(
    #[values(false, true)] single_css: bool,
) -> Result<(), String> {
    // Given: authored metadata cannot see through the evaluated selector-map call.
    let _state = crate::test_state::TestStateGuard::new();
    let rule = "export const card=css({props:data,...rules()});";
    let source = format!(
        "import{{css}}from'@devup-ui/react';\nlet data=0;for(let i=0;i<16;i++)data={{payload:data}};function rules(){{return {{selectors:{{_invalidSelector:{{boxAlign:'center'}}}}}}}}\n{rule}"
    );
    let column = rule.find("css(").ok_or("missing real styling call")? + 1;
    // When
    let error = extract_without_source_map(
        "/dead-selector-params.css.ts",
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    )
    .err()
    .ok_or("dead selector-map declaration accepted")?
    .to_string();
    // Then
    assert!(
        error.starts_with(&format!("/dead-selector-params.css.ts:3:{column}:")),
        "{error}"
    );
    assert!(error.contains("unprefixed box-align"), "{error}");
    assert!(!error.contains(crate::utils::SELECTOR_NAME), "{error}");
    Ok(())
}

#[test]
fn nonparams_matrix_keeps_exact_legacy_materialization() -> Result<(), String> {
    let contexts = [
        (
            "/matrix.tsx",
            "export const view=<Box p={values()}/>;",
            false,
        ),
        (
            "/matrix.tsx",
            "export const view=jsx(Box,{p:values()});",
            false,
        ),
        ("/matrix.tsx", "export const card=css({p:values()});", true),
        (
            "/matrix.tsx",
            "export const Card=styled('div',{p:values()});",
            false,
        ),
        (
            "/matrix.tsx",
            "export const animation=keyframes({from:{p:values()}});",
            true,
        ),
        (
            "/matrix.css.ts",
            "export const card=css({p:values()});",
            true,
        ),
        (
            "/matrix.css.ts",
            concat!(
                "export const value=values();export const card=css({",
                "p:2});"
            ),
            false,
        ),
    ];
    for (filename, body, reaches) in contexts {
        for middle in ["", "undefined", "null"] {
            for single_css in [false, true] {
                // Given: independent main normalization for each of the 42 nonparams rows.
                let source = format!(
                    "import{{Box,css,styled,keyframes}}from'@devup-ui/react';import{{jsx}}from'react/jsx-runtime';function values(){{return [1,{middle},3]}}{body}"
                );
                let expected = reaches.then(|| source.replace("p:values()", "p:[1,null,3]"));
                // When
                let actual = crate::build_time_values::evaluate_located(
                    &source,
                    filename,
                    &ExtractOption {
                        single_css,
                        ..ExtractOption::default()
                    },
                    None,
                    &crate::imported_constants::Unknown::default(),
                )
                .map_err(|error| format!("{error:?}"))?;
                // Then
                assert_eq!(
                    actual.map(|value| value.0),
                    expected,
                    "{filename}/{body}/{middle}/{single_css}"
                );
            }
        }
    }
    Ok(())
}

#[rstest::rstest]
#[case(
    "_nthChild",
    "get params(){reads++;return ['2n']}",
    "&:nth-child(2n):hover",
    1
)]
#[case("_host", "set params(value){reads++}", "&:host:hover", 0)]
#[case("_nthChild", "params:[...values()]", "&:nth-child(2n):hover", 1)]
#[serial]
fn coverage_contract_partial_authored_params_use_native_values_once(
    #[case] selector: &str,
    #[case] metadata: &str,
    #[case] expected_selector: &str,
    #[case] reads: usize,
    #[values(false, true)] single_css: bool,
) -> Result<(), String> {
    // Given: valid native metadata sits beside an independently valid sibling.
    let _state = crate::test_state::TestStateGuard::new();
    let expected = compile(&format!(
        "export const card=css({{color:'red',selectors:{{'{expected_selector}':{{p:2}}}}}});"
    ))?;
    let source = format!(
        "import{{css}}from'@devup-ui/react';let reads=0;function values(){{reads++;return ['2n']}}let data=0;for(let i=0;i<16;i++)data={{payload:data}};export const card=css({{props:data,color:'red',{selector}:{{{metadata},_hover:{{params:[],p:2}}}}}});export const count=reads;"
    );
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    // When
    let actual = extract_without_source_map(
        "/partial-params.css.ts",
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    )
    .map_err(|error| error.to_string())?;
    // Then
    assert_eq!(actual.styles, expected.styles);
    assert!(
        actual
            .code
            .contains(&format!("export const count = {reads};")),
        "{}",
        actual.code
    );
    Ok(())
}

#[rstest::rstest]
#[case("get params(){return ['2n']},_is:{params:['a',{}],p:2}", "{}")]
#[case("set params(value){},_is:{params:['a',{}],p:2}", "{}")]
#[case("params:[...values(),{}],p:2", "{}")]
#[case("get params(){return ['2n']},_nthChild:{params:[],p:2}", "[]")]
#[case("params:[...values()],_nthChild:{params:[],p:2}", "[]")]
fn coverage_contract_partial_params_preserve_known_leaf_and_required_errors(
    #[case] rules: &str,
    #[case] witness: &str,
) -> Result<(), String> {
    // Given: uncertain parent params cannot hide a known bad leaf or a fresh required target.
    let source = format!(
        "import{{css}}from'@devup-ui/react';function values(){{return ['2n']}}export const card=css({{color:'red',_nthChild:{{{rules}}}}});"
    );
    let offset = source
        .rfind(witness)
        .ok_or("missing authored refusal witness")?;
    let location = crate::locate("/partial-errors.css.ts", &source, offset);
    // When
    let error =
        crate::dead_properties::instrument(&source, "/partial-errors.css.ts", "@devup-ui/react")
            .err()
            .ok_or("known bad params were deferred")?;
    // Then
    assert_eq!(
        error,
        format!(
            "{location}: {}",
            crate::utils::build_time_error(
                "css",
                witness,
                crate::extractor::selector_params::SELECTOR_PARAMS
            )
        )
    );
    Ok(())
}
