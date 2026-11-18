//! A component selector `${A}` picks out what `A` renders. Emotion renders
//! `styled(A)` on `A`'s base, without the class the selector reads, while
//! `A.withComponent(tag)` keeps it; styled-components gives `styled(A)` the
//! class of `A` too: compiled modules match the same elements

use super::namespace_module_execution as modules;
use rstest::rstest;
use serial_test::serial;

const EMOTION_BASE: &str = "import styled from '@emotion/styled';\nexport const A = styled.div`color: red;`;\nexport const P = styled.section`${A} { margin: 1px; }`;";

const EMOTION_EXT: &str = "import styled from '@emotion/styled';\nimport { A } from './base.tsx';\nexport const B = styled(A)`width: 100%;`;\nexport const D = styled(B)`height: 1px;`;\nexport const W = A.withComponent('span');\nexport const App = () => [<A />, <B />, <D />, <W />];";

const EMOTION_SAME: &str = "import styled from '@emotion/styled';\nexport const A = styled.div`color: red;`;\nexport const P = styled.section`${A} { margin: 1px; }`;\nexport const B = styled(A)`width: 100%;`;\nexport const D = styled(B)`height: 1px;`;\nexport const W = A.withComponent('span');\nexport const App = () => [<A />, <B />, <D />, <W />];";

const SC_BASE: &str = "import styled from 'styled-components';\nexport const A = styled.div`color: red;`;\nexport const P = styled.section`${A} { margin: 1px; }`;";

const SC_EXT: &str = "import styled from 'styled-components';\nimport { A } from './base.tsx';\nexport const B = styled(A)`width: 100%;`;\nexport const D = styled(B)`height: 1px;`;\nexport const App = () => [<A />, <B />, <D />];";

const OBSERVE: &str = "import {App} from '/t/ext.tsx';globalThis.observed=App().map(e=>{const r=render(e);return [r.target,r.props.className.split(' ')];});";

/// The class `${A} { margin: 1px; }` selects `A` by
fn selector(outputs: &[(String, crate::ExtractOutput)]) -> String {
    let rule = outputs
        .iter()
        .flat_map(|(_, output)| &output.styles)
        .map(|style| format!("{style:?}"))
        .find(|style| style.contains("\"margin\""))
        .unwrap_or_else(|| panic!("no rule selecting the component"));
    let at = rule
        .rfind('.')
        .unwrap_or_else(|| panic!("no class in {rule}"));
    rule[at + 1..]
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        .collect()
}

/// Which elements the selector of `A` matches, as `[element, matched]`
fn matched(outputs: &[(String, crate::ExtractOutput)]) -> serde_json::Value {
    let marker = selector(outputs);
    let observed = modules::execute(outputs, OBSERVE);
    serde_json::Value::Array(
        observed
            .as_array()
            .unwrap_or_else(|| panic!("expected array"))
            .iter()
            .map(|element| {
                let classes = element[1]
                    .as_array()
                    .unwrap_or_else(|| panic!("expected classes"));
                serde_json::json!([element[0], classes.iter().any(|class| *class == *marker)])
            })
            .collect(),
    )
}

/// `A`, `B = styled(A)`, `D = styled(B)` and `W = A.withComponent('span')`,
/// as Emotion matches `${A}`
fn emotion() -> serde_json::Value {
    serde_json::json!([
        ["div", true],
        ["div", false],
        ["div", false],
        ["span", true]
    ])
}

#[test]
#[serial]
fn emotion_selector_when_extension_is_in_the_same_file_matches_the_component_alone() {
    // Given: the component, its selector, extensions and `withComponent` in one file
    let outputs = modules::compile(&[("/t/ext.tsx", EMOTION_SAME)], false);
    // When: they render
    let observed = matched(&outputs);
    // Then: the selector matches `A` and `withComponent`, not the extensions
    assert_eq!(observed, emotion());
}

#[rstest]
#[case::together(true)]
#[case::unknown_to_the_build(false)]
#[serial]
fn emotion_selector_when_extension_is_in_another_file_matches_the_component_alone(
    #[case] together: bool,
) {
    // Given: the extensions in a module importing the component, compiled
    // with the module defining it or without it
    let outputs = if together {
        modules::compile(
            &[("/t/base.tsx", EMOTION_BASE), ("/t/ext.tsx", EMOTION_EXT)],
            false,
        )
    } else {
        let mut outputs = modules::compile(&[("/t/base.tsx", EMOTION_BASE)], false);
        outputs.extend(modules::compile(&[("/t/ext.tsx", EMOTION_EXT)], false));
        outputs
    };
    // When: they render
    let observed = matched(&outputs);
    // Then: the selector matches what Emotion matches
    assert_eq!(observed, emotion());
}

#[rstest]
#[case::together(true)]
#[case::unknown_to_the_build(false)]
#[serial]
fn styled_components_selector_matches_the_extensions(#[case] together: bool) {
    // Given: styled-components extensions of a selected component
    let outputs = if together {
        modules::compile(&[("/t/base.tsx", SC_BASE), ("/t/ext.tsx", SC_EXT)], false)
    } else {
        let mut outputs = modules::compile(&[("/t/base.tsx", SC_BASE)], false);
        outputs.extend(modules::compile(&[("/t/ext.tsx", SC_EXT)], false));
        outputs
    };
    // When: they render
    let observed = matched(&outputs);
    // Then: the extensions keep the class, as styled-components gives it them
    assert_eq!(
        observed,
        serde_json::json!([["div", true], ["div", true], ["div", true]])
    );
}
