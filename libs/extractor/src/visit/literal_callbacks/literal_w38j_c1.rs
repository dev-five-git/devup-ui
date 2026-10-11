use super::{compile, evaluate};
use crate::ExtractStyleValue;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("p=>{if(p.stop)return;return 2;}", vec!["built", "get", "get"])]
#[case("function(p){if(p.stop)return;return 2;}", vec!["built", "get", "get"])]
#[case(
    "p=>{if(p.stop){trace.push('early');return;}trace.push('tail');return 2;}",
    vec!["built", "get", "early", "get", "tail"]
)]
#[case(
    "function(p){if(p.stop){trace.push('early');return;}trace.push('tail');return 2;}",
    vec!["built", "get", "early", "get", "tail"]
)]
#[serial]
fn bare_return_when_stop_is_true_leaves_red_unlayered(
    #[case] callback: &str,
    #[case] expected_trace: Vec<&str>,
) {
    // Given: actual parsed callbacks return no order on stop, and order 2 otherwise.
    let source = format!(
        "import {{styled}} from '@devup-ui/react';const __devupForwardRef=render=>render;const Card=styled.div`style-order:${{{callback}}};color:red`;trace.push('built');const a=Card({{get stop(){{trace.push('get');return true}}}},null);const b=Card({{get stop(){{trace.push('get');return false}}}},null);"
    );
    // When: extraction prepares the wrapper/captures and its generated renders execute.
    let compiled = compile(&source).unwrap_or_else(|error| panic!("{error}"));
    let (classes, actual_trace) = evaluate(&compiled.code);
    // Then: one getter/effect execution per path, with omitted-order and order-2 red.
    assert_eq!(actual_trace, expected_trace);
    assert_eq!(
        classes
            .iter()
            .map(|class| class.split_whitespace().collect::<Vec<_>>())
            .collect::<Vec<_>>(),
        vec![vec!["color-0-red--255-a"], vec!["color-0-red--2-a"]]
    );
    let mut red_orders = compiled
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style)
                if style.property() == "color" && style.value() == "red" =>
            {
                Some(style.style_order())
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    red_orders.sort_unstable();
    assert_eq!(red_orders, vec![None, Some(2)]);
}

#[rstest]
#[case("p=>{if(p.stop){trace.push('early');return 6;}trace.push('tail');return 7;}")]
#[case("function(p){if(p.stop){trace.push('early');return 6;}trace.push('tail');return 7;}")]
#[serial]
fn valued_returns_when_stop_selects_earlier_return_keep_its_order(#[case] callback: &str) {
    // Given: distinct valued returns make losing the earlier selection observable.
    let source = format!(
        "import {{styled}} from '@devup-ui/react';const __devupForwardRef=render=>render;const Card=styled.div`style-order:${{{callback}}};color:red`;trace.push('built');const a=Card({{get stop(){{trace.push('get');return true}}}},null);const b=Card({{get stop(){{trace.push('get');return false}}}},null);"
    );
    // When: the same public wrapper/capture path executes both generated renders.
    let compiled = compile(&source).unwrap_or_else(|error| panic!("{error}"));
    let (classes, actual_trace) = evaluate(&compiled.code);
    // Then: the first valued return wins only when selected; neither effect replays.
    assert_eq!(actual_trace, vec!["built", "get", "early", "get", "tail"]);
    assert_eq!(
        classes
            .iter()
            .map(|class| class.split_whitespace().collect::<Vec<_>>())
            .collect::<Vec<_>>(),
        vec![vec!["color-0-red--6-a"], vec!["color-0-red--7-a"]]
    );
}
