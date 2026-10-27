use std::error::Error;

use rstest::rstest;
use serde_json::json;

use super::tests::facts;

#[test]
fn retains_order_when_import_only_requests_repeat() -> Result<(), Box<dyn Error>> {
    // Given: only ordinary empty-specifier imports, including a duplicate.
    let code = "import 'b'; import {} from 'a'; import 'b';";
    // When: observation accompanies otherwise empty public facts.
    let result = facts(code, true)?;
    // Then: requests have first-insertion order and duplicates are removed.
    assert_eq!(
        result,
        json!({
            "references":[], "exports":[], "stars":[],
            "observation": {"defaultCallable":false,"unsupportedRequests":[],"importOnlyRequests":["b","a"]},
        })
    );
    Ok(())
}

#[rstest]
#[case("")]
#[case("'use client'; import 'a';")]
#[case("import 'a'; ;")]
#[case("import value from 'a';")]
#[case("import 'a' with {type:'json'};")]
#[case("import 'a' assert {type:'json'};")]
fn omits_import_only_when_program_has_other_demand(
    #[case] code: &str,
) -> Result<(), Box<dyn Error>> {
    // Given: a program that cannot prove ordinary import-only demand.
    // When: observation examines all statements and directives.
    let result = facts(code, true)?;
    // Then: the optional erasure proof is absent, rather than an empty set.
    assert!(result["observation"].get("importOnlyRequests").is_none());
    Ok(())
}

#[test]
fn retains_unsupported_order_when_attributes_occur_on_imports_and_exports()
-> Result<(), Box<dyn Error>> {
    // Given: import, named-forwarding and star attributes with repeated sources.
    let code = "import 'b' with {type:'json'}; export {default as x} from 'a' with {type:'json'}; export * from 'b' with {type:'json'};";
    // When: observation collects unsupported request shapes.
    let result = facts(code, true)?;
    // Then: ordered-set semantics include export attributes and deduplicate b.
    assert_eq!(
        result["observation"]["unsupportedRequests"],
        json!(["b", "a"])
    );
    Ok(())
}

#[test]
fn omits_observation_when_not_requested() -> Result<(), Box<dyn Error>> {
    // Given: a source whose callable proof would otherwise be observable.
    // When: facts are requested without observation.
    let result = facts("export default ()=>0", false)?;
    // Then: only the established facts wire fields are present.
    assert_eq!(
        result,
        json!({"references":[],"exports":[["default",null]],"stars":[]})
    );
    Ok(())
}
