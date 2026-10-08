use super::{Use, uses};
use rstest::rstest;

#[rstest]
#[case("Object.freeze(made.child).p=2;", true)]
#[case("Object.freeze(made.child).custom();", false)]
fn returned_freeze_when_used_as_a_receiver_propagates_the_original_path(
    #[case] consumer: &str,
    #[case] writes: bool,
) {
    // Given
    let allocator = oxc_allocator::Allocator::default();
    let source = format!("const made={{child:{{p:1}}}};{consumer}");
    let parsed = oxc_parser::Parser::new(&allocator, &source, oxc_span::SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    // When
    let found = uses(&parsed.program, &|_| false, None);
    // Then
    let [usage] = found["made"].as_slice() else {
        panic!("one receiver use: {found:?}")
    };
    match usage {
        Use::Changes { depth, .. } => {
            assert!(writes);
            assert_eq!(*depth, 2);
        }
        Use::Calls { path, .. } => {
            assert!(!writes);
            assert_eq!(path, &[Some("child".to_string())]);
        }
        Use::Escapes { .. } => panic!("receiver use must not be an escape"),
    }
}
