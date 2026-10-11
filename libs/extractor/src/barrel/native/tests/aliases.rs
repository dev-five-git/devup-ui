use rstest::rstest;

use super::{PACKAGE, ResolvedModule, Terminal, TestResult, walker};
use crate::barrel::{Link, analyze_mode};

#[rstest]
#[case("const key='style';const make=ve[key];")]
#[case("const {style:make=()=>{throw new Error('fallback executed')}}=ve;")]
#[case("const key='style';const {[key]:make=()=>0}=ve;")]
fn exported_alias_retains_package_identity_when_its_member_is_exact(#[case] alias: &str) {
    // Given
    let module = ResolvedModule {
        path: "/aliases.ts".into(),
        code: format!("import * as ve from '{PACKAGE}';{alias}export {{make}};"),
    };
    let exports = analyze_mode(&module, PACKAGE, true);
    let resolver = |_: &str, _: &str| None;
    let mut walker = walker(&resolver);
    // When
    let terminal = walker.native_origin(&exports, "make", &mut Vec::new());
    // Then
    assert!(
        matches!(terminal, Terminal::Binding { owner, name, api: Some("style") }
        if owner == PACKAGE && name == "style")
    );
}

#[rstest]
#[case("const [make]=ve;")]
#[case("const key=window.name;const make=ve[key];")]
fn exported_alias_stays_ordinary_when_its_pattern_or_key_has_no_exact_native_member(
    #[case] alias: &str,
) {
    // Given
    let module = ResolvedModule {
        path: "/ordinary-alias.ts".into(),
        code: format!("import * as ve from '{PACKAGE}';{alias}export {{make}};"),
    };
    let exports = analyze_mode(&module, PACKAGE, true);
    let resolver = |_: &str, _: &str| None;
    let mut walker = walker(&resolver);
    // When
    let terminal = walker.native_origin(&exports, "make", &mut Vec::new());
    // Then
    assert!(
        matches!(terminal, Terminal::Binding { owner, name, api: None }
        if owner == module.path && name == "make")
    );
}

#[test]
fn declared_link_has_no_native_terminal_when_it_is_not_an_import_alias() -> TestResult {
    // Given
    let module = ResolvedModule {
        path: "/own.ts".into(),
        code: "export const style=()=>13;".into(),
    };
    let exports = analyze_mode(&module, PACKAGE, true);
    let link = exports.named.get("style").ok_or("source declares style")?;
    assert!(matches!(link, Link::Own));
    let resolver = |_: &str, _: &str| None;
    let mut walker = walker(&resolver);
    // When
    let terminal = walker.native_link(&exports, link, &mut Vec::new());
    // Then
    assert!(matches!(terminal, Terminal::Absent));
    Ok(())
}
