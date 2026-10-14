use super::*;
use rstest::rstest;

const IMPORT: &str = "import * as E from '@emotion/css';\n";

#[rstest]
#[case(
    "let result; try {result=c({ padding: 8 })}catch{result='tdz'} const c=E.css; export const a=result;",
    "c"
)]
#[case("const N=A; const A=E; export const a=N.css({ padding: 8 });", "A")]
#[case(
    "const early=()=>c({ padding: 8 }); const c=E.css; export const a=early();",
    "c"
)]
#[case(
    "export const a=make(); const { css }=E; function make(){return css({ padding: 8 })}",
    "css"
)]
#[case(
    "switch(1){case 1: const c=E.css; break; case 2: c({ padding: 8 })}",
    "c"
)]
#[case("export const a=E[key]({ padding: 8 }); const key='css';", "key")]
#[case(
    "const c=E[key]; const key='css'; export const a=c({ padding: 8 });",
    "key"
)]
#[case(
    "const {[key]: c}=E; const key='css'; export const a=c({ padding: 8 });",
    "key"
)]
#[serial]
fn reads_before_initialization_report_the_alias_when_elimination_would_hide_tdz(
    #[case] body: &str,
    #[case] alias: &str,
) {
    // Given: the original code reads the alias while it is still uninitialized
    // When
    let error = compile(&format!("{IMPORT}{body}")).unwrap_err();

    // Then
    assert!(error.starts_with("namespace.tsx:2:"), "{error}");
    assert!(error.contains(&format!("cannot use `{alias}`")), "{error}");
    assert!(error.contains("must be initialized"), "{error}");
}

#[rstest]
#[case("const c=E.css; export const a=c({ padding: 8 });")]
#[case("const N=E, A=N; const { css }=A; export const a=css({ padding: 8 });")]
#[case("const { css }=E; export const make=()=>css({ padding: 8 });")]
#[case("const { css }=E; export function make(){return css({ padding: 8 })} export const a=make();")]
#[case("function make(){return css({ padding: 8 })} const { css }=E; export const a=make();")]
#[case("function outer(){const c=E.css; return ()=>c({ padding: 8 })} export const a=outer()();")]
#[case("const { css }=E; function f(n){return n ? f(n-1) : css({ padding: 8 })} export const a=f(1);")]
#[case("const { css }=E; export default function(){return css({ padding: 8 })}")]
#[case("const { css }=E; export class K{ m(){return css({ padding: 8 })} }")]
#[case("switch(1){case 1: const c=E.css; console.log(c({ padding: 8 }));}")]
#[serial]
fn ordered_reads_compile_when_aliases_are_initialized_first(#[case] body: &str) {
    // Given
    let code = format!("{IMPORT}{body}");

    // When
    let output = compile(&code).unwrap();

    // Then
    assert!(format!("{:?}", output.styles).contains("8px"), "{}", output.code);
    assert!(!output.code.contains("@emotion/css"), "{}", output.code);
}

#[test]
#[serial]
fn eliminated_bindings_survive_a_shadowed_undefined() {
    for body in [
        "const c=E.css; const undefined=1; export const a=c({ padding: 8 });",
        "const {css: c}=E; const undefined=1; export const a=c({ padding: 8 });",
    ] {
        // Given: `undefined` is a user binding here
        // When
        let output = compile(&format!("{IMPORT}{body}")).unwrap();

        // Then
        assert!(output.code.contains("void 0"), "{}", output.code);
        assert!(!output.code.contains("c = undefined"), "{}", output.code);
    }
}

#[rstest]
#[case("const c=(E.css);")]
#[case("const c=E.css as typeof E.css;")]
#[case("const c=E.css satisfies typeof E.css;")]
#[case("const c=E.css!;")]
#[case("const c=(E.css as typeof E.css)!;")]
#[case("const c=E.css<number>;")]
#[case("const N=(E) as typeof E, c=N.css;")]
#[serial]
fn aliases_compile_when_wrapped_in_syntax_only_expressions(#[case] alias: &str) {
    // Given
    let code = format!("{IMPORT}{alias} export const a=c({{padding:8}});");

    // When
    let output = compile(&code).unwrap();

    // Then
    assert!(format!("{:?}", output.styles).contains("8px"), "{}", output.code);
    assert!(!output.code.contains("@emotion/css"), "{}", output.code);
}
