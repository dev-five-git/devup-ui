use super::*;
use css::{class_map::reset_class_map, file_map::reset_file_map};
use serial_test::serial;
mod mapping {
    include!("emotion_namespace_tests/mapping.rs");
}
mod diagnostics {
    include!("emotion_namespace_tests/diagnostics.rs");
}
mod exports {
    include!("emotion_namespace_tests/exports.rs");
}
mod order {
    include!("emotion_namespace_tests/order.rs");
}
mod acquisition {
    include!("emotion_namespace_tests/acquisition.rs");
}
mod loaders {
    include!("emotion_namespace_tests/loaders.rs");
}

fn compile(code: &str) -> Result<ExtractOutput, String> {
    reset_class_map();
    reset_file_map();
    extract(
        "namespace.tsx",
        code,
        ExtractOption {
            import_aliases: HashMap::from([("@emotion/css".into(), ImportAlias::NamedToNamed)]),
            ..ExtractOption::default()
        },
    )
    .map_err(|error| error.to_string())
}

#[test]
#[serial]
fn namespace_members_compile_when_called() {
    let code = "import * as E from '@emotion/css';\nconst a = E.css({ padding: 8, opacity: 0.5 }); const b = E['css']`color:red;`; export const c = E.cx(a, { selected: 8 }); export const d = E.merge('external'); export const k = E.keyframes({from:{opacity:0}}); E.injectGlobal`body{margin:0}`;";
    let output = compile(code).unwrap();
    assert!(!output.code.contains("@emotion/css"), "{}", output.code);
    assert!(!output.code.contains("E."), "{}", output.code);
    let styles = format!("{:?}", output.styles);
    assert!(styles.contains("8px"), "{styles}");
    assert!(styles.contains(".5"), "{styles}");
    assert!(!styles.contains("selected"), "{styles}");
}

#[test]
#[serial]
fn namespace_destructure_has_named_import_semantics() {
    let named = compile("import {css,cx} from '@emotion/css'; const a=css({padding:8,opacity:0.5}); export const b=cx(a,{picked:8});").unwrap();
    let namespace = compile("import * as E from '@emotion/css'; const {css: c, ['cx']: join}=E; const a=c({padding:8,opacity:0.5}); export const b=join(a,{picked:8});").unwrap();
    assert_eq!(named.styles, namespace.styles);
    assert!(
        !namespace.code.contains("@emotion/css"),
        "{}",
        namespace.code
    );
}

#[test]
#[serial]
fn runtime_members_keep_namespace_when_used() {
    let output = compile("import * as E from '@emotion/css'; const {css,flush,cache}=E; export const a=css({padding:8}); flush(); console.log(cache,E.sheet,E.hydrate,E.getRegisteredStyles);").unwrap();
    assert!(output.code.contains("@emotion/css"), "{}", output.code);
    assert!(output.code.contains("E.sheet"), "{}", output.code);
    assert!(!output.code.contains("css("), "{}", output.code);
}

#[test]
#[serial]
fn constant_aliases_follow_symbols_when_scopes_shadow_names() {
    let output = compile(r"import * as E from '@emotion/css'; const __emotion_css_0=1; const key='css', N=E, c=N[key]; function f(E){return E.css({ padding: 3 })} function g(){const {css: c}=N; return c({ padding: 8 })} export const a=c({ opacity: 0.5 });").unwrap();
    assert!(!output.code.contains("@emotion/css"), "{}", output.code);
    assert!(output.code.contains("E.css"), "{}", output.code);
    assert!(format!("{:?}", output.styles).contains("8px"));
}

#[test]
#[serial]
fn unsafe_namespace_reads_report_original_locations() {
    for body in [
        "E[key]({color:'red'});",
        "consume(E);",
        "consume(E.css);",
        "E.css?.({});",
        "E.css=fn;",
        "const {...rest}=E;",
        "const {css=fn}=E;",
        "const {css:{call}}=E;",
        "export const {css}=E;",
        "export {E};",
    ] {
        let error = compile(&format!("import * as E from '@emotion/css';\n{body}")).unwrap_err();
        assert!(error.starts_with("namespace.tsx:2:"), "{body}\n{error}");
        assert!(error.contains("cannot use"), "{error}");
        assert!(!error.contains("__emotion_"), "{error}");
    }
}

#[test]
#[serial]
fn default_import_reports_error_when_value_imported() {
    for import in [
        "import E from '@emotion/css';",
        "import E, * as N from '@emotion/css';",
        "import {default as E} from '@emotion/css';",
    ] {
        let error = compile(import).unwrap_err();
        assert!(error.starts_with("namespace.tsx:1:"), "{error}");
        assert!(error.contains("default"), "{error}");
    }
}

#[test]
#[serial]
fn disabled_alias_preserves_namespace_source() {
    let code = r"import * as E from '@emotion/css'; E.css({ padding: 8 });";
    let output = extract("namespace.tsx", code, ExtractOption::default()).unwrap();
    assert_eq!(output.code, code);
}

#[test]
#[serial]
fn style_error_keeps_original_position_after_normalization() {
    let error =
        compile("import * as E from '@emotion/css';\nexport const a=E.css({color: unknown});")
            .unwrap_err();
    assert!(error.contains("namespace.tsx:2:16:"), "{error}");
    assert!(error.contains("unknown"), "{error}");
}

