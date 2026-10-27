use std::error::Error;

use rstest::rstest;
use serde_json::json;

use super::tests::facts;

#[rstest]
#[case("{var local;} local; css();", 1)]
#[case("function f(){css(); {var css;}}", 0)]
#[case("function f(){css(); {let css;}}", 1)]
#[case("function f(){css(); {const css=0;}}", 1)]
#[case("function f(){function g(){if(0){var css;} css();} css();}", 1)]
#[case("function f(){if(0){var css;} function g(){css();}}", 0)]
#[case("function f(...css){css();} css();", 1)]
#[case("const f=(...css)=>css(); css();", 1)]
#[case("function f(...[css]){css();} css();", 1)]
fn references_when_var_hoists_but_lexical_and_parameter_bindings_stay_local(
    #[case] body: &str,
    #[case] count: usize,
) -> Result<(), Box<dyn Error>> {
    // Given: imported css and nested declarations, including reads before var.
    let code = format!("import {{css}} from 'ui'; {body}");
    // When: the real parser and scope builder collect reference provenance.
    let result = facts(&code, false)?;
    // Then: only reads outside the shadow's owning scope retain the import.
    let references = result["references"].as_array().ok_or("reference array")?;
    assert_eq!(references.len(), count);
    for reference in references {
        assert_eq!(reference["request"], json!("ui"));
        assert_eq!(reference["ids"], json!(["css"]));
    }
    Ok(())
}

#[rstest]
#[case("{var Content;} Content=0;", false)]
#[case("{let Content;} Content=0;", false)]
#[case("function f(){Content=0; {var Content;}}", true)]
#[case("function f(){Content=0; {let Content;}}", false)]
#[case("function f(){function g(){if(0){var Content;} Content=0;}}", true)]
#[case("function f(...Content){Content=0;}", true)]
fn callable_when_writes_resolve_to_root_or_function_hoisted_shadow(
    #[case] body: &str,
    #[case] callable: bool,
) -> Result<(), Box<dyn Error>> {
    // Given: a root function and writes in scopes with differing ownership.
    let code = format!("function Content(){{}} {body} export default Content;");
    // When: observation invalidates callable proofs using actual scope owners.
    let result = facts(&code, true)?;
    // Then: only writes to the root binding invalidate its callable proof.
    assert_eq!(result["observation"]["defaultCallable"], json!(callable));
    Ok(())
}
