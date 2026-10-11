use serial_test::serial;

use super::demand_support::{TestResult, reset};
use crate::vanilla_extract::Stylesheet;

#[test]
#[serial]
fn declines_invalid_native_syntax_before_any_evaluation() -> TestResult {
    reset();
    let source = "import {style} from '@vanilla-extract/css';export const box=style({ padding: });";
    let stylesheet = Stylesheet {
        filename: "/invalid.ts",
        code: source,
        source,
        edits: &[],
    };
    let prepared = super::super::prepare(stylesheet, &super::option(), None)?;
    assert_eq!(prepared.map(|prepared| prepared.code), None);
    Ok(())
}
