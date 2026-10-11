use super::inventory;
use super::*;
use rstest::rstest;

fn emotion_styled(source: &str) -> ExtractOutput {
    compile_with(
        source,
        ExtractOption {
            import_aliases: HashMap::from([
                (
                    "@emotion/styled".to_string(),
                    ImportAlias::DefaultToNamed("styled".to_string()),
                ),
                ("@emotion/react".to_string(), ImportAlias::NamedToNamed),
            ]),
            ..ExtractOption::default()
        },
    )
    .required("Emotion styled and css props compile")
}

#[rstest]
#[case::ordered("{styleOrder:2,color:'red'}", "color-0-red--2-a", &[ ("color", "red", 0, Some(2)) ])]
#[case::responsive("{margin:['1px','2px']}", "margin-0-1px--255-a margin-1-2px--255-a", &[ ("margin", "1px", 0, None), ("margin", "2px", 1, None) ])]
#[serial]
fn w38r_u4_unprepared_styled_when_argument_is_literal_spread_retains_order_and_breakpoints(
    #[case] rules: &str,
    #[case] classes: &str,
    #[case] expected: &[(&str, &str, u8, Option<u8>)],
) {
    // Given: a literal spread is directly readable but intentionally not prepared.
    let source = format!(
        "import {{styled}} from '@devup-ui/react';const __devupForwardRef=r=>r;const Card=styled.div(...{rules});const a=Card({{}},null);"
    );
    // When: public extraction builds and renders that fallback definition.
    let actual = output(&source);
    let evaluated = whole::evaluate_code(&actual.code, "a.props.className");
    // Then: fallback metadata applies to every emitted declaration.
    assert_eq!(evaluated.element, serde_json::json!(classes));
    assert_eq!(evaluated.trace, serde_json::json!([]));
    inventory(&actual, expected);
}

#[rstest]
#[case::yes(true, "color-0-red--255-a")]
#[case::no(false, "color-0-blue--255-a")]
#[serial]
fn w38r_u4_css_prop_when_spread_definition_is_responsive_or_conditional_respects_inline_eligibility(
    #[case] active: bool,
    #[case] conditional: &str,
) {
    // Given: no construction captures; responsive props inline, conditions do not.
    let source = format!(
        "import styled from '@emotion/styled';const __devupForwardRef=r=>r;let on={active};const Wide=styled.div(...{{margin:['1px','2px']}});const Toggle=styled.div(...{{color:on?'red':'blue'}});const a=<Wide css={{{{color:'red'}}}}/>;const b=<Toggle css={{{{margin:'3px'}}}}/>;const c=Toggle(b.props,null);"
    );
    // When: each css prop asks the genuine definition whether it can inline.
    let actual = emotion_styled(&source);
    let evaluated = whole::evaluate_code(
        &actual.code,
        "[a.type,a.props.className,b.props.className,c.props.className]",
    );
    // Then: responsive base declarations join css; conditional base renders later.
    assert_eq!(
        evaluated.element,
        serde_json::json!([
            "div",
            "margin-0-1px--255-a margin-1-2px--255-a color-0-red--255-a",
            "margin-0-3px--255-a",
            format!("{conditional} margin-0-3px--255-a")
        ])
    );
    assert_eq!(evaluated.trace, serde_json::json!([]));
    inventory(
        &actual,
        &[
            ("margin", "1px", 0, None),
            ("margin", "2px", 1, None),
            ("margin", "3px", 0, None),
            ("color", "red", 0, None),
            ("color", "blue", 0, None),
        ],
    );
}

