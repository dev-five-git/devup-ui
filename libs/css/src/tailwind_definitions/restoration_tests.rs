use super::*;
use crate::test_state::TestStateGuard;
use serial_test::serial;

#[test]
#[serial]
fn complete_definitions_restore_when_nested_or_unwinding() {
    let _state = TestStateGuard::new();
    for reset in ["--color-*: initial;", "--*: initial;"] {
        let source = format!(
            "@theme {{{reset} --color-brand: teal; @keyframes bounce {{to {{opacity: 0.5}}}}}} \
             @utility card {{color: red; @apply p-4;}} \
             @utility tab-* {{tab-size: --value(integer);}} \
             @custom-variant night (&:where(.night *));"
        );
        let expected = read(&source).unwrap_or_else(|| panic!("invalid definitions fixture"));
        assert_eq!(expected.reset_all, reset == "--*: initial;");
        assert_eq!(
            expected.resets.contains("--color-"),
            reset == "--color-*: initial;"
        );
        assert_eq!(expected.theme.len(), 1);
        assert_eq!(expected.utilities.len(), 1);
        assert_eq!(expected.functional.len(), 1);
        assert_eq!(expected.variants.len(), 1);
        assert_eq!(expected.keyframes.len(), 1);
        set_tailwind_css(&source);
        {
            let _inner = TestStateGuard::new();
            assert_eq!(
                *DEFINITIONS.read().unwrap_or_else(|error| panic!("{error}")),
                Definitions::default()
            );
            set_tailwind_css("@utility inner {color: blue;}");
        }
        assert_eq!(
            *DEFINITIONS.read().unwrap_or_else(|error| panic!("{error}")),
            expected
        );
        let result = std::panic::catch_unwind(|| {
            let _inner = TestStateGuard::new();
            set_tailwind_css("@theme {--*: initial;}");
            panic!("restore definitions");
        });
        assert!(result.is_err());
        assert_eq!(
            *DEFINITIONS.read().unwrap_or_else(|error| panic!("{error}")),
            expected
        );
    }
}

#[test]
#[serial]
fn replacement_recovers_when_registry_is_poisoned() {
    let _state = TestStateGuard::new();
    set_tailwind_css("@utility card {color: red;}");
    let result = std::panic::catch_unwind(|| {
        let _lock = DEFINITIONS
            .write()
            .unwrap_or_else(|error| panic!("{error}"));
        panic!("poison registry");
    });
    assert!(result.is_err());
    let old = replace_definitions(Definitions::default());
    assert_eq!(old.utilities.len(), 1);
    assert!(!DEFINITIONS.is_poisoned());
    assert_eq!(custom_utility("card"), None);
    replace_definitions(old);
    assert_eq!(
        custom_utility("card"),
        Some(vec![UtilityItem::Declaration("color".into(), "red".into())])
    );
}
