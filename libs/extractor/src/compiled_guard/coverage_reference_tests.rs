use std::error::Error;

use rstest::rstest;
use serde_json::{Value, json};

use super::tests::facts;

const DESTRUCTURED_ALIAS_SOURCE: &str = "let renamed; ({css:renamed}=DU);";

#[rstest]
#[case("const Alias=DU['css']; Alias.child;", json!([["css"], ["css", "child"]]))]
#[case("class C{#css; m(){DU.#css.child;}}", json!([["css", "child"]]))]
#[case("class C{#css; m(){const Alias=DU.#css; Alias.child;}}", json!([["css"], ["css", "child"]]))]
#[case("let css; ({css}=DU);", json!([["css"]]))]
#[case(DESTRUCTURED_ALIAS_SOURCE, json!([["css"]]))]
fn paths_when_members_are_computed_private_or_destructured(
    #[case] body: &str,
    #[case] expected: Value,
) -> Result<(), Box<dyn Error>> {
    // Given: a definite namespace and a member or destructured read of it.
    let code = format!("import * as DU from 'ui'; {body}");
    // When: parser-created member nodes propagate their actual origin.
    let result = facts(&code, false)?;
    // Then: each outer read retains the complete path without inner duplicates.
    let references = result["references"].as_array().ok_or("reference array")?;
    let paths: Vec<_> = references
        .iter()
        .map(|value| value["ids"].clone())
        .collect();
    assert_eq!(json!(paths), expected);
    assert!(references.iter().all(|value| value["request"] == "ui"));
    Ok(())
}

#[rstest]
#[case("true")]
#[case("null")]
#[case("1")]
#[case("1n")]
#[case("/key/")]
fn empty_literal_name_when_computed_key_is_not_a_string(
    #[case] key: &str,
) -> Result<(), Box<dyn Error>> {
    // Given: the established non-string literal naming policy.
    let code = format!("import * as DU from 'ui'; DU[{key}];");
    // When: the real member visitor resolves a non-string literal key.
    let result = facts(&code, false)?;
    // Then: the wire keeps an empty path segment, not a coerced JS value.
    assert_eq!(
        result["references"],
        json!([{"request":"ui","ids":[""],"pos":26}])
    );
    Ok(())
}

#[rstest]
#[case("get().css;")]
#[case("({get}).get().css;")]
#[case("require(get).css;")]
#[case("class C{#css; m(){get().#css;}}")]
fn imported_reads_when_call_cannot_be_a_require_origin(
    #[case] body: &str,
) -> Result<(), Box<dyn Error>> {
    // Given: a call with an ordinary, member or nonliteral require callee shape.
    let code = format!("import {{get}} from 'ui'; {body}");
    // When: rejecting the origin causes traversal of the real call contents.
    let result = facts(&code, false)?;
    // Then: the imported argument/callee is retained, but no css origin is invented.
    let references = result["references"].as_array().ok_or("reference array")?;
    assert_eq!(references.len(), 1);
    assert_eq!(references[0]["request"], json!("ui"));
    assert_eq!(references[0]["ids"], json!(["get"]));
    Ok(())
}

#[rstest]
#[case("const obj={[css()]:0};")]
#[case("class C{[css()](){}}")]
#[case("let local; ({local=css}={});")]
fn references_when_keys_and_shorthand_defaults_are_expressions(
    #[case] body: &str,
) -> Result<(), Box<dyn Error>> {
    // Given: a computed non-Identifier key or a shorthand assignment default.
    let code = format!("import {{css}} from 'ui'; {body}");
    // When: the specialized key and pattern visitors walk expression reads.
    let result = facts(&code, false)?;
    // Then: the expression contributes exactly the imported css read.
    let references = result["references"].as_array().ok_or("reference array")?;
    assert_eq!(references.len(), 1);
    assert_eq!(references[0]["ids"], json!(["css"]));
    Ok(())
}

#[rstest]
#[case("meta", "import.meta;")]
#[case("meta", "function f(){import.meta;}")]
#[case("meta", "{let meta; import.meta;}")]
#[case("meta", "function f(meta){import.meta;}")]
#[case("target", "function f(){new.target;}")]
#[case("target", "function f(target){new.target;}")]
fn references_when_meta_properties_match_imported_local_names(
    #[case] name: &str,
    #[case] body: &str,
    #[values(false, true)] after: bool,
) -> Result<(), Box<dyn Error>> {
    // Given: same-named css imports before or after shadowed/unshadowed meta properties.
    let declaration = format!("import {{css as {name}}} from 'ui';");
    let code = if after {
        format!("{body}{declaration}")
    } else {
        format!("{declaration}{body}")
    };
    // When: the real parser collects facts in both observation modes.
    let results = [facts(&code, false)?, facts(&code, true)?];
    // Then: neither keyword suffix is a lexical reference to the imported binding.
    for result in results {
        assert_eq!(result["references"], json!([]));
    }
    Ok(())
}

#[rstest]
#[case("meta", "import.meta; void meta;")]
#[case("meta", "import.meta[meta];")]
#[case("target", "function f(){new.target; void target;}")]
#[case("target", "function f(){new.target[target];}")]
fn references_when_meta_properties_have_real_binding_reads(
    #[case] name: &str,
    #[case] body: &str,
) -> Result<(), Box<dyn Error>> {
    // Given: an actual identifier or computed expression read next to a meta property.
    let code = format!("import {{css as {name}}} from 'ui';{body}");
    // When: the real parser collects reference facts.
    let result = facts(&code, false)?;
    // Then: only the actual binding read carries css provenance and its own position.
    assert_eq!(
        result["references"],
        json!([{
            "request":"ui", "ids":["css"], "pos":code.rfind(name).ok_or("binding read")?
        }])
    );
    Ok(())
}

#[rstest]
#[case("export class Content{}; export default Content;", false)]
#[case("function Content(){}; ({Content}={}); export default Content;", false)]
#[case(
    "function Content(){}; ({Content=0}={}); export default Content;",
    false
)]
#[case(
    "function Content(){}; function f(Content){({Content}={});} export default Content;",
    true
)]
fn callable_when_class_exports_and_shorthand_writes_have_real_provenance(
    #[case] code: &str,
    #[case] expected: bool,
) -> Result<(), Box<dyn Error>> {
    // Given: class declarations or object shorthand writes to root and shadow bindings.
    // When: observation prepares exports and then collects writes.
    let result = facts(code, true)?;
    // Then: neither a class nor an overwritten root function is certified callable.
    assert_eq!(result["observation"]["defaultCallable"], json!(expected));
    Ok(())
}

#[test]
fn wire_strings_when_requests_and_exports_contain_json_escapes() -> Result<(), Box<dyn Error>> {
    // Given: JavaScript literals containing quotes, slashes and control characters.
    let code = r#"import { 'a"b\\c\n\t\0' as local } from 'u"i\\\r'; export {local as 'out"\\\b'}; local;"#;
    // When: literal UTF-16 data crosses the real facts JSON encoder.
    let result = facts(code, false)?;
    // Then: decoding the wire recovers the original literal values exactly.
    assert_eq!(
        result["exports"],
        json!([["out\"\\\u{8}", {"request":"u\"i\\\r","ids":["a\"b\\c\n\t\0"]}]])
    );
    assert_eq!(result["references"][0]["request"], json!("u\"i\\\r"));
    Ok(())
}
