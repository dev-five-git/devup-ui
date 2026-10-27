use std::error::Error;

use rstest::rstest;
use serde_json::{Value, json};

use super::compiled_guard_facts_internal;

const REQUIRE_ASSIGNMENT_SOURCE: &str = "let c; ({css:c}=require('ui'))";
const CATCH_SHADOW_SOURCE: &str = "import {css} from 'ui'; try{}catch({message:css}){css()}";

pub(super) fn facts(code: &str, observe: bool) -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::from_str(&compiled_guard_facts_internal(
        code, observe,
    )?)?)
}

#[rstest]
#[case("import {css as c} from 'ui'; c()", false)]
#[case("import * as DU from 'ui'; DU.css()", false)]
#[case("import * as DU from 'ui'; DU['css']()", false)]
#[case("import * as DU from 'ui'; const Alias=(DU); Alias.css()", false)]
#[case("import * as DU from 'ui'; (DU).css()", false)]
#[case("require('ui').css()", true)]
#[case("const DU=require('ui'); const Alias=DU; Alias.css()", true)]
#[case("const {css:c,...rest}=require('ui')", true)]
#[case(REQUIRE_ASSIGNMENT_SOURCE, true)]
fn retains_origins_when_statically_named(
    #[case] code: &str,
    #[case] require: bool,
) -> Result<(), Box<dyn Error>> {
    // Given: a definite imported surface in final JavaScript.
    // When: the source API collects facts.
    let result = facts(code, false)?;
    // Then: the request, export path and require kind are retained.
    let references = result["references"]
        .as_array()
        .ok_or("references must be an array")?;
    assert!(
        references
            .iter()
            .any(|reference| reference["request"] == "ui"
                && reference["ids"] == json!(["css"])
                && reference.get("kind").and_then(Value::as_str)
                    == require.then_some("require-call"))
    );
    Ok(())
}

#[rstest]
#[case("import * as DU from 'ui'; function f(DU){return DU.css()}")]
#[case("import * as DU from 'ui'; { DU.css(); let DU; }")]
#[case("function f(require){return require('ui').css()}")]
#[case("function f(){ require('ui').css(); if(0){var require;} }")]
#[case("const DU=require('ui'); DU[unknown]()")]
#[case("const DU=import('ui'); DU.css()")]
#[case("require('ui', extra).css()")]
#[case("require('').css()")]
#[case("import * as DU from 'ui'; class C { method(){var DU; return DU.css()} }")]
#[case("import * as DU from 'ui'; const C=class DU { method(){return DU.css()} }")]
#[case("import {css} from 'ui'; for(let css of []){css()}")]
#[case(CATCH_SHADOW_SOURCE)]
fn rejects_origins_when_opaque_or_predeclared_shadowed(
    #[case] code: &str,
) -> Result<(), Box<dyn Error>> {
    // Given: a source with no definite imported export read.
    // When: scopes and origins are collected.
    let result = facts(code, false)?;
    // Then: no reference is manufactured.
    assert_eq!(result["references"], json!([]));
    Ok(())
}

#[test]
fn preserves_order_when_aliases_are_declared_together() -> Result<(), Box<dyn Error>> {
    // Given: const alias propagation happens before visiting all initializers.
    let code = "import {css} from 'ui'; const a=css,b=a; b();";
    // When: the declaration is visited in the TypeScript order.
    let result = facts(code, false)?;
    // Then: initializer reads precede the later use, with exact positions.
    assert_eq!(
        result["references"],
        json!([
            {"request":"ui","ids":["css"],"pos":32},
            {"request":"ui","ids":["css"],"pos":38},
            {"request":"ui","ids":["css"],"pos":41},
        ])
    );
    Ok(())
}

