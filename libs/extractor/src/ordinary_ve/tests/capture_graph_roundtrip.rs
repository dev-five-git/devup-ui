use serial_test::serial;

use super::mixed_canonical::assert_preserved;
use super::mixed_review::emitted_predicate;
use super::mixed_support::{assert_consumed, extract, has_static};

#[test]
#[serial]
fn captured_root_roundtrips_when_result_is_a_null_prototype_array()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = "import {style} from '@vanilla-extract/css';export const result=(()=>{style({color:'red'});return Object.setPrototypeOf([null,true,false],null)})();const browser=window.document;";
    // When
    let output = extract("ts", source)?;
    // Then
    assert_consumed(&output);
    assert!(has_static(&output, "color", "red"));
    assert_preserved(&output.code, "const browser=window.document;");
    assert!(
        emitted_predicate(
            &output.code,
            "Array.isArray(result) && Object.getPrototypeOf(result)===null && result.length===3 && result[0]===null && result[1]===true && result[2]===false && Object.keys(result).join(',')==='0,1,2'"
        ),
        "{}",
        output.code
    );
    Ok(())
}