#[rstest]
#[case::yes(true, "ext font-size-0-12px--255-a color-0-red--255-a")]
#[case::no(false, "ext font-size-0-12px--255-a color-0-blue--255-a")]
#[serial]
fn w38r_u4_normalised_when_declaration_and_conditional_and_supplied_roots_coexist_preserves_all(
    #[case] active: bool,
    #[case] classes: &str,
) {
    // Given: a typography declaration activates skip traversal of other root kinds.
    let source = "import {ClassNames} from '@emotion/react';const render=(active,external)=><ClassNames>{({css,cx})=>css({fontSize:'12px',color:active?'red':'blue'},external)}</ClassNames>;";
    // When: local rules normalize and generate a real runtime class selection.
    let actual = compile_emotion(source).required("normalized mixed roots compile");
    let evaluated = whole::evaluate_code(&actual.code, &format!("render({active},'ext')"));
    // Then: conditional values remain conditional; supplied class is not discarded.
    assert_eq!(evaluated.element, serde_json::json!(classes));
    assert_eq!(evaluated.trace, serde_json::json!([]));
    inventory(
        &actual,
        &[
            ("font-size", "12px", 0, None),
            ("color", "red", 0, None),
            ("color", "blue", 0, None),
        ],
    );
}

#[rstest]
#[case::yes(true, "color-0-red--255-a")]
#[case::no(false, "color-0-blue--255-a")]
#[serial]
fn w38r_u4_finite_spread_when_condition_selects_arrays_reads_test_once(
    #[case] active: bool,
    #[case] classes: &str,
) {
    // Given: both sides are finite arrays, not opaque iterable runtime values.
    let source = format!(
        "import {{css}} from '@devup-ui/react';const mark=v=>(trace.push('test'),v);const a=css(...(mark({active})?[{{color:'red'}}]:[{{color:'blue'}}]));"
    );
    // When: classify the spread shape, capture the branch, then emit the selection.
    let actual = output(&source);
    let evaluated = whole::evaluate_code(&actual.code, "a");
    // Then: only the selected class is emitted and the condition runs once.
    assert_eq!(evaluated.element, serde_json::json!(classes));
    assert_eq!(evaluated.trace, serde_json::json!(["test"]));
    inventory(
        &actual,
        &[("color", "red", 0, None), ("color", "blue", 0, None)],
    );
}

#[rstest]
#[case::nested(
    "cx(cx('left',css({color:'red'})),'right')",
    "left color-0-red--255-a right"
)]
#[serial]
fn w38r_u4_nested_class_calls_when_preflight_decomposes_arguments_keep_literal_order(
    #[case] expression: &str,
    #[case] classes: &str,
) {
    // Given: nested local helpers must be read before their child calls are replaced.
    let source = format!(
        "import {{ClassNames}} from '@emotion/react';const render=()=> <ClassNames>{{({{css,cx}})=>{expression}}}</ClassNames>;"
    );
    // When: public preflight and local emission process the nested call tree.
    let actual = compile_emotion(&source).required("nested local class helpers compile");
    let evaluated = whole::evaluate_code(&actual.code, "render()");
    // Then: the compiled inner class stays between outer literals without duplication.
    assert_eq!(evaluated.element, serde_json::json!(classes));
    assert_eq!(evaluated.trace, serde_json::json!([]));
    inventory(&actual, &[("color", "red", 0, None)]);
}

#[rstest]
#[case::order(2, "color-0-red--2-a")]
#[case::other_order(7, "color-0-red--7-a")]
#[serial]
fn w38r_u4_css_prop_when_tag_contains_metadata_uses_lowered_literal(
    #[case] order: u8,
    #[case] classes: &str,
) {
    // Given: metadata forces the tagged css prop through the literal lowerer.
    let source = format!(
        "import {{jsx}} from '@emotion/react';import {{css}} from '@devup-ui/react';const a=jsx('div',{{css:css`style-order:{order};color:red;`}});"
    );
    // When: the native element consumes the tag as a css prop before tag visitation.
    let actual = compile_emotion(&source).required("ordered tagged css prop compiles");
    let evaluated = whole::evaluate_code(&actual.code, "a.props.className");
    // Then: metadata is absent from CSS inventory and controls the class order.
    assert_eq!(evaluated.element, serde_json::json!(classes));
    assert_eq!(evaluated.trace, serde_json::json!([]));
    inventory(&actual, &[("color", "red", 0, Some(order))]);
}

#[cfg(test)]
#[path = "w38r_u4_errors.rs"]
mod w38r_u4_errors;

#[cfg(test)]
#[path = "w38r_u4_recovery.rs"]
mod w38r_u4_recovery;
