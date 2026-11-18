//! The props a compiled styled component passes on, as the library it comes
//! from does: styled-components v6 forwards every prop but `$` props and
//! `theme`, while Emotion keeps the props its styles read off a tag.

use std::collections::HashMap;

use boa_engine::{Context, Source};
use rstest::rstest;
use serial_test::serial;

use super::styled_transient_tests::executable_component;
use crate::{ExtractOption, ImportAlias, extract};

fn library_code(source: &str) -> String {
    let _state = super::test_state::TestStateGuard::new();
    super::reset_class_map();
    super::reset_file_map();
    let option = ExtractOption {
        package: "@devup-ui/core".to_string(),
        css_dir: "@devup-ui/core".to_string(),
        single_css: true,
        import_main_css: false,
        import_aliases: HashMap::from([
            (
                "styled-components".to_string(),
                ImportAlias::DefaultToNamed("styled".to_string()),
            ),
            (
                "@emotion/styled".to_string(),
                ImportAlias::DefaultToNamed("styled".to_string()),
            ),
        ]),
    };
    extract("test.tsx", source, option).unwrap().code
}

/// Whether the rendered element gets `tone`, `customPayload`, `theme` and a
/// `$` prop, and the `tone` the style read
fn rendered(source: &str) -> serde_json::Value {
    let component = executable_component(library_code(source));
    let script = format!(
        r"
        const Link = 'component';
        {component}
        const props = Subject.render({{
            tone: 'navy', $tone: 'orchid', customPayload: 42,
            theme: {{ marker: true }}, id: 'kept'
        }}, undefined).props;
        JSON.stringify([
            'tone' in props, 'customPayload' in props, 'theme' in props,
            Object.keys(props).some((k) => k[0] === '$'), props.id,
            Object.values(props.style || {{}}).includes('navy')
        ])
        "
    );
    let value = Context::default()
        .eval(Source::from_bytes(&script))
        .expect("generated component must execute");
    serde_json::from_str(
        &value
            .as_string()
            .expect("JSON result")
            .to_std_string_escaped(),
    )
    .unwrap()
}

#[rstest]
#[case::styled_components_tag(
    "import styled from 'styled-components'",
    "styled.div",
    [true, true, false]
)]
#[case::styled_components_call(
    "import styled from 'styled-components'",
    "styled('div')",
    [true, true, false]
)]
#[case::styled_components_component(
    "import styled from 'styled-components'",
    "styled(Link)",
    [true, true, false]
)]
#[case::styled_components_predicate(
    "import styled from 'styled-components'",
    "styled.div.withConfig({ shouldForwardProp: (prop) => prop !== 'tone' })",
    [false, true, false]
)]
#[case::styled_components_always(
    "import styled from 'styled-components'",
    "styled.div.withConfig({ shouldForwardProp: () => true })",
    [true, true, false]
)]
#[case::styled_components_composed(
    "import styled from 'styled-components'",
    "styled(Link).withConfig({ shouldForwardProp: (prop) => prop !== 'tone' }).withConfig({ shouldForwardProp: (prop) => prop !== 'customPayload' })",
    [true, false, false]
)]
#[case::emotion_tag("import styled from '@emotion/styled'", "styled.div", [false, true, false])]
#[case::emotion_component(
    "import styled from '@emotion/styled'",
    "styled(Link)",
    [true, true, true]
)]
#[case::emotion_predicate(
    "import styled from '@emotion/styled'",
    "styled('div', { shouldForwardProp: (prop) => prop !== 'customPayload' })",
    [true, false, true]
)]
#[serial]
fn forwards_the_props_the_library_forwards(
    #[case] import: &str,
    #[case] factory: &str,
    #[case] forwarded: [bool; 3],
) {
    // Given: a style reading `tone` and `$tone`, from either library
    let source = format!(
        "{import}\nconst Subject = {factory}`color: ${{(p) => p.$tone}}; background: ${{(p) => p.tone}};`"
    );

    // When: the compiled component renders
    let result = rendered(&source);

    // Then: `tone`, `customPayload` and `theme` reach the element as the
    // library passes them, `$` props never do, and the style still reads `tone`
    let [tone, payload, theme] = forwarded;
    assert_eq!(
        result,
        serde_json::json!([tone, payload, theme, false, "kept", true]),
        "{source}"
    );
}
