//! A component selector `${A}` matches by a marker class on what `A`
//! renders. A component the file keeps to itself and no selector names can
//! never be selected, so it renders without a marker; one a selector here
//! names, or one the file exports, which a module importing it may select,
//! keeps its marker

use crate::{ExtractOption, ImportAlias, extract};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use rstest::rstest;
use serial_test::serial;
use std::collections::HashMap;

const EMOTION: &str = "import styled from '@emotion/styled';\n";
const STYLED_COMPONENTS: &str = "import styled from 'styled-components';\n";

fn option() -> ExtractOption {
    ExtractOption {
        single_css: true,
        import_aliases: HashMap::from([
            (
                "@emotion/styled".to_string(),
                ImportAlias::DefaultToNamed("styled".to_string()),
            ),
            (
                "styled-components".to_string(),
                ImportAlias::DefaultToNamed("styled".to_string()),
            ),
        ]),
        ..ExtractOption::default()
    }
}

/// The marker classes the compiled code gives elements, in order, once each
fn markers(code: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for token in code.split(|c: char| !c.is_ascii_alphanumeric() && c != '-') {
        if token.contains("--") && !found.iter().any(|marker| marker == token) {
            found.push(token.to_string());
        }
    }
    found
}

#[rstest]
#[case::private_rendered(
    "const Child = styled.div`color: red;`;\nexport const App = () => <Child />;",
    &[]
)]
#[case::private_extended_privately(
    "const Child = styled.div`color: red;`;\nconst Wide = styled(Child)`width: 1px;`;\nexport const App = () => <><Child /><Wide /></>;",
    &[]
)]
#[case::private_base_of_an_export(
    "const Child = styled.div`color: red;`;\nexport const Wide = styled(Child)`width: 1px;`;",
    &[("Wide", 0)]
)]
#[case::private_selected(
    "const Child = styled.div`color: red;`;\nconst Parent = styled.div`${Child} { color: blue; }`;\nexport const App = () => <Parent><Child /></Parent>;",
    &[("Child", 1)]
)]
#[case::exported(
    "export const Child = styled.div`color: red;`;",
    &[("Child", 0)]
)]
#[case::exported_by_default(
    "const Child = styled.div`color: red;`;\nexport default Child;",
    &[("default", 0)]
)]
#[serial]
fn markers_go_only_where_a_selector_can_read_them(
    #[values(EMOTION, STYLED_COMPONENTS)] import: &str,
    #[case] source: &str,
    #[case] expected: &[(&str, usize)],
) {
    // Given
    reset_class_map();
    reset_file_map();
    // When
    let output = extract("test.tsx", &format!("{import}{source}"), option())
        .unwrap_or_else(|error| panic!("{source}\n{error}"));
    // Then
    assert_eq!(
        markers(&output.code),
        expected_markers(expected),
        "{import}{source}"
    );
}

#[rstest]
#[case::private("const Other = Child.withComponent('a');\nexport const App = () => <><Child /><Other /></>;", &[])]
#[case::exported("export const Other = Child.withComponent('a');", &[("Other", 1)])]
#[serial]
fn emotion_with_component_results_share_a_marker_only_when_they_can_be_selected(
    #[case] source: &str,
    #[case] expected: &[(&str, usize)],
) {
    // Given
    reset_class_map();
    reset_file_map();
    let code = format!("{EMOTION}const Child = styled.div`color: red;`;\n{source}");
    // When
    let output =
        extract("test.tsx", &code, option()).unwrap_or_else(|error| panic!("{code}\n{error}"));
    // Then
    assert_eq!(markers(&output.code), expected_markers(expected), "{code}");
}

/// The classes of the markers `test.tsx` gives its components, by name and
/// index among the components of the file
fn expected_markers(expected: &[(&str, usize)]) -> Vec<String> {
    expected
        .iter()
        .map(|(name, index)| css::component_marker(name, *index, "test.tsx"))
        .collect()
}

const CHILD_MODULE: &str = "import styled from '@emotion/styled';\nexport const Child = styled.div`color: red;`;\nexport const Other = styled.span`color: blue;`;";

fn child_module(specifier: &str, _: &str) -> Option<crate::ResolvedModule> {
    (specifier == "./child").then(|| crate::ResolvedModule {
        path: "/src/child.tsx".to_string(),
        code: CHILD_MODULE.to_string(),
    })
}

#[rstest]
#[case::exported("export const Child = styled.div`color: red;`;", &[("Child", 0)], &[])]
#[case::exported_anonymous_default("export default styled.div`color: red;`;", &[("default", 0)], &[])]
#[case::exported_and_selected_here(
    "export const Child = styled.div`color: red;`;\nexport const Parent = styled.div`${Child} { color: blue; }`;",
    &[("Child", 0), ("Parent", 1)],
    &[("Child", 0)]
)]
#[case::private_selected(
    "const Child = styled.div`color: red;`;\nconst Parent = styled.div`${Child} { color: blue; }`;\nexport const App = () => <Parent><Child /></Parent>;",
    &[("Child", 1)],
    &[("Child", 1)]
)]
#[case::shared_with_component_target(
    "const Child = styled.div`color: red;`;\nconst Link = Child.withComponent('a');\nexport const Parent = styled.div`${Link} { color: blue; }`;\nexport const App = () => <Link />;",
    &[("Parent", 1), ("Child", 2)],
    &[("Child", 2)]
)]
#[serial]
fn extraction_lists_the_markers_it_gives_and_selects_by(
    #[case] source: &str,
    #[case] given: &[(&str, usize)],
    #[case] selected: &[(&str, usize)],
) {
    // Given
    reset_class_map();
    reset_file_map();
    let code = format!("{EMOTION}{source}");
    // When
    let output =
        extract("test.tsx", &code, option()).unwrap_or_else(|error| panic!("{code}\n{error}"));
    // Then
    assert_eq!(
        sorted(output.component_markers.given),
        sorted(expected_markers(given)),
        "{code}"
    );
    assert_eq!(
        sorted(output.component_markers.selected),
        sorted(expected_markers(selected)),
        "{code}"
    );
}

fn sorted(mut markers: Vec<String>) -> Vec<String> {
    markers.sort();
    markers
}

#[rstest]
#[case::named(
    "import { Child } from './child';\nexport const Parent = styled.div`${Child} { color: green; }`;",
    1
)]
#[case::namespace(
    "import * as kit from './child';\nexport const Parent = styled.div`${kit.Child} { color: green; }`;",
    1
)]
#[serial]
fn markers_of_imported_components_are_selected_by_the_importer(
    #[case] source: &str,
    #[case] selected: usize,
) {
    // Given
    reset_class_map();
    reset_file_map();
    let code = format!("{EMOTION}{source}");
    // When
    let output =
        crate::extract_with_modules("/src/parent.tsx", &code, option(), false, &child_module)
            .unwrap_or_else(|error| panic!("{code}\n{error}"));
    // Then
    let markers = &output.component_markers;
    assert_eq!(markers.selected.len(), selected, "{code}\n{markers:?}");
    assert_eq!(markers.given.len(), 1, "{code}\n{markers:?}");
    assert!(
        markers
            .selected
            .iter()
            .all(|marker| !markers.given.contains(marker)),
        "{code}\n{markers:?}"
    );
}
