use std::error::Error;

use rstest::rstest;
use serde_json::json;

use super::tests::facts;

#[test]
fn records_outer_path_when_namespace_members_are_nested() -> Result<(), Box<dyn Error>> {
    // Given: a chain whose inner member already has a definite origin.
    let code = "import * as DU from 'ui'; DU.stylex.create({});";
    // When: the member visitor records the outer origin.
    let result = facts(code, false)?;
    // Then: the complete path occurs once, without a duplicate inner read.
    assert_eq!(
        result["references"],
        json!([
            {"request":"ui","ids":["stylex","create"],"pos":26},
        ])
    );
    Ok(())
}

#[rstest]
#[case("import {css} from 'ui'; css[unknown];")]
#[case("import {css} from 'ui'; const holder={}; [holder[css]]=[];")]
#[case("import {css} from 'ui'; const obj={[css]:0}; class C{[css](){}}")]
fn avoids_extra_reads_when_typescript_traversal_suppresses_them(
    #[case] code: &str,
) -> Result<(), Box<dyn Error>> {
    // Given: a dynamic member, a pattern member target or an identifier key.
    // When: the narrow current traversal is reproduced, rather than broadened.
    let result = facts(code, false)?;
    // Then: suppressed subexpressions do not become definite references.
    assert_eq!(result["references"], json!([]));
    Ok(())
}

#[test]
fn retains_literal_empty_names_when_computed_key_values_are_not_strings()
-> Result<(), Box<dyn Error>> {
    // Given: nodeName ignores non-string Literal values in the TypeScript guard.
    let code = "import * as DU from 'ui'; DU[1];";
    // When: a numeric literal member is resolved.
    let result = facts(code, false)?;
    // Then: it retains the existing empty-name semantics, not numeric coercion.
    assert_eq!(result["references"][0]["ids"], json!([""]));
    Ok(())
}

#[test]
fn retains_traversal_time_origins_when_exports_precede_aliases() -> Result<(), Box<dyn Error>> {
    // Given: an export before a const alias and a default export after it.
    let code =
        "import * as DU from 'ui'; export {Alias}; const Alias=DU.css; export default Alias;";
    // When: const origins are seeded in original traversal order.
    let result = facts(code, false)?;
    // Then: the early export remains local while the later default gains origin.
    assert_eq!(
        result["exports"],
        json!([
            ["Alias",null], ["default",{"request":"ui","ids":["css"]}],
        ])
    );
    Ok(())
}

#[test]
fn retains_require_kind_when_default_forwards_a_member() -> Result<(), Box<dyn Error>> {
    // Given: a default directly exporting a definite require member.
    // When: default export provenance is collected.
    let result = facts("export default require('ui').css;", false)?;
    // Then: export forwarding retains require-call resolution kind as well.
    assert_eq!(
        result["exports"],
        json!([
            ["default",{"request":"ui","ids":["css"],"kind":"require-call"}],
        ])
    );
    Ok(())
}
