//! The props a compiled styled component passes on, as the library it comes
//! from does: styled-components v6 forwards every prop but `$` props and
//! `theme`, while Emotion keeps the props its styles read off a tag.

use std::collections::HashMap;

use boa_engine::{Context, Source};
use rstest::rstest;
use serial_test::serial;

use super::styled_transient_tests::executable_component;
use crate::{ExtractOption, ImportAlias, extract, styled_reads::Forward};

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

/// The props the element renders with, sorted, leaving out what every
/// component sets (`className`, `style`, `as`, `ref`), or the error thrown
fn forwarded(source: &str, extra: &str) -> String {
    let code = library_code(source)
        .replace("import isPropValid from \"@emotion/is-prop-valid\";\n", "")
        .replace(
            "import __devupIsPropValid from \"@emotion/is-prop-valid\";\n",
            "",
        );
    let component = executable_component(code);
    let script = format!(
        r"
        const Link = function Link() {{}};
        const __devupIsPropValid = (k) => k === 'id' || k.startsWith('data-');
        {component}
        let result;
        try {{
            const rendered = Subject.render({{
                tone: 'navy', $tone: 'orchid', customPayload: '42', id: 'kept',
                'data-x': '1'{extra}
            }}, undefined);
            result = Object.keys(rendered.props)
                .filter((k) => !['className', 'style', 'as', 'ref'].includes(k))
                .sort()
                .join(',');
        }} catch (error) {{
            result = 'throws';
        }}
        result
        "
    );
    Context::default()
        .eval(Source::from_bytes(&script))
        .expect("generated component must execute")
        .as_string()
        .expect("text result")
        .to_std_string_escaped()
}

/// Each case renders with `tone`, `$tone`, `customPayload`, `id` and
/// `data-x`. The expected props are what styled-components 6.6.0 and
/// @emotion/styled 11 render, with @emotion/is-prop-valid, under React 19's
/// `renderToStaticMarkup`: the predicate meets every prop, read by a style or
/// not, and styled-components passes it the element it renders.
#[rstest]
#[case::styled_components_never(
    "import styled from 'styled-components'\nconst Subject = styled.div.withConfig({ shouldForwardProp: () => false })`color: ${(p) => p.$tone};`",
    "",
    ""
)]
#[case::styled_components_only_id(
    "import styled from 'styled-components'\nconst Subject = styled.div.withConfig({ shouldForwardProp: (prop) => prop === 'id' })`color: red;`",
    "",
    "id"
)]
#[case::styled_components_valid(
    "import styled from 'styled-components'\nimport isPropValid from '@emotion/is-prop-valid'\nconst Subject = styled.div.withConfig({ shouldForwardProp: (prop) => isPropValid(prop) })`color: red;`",
    "",
    "data-x,id"
)]
#[case::styled_components_tag_element(
    "import styled from 'styled-components'\nimport isPropValid from '@emotion/is-prop-valid'\nconst Subject = styled.div.withConfig({ shouldForwardProp: (prop, element) => typeof element !== 'string' || isPropValid(prop) })`color: red;`",
    "",
    "data-x,id"
)]
#[case::styled_components_component_element(
    "import styled from 'styled-components'\nimport isPropValid from '@emotion/is-prop-valid'\nconst Subject = styled(Link).withConfig({ shouldForwardProp: (prop, element) => 'string' != typeof element || isPropValid(prop) })`color: red;`",
    "",
    "customPayload,data-x,id,tone"
)]
#[case::styled_components_as_component(
    "import styled from 'styled-components'\nimport isPropValid from '@emotion/is-prop-valid'\nconst Subject = styled.div.withConfig({ shouldForwardProp: (prop, element) => !(typeof element === 'string') || isPropValid(prop) })`color: red;`",
    ", as: Link",
    "customPayload,data-x,id,tone"
)]
#[case::styled_components_listed(
    "import styled from 'styled-components'
const Subject = styled.div.withConfig({ shouldForwardProp: (prop) => ['id', 'tone'].includes(prop) })`color: red;`",
    "",
    "id,tone"
)]
#[case::emotion_never(
    "import styled from '@emotion/styled'\nconst Subject = styled('div', { shouldForwardProp: () => false })`color: red;`",
    "",
    ""
)]
#[case::emotion_only_id(
    "import styled from '@emotion/styled'\nconst Subject = styled('div', { shouldForwardProp: (prop) => prop === 'id' })`color: ${(p) => p.tone};`",
    "",
    "id"
)]
#[case::emotion_valid_component(
    "import styled from '@emotion/styled'\nimport isPropValid from '@emotion/is-prop-valid'\nconst Subject = styled(Link, { shouldForwardProp: (prop) => isPropValid(prop) })`color: red;`",
    "",
    "data-x,id"
)]
#[serial]
fn applies_should_forward_prop_to_every_prop_as_the_libraries_do(
    #[case] source: &str,
    #[case] extra: &str,
    #[case] expected: &str,
) {
    // Given: a predicate, and props the styles mostly do not read

    // When: the compiled component renders them
    let props = forwarded(source, extra);

    // Then: the element gets what the library renders it with
    assert_eq!(props, expected, "{source}");
}

#[rstest]
#[case::calls_the_element(
    "import styled from 'styled-components'\nconst A = styled.div.withConfig({ shouldForwardProp: (prop, valid) => valid(prop) })`color: red;`",
    true
)]
#[case::other_code(
    "import styled from 'styled-components'\nconst A = styled.div.withConfig({ shouldForwardProp: (prop, element) => element === 'a' })`color: red;`",
    false
)]
#[case::not_a_function(
    "import styled from 'styled-components'
