use std::collections::BTreeSet;

use super::super::lowering::{Lowered, hygienic, literal};
use super::demand_support::TestResult;

#[test]
fn generated_css_import_avoids_both_existing_and_prior_collision_names() -> TestResult {
    let generated = Lowered {
        code: "import { css } from '@devup-ui/react';export const box=css({ padding:8 });".into(),
        css: None,
    };
    let mut reserved = BTreeSet::from(["__ve_css__".into(), "__ve_css___".into()]);
    let output = hygienic(generated, &mut reserved)?;
    assert_eq!(output.css.as_deref(), Some("__ve_css____"));
    assert_eq!(
        output.code,
        "import { css as __ve_css____ } from '@devup-ui/react';export const box=__ve_css____({ padding:8 });"
    );
    Ok(())
}

#[rstest::rstest]
#[case("[undefined,{ value:undefined }]", "([(void 0),{ value:(void 0) }])")]
#[case(
    "({ undefined:1,value:undefined })",
    "(({ undefined:1,value:(void 0) }))"
)]
fn captured_undefined_stays_undefined_when_the_authored_module_shadows_it(
    #[case] captured: &str,
    #[case] expected: &str,
) -> TestResult {
    let reserved = BTreeSet::from(["undefined".into()]);
    let expression = literal(captured, &reserved)?;
    assert_eq!(expression, expected);
    Ok(())
}
