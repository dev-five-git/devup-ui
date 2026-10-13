use serial_test::serial;

use super::visit;

#[test]
#[serial]
fn class_names_calls_follow_the_bindings_the_child_function_declares() {
    let visited = visit(
        "import { ClassNames } from '@devup-ui/react/compat';\n\
         export const a = <ClassNames>{({ css, cx }) => <div className={cx(css({ color: 'red' }), { on: flag })} />}</ClassNames>;\n\
         export const b = <ClassNames>{({ css }) => <div className={((css) => css({ color: 'blue' }))((x) => x)} />}</ClassNames>;\n\
         export const c = ({ ClassNames }) => <ClassNames>{({ css }) => <div className={css({ color: 'green' })} />}</ClassNames>;\n\
         export const d = <ClassNames>{({ css: paint }) => <div className={paint`color: orange;`} />}</ClassNames>;",
    );
    assert_eq!(visited.errors, Vec::<String>::new());
    assert_eq!(visited.styles, 2);
    for expected in [
        "export const a = <div className={((__devupValue0) => `a ${__devupValue0 ? \"on\" : \"\"}`)(flag)} />;",
        "((css) => css({ color: \"blue\" }))((x) => x)",
        "<ClassNames>{({ css }) => <div className={css({ color: \"green\" })} />}</ClassNames>",
        "export const d = <div className={\"b\"} />;",
    ] {
        assert!(
            visited.code.contains(expected),
            "{expected}: {}",
            visited.code
        );
    }
}

#[test]
#[serial]
fn class_names_reports_reads_of_the_bindings_it_takes_only() {
    let visited = visit(
        "import { ClassNames } from '@devup-ui/react/compat';\n\
         export const a = <ClassNames>{({ css, theme }) => <div className={[css, theme]} />}</ClassNames>;\n\
         export const b = <ClassNames>{({ css, theme }) => <div className={((css, theme) => [css, theme])(1, 2)} />}</ClassNames>;",
    );
    assert_eq!(visited.errors.len(), 1, "{:?}", visited.errors);
    assert!(
        visited.errors[0].contains("`<ClassNames>` cannot use `css`"),
        "{:?}",
        visited.errors
    );
}
