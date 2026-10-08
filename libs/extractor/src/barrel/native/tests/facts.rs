use rstest::rstest;

use super::{TestResult, facts};

#[rstest]
#[case(
    "const make=ve.style;make.extra=1;export {make};",
    "import {make} from './api';",
    "make.extra"
)]
#[case(
    "const native=ve;native.extra=1;export default native.style;",
    "import make from './api';",
    "native.extra"
)]
fn original_failure_keeps_the_mutation_site_when_a_changed_api_is_imported(
    #[case] declaration: &str,
    #[case] source: &str,
    #[case] mutation: &str,
) -> TestResult {
    // Given
    let code =
        format!("const 한글='😀';\r\nimport * as ve from '@vanilla-extract/css';\r\n{declaration}");
    let place = crate::locate("/changed.ts", &code, code.find(mutation).ok_or("mutation")?);
    // When
    let result = facts(source, &[("./api", "/changed.ts", &code)]);
    // Then
    let failure = result.failure.ok_or("changed native alias must fail")?;
    assert!(failure.starts_with(&place), "{failure}");
    assert!(failure.contains("may be changed"), "{failure}");
    assert!(failure.contains("Fix:"), "{failure}");
    assert_eq!(result.errors.len(), 0);
    assert_eq!(result.bindings.len(), 0);
    assert_eq!(
        result.dependencies.into_iter().collect::<Vec<_>>(),
        ["/changed.ts"]
    );
    Ok(())
}

#[rstest]
#[case(
    "export {style as make} from './missing';",
    "import {make} from './api';",
    "make"
)]
#[case(
    "import * as missing from './missing';export default missing.style;",
    "import make from './api';",
    "make"
)]
#[case("export * from './missing';", "import {style} from './api';", "style")]
fn failed_reexport_is_attached_to_the_import_when_its_native_source_is_unreadable(
    #[case] reexport: &str,
    #[case] source: &str,
    #[case] imported: &str,
) {
    // Given
    let code = format!("import '@vanilla-extract/css';{reexport}");
    // When
    let result = facts(source, &[("./api", "/unreadable.ts", &code)]);
    // Then
    assert_eq!(result.failure, None);
    assert_eq!(result.bindings.len(), 0);
    assert_eq!(result.errors.len(), 1);
    let (span, cause) = &result.errors[0];
    assert_eq!(span.source_text(source), imported);
    assert_eq!(cause, "native re-export `./missing` cannot be read");
    assert_eq!(
        result.dependencies.into_iter().collect::<Vec<_>>(),
        ["/unreadable.ts"]
    );
}