#[test]
fn preserves_forwarding_when_exports_are_local_named_and_starred() -> Result<(), Box<dyn Error>> {
    // Given: public forwarding plus local declarations and a default import.
    let code = "import {css} from 'ui'; export {css as style}; export {Box as Widget} from 'components'; export * from 'star'; export * as namespace from 'namespace'; export const {x:y=css,...rest}={}; export function local(){} export default css;";
    // When: export entries are collected.
    let result = facts(code, false)?;
    // Then: insertion order and null local origins match the wire contract.
    assert_eq!(
        result["exports"],
        json!([
            ["style", {"request":"ui","ids":["css"]}],
            ["Widget", {"request":"components","ids":["Box"]}],
            ["namespace", {"request":"namespace","ids":[]}],
            ["y",null], ["rest",null], ["local",null],
            ["default", {"request":"ui","ids":["css"]}],
        ])
    );
    assert_eq!(result["stars"], json!(["star"]));
    Ok(())
}

#[test]
fn visits_patterns_when_defaults_and_computed_keys_are_reads() -> Result<(), Box<dyn Error>> {
    // Given: binding names, computed keys, defaults and nested destructuring.
    let code = "import {css} from 'ui'; function f({[css]: value=css},[head=css,,...tail]){const {nested:{x=css}}={};} let target; ({[css]:target=css}={});";
    // When: pattern expressions alone are visited as reads.
    let result = facts(code, false)?;
    // Then: each of the six expression reads is retained once.
    let refs = result["references"]
        .as_array()
        .ok_or("references must be an array")?;
    assert_eq!(refs.len(), 6);
    assert!(refs.iter().all(|value| value["ids"] == json!(["css"])));
    Ok(())
}

#[test]
fn measures_utf16_when_astral_text_precedes_a_read() -> Result<(), Box<dyn Error>> {
    // Given: one astral character before a namespace read.
    let prefix = "import * as DU from 'ui'; '😀'; ";
    let code = format!("{prefix}DU.css()");
    // When: Oxc byte spans cross the JSON boundary.
    let result = facts(&code, false)?;
    // Then: the position counts UTF-16 units, not UTF-8 bytes.
    assert_eq!(
        result["references"][0]["pos"],
        json!(prefix.encode_utf16().count())
    );
    Ok(())
}

#[test]
fn retains_lone_surrogates_when_requests_and_keys_are_escaped() -> Result<(), Box<dyn Error>> {
    // Given: JavaScript strings that Rust UTF-8 cannot represent as values.
    let code = r"import * as DU from '\ud800'; DU['\udfff']";
    // When: wire text is encoded without a lossy UTF-8 conversion.
    let result = compiled_guard_facts_internal(code, false)?;
    // Then: JSON retains the original UTF-16 units, not replacement characters.
    assert!(result.contains(r#""request":"\ud800""#));
    assert!(result.contains(r#""ids":["\udfff"]"#));
    assert!(!result.contains('�'));
    Ok(())
}

#[rstest]
#[case("const broken = ;")]
#[case("const value: number = 1;")]
fn returns_located_errors_when_final_js_is_invalid(#[case] code: &str) {
    // Given: invalid JavaScript or uncompiled TypeScript.
    // When: the source API parses final JavaScript.
    let result = compiled_guard_facts_internal(code, false);
    // Then: no fallback facts are returned, and the diagnostic is located.
    assert!(matches!(result, Err(error) if error.to_string().starts_with("compiled output:1:")));
}

#[test]
fn skips_jsx_names_when_only_brace_expressions_are_reads() -> Result<(), Box<dyn Error>> {
    // Given: JSX tag names are JSXIdentifier, not ordinary Identifier reads.
    let code = "import {Box,css} from 'ui'; const view=<Box value={css}/>;";
    // When: JSX is accepted by the final JavaScript parser.
    let result = facts(code, false)?;
    // Then: only the expression-container read is collected.
    let refs = result["references"]
        .as_array()
        .ok_or("references must be an array")?;
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0]["ids"], json!(["css"]));
    Ok(())
}
