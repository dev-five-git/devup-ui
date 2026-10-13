use super::compile;
use super::literal_w38m_support::{fixture, located};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("styleOrder:p=>2", "p=>2")]
#[case("styleOrder:function(p){return 2}", "function(p)")]
#[case("_hover:`style-order:${async p=>{return;}};color:blue`", "async")]
#[case(
    "_hover:`style-order:${function*(p){return;}};color:blue`",
    "function*"
)]
#[case(
    "_hover:`style-order:${p=>{while(p.stop){}return;}};color:blue`",
    "p=>"
)]
#[case(
    "_hover:`style-order:${p=>{try{return;}finally{trace.push('tail')}}};color:blue`",
    "p=>"
)]
#[serial]
fn authored_or_unsupported_order_when_root_has_real_spread_retains_located_rejection(
    #[case] property: &str,
    #[case] token: &str,
) {
    // Given: a source spread is not proof that an authored function is generated metadata.
    let source = fixture(&format!("...rest,{property}"), "");
    let location = located(&source, token);
    // When: public extraction validates the actual authored shape.
    let error = compile(&source)
        .err()
        .unwrap_or_else(|| panic!("unsupported order compiled"));
    // Then: the original located style-order rejection stays in force.
    assert!(error.starts_with(&location), "{error}");
    assert!(error.contains("styleOrder"), "{error}");
}

#[rstest]
#[case("", "unknown")]
#[case(
    "const unknown={get color(){trace.push('read');return 'red'}};",
    "unknown"
)]
#[case("let unknown={color:'red'};unknown.color='blue';", "unknown")]
#[serial]
fn ineligible_spread_when_source_is_unknown_accessor_or_mutated_retains_rejection(
    #[case] setup: &str,
    #[case] operand: &str,
) {
    // Given: these operands lack the known immutable data-record contract.
    let source = fixture(
        &format!("...{operand},_hover:`style-order:${{p=>{{return;}}}};color:blue`"),
        "",
    )
    .replace("const rest=", &format!("{setup}const rest="));
    // When: the unchanged source eligibility policy sees the real spread operand.
    let error = compile(&source)
        .err()
        .unwrap_or_else(|| panic!("ineligible spread compiled"));
    // Then: no source-capture integration silently loosens that policy.
    assert!(error.starts_with("a.tsx:1:"), "{error}");
    assert!(error.contains("cannot use"), "{error}");
}
