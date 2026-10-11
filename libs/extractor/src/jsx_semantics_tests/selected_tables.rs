use super::whole::evaluate;
use super::*;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("({a:first(),b:second()})", "'a'", Some("red"))]
#[case("({a:first(),b:second()})", "'missing'", None)]
#[case("([first(),second()])", "1", Some("navy"))]
#[case("([first(),second()])", "9", None)]
#[serial]
fn selected_table_when_its_key_selects_nothing_runs_without_reading_the_missing_choice(
    #[case] table: &str,
    #[case] key: &str,
    #[case] color: Option<&str>,
) {
    let source = format!(
        "{BOX}{JSX_RUNTIME}export const a=(key,first,second,later)=>jsx(Box,{{color:{table}[key],id:later()}});"
    );
    let actual = evaluate(
        &source,
        &format!(
            "a({key},()=>{{trace.push('first');return 'red';}},()=>{{trace.push('second');return 'navy';}},()=>{{trace.push('later');return 'id';}})"
        ),
    );
    assert_eq!(
        actual.trace,
        serde_json::json!(["first", "second", "later"])
    );
    let props = &actual.element["props"];
    assert_eq!(props["id"], "id");
    assert_eq!(
        props["style"]
            .as_object()
            .into_iter()
            .flat_map(|style| style.values())
            .filter_map(|value| value.as_str())
            .collect::<Vec<_>>(),
        color.into_iter().collect::<Vec<_>>()
    );
    assert_eq!(
        props["className"].as_str().map(str::is_empty),
        Some(color.is_none())
    );
}
