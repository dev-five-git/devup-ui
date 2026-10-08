use super::*;

#[test]
#[serial]
fn local_styling_binding_compiles_when_function_is_exported() {
    let code = "import * as E from '@emotion/css'; export function scoped(){const {css:make}=E; return make({padding:8});}";
    let output = compile(code).unwrap();
    assert!(format!("{:?}", output.styles).contains("8px"));
    assert!(!output.code.contains("@emotion/css"), "{}", output.code);
    assert!(!output.code.contains("make("), "{}", output.code);
}

#[test]
#[serial]
fn local_styling_binding_compiles_when_class_is_exported() {
    let code = "import * as E from '@emotion/css'; export class Scoped{method(){const {css:make}=E; return make({padding:8});}}";
    let output = compile(code).unwrap();
    assert!(format!("{:?}", output.styles).contains("8px"));
    assert!(!output.code.contains("@emotion/css"), "{}", output.code);
    assert!(!output.code.contains("make("), "{}", output.code);
}

#[test]
#[serial]
fn styling_binding_errors_with_single_quotes_when_directly_exported() {
    let code = "import * as E from '@emotion/css';\nexport const {css}=E;";
    let error = compile(code).unwrap_err();
    assert_eq!(
        error,
        "namespace.tsx:2:14: `@emotion/css()` cannot use `{css}=E` at build time: namespace and styling function bindings cannot be exported"
    );
}
