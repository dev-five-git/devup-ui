use super::*;

#[rstest]
#[case("...stylex.include(base.red),color:'blue'", "base.blue")]
#[case("color:'blue',...stylex.include(base.red)", "base.red")]
#[case("...stylex.include(base.red),color:null", "\"\"")]
#[case("color:null,...stylex.include(base.red)", "base.red")]
#[case("...stylex.include(base.red),...stylex.include(base.reset)", "\"\"")]
#[case(
    "...stylex.include(base.reset),...stylex.include(base.red)",
    "base.red"
)]
#[case(
    "...stylex.include(base.red),...stylex.include(base.blue)",
    "base.blue"
)]
#[case(
    "...stylex.include(base.red),...stylex.include(base.blue),...stylex.include(base.red)",
    "base.red"
)]
#[case(
    "...stylex.include(base.red),color:stylex.firstThatWorks()",
    "base.red"
)]
#[serial]
fn stylex_include_when_keys_overlap_preserves_source_order_and_nulls(
    #[values("props", "attrs")] api: &str,
    #[case] body: &str,
    #[case] expected: &str,
) {
    // Given spreads and own keys with distinguishable winning values.
    let source = format!(
        "import stylex from '@stylexjs/stylex'; const base=stylex.create({{red:{{color:'red'}},blue:{{color:'blue'}},reset:{{color:null}}}}); const composed=stylex.create({{item:{{{body}}}}}); const result=stylex.{api}(composed.item);"
    );
    // When the emitted class expression runs.
    let output = extract(&source).expect("ordered include");
    let field = if api == "props" { "className" } else { "class" };
    let actual = execute(
        &output,
        "",
        &format!("[composed.item === {expected},result.{field} === {expected}]"),
    );
    // Then exactly the later color key survives both declaration and use.
    assert_eq!(actual, "[true,true]");
    assert!(!output.code.contains("stylex."), "{}", output.code);
}

#[rstest]
#[case("composed.item,base.blue", "base.blue")]
#[case("base.blue,composed.item", "base.red")]
#[case("base.red,composed.reset", "\"\"")]
#[case("composed.reset,base.red", "base.red")]
#[serial]
fn stylex_include_when_reincluded_publishes_merged_keys(
    #[values("props", "attrs")] api: &str,
    #[case] arguments: &str,
    #[case] expected: &str,
) {
    // Given recursive static includes and a copied null tombstone.
    let source = format!(
        "import stylex from '@stylexjs/stylex'; const base=stylex.create({{red:{{color:'red'}},blue:{{color:'blue'}},reset:{{color:null}}}}); const middle=stylex.create({{item:{{...stylex.include(base.red)}},reset:{{...stylex.include(base.reset)}}}}); const alias=middle; const composed=stylex.create({{item:{{...stylex.include(alias.item)}},reset:{{...stylex.include(alias.reset)}}}}); const result=stylex.{api}({arguments});"
    );
    // When props/attrs composes the derived namespace with another color.
    let output = extract(&source).expect("recursive static include");
    let field = if api == "props" { "className" } else { "class" };
    // Then its exported keys participate in the same later-key-wins rule.
    assert_eq!(
        execute(&output, "", &format!("result.{field} === {expected}")),
        "true"
    );
}

#[rstest]
#[case("...stylex.include(base.red),opacity:1,...stylex.include(base.size)")]
#[case("opacity:1,...stylex.include(base.red),...stylex.include(base.size)")]
#[case("...stylex.include(base.size),...stylex.include(base.red),opacity:1")]
#[serial]
fn stylex_include_when_keys_are_disjoint_retains_every_key(#[case] body: &str) {
    // Given multiple includes interleaved with an unrelated own property.
    let source = format!(
        "import stylex from '@stylexjs/stylex'; const base=stylex.create({{red:{{color:'red'}},size:{{fontSize:'16px'}},opacity:{{opacity:1}}}}); const composed=stylex.create({{item:{{{body}}}}}); const result=stylex.props(composed.item);"
    );
    // When the result is evaluated.
    let output = extract(&source).expect("disjoint includes");
    let actual = execute(
        &output,
        "",
        "[base.red,base.size,base.opacity].every(x=>result.className.split(' ').includes(x)) && result.className.split(' ').length===3",
    );
    // Then no disjoint key is discarded or duplicated.
    assert_eq!(actual, "true");
}

#[test]
#[serial]
fn stylex_include_when_target_is_uncalled_dynamic_is_located() {
    // Given a function namespace that needs its scalar CSS-variable assignment.
    let usage = "const composed=stylex.create({item:{...stylex.include(base.tone)}});";
    let source = format!(
        "import stylex from '@stylexjs/stylex'; const base=stylex.create({{tone:color=>({{color}})}});\n{usage}"
    );
    // When an include attempts to copy only its class.
    let error = extract(&source).expect_err("dynamic include").to_string();
    // Then the include call itself is located and the missing assignment cannot ship.
    let column = usage.find("stylex.include").expect("include") + 1;
    assert!(
        error.contains(&format!("/src/stylex.tsx:2:{column}:")),
        "{error}"
    );
    assert!(
        error.contains("uncalled dynamic namespace cannot be included exactly"),
        "{error}"
    );
}
