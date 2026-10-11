use rstest::rstest;

use super::super::Demand;
use super::test_support::{TestResult, path, selected, span};

#[test]
fn only_leaf_require_is_forwarded_when_browser_property_is_omitted() -> TestResult {
    // Given
    let source = "module.exports={space:require('./leaf').tokens.space,browser:require('never')};";
    let demand = path(&["space"]);
    // When
    let view = selected(source, &demand)?;
    // Then
    assert_eq!(view.units, vec![(span(source, source)?, demand)]);
    assert_eq!(
        view.omitted,
        vec![span(source, "browser:require('never')")?]
    );
    assert_eq!(
        view.forwarded,
        vec![(
            "./leaf".to_string(),
            path(&["tokens", "space"]),
            span(source, "'./leaf'")?
        )]
    );
    assert_eq!(view.errors, vec![]);
    Ok(())
}

#[rstest]
#[case("exports.space=require('./leaf').tokens.space;")]
#[case("exports.space=require('./leaf')['tokens']['space'];")]
fn member_require_is_forwarded_with_exact_path_when_assignment_is_selected(
    #[case] source: &str,
) -> TestResult {
    // Given
    let demand = path(&["space"]);
    // When
    let view = selected(source, &demand)?;
    // Then
    assert_eq!(view.units, vec![(span(source, source)?, Demand::whole())]);
    assert_eq!(
        view.forwarded,
        vec![(
            "./leaf".to_string(),
            path(&["tokens", "space"]),
            span(source, "'./leaf'")?
        )]
    );
    Ok(())
}

#[test]
fn initializer_require_forwards_owner_demand_when_whole_carrier_is_assigned() -> TestResult {
    // Given
    let source = "module.exports=require('./leaf');";
    let demand = path(&["tokens", "space"]);
    // When
    let view = selected(source, &demand)?;
    // Then
    assert_eq!(view.units, vec![(span(source, source)?, demand.clone())]);
    assert_eq!(
        view.forwarded,
        vec![("./leaf".to_string(), demand, span(source, "'./leaf'")?)]
    );
    Ok(())
}

#[test]
fn initializer_require_forwards_owner_demand_when_selected_binding_carries_export() -> TestResult {
    // Given
    let source = "const carrier=require('./leaf');module.exports=carrier;";
    let demand = path(&["tokens", "space"]);
    // When
    let view = selected(source, &demand)?;
    // Then
    assert_eq!(
        view.units,
        vec![
            (span(source, "carrier=require('./leaf')")?, demand.clone()),
            (span(source, "module.exports=carrier;")?, demand.clone()),
        ]
    );
    assert_eq!(
        view.forwarded,
        vec![("./leaf".to_string(), demand, span(source, "'./leaf'")?)]
    );
    Ok(())
}

#[test]
fn whole_require_is_forwarded_when_selected_expression_hands_it_to_callable() -> TestResult {
    // Given
    let source = "module.exports={space:consume(require('./whole')),browser:window};";
    let demand = path(&["space"]);
    // When
    let view = selected(source, &demand)?;
    // Then
    assert_eq!(view.units, vec![(span(source, source)?, demand)]);
    assert_eq!(
        view.forwarded,
        vec![(
            "./whole".to_string(),
            Demand::whole(),
            span(source, "'./whole'")?
        )]
    );
    assert_eq!(view.omitted, vec![span(source, "browser:window")?]);
    Ok(())
}

#[rstest]
#[case("require(source)")]
#[case("require()")]
#[case("require('./one','./two')")]
#[case("require(`./template`)")]
#[case("require(...['./spread'])")]
#[case("loader('./other')")]
#[case("loader.require('./other')")]
fn no_request_is_forwarded_when_selected_call_is_not_global_literal_require(
    #[case] expression: &str,
) -> TestResult {
    // Given
    let source = format!("exports.space={expression};");
    let demand = path(&["space"]);
    // When
    let view = selected(&source, &demand)?;
    // Then
    assert_eq!(view.units, vec![(span(&source, &source)?, Demand::whole())]);
    assert_eq!(view.forwarded, vec![]);
    Ok(())
}

#[test]
fn no_request_is_forwarded_when_literal_require_owner_is_unselected() -> TestResult {
    // Given
    let source = "const ignored=require('./ignored');exports.space=8;require('./orphan');";
    let demand = path(&["space"]);
    // When
    let view = selected(source, &demand)?;
    // Then
    assert_eq!(
        view.units,
        vec![(span(source, "exports.space=8;")?, Demand::whole())]
    );
    assert_eq!(view.forwarded, vec![]);
    Ok(())
}

#[test]
fn no_request_is_forwarded_when_selected_require_is_shadowed() -> TestResult {
    // Given
    let source = "const require=loader;exports.space=require('./local');";
    let demand = path(&["space"]);
    // When
    let view = selected(source, &demand)?;
    // Then
    assert_eq!(
        view.units,
        vec![
            (span(source, "require=loader")?, Demand::whole()),
            (
                span(source, "exports.space=require('./local');")?,
                Demand::whole()
            ),
        ]
    );
    assert_eq!(view.forwarded, vec![]);
    Ok(())
}
