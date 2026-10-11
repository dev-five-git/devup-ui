use rstest::rstest;

use super::planned;

#[test]
fn outermost_slot_when_nested_reads_are_exact_keeps_one_observation() {
    // Given
    let source = "import {css} from '@devup-ui/react';import {palette,key} from './data';const app=palette.fg;css({color:palette[key]+palette.fg});";
    // When
    let plan = planned(source);
    // Then
    let slots: Vec<_> = plan
        .slots
        .iter()
        .map(|span| span.source_text(source))
        .collect();
    assert_eq!(slots, ["palette[key]+palette.fg"]);
    let reads: Vec<_> = plan
        .reads
        .iter()
        .map(|span| span.source_text(source))
        .collect();
    assert_eq!(reads, ["palette", "key", "palette"]);
    assert_eq!(plan.failures.len(), 0);
}

#[rstest]
#[case("import {Box} from '@devup-ui/react';", "<Box {...props}/>", true)]
#[case("import * as UI from '@devup-ui/react';", "<UI.Box {...props}/>", true)]
#[case("", "<div {...props}/>", false)]
#[case(
    "import {Box} from '@devup-ui/react';",
    "function view(Box){return <Box {...props}/>}",
    false
)]
fn spread_when_component_binding_is_real_routes_the_producer_read(
    #[case] import: &str,
    #[case] view: &str,
    #[case] expected: bool,
) {
    // Given
    let source = format!("{import}import {{props}} from './data';{view};");
    // When
    let plan = planned(&source);
    // Then
    let slots: Vec<_> = plan
        .slots
        .iter()
        .map(|span| span.source_text(&source))
        .collect();
    assert_eq!(slots, if expected { vec!["props"] } else { vec![] });
}

#[test]
fn special_props_when_only_runtime_channels_read_data_create_no_slots() {
    // Given
    let source = "import {Box} from '@devup-ui/react';import {value} from './data';<Box as={value} props={value} styleVars={value} onClick={value}>{value}</Box>;";
    // When
    let plan = planned(source);
    // Then
    assert_eq!(plan.slots.len(), 0);
    assert_eq!(plan.reads.len(), 0);
}
