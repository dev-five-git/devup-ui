use super::w38r_u2_fallback::inventory;
use super::*;
use rstest::rstest;

#[rstest]
#[case::plain(".attrs({title:'first',id:'kept'}).attrs({title:'last'})", "last", "kept", "color-0-red--255-a external caller", serde_json::json!({"padding":1}))]
#[case::computed(".attrs({['title']:'computed',className:'attrs'})", "computed", "caller-id", "color-0-red--255-a external attrs caller", serde_json::json!({"padding":1}))]
#[case::spread(".attrs({...{title:'spread'},className:'attrs'})", "spread", "caller-id", "color-0-red--255-a external attrs caller", serde_json::json!({"padding":1}))]
#[case::object_merge(".attrs({title:'object',className:'attrs',style:{padding:2,color:'blue'}})", "object", "caller-id", "color-0-red--255-a external attrs caller", serde_json::json!({"padding":2,"color":"blue"}))]
#[case::arrow(concat!(".attrs(p=>({'title':", "p.title+'-arrow',className:'attrs',style:", "{padding:2}}))"), "caller-arrow", "caller-id", "color-0-red--255-a external attrs caller", serde_json::json!({"padding":2}))]
#[case::function(concat!(".attrs(function(p){return {'title':", "p.title+'-function',className:'attrs',style:", "{padding:3}}})"), "caller-function", "caller-id", "color-0-red--255-a external attrs caller", serde_json::json!({"padding":3}))]
#[serial]
fn w38r_u2_attrs_when_tag_preparation_rolls_back_merge_original_props(
    #[case] attrs: &str,
    #[case] title: &str,
    #[case] id: &str,
    #[case] classes: &str,
    #[case] style: serde_json::Value,
) {
    // Given: nested statement leaves the tag intact and restores un-captured attrs.
    let source = format!(
        "import {{styled}} from '@devup-ui/react';const __devupForwardRef=r=>r;let external='external';const Card=styled.div{attrs}`color:red;&:hover{{${{external}};}}`;const a=Card({{title:'caller',id:'caller-id',className:'caller',style:{{padding:1}}}},null);"
    );
    // When: compile and render once through the actual attrs wrapper.
    let actual = output(&source);
    let evaluated = whole::evaluate_code(
        &actual.code,
        "[a.props.title,a.props.id,a.props.className,a.props.style]",
    );
    // Then: later plain props win; attrs classes precede caller; styles overlay.
    assert_eq!(
        evaluated.element,
        serde_json::json!([title, id, classes, style])
    );
    inventory(&actual, &[("color", "red", 0, None)]);
}

#[rstest]
#[case::arrow(
    "p=>{if(p.active)return css({color:'blue',styleOrder:7});return 2}",
    "(p) => { if (p.active) return [\"0\"]; return [\"1\"]; }",
    60
)]
#[case::function(
    "function(p){if(p.active)return css({color:'blue',styleOrder:7});return 2}",
    "(function(p) { if (p.active) return [\"0\"]; return [\"1\"]; })",
    68
)]
#[serial]
fn w38r_u2_order_return_when_provenance_is_mixed_keeps_producer_distinct_from_metadata(
    #[case] callback: &str,
    #[case] rendered_callback: &str,
    #[case] producer_column: u32,
) {
    // Given: a class-producing return must never be reinterpreted as an order number.
    let source = format!(
        "import {{styled,css}} from '@devup-ui/react';\nconst Card=styled.div`style-order:${{{callback}}};color:red;`;"
    );
    // When: compile both return paths of the literal order callback.
    let actual = error(&source);
    // Then: producer classes are invalid explicit metadata, located at their source.
    let requirement = "an explicit styleOrder must be an ECMAScript Number integer from 1 to 254 or canonical decimal string without signs, spaces or leading zeros";
    assert_eq!(
        actual,
        format!(
            "a.tsx:2:37: `styleOrder()` cannot use `{rendered_callback}` at build time: {requirement}\na.tsx:2:{producer_column}: `styleOrder()` cannot use `\"color-0-blue--7-a\"` at build time: {requirement}"
        )
    );
}
