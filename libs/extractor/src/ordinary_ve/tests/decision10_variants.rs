use rstest::rstest;
use serial_test::serial;

use super::mixed_canonical::assert_preserved;
use super::mixed_support::{assert_consumed, extract, has_static};

#[rstest]
#[case("styleVariants(data)", "")]
#[case(
    "styleVariants(data,(value,key)=>{visits.push(key+':'+value.color);return {color:value.color}})",
    "2:teal,10:blue,z:navy,shadowed:green,1:red,inherited:purple"
)]
#[serial]
fn decision10_variants_enumerate_inherited_keys_in_order_and_respect_hidden_shadows(
    #[case] call: &str,
    #[case] visits: &str,
    #[values("tsx", "css.ts")] extension: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = format!(
        r"import {{styleVariants,globalStyle}} from '@vanilla-extract/css';
export const result=(()=>{{
const visits=[];
const rule=value=>({{color:value}});
const proto={{'1':rule('red'),inherited:rule('purple'),shadowed:rule('orange'),blocked:rule('pink')}};
const data=Object.create(proto);
data.z=rule('navy');data['10']=rule('blue');data['2']=rule('teal');data.shadowed=rule('green');
Object.defineProperty(data,'hidden',{{value:rule('yellow')}});
Object.defineProperty(data,'blocked',{{value:rule('brown')}});
data[Symbol('symbol')]=rule('gold');
const variants={call};
for(const key of Object.keys(variants))globalStyle('.variant-'+key,{{color:data[key].color}});
return Object.keys(variants).join(',')+'|'+visits.join(',');
}})();"
    );
    // When
    let output = extract(extension, &source)?;
    // Then
    assert_consumed(&output);
    assert_preserved(
        &output.code,
        &format!("export const result='1,2,10,z,shadowed,inherited|{visits}';"),
    );
    for color in ["red", "teal", "blue", "navy", "green", "purple"] {
        assert!(
            has_static(&output, "color", color),
            "{color}: {:?}",
            output.styles
        );
    }
    for color in ["orange", "pink", "yellow", "brown", "gold"] {
        assert!(
            !has_static(&output, "color", color),
            "{color}: {:?}",
            output.styles
        );
    }
    Ok(())
}

#[rstest]
#[case("styleVariants(data)", "{}")]
#[case(
    "styleVariants(data,(value,key)=>({color:value,content:'\"'+key+'\"'}))",
    "'never'"
)]
#[serial]
fn decision10_variants_do_not_read_non_enumerable_getters(
    #[case] call: &str,
    #[case] shadow: &str,
    #[values("tsx", "css.ts")] extension: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = format!(
        "import {{styleVariants}} from '@vanilla-extract/css';\nexport const variants=(()=>{{const data=Object.create({{shadowed:{shadow}}});Object.defineProperty(data,'shadowed',{{get(){{throw new Error('hidden shadow read')}}}});Object.defineProperty(data,'hidden',{{get(){{throw new Error('hidden key read')}}}});return {call};}})();"
    );
    // When
    let output = extract(extension, &source)?;
    // Then
    assert_consumed(&output);
    assert_preserved(&output.code, "export const variants={};");
    assert_eq!(output.styles.len(), 0);
    Ok(())
}
