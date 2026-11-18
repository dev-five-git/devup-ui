//! Emotion creates `W = A.withComponent(tag)` with `A`'s options, so `W`
//! shares `A`'s target: `${W}` selects what `A` renders as well as `W`, and
//! `styled(W)` renders without it, as `styled(A)` does. Compiled modules give
//! `W` the marker of `A` instead of one of its own

use super::namespace_module_execution as modules;
use rstest::rstest;
use serial_test::serial;

/// `A`, `W`, `styled(W)`, `styled(A)` and a `withComponent` result of `W`
const SAME: &str = "import styled from '@emotion/styled';\nEXPORT_A const A = styled.div`color: red;`;\nEXPORT_W const W = A.withComponent('span');\nconst X = W.withComponent('b');\nexport const SW = styled(W)`width: 1px;`;\nconst B = styled(A)`width: 1px;`;\nexport const P = styled.section`${SEL} { margin: 1px; }`;\nexport const App = () => [<A />, <W />, <SW />, <B />, <X />];";

/// A module exporting `W`, keeping `A` to itself
const BASE: &str = "import styled from '@emotion/styled';\nconst A = styled.div`color: red;`;\nexport const W = A.withComponent('span');\nexport const X = W.withComponent('b');\nexport const B = styled(A)`width: 1px;`;\nexport const Base = () => <A />;";

/// A module selecting and extending the `W` it imports
const EXT: &str = "import styled from '@emotion/styled';\nimport { W, X, B, Base } from './base.tsx';\nexport const SW = styled(W)`width: 1px;`;\nexport const P = styled.section`${SEL} { margin: 1px; }`;\nexport const App = () => [Base(), <W />, <SW />, <B />, <X />];";

/// A module exporting `A` alone
const RECEIVER: &str = "import styled from '@emotion/styled';\nexport const A = styled.div`color: red;`;\nexport const B = styled(A)`width: 1px;`;";

/// A module making `W` of the `A` it imports, and selecting it
const IMPORTER: &str = "import styled from '@emotion/styled';\nimport { A, B } from './base.tsx';\nconst W = A.withComponent('span');\nconst X = W.withComponent('b');\nconst SW = styled(W)`width: 1px;`;\nexport const P = styled.section`${SEL} { margin: 1px; }`;\nexport const App = () => [<A />, <W />, <SW />, <B />, <X />];";

const OBSERVE: &str = "import {App} from '/t/ext.tsx';globalThis.observed=App().map(e=>{const r=render(e);return [r.target,r.props.className.split(' ')];});";

/// The class the `margin` rule selects
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

/// Which elements the selector matches, as `[element, matched]`
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

/// `A`, `W`, `styled(W)`, `styled(A)` and `W.withComponent('b')`, as Emotion
/// matches `${A}`, `${W}` and `${X}` alike
fn emotion() -> serde_json::Value {
    serde_json::json!([
        ["div", true],
        ["span", true],
        ["span", false],
        ["div", false],
        ["b", true]
    ])
}

/// `template` with the selected component and, in a module of its own,
/// the exports of `A` and `W`
fn source(template: &str, selected: &str, exports: (&str, &str)) -> String {
    template
        .replace("SEL", selected)
        .replace("EXPORT_A ", exports.0)
        .replace("EXPORT_W ", exports.1)
}

#[rstest]
#[case::local("", "")]
#[case::exported_result("", "export ")]
#[case::exported_component("export ", "")]
#[case::exported("export ", "export ")]
#[serial]
fn emotion_with_component_selector_in_the_same_file_matches_the_component_too(
    #[case] component: &str,
    #[case] result: &str,
    #[values("W", "A", "X")] selected: &str,
) {
    // Given: `${selected}` beside `A`, its `withComponent` results and extensions
    let code = source(SAME, selected, (component, result));
    let outputs = modules::compile(&[("/t/ext.tsx", code.as_str())], false);
    // When: they render
    let observed = matched(&outputs);
    // Then: the selector matches `A` and its results, not the extensions
    assert_eq!(observed, emotion());
}

#[rstest]
#[serial]
fn emotion_with_component_selector_of_an_imported_result_matches_the_component_too(
    #[values("W", "X")] selected: &str,
) {
    // Given: a module selecting the `withComponent` result another exports
    let ext = source(EXT, selected, ("", ""));
    let outputs = modules::compile(
        &[("/t/base.tsx", BASE), ("/t/ext.tsx", ext.as_str())],
        false,
    );
    // When: they render
    let observed = matched(&outputs);
    // Then: the selector matches `A` and its results, not the extensions
    assert_eq!(observed, emotion());
}

#[rstest]
#[serial]
fn emotion_with_component_selector_of_an_imported_component_matches_the_results_too(
    #[values("W", "A", "X")] selected: &str,
) {
    // Given: a module making `withComponent` results of an imported component
    let importer = source(IMPORTER, selected, ("", ""));
    let outputs = modules::compile(
        &[("/t/base.tsx", RECEIVER), ("/t/ext.tsx", importer.as_str())],
        false,
    );
    // When: they render
    let observed = matched(&outputs);
    // Then: the selector matches `A` and its results, not the extensions
    assert_eq!(observed, emotion());
}
