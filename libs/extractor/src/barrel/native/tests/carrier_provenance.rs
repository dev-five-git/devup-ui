use super::{Allocator, Facts, PACKAGE, Parser, SemanticBuilder, SourceType, TestResult, resolver};
use crate::module_loader::demand::Demand;
use oxc_span::Span;
use rstest::rstest;
use std::{cell::RefCell, collections::BTreeSet, rc::Rc};

#[rstest]
#[case::unreadable(
    "import '@vanilla-extract/css';export {style as make} from './missing';",
    false
)]
#[case::changed(
    "import {style} from '@vanilla-extract/css';\nconst make=style;\nmake.extra=1;\nexport {make};",
    true
)]
fn failure_routes_through_the_real_carrier_when_its_requested_terminal_is_invalid(
    #[case] bad: &str,
    #[case] original: bool,
) -> TestResult {
    // Given
    let source = "export {make} from './bad';";
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let built = SemanticBuilder::new()
        .with_build_nodes(true)
        .build(&parsed.program);
    assert_eq!(built.diagnostics.len(), 0);
    let records = Rc::new(RefCell::new(BTreeSet::new()));
    let recording = records.clone();
    let delegate = resolver(&[("./bad", "/bad.ts", bad)]);
    let resolve = move |specifier: &str, importer: &str| {
        recording
            .borrow_mut()
            .insert((specifier.to_string(), importer.to_string()));
        delegate(specifier, importer)
    };
    let mut facts = Facts::resolve(&parsed.program, ("/carrier.ts", PACKAGE), Some(&resolve));
    let demand = Demand::prefixed("make", &Demand::whole());
    // When
    let proven = facts.prove_carrier(
        (&parsed.program, source),
        ("/carrier.ts", PACKAGE),
        (&demand, Some(&resolve)),
    );
    // Then
    assert!(!proven);
    assert_eq!(facts.bindings.len(), 0);
    assert_eq!(
        facts.dependencies.into_iter().collect::<Vec<_>>(),
        ["/bad.ts"]
    );
    let mut expected = BTreeSet::from([("./bad".to_string(), "/carrier.ts".to_string())]);
    if original {
        assert_eq!(facts.errors.len(), 0);
        let failure = facts.failure.ok_or("original failure missing")?;
        assert!(failure.starts_with("/bad.ts:3:1:"), "{failure}");
        assert!(
            failure
                .contains("native API alias may be changed outside its exact initialization slice"),
            "{failure}"
        );
        assert!(failure.contains("Fix:"), "{failure}");
        assert!(!failure.contains("/carrier.ts"), "{failure}");
    } else {
        assert_eq!(facts.failure, None);
        assert_eq!(
            facts.errors,
            [(
                Span::new(8, 12),
                "native re-export `./missing` cannot be read".to_string()
            )]
        );
        assert_eq!(facts.errors[0].0.source_text(source), "make");
        expected.insert(("./missing".to_string(), "/bad.ts".to_string()));
    }
    assert_eq!(*records.borrow(), expected);
    Ok(())
}
