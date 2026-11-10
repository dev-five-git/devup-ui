use super::whole::{evaluate, execute};
use super::*;
use oxc_ast_visit::{Visit, walk};
use rstest::rstest;
use serial_test::serial;

fn array_count(source: &str) -> usize {
    struct Arrays(usize);
    impl<'a> Visit<'a> for Arrays {
        fn visit_array_expression(&mut self, array: &oxc_ast::ast::ArrayExpression<'a>) {
            self.0 += 1;
            walk::walk_array_expression(self, array);
        }
    }
    let arena = oxc_allocator::Allocator::default();
    let parsed = Parser::new(&arena, source, SourceType::tsx()).parse();
    assert!(parsed.diagnostics.is_empty(), "{source}");
    let mut arrays = Arrays(0);
    arrays.visit_program(&parsed.program);
    arrays.0
}

#[rstest]
#[case("({lg:'buttonLg',md:'button'})", "'lg'", "typo-buttonLg", 0)]
#[case("({lg:'buttonLg',md:'button'})", "'md'", "typo-button", 0)]
#[case("({lg:'buttonLg',md:'button'})", "'missing'", "", 0)]
#[case("({lg:'buttonLg',md:'button'})", "undefined", "", 0)]
#[case("({lg:'buttonLg',md:'button'})", "null", "", 0)]
#[case("({lg:'buttonLg',md:'button'})", "Symbol('missing')", "", 0)]
#[case("['buttonLg','button']", "0", "typo-buttonLg", 3)]
#[case("['buttonLg','button']", "1", "typo-button", 3)]
#[case("['buttonLg','button']", "2", "", 3)]
#[case("['buttonLg',,'button']", "1", "", 3)]
#[case("['buttonLg',,'button']", "2", "typo-button", 3)]
#[case("['buttonLg','button']", "'length'", "", 3)]
#[case("['buttonLg','button']", "'constructor'", "", 3)]
#[case("['buttonLg','button']", "'toString'", "", 3)]
#[case("['buttonLg','button']", "Symbol.iterator", "", 3)]
#[serial]
fn static_typography_selects_classes_and_only_retains_required_tag_arrays(
    #[case] table: &str,
    #[case] key: &str,
    #[case] expected: &str,
    #[case] arrays: usize,
) {
    let source = format!(
        "import {{Text}} from '@devup-ui/react';export const a=(index,child)=><Text typography={{({table} as const)[index]}}>{{child()}}</Text>;"
    );
    let compiled = code(&source);
    assert_eq!(array_count(&compiled), arrays, "{compiled}");
    assert_eq!(compiled.matches("=>").count(), 1, "{compiled}");
    assert!(!compiled.contains("__devupChoice"), "{compiled}");
    let actual = execute(
        &compiled,
        &format!("a({key},()=>{{trace.push('child');return 'text';}})"),
    );
    assert_eq!(actual.element["props"]["className"], expected);
    assert_eq!(actual.trace, serde_json::json!(["child"]));
    assert_eq!(actual.element["children"], serde_json::json!(["text"]));
}

#[rstest]
#[case("({lg:'buttonLg',md:'button'})", "'lg'")]
#[case("['buttonLg','button']", "'0'")]
#[serial]
fn projected_selection_coerces_once_before_later_props_and_stays_eager_on_error(
    #[case] table: &str,
    #[case] key: &str,
    #[values(false, true)] throws: bool,
) {
    let source = format!(
        "import {{Text}} from '@devup-ui/react';export const a=(index,later,child)=><Text typography={{{table}[index]}} id={{later()}}>{{child()}}</Text>;"
    );
    let compiled = code(&source);
    assert_eq!(compiled.matches("=>").count(), 1, "{compiled}");
    let resolve = if throws { "throw 'key-failed'" } else { "" };
    let actual = execute(
        &compiled,
        &format!(
            "(()=>{{try{{return a({{[Symbol.toPrimitive](){{trace.push('key');{resolve};return {key};}}}},()=>{{trace.push('later');return 'id';}},()=>{{trace.push('child');return 'text';}});}}catch(error){{return {{error:String(error)}};}}}})()"
        ),
    );
    if throws {
        assert_eq!(actual.element, serde_json::json!({"error":"key-failed"}));
        assert_eq!(actual.trace, serde_json::json!(["key"]));
    } else {
        assert_eq!(actual.element["props"]["className"], "typo-buttonLg");
        assert_eq!(actual.element["props"]["id"], "id");
        assert_eq!(actual.trace, serde_json::json!(["key", "later", "child"]));
    }
}

