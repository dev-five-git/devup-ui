//! Emotion's `A.withComponent(tag)` creates the component again on `tag`
//! with `A`'s styles and options, so it keeps `A`'s `target`, `as` still
//! replaces the tag, and `styled(W)` renders it without the target: compiled
//! modules render the same, wherever `A` comes from and whether or not the
//! build sees it

use super::namespace_module_execution as modules;
use rstest::rstest;
use serial_test::serial;

const BASE: &str = "import styled from '@emotion/styled';\nexport const A = styled('div', { target: 'tA' })`color: red;`;";

const LINK: &str = "export const Link = (props) => <a {...props} />;";

const LINK_BASE: &str = "import styled from '@emotion/styled';\nimport { Link } from './link.tsx';\nexport const A = styled(Link, { target: 'tA' })`color: red;`;";

const BINDING_BASE: &str = "import styled from '@emotion/styled';\nlet tone = 'red';\nexport const setTone = (next) => { tone = next; };\nexport const A = styled('div', { target: 'tA' })({ color: tone });";

const DYNAMIC_BASE: &str = "import styled from '@emotion/styled';\nimport { Link } from './link.tsx';\nexport const A = styled(Link, { target: 'tA' })(({ $c }) => ({ color: $c }));";

const EXPORTED_BASE: &str = "import styled from '@emotion/styled';\nconst A = styled('div', { target: 'tA' })`color: red;`;\nexport const W = A.withComponent('span');";

const EXT: &str = "import styled from '@emotion/styled';\nimport { A } from './base.tsx';\nexport const W = A.withComponent('span');\nexport const X = W.withComponent('nav');\nexport const E = styled(W)`width: 1px;`;\nexport const App = () => [<W />, <X />, <E />, <W as=\"b\" />];";

const EXPORTED_EXT: &str = "import styled from '@emotion/styled';\nimport { W } from './base.tsx';\nexport const X = W.withComponent('nav');\nexport const E = styled(W)`width: 1px;`;\nexport const App = () => [<W />, <X />, <E />, <W as=\"b\" />];";

const OBSERVE: &str = "import {App} from '/t/ext.tsx';globalThis.observed=App().map(e=>{const r=render(e);return [r.target,r.props.className.split(' ').includes('tA')];});";

/// What `W = A.withComponent('span')`, `W.withComponent('nav')`,
/// `styled(W)` and `<W as="b" />` render under Emotion, as
/// `[element, has tA]`
fn emotion() -> serde_json::Value {
    serde_json::json!([["span", true], ["nav", true], ["span", false], ["b", true]])
}

#[rstest]
#[case::cross_file(&[("/t/base.tsx", BASE)], EXT)]
#[case::runtime_base(&[("/t/link.tsx", LINK), ("/t/base.tsx", LINK_BASE)], EXT)]
#[case::module_binding(&[("/t/base.tsx", BINDING_BASE)], EXT)]
#[case::dynamic(&[("/t/link.tsx", LINK), ("/t/base.tsx", DYNAMIC_BASE)], EXT)]
#[case::exported(&[("/t/base.tsx", EXPORTED_BASE)], EXPORTED_EXT)]
#[serial]
fn with_component_when_component_is_imported_renders_as_emotion(
    #[case] base: &[(&str, &str)],
    #[case] ext: &str,
) {
    // Given: `withComponent` on an Emotion component another module defines
    let mut files = base.to_vec();
    files.push(("/t/ext.tsx", ext));
    // When: the modules compile together and render
    let observed = modules::execute(&modules::compile(&files, false), OBSERVE);
    // Then: they render what Emotion renders
    assert_eq!(observed, emotion());
}

const RUNTIME_EXT: &str = "import styled from '@emotion/styled';\nimport { A } from './base.tsx';\nconst pick = (tag) => tag;\nexport const W = A.withComponent(pick('span'));\nexport const X = W.withComponent(pick('nav'));\nexport const E = styled(W)`width: 1px;`;\nexport const App = () => [<W />, <X />, <E />, <W as=\"b\" />];";

const RUNTIME_SAME: &str = "import styled from '@emotion/styled';\nconst A = styled('div', { target: 'tA' })`color: red;`;\nconst pick = (tag) => tag;\nexport const W = A.withComponent(pick('span'));\nexport const X = W.withComponent(pick('nav'));\nexport const E = styled(W)`width: 1px;`;\nexport const App = () => [<W />, <X />, <E />, <W as=\"b\" />];";

#[test]
#[serial]
fn with_component_when_target_is_known_at_runtime_renders_as_emotion() {
    // Given: a target only the runtime knows, on a component the build reads
    // in another module and in the same one
    let together = modules::compile(&[("/t/base.tsx", BASE), ("/t/ext.tsx", RUNTIME_EXT)], false);
    let same = modules::compile(&[("/t/ext.tsx", RUNTIME_SAME)], false);
    // When: they render
    let observed = [
        modules::execute(&together, OBSERVE),
        modules::execute(&same, OBSERVE),
    ];
    // Then: the component renders it at runtime, as Emotion's does
    assert_eq!(observed, [emotion(), emotion()]);
}

#[rstest]
#[case(
    "A.withComponent('span', { target: 'tW' })",
    "\"span\", { target: \"tW\" }"
)]
#[case("A.withComponent()", "")]
#[case("A.withComponent(...tags)", "...tags")]
#[serial]
fn with_component_when_options_are_given_is_a_build_error(#[case] call: &str, #[case] shown: &str) {
    // Given: an Emotion component given options or no single target
    let source = format!(
        "import styled from '@emotion/styled';\nconst A = styled('div', {{ target: 'tA' }})`color: red;`;\nexport const W = {call};"
    );
    let option = crate::ExtractOption {
        import_aliases: std::collections::HashMap::from([(
            "@emotion/styled".into(),
            crate::ImportAlias::DefaultToNamed("styled".into()),
        )]),
        ..crate::ExtractOption::default()
    };
    // When: it compiles
    let error = crate::extract("test.tsx", &source, option)
        .err()
        .map(|error| error.to_string());
    // Then: the build reports the options it cannot give the component
    assert_eq!(
        error.as_deref(),
        Some(
            format!(
                "test.tsx:3:18: `withComponent()` cannot use `{shown}` at build time: it takes one tag or component, as options beside it would give the component classes and props the build cannot see. Fix: pass the tag or component alone, or create the component with `styled(tag, options)`"
            )
            .as_str()
        )
    );
}

#[rstest]
#[case::cross_file(BASE, EXT)]
#[case::exported(EXPORTED_BASE, EXPORTED_EXT)]
#[case::runtime_target(BASE, RUNTIME_EXT)]
#[serial]
fn with_component_when_component_is_unknown_to_the_build_renders_as_emotion(
    #[case] base: &str,
    #[case] ext: &str,
) {
    // Given: `withComponent` compiled without the module defining the component
    let mut outputs = modules::compile(&[("/t/base.tsx", base)], false);
    outputs.extend(modules::compile(&[("/t/ext.tsx", ext)], false));
    // When: the modules render together
    let observed = modules::execute(&outputs, OBSERVE);
    // Then: the compiled component takes `withComponent` at runtime, as
    // Emotion's does
    assert_eq!(observed, emotion());
}