#[test]
#[serial]
fn computed_values_compile_through_boa_when_namespace_normalized() {
    let output = compile("import * as E from '@emotion/css'; function double(n){return n*2} export const a=E.css({padding:double(4)});").unwrap();
    assert!(
        format!("{:?}", output.styles).contains("8px"),
        "{:?}",
        output.styles
    );
    assert!(!output.code.contains("@emotion/css"), "{}", output.code);
}

#[test]
#[serial]
fn constants_keep_emotion_units_when_inlined() {
    let output = compile("import * as E from '@emotion/css'; const n=2; export const a=E.css({padding:n*4,opacity:n/4});").unwrap();
    let styles = format!("{:?}", output.styles);
    assert!(styles.contains("8px"), "{styles}");
    assert!(styles.contains(".5"), "{styles}");
}

#[test]
#[serial]
fn bracket_members_compile_when_all_keys_are_static() {
    let output = compile(r"import * as E from '@emotion/css'; const key='css', k2=key; const a=E[k2]({ margin: 3 }); const b=E[`css`]`color:red`; export const c=E['cx'](a,b); export const d=E['merge']('a b'); export const k=E['keyframes']`from{ opacity: 0 }to{ opacity: 1 }`; E['injectGlobal']({body:{ margin: 2 }});").unwrap();
    assert!(!output.code.contains("@emotion/css"), "{}", output.code);
    assert!(format!("{:?}", output.styles).contains("3px"));
}

#[test]
#[serial]
fn runtime_alias_chains_keep_source_bindings_when_destructured() {
    let output = compile("import * as E from '@emotion/css'; const N=E, A=N; const {css,flush}=A; export const a=css({padding:8}); flush();").unwrap();
    assert!(output.code.contains("N = E"), "{}", output.code);
    assert!(output.code.contains("A = N"), "{}", output.code);
    assert!(output.code.contains("A[\"flush\"]"), "{}", output.code);
}

#[test]
#[serial]
fn destructuring_in_for_initializer_stays_valid_when_lowered() {
    let output = compile(r"import * as E from '@emotion/css'; for(const {css}=E, other=1; other<2;){console.log(css({ padding: 8 }));break}").unwrap();
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &output.code, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{}", output.code);
    assert!(!output.code.contains("@emotion/css"), "{}", output.code);
}

#[test]
#[serial]
fn unsupported_patterns_and_reexports_fail_when_alias_enabled() {
    for body in [
        "let N=E; N.css({});",
        "const [css]=E;",
        "const {[key]:css}=E;",
        "const {unknown}=E;",
        "E.unknown({});",
        "E?.css({});",
        "const c=E.css; export {c};",
        "const c=E.css; consume(c);",
        "const N=E; N={};",
        "const key='css'; key='cx'; E[key]({});",
    ] {
        let error = compile(&format!("import * as E from '@emotion/css';\n{body}")).unwrap_err();
        assert!(error.starts_with("namespace.tsx:2:"), "{body}\n{error}");
    }
    for code in [
        "export * from '@emotion/css';",
        "export * as E from '@emotion/css';",
        "export {css} from '@emotion/css';",
        "export {default} from '@emotion/css';",
    ] {
        assert!(compile(code).unwrap_err().starts_with("namespace.tsx:1:"));
    }
}

#[test]
#[serial]
fn tagged_class_error_mentions_original_expression_when_normalized() {
    let error =
        compile("import * as E from '@emotion/css';\nexport const a=E.cx`a b`;").unwrap_err();
    assert!(error.contains("namespace.tsx:2:16:"), "{error}");
    assert!(error.contains("E.cx`a b`"), "{error}");
    assert!(!error.contains("__emotion_"), "{error}");
}

#[test]
#[serial]
fn type_only_reads_do_not_trigger_escape_errors() {
    let output = compile(r"import type Default from '@emotion/css'; import * as E from '@emotion/css'; type Api=typeof E.css; const a=E.css({ padding: 8 });").unwrap();
    assert!(format!("{:?}", output.styles).contains("8px"));
}

#[test]
#[serial]
fn unused_macros_drop_emotion_when_no_runtime_reads_remain() {
    let output = compile("import * as E from '@emotion/css'; const {css}=E; const N=E;").unwrap();
    assert!(!output.code.contains("@emotion/css"), "{}", output.code);
}

#[test]
#[serial]
fn generated_import_skips_existing_names_when_name_collides() {
    let output = compile(r"import * as E from '@emotion/css'; const __emotion_css_3=7; export const a=E.css({ padding: 8 }); console.log(__emotion_css_3);").unwrap();
    assert!(
        output.code.contains("console.log(__emotion_css_3)"),
        "{}",
        output.code
    );
    assert!(format!("{:?}", output.styles).contains("8px"));
    assert!(!output.code.contains("@emotion/css"), "{}", output.code);
}

#[test]
#[serial]
fn type_only_and_runtime_reexports_remain_when_not_styling() {
    for code in [
        "export type * from '@emotion/css';",
        "export type {css} from '@emotion/css';",
        "export {flush} from '@emotion/css';",
        "import {type css} from '@emotion/css';",
        "import * as E from '@emotion/css'; console.log(E.cache);",
        "import {flush} from '@emotion/css'; flush();",
    ] {
        assert!(compile(code).is_ok(), "{code}");
    }
}