#[rstest]
#[case("({a:.5,b:1})", "'a'", "opacity-0-_d_5--255-a")]
#[case("[.5,1]", "'1'", "opacity-0-1--255-a")]
#[case("({a:.5,b:null})", "'b'", "")]
#[case("({a:.5,b:false})", "'b'", "opacity-0-false--255-a")]
#[case("({a:[1,2],b:[3,4]})", "'a'", "opacity-0-1--255-a opacity-1-2--255-a")]
#[serial]
fn static_style_tables_keep_their_classes_without_tag_arrays(
    #[case] table: &str,
    #[case] key: &str,
    #[case] expected: &str,
) {
    let source = if table.starts_with('[') {
        format!("{BOX}export const a=index=><Box opacity={{{table}[{key}]}}/>;")
    } else {
        format!("{BOX}export const a=index=><Box opacity={{{table}[index]}}/>;")
    };
    let compiled = code(&source);
    assert_eq!(array_count(&compiled), 0, "{compiled}");
    let actual = execute(&compiled, &format!("a({key})"));
    assert_eq!(actual.element["props"]["className"], expected);
}

#[test]
#[serial]
fn duplicate_keys_keep_the_last_static_variant() {
    let source = "import {Text} from '@devup-ui/react';export const a=index=><Text typography={{a:'button',a:'buttonLg'}[index]}/>;";
    let compiled = code(source);
    assert_eq!(array_count(&compiled), 0, "{compiled}");
    let actual = execute(&compiled, "a('a')");
    assert_eq!(actual.element["props"]["className"], "typo-buttonLg");
}

#[rstest]
#[case("({a:.5,b:1})", "Object")]
#[case("[.5,1]", "Array")]
#[serial]
fn projection_keeps_one_inherited_lookup_before_later_attributes(
    #[case] table: &str,
    #[case] prototype: &str,
) {
    let source = format!(
        "{BOX}export const a=(later)=><Box opacity={{{table}['missing']}} id={{later()}}/>;"
    );
    let compiled = code(&source);
    let actual = execute(
        &format!(
            "Object.defineProperty({prototype}.prototype,'missing',{{get(){{trace.push('inherited');return undefined;}}}});{compiled}"
        ),
        "a(()=>{trace.push('later');return 'id';})",
    );
    assert_eq!(actual.trace, serde_json::json!(["inherited", "later"]));
    assert_eq!(actual.element["props"]["className"], "");
}

#[test]
#[serial]
fn a_known_static_variant_collapses_to_one_class_literal() {
    let source = format!("{BOX}export const a=<Box opacity={{{{a:.5,b:1}}['a']}}/>;");
    let compiled = code(&source);
    assert!(!compiled.contains("||"), "{compiled}");
    assert!(!compiled.contains("${"), "{compiled}");
    assert_eq!(array_count(&compiled), 0, "{compiled}");
    let actual = execute(&compiled, "a");
    assert_eq!(
        actual.element["props"]["className"],
        "opacity-0-_d_5--255-a"
    );
}

#[test]
#[serial]
fn selecting_dynamic_values_keeps_all_branch_evaluations_before_the_key() {
    let source = format!(
        "{BOX}export const a=(key,first,second)=><Box opacity={{{{a:first(),b:second()}}[key]}}/>;"
    );
    let actual = evaluate(
        &source,
        "a({toString(){trace.push('key');return 'a';}},()=>{trace.push('first');return .5;},()=>{trace.push('second');return 1;})",
    );
    assert_eq!(actual.trace, serde_json::json!(["first", "second", "key"]));
    assert!(
        actual.element["props"]["style"]
            .as_object()
            .is_some_and(|style| { style.values().any(|value| value == 0.5) })
    );
}