const A = styled.div.withConfig({ shouldForwardProp: check })`color: red;`",
    false
)]
#[case::one_parameter(
    "import styled from 'styled-components'
const A = styled.div.withConfig({ shouldForwardProp: (prop) => check(prop) })`color: red;`",
    false
)]
#[case::compares_the_type_otherwise(
    "import styled from 'styled-components'
const A = styled.div.withConfig({ shouldForwardProp: (prop, element) => typeof element > 'string' })`color: red;`",
    false
)]
#[case::emotion_validator(
    "import styled from '@emotion/styled'\nconst A = styled('div', { shouldForwardProp: (prop, valid) => valid(prop) })`color: red;`",
    false
)]
#[serial]
fn styled_components_v5_validators_are_build_errors(#[case] source: &str, #[case] v6: bool) {
    // Given: a predicate reading its second argument

    // When: the build compiles it
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
    let result = extract("test.tsx", source, option);

    // Then: styled-components v6 rejects calling the element it passes; a
    // predicate the build cannot read is the usual error, and Devup UI's
    // validator stays readable outside styled-components
    match result {
        Ok(_) => assert!(!v6 && source.contains("@emotion/styled"), "{source}"),
        Err(error) => assert_eq!(
            error.to_string().contains("styled-components v6 passes"),
            v6,
            "{error}"
        ),
    }
}

#[rstest]
#[case::both_pass_every_prop(Forward::Always(true), Forward::Always(true), Forward::Always(true))]
#[case::one_passes_every_prop(
    Forward::Always(true),
    Forward::Is("id".to_string()),
    Forward::Is("id".to_string())
)]
#[case::held_already(
    Forward::And(
        Box::new(Forward::Is("id".to_string())),
        Box::new(Forward::Tag)
    ),
    Forward::Tag,
    Forward::And(
        Box::new(Forward::Is("id".to_string())),
        Box::new(Forward::Tag)
    )
)]
fn composes_predicates_without_repeating_them(
    #[case] left: Forward,
    #[case] right: Forward,
    #[case] expected: Forward,
) {
    // Given: two predicates a prop must both pass

    // When: they are composed
    let composed = Forward::and(left, right);

    // Then: what one already holds, or what passes every prop, adds nothing
    assert_eq!(composed, expected);
}

#[rstest]
#[case::tag(Forward::Tag, true, true)]
#[case::component(Forward::Tag, false, false)]
#[case::not_a_tag(Forward::Not(Box::new(Forward::Tag)), true, false)]
fn tests_whether_the_element_is_a_tag(
    #[case] rule: Forward,
    #[case] tag: bool,
    #[case] expected: bool,
) {
    // Given: a predicate reading the element styled-components renders

    // When: the build asks it about an element
    let forwards = rule.forwards("tone", tag);

    // Then: it reads whether the element is a tag, leaving the rest to the runtime
    assert_eq!(forwards, expected);
    assert!(rule.reads_element());
}

fn library_option() -> ExtractOption {
    ExtractOption {
        package: "@devup-ui/core".to_string(),
        css_dir: "@devup-ui/core".to_string(),
        single_css: true,
        import_main_css: false,
        import_aliases: HashMap::from([(
            "styled-components".to_string(),
            ImportAlias::DefaultToNamed("styled".to_string()),
        )]),
    }
}

#[rstest]
#[case::module(
    "test.tsx",
    "import __devupIsPropValid from \"@emotion/is-prop-valid\";"
)]
#[case::commonjs(
    "test.cjs",
    "const __devupIsPropValid = require(\"@emotion/is-prop-valid\").default;"
)]
#[serial]
fn imports_is_prop_valid_where_the_predicate_runs(#[case] filename: &str, #[case] binding: &str) {
    // Given: a predicate calling `isPropValid`, in a module or CommonJS
    let _state = super::test_state::TestStateGuard::new();
    super::reset_class_map();
    super::reset_file_map();
    let source = "import styled from 'styled-components'\nexport const A = styled.div.withConfig({ shouldForwardProp: (prop) => isPropValid(prop) })`color: red;`";

    // When: the build compiles it
    let code = extract(filename, source, library_option()).unwrap().code;

    // Then: the runtime filter calls the library the build evaluated it as
    assert!(code.contains(binding), "{code}");
    assert!(code.contains("__devupIsPropValid(k)"), "{code}");
}

#[test]
#[serial]
fn imports_is_prop_valid_where_an_extension_of_another_module_renders() {
    // Given: a component of another module filtering with `isPropValid`
    let _state = super::test_state::TestStateGuard::new();
    super::reset_class_map();
    super::reset_file_map();
    let resolver = |specifier: &str, _: &str| {
        (specifier == "./base").then(|| crate::ResolvedModule {
            path: "/src/base.ts".to_string(),
            code: "import styled from 'styled-components'\nimport isPropValid from '@emotion/is-prop-valid'\nexport const Base = styled.div.withConfig({ shouldForwardProp: (prop) => isPropValid(prop) })`color: red;`".to_string(),
        })
    };

    // When: this module extends it
    let code = crate::extract_with_modules(
        "/src/App.tsx",
        "import styled from 'styled-components'\nimport { Base } from './base'\nexport const A = styled(Base)`margin: 1px;`",
        library_option(),
        false,
        &resolver,
    )
    .unwrap()
    .code;

    // Then: the filter it renders with has the function it calls
    assert!(
        code.contains("import __devupIsPropValid from \"@emotion/is-prop-valid\";"),
        "{code}"
    );
    assert!(code.contains("__devupIsPropValid(k)"), "{code}");
}
