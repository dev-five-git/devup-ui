use super::exact_tests::{extracted, static_values};
use rstest::rstest;

#[rstest]
#[case("export const x=<Box p={watch(made)}/>;")]
#[case("export const x=<Box p={made.custom()}/>;")]
#[case("export const x=<Box p={watch({made})}/>;")]
#[case("setTheme(made);")]
#[case("theme(made);")]
#[case("Devup.setTheme(made);")]
#[case("const themeAlias=theme;themeAlias(made);")]
#[case("const themeAlias=Devup.setTheme;themeAlias(made);")]
#[case("Devup['setTheme'](made);")]
#[case("Devup.unknown(made);")]
#[case("eval('made.p=2');")]
#[serial_test::serial]
fn hidden_runtime_escape_when_followed_by_build_only_read_is_rejected(#[case] hazard: &str) {
    // Given
    let source = format!(
        "import {{Box,css,setTheme,setTheme as theme}} from '@devup-ui/react';import * as Devup from '@devup-ui/react';const made={{p:1,toString(){{this.p=2;return 'dark'}},custom(){{this.p=2;return 2}}}};{hazard}css({{p:made.p}});"
    );
    // When
    let result = extracted(&source, "");
    // Then
    assert!(result.is_err(), "{hazard}: {result:?}");
}

#[test]
#[serial_test::serial]
fn hidden_runtime_escape_when_element_reads_later_preserves_css_variable() {
    // Given
    let source = r"import {Box} from '@devup-ui/react';const made={ p: 1 };export const first=<Box p={watch(made)}/>;export const second=<Box p={made.p}/>;";
    // When
    let output = extracted(source, "").unwrap_or_else(|error| panic!("{error}"));
    // Then
    assert_eq!(static_values(&output), Vec::<String>::new());
    assert!(
        output.code.contains("made.p")
            && output.code.contains("watch(made)")
            && output.code.contains("--"),
        "{}",
        output.code
    );
}

#[rstest]
#[case("const alias=Object.freeze(made);watch(alias);")]
#[case("const alias=Object['freeze'](made);const second=alias;watch(second);")]
#[case("const alias=Object.freeze(made);export const x=<Box p={watch(alias)}/>;")]
#[serial_test::serial]
fn freeze_alias_when_nested_child_escapes_does_not_protect_child(#[case] hazard: &str) {
    // Given
    let source = format!(
        "import {{Box,css}} from '@devup-ui/react';const made={{p:1,child:{{p:1}}}};{hazard}css({{p:made.child.p}});"
    );
    // When
    let result = extracted(&source, "");
    // Then
    assert!(result.is_err(), "{hazard}: {result:?}");
}

#[test]
#[serial_test::serial]
fn freeze_alias_when_only_own_scalar_is_read_keeps_exact_value() {
    // Given
    let source = r"import {css} from '@devup-ui/react';const made={ p: 1,child:{ p: 1 }};const alias=Object.freeze(made);watch(alias);css({ p: made.p });";
    // When
    let output = extracted(source, "").unwrap_or_else(|error| panic!("{error}"));
    // Then
    assert_eq!(static_values(&output), vec!["4px".to_string()]);
}

#[rstest]
#[case("const read=x=>x.p;")]
#[case("function read(x){return x.p}")]
#[case("const read=function(x){return x.p};")]
#[case("const read=x=>x.p*2;")]
#[serial_test::serial]
fn local_helper_when_proven_scalar_readonly_preserves_later_exact_read(#[case] helper: &str) {
    // Given
    let source = format!(
        "import {{Box,css}} from '@devup-ui/react';const made={{p:1}};{helper}export const x=<Box p={{read(made)}}/>;css({{p:made.p}});"
    );
    // When
    let output = extracted(&source, "").unwrap_or_else(|error| panic!("{helper}: {error}"));
    // Then
    assert!(
        static_values(&output).contains(&"4px".to_string()),
        "{output:?}"
    );
}

#[rstest]
#[case("const read=x=>(x.p=2);", "read(made)")]
#[case("const read=x=>{x.p=2;return x.p};", "read(made)")]
#[case("const read=x=>watch(x);", "read(made)")]
#[case("const read=x=>x;", "read(made)")]
#[case(r"const read=x=>({ child: x });", "read(made)")]
#[case("const read=x=>eval('x.p=2');", "read(made)")]
#[case("const read=(x,cb)=>cb(x);", "read(made,watch)")]
#[case("const read=({p=watch(made)})=>p;", "read(made)")]
#[case("let read=x=>x.p;read=watch;", "read(made)")]
#[serial_test::serial]
fn local_helper_when_summary_cannot_prove_readonly_blocks_folding(
    #[case] helper: &str,
    #[case] call: &str,
) {
    // Given
    let source = format!(
        "import {{Box,css}} from '@devup-ui/react';const made={{p:1}};{helper}export const x=<Box p={{{call}}}/>;css({{p:made.p}});"
    );
    // When
    let result = extracted(&source, "");
    // Then
    assert!(result.is_err(), "{helper}: {result:?}");
}

#[rstest]
#[case("css(made);", "import {css} from '@devup-ui/react';")]
#[case("compile(made);", "import {css as compile} from '@devup-ui/react';")]
#[case("Devup.css(made);", "import * as Devup from '@devup-ui/react';")]
#[case(
    "const compile=css;compile(made);",
    "import {css} from '@devup-ui/react';"
)]
#[case(
    "const compile=Devup.css;compile(made);",
    "import * as Devup from '@devup-ui/react';"
)]
#[case(
    "const first=css;const compile=first;compile(made);",
    "import {css} from '@devup-ui/react';"
)]
#[case(
    "const ui=Devup;ui.css(made);",
    "import * as Devup from '@devup-ui/react';"
)]
#[serial_test::serial]
fn compiled_api_when_lexically_identified_does_not_escape_its_input(
    #[case] consumer: &str,
    #[case] import: &str,
) {
    // Given
    let source = format!(
        "{import}import {{css as last}} from '@devup-ui/react';const made={{p:1}};{consumer}last({{p:made.p}});"
    );
    // When
    let output = extracted(&source, "").unwrap_or_else(|error| panic!("{consumer}: {error}"));
    // Then
    assert_eq!(static_values(&output), vec!["4px".to_string()]);
}
