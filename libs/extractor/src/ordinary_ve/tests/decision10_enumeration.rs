use rstest::rstest;
use serial_test::serial;

use super::mixed_canonical::assert_preserved;
use super::mixed_support::{assert_consumed, extract};
use crate::ExtractStyleValue;

struct EnumerationCase {
    setup: &'static str,
    change: &'static str,
    keys: &'static str,
    colors: &'static [&'static str],
}

#[rstest]
#[case::deleted_inherited_key(EnumerationCase {
    setup: "proto.inherited={color:'blue'};",
    change: "delete proto.inherited;",
    keys: "first",
    colors: &["red"],
})]
#[case::deleted_own_key_reveals_inherited_after_remaining_own_keys(EnumerationCase {
    setup: "proto.revealed={color:'blue'};proto.tail={color:'purple'};data.revealed={color:'orange'};data.middle={color:'green'};",
    change: "delete data.revealed;",
    keys: "first,middle,revealed,tail",
    colors: &["blue", "green", "purple", "red"],
})]
#[case::inherited_addition_is_seen_but_own_addition_is_not(EnumerationCase {
    setup: "",
    change: "proto.added={color:'blue'};data.ownAdded={color:'orange'};",
    keys: "first,added",
    colors: &["blue", "red"],
})]
#[case::newly_hidden_own_key_still_shadows_inherited_key(EnumerationCase {
    setup: "proto.blocked={color:'blue'};data.blocked={color:'orange'};",
    change: concat!("Object.defineProperty(data,'blocked',", "{enumerable:", "false});"),
    keys: "first",
    colors: &["red"],
})]
#[serial]
fn decision10_variants_follow_live_for_in_when_getter_or_mapper_mutates_data(
    #[case] scenario: EnumerationCase,
    #[values(false, true)] mapped: bool,
    #[values("tsx", "css.ts")] extension: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let call = if mapped {
        "styleVariants(data,(value,key)=>{visits.push(key);if(key==='first')change();return {color:value.color}})"
    } else {
        "styleVariants(data)"
    };
    let source = format!(
        r"import {{styleVariants,globalStyle}} from '@vanilla-extract/css';
export const result=(()=>{{
const visits=[];
const proto={{}};
const data=Object.create(proto);
Object.defineProperty(data,'first',{{enumerable:true,get(){{if(!{mapped})change();return {{color:'red'}}}}}});
{setup}
const change=()=>{{{change}}};
const variants={call};
for(const key of Object.keys(variants))globalStyle(variants[key]+':hover',{{opacity:0.5}});
return Object.keys(variants).join(',')+'|'+visits.join(',');
}})();",
        setup = scenario.setup,
        change = scenario.change,
    );
    // When
    let output = extract(extension, &source)?;
    // Then
    assert_consumed(&output);
    let visits = if mapped { scenario.keys } else { "" };
    assert_preserved(
        &output.code,
        &format!("export const result='{}|{visits}';", scenario.keys),
    );
    let mut colors: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) if style.property() == "color" => Some(style.value()),
            _ => None,
        })
        .collect();
    colors.sort_unstable();
    assert_eq!(colors, scenario.colors, "{:?}", output.styles);
    Ok(())
}
