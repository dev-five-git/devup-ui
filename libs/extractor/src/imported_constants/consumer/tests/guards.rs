use rstest::rstest;

use super::planned;
use crate::imported_constants::consumer::{GuardKind, ReadPlan};

fn conditions<'a>(plan: &ReadPlan, source: &'a str, slot: &str) -> Vec<(&'a str, &'static str)> {
    plan.guards
        .iter()
        .find(|(span, _)| span.source_text(source) == slot)
        .unwrap_or_else(|| panic!("missing guarded slot {slot}"))
        .1
        .iter()
        .map(|guard| {
            (
                guard.test.source_text(source),
                match guard.kind {
                    GuardKind::Truthy => "truthy",
                    GuardKind::Falsy => "falsy",
                    GuardKind::Nullish => "nullish",
                },
            )
        })
        .collect()
}

#[test]
fn if_branches_when_helpers_are_observed_keep_opposite_polarity() {
    // Given
    let source = "import {css} from '@devup-ui/react';import {read,other} from './data';const enabled=true;if(enabled){css({color:read()})}else{css({color:other()})}";
    // When
    let plan = planned(source);
    // Then
    assert_eq!(conditions(&plan, source, "read()"), [("enabled", "truthy")]);
    assert_eq!(conditions(&plan, source, "other()"), [("enabled", "falsy")]);
    let guard_reads = plan
        .reads
        .iter()
        .filter(|span| span.source_text(source) == "enabled")
        .count();
    assert_eq!(guard_reads, 1);
    assert_eq!(plan.failures.len(), 0);
}

#[rstest]
#[case("enabled ? <Box color={read()}/> : null", "truthy")]
#[case("enabled ? null : <Box color={read()}/>", "falsy")]
#[case("enabled && <Box color={read()}/>", "truthy")]
#[case("enabled || <Box color={read()}/>", "falsy")]
#[case("enabled ?? <Box color={read()}/>", "nullish")]
fn jsx_guard_when_helper_is_in_a_branch_keeps_javascript_polarity(
    #[case] view: &str,
    #[case] polarity: &str,
) {
    // Given
    let source = format!(
        "import {{Box}} from '@devup-ui/react';import {{read}} from './data';const enabled=true;{view};"
    );
    // When
    let plan = planned(&source);
    // Then
    assert_eq!(
        conditions(&plan, &source, "read()"),
        [("enabled", polarity)]
    );
    assert_eq!(plan.failures.len(), 0);
}

#[test]
fn nested_guards_when_inner_slot_is_selected_are_outer_first() {
    // Given
    let source = "import {Box} from '@devup-ui/react';import {read} from './data';const outer=true,inner=false;if(outer){inner || <Box color={read()}/>;}";
    // When
    let plan = planned(source);
    // Then
    assert_eq!(
        conditions(&plan, source, "read()"),
        [("outer", "truthy"), ("inner", "falsy")]
    );
    let reads: Vec<_> = plan
        .reads
        .iter()
        .map(|span| span.source_text(source))
        .collect();
    assert_eq!(reads, ["read", "outer", "inner"]);
}

#[rstest]
#[case("window.flag", "")]
#[case("enabled", "function view(enabled)")]
#[case("gate()", "")]
fn helper_when_guard_is_runtime_only_records_original_test_span(
    #[case] guard: &str,
    #[case] wrapper: &str,
) {
    // Given
    let source = format!(
        "import {{css}} from '@devup-ui/react';import {{read,gate}} from './data';{wrapper}{{if({guard}){{css({{color:read()}})}}}}"
    );
    // When
    let plan = planned(&source);
    // Then
    assert_eq!(plan.failures.len(), 1);
    assert_eq!(plan.failures[0].0.source_text(&source), guard);
    assert!(plan.failures[0].1.contains("runtime execution guard"));
    assert_eq!(conditions(&plan, &source, "read()"), vec![]);
}

#[rstest]
#[case("for(let n=0;n<1;n++)", "")]
#[case(concat!("for(const key in {", "a:1})"), "")]
#[case("for(const value of [1])", "")]
#[case("while(false)", "")]
#[case("do", "while(false);")]
fn helper_when_application_loop_schedules_it_records_loop_span(
    #[case] prefix: &str,
    #[case] suffix: &str,
) {
    // Given
    let body = format!("{prefix}{{css({{color:read()}})}}{suffix}");
    let source =
        format!("import {{css}} from '@devup-ui/react';import {{read}} from './data';{body}");
    // When
    let plan = planned(&source);
    // Then
    assert_eq!(plan.failures.len(), 1);
    assert_eq!(plan.failures[0].0.source_text(&source), body);
    assert!(
        plan.failures[0]
            .1
            .contains("application iteration schedule")
    );
}

#[test]
fn runtime_guard_when_observation_is_call_free_adds_no_helper_failure() {
    // Given
    let source = "import {css} from '@devup-ui/react';import {palette} from './data';if(window.flag){css({color:palette.fg})}";
    // When
    let plan = planned(source);
    // Then
    assert_eq!(
        plan.slots
            .iter()
            .map(|span| span.source_text(source))
            .collect::<Vec<_>>(),
        ["palette.fg"]
    );
    assert_eq!(plan.failures.len(), 0);
    assert_eq!(conditions(&plan, source, "palette.fg"), vec![]);
}
