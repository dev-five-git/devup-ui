//! Runtime-only transient props at the generated styled forwarding boundary.

use boa_engine::{Context, Source};
use oxc_allocator::Allocator;
use oxc_ast::ast::{JSXAttributeItem, JSXAttributeName, JSXAttributeValue, JSXElement};
use oxc_ast_visit::Visit;
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType, Span};
use rstest::rstest;
use serial_test::serial;

use super::readable_code;

/// Replace only the generated JSX renderer with an observable target/props record.
/// All generated forwarding, style reads and attrs expressions execute unchanged.
#[derive(Default)]
struct Renderer {
    replacement: Option<(Span, String)>,
}

impl<'a> Visit<'a> for Renderer {
    fn visit_jsx_element(&mut self, element: &JSXElement<'a>) {
        assert_eq!(self.replacement, None, "fixture must generate one renderer");
        assert_eq!(element.children.len(), 0);
        let mut properties = Vec::new();
        for attribute in &element.opening_element.attributes {
            match attribute {
                JSXAttributeItem::SpreadAttribute(spread) => properties.push(format!(
                    "...({})",
                    crate::utils::readable_code(&spread.argument)
                )),
                JSXAttributeItem::Attribute(attribute) => {
                    let JSXAttributeName::Identifier(name) = &attribute.name else {
                        panic!("generated renderer must use named attributes");
                    };
                    let Some(JSXAttributeValue::ExpressionContainer(value)) = &attribute.value
                    else {
                        panic!("generated renderer must use expression attributes");
                    };
                    let expression = value.expression.as_expression().unwrap();
                    properties.push(format!(
                        "\"{}\": ({})",
                        name.name,
                        crate::utils::readable_code(expression)
                    ));
                }
            }
        }
        self.replacement = Some((
            element.span(),
            format!(
                "({{ target: DevupAs, props: {{ {} }} }})",
                properties.join(", ")
            ),
        ));
    }
}

fn executable_component(code: String) -> String {
    let code = code
        .strip_prefix("import \"@devup-ui/core/devup-ui.css\";\n")
        .unwrap_or(&code)
        .to_owned();
    let code = code
        .strip_prefix("import { forwardRef as __devupForwardRef } from \"react\";\n")
        .expect("generated styled component must import forwardRef");
    let mut code = format!("const __devupForwardRef = render => ({{ render }});\n{code}");
    assert!(
        !code.contains("import "),
        "unexpected generated import: {code}"
    );
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &code, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{code}");
    let mut renderer = Renderer::default();
    renderer.visit_program(&parsed.program);
    let (span, replacement) = renderer.replacement.expect("generated styled renderer");
    code.replace_range(
        usize::try_from(span.start).unwrap()..usize::try_from(span.end).unwrap(),
        &replacement,
    );
    code
}

#[rstest]
#[case::tag_default("styled('div')", (false, false))]
#[case::component_default("styled(Link)", (true, true))]
#[case::tag_predicate(
    "styled('div', { shouldForwardProp: (prop) => prop !== 'tone' })",
    (true, false)
)]
#[case::component_predicate(
    "styled(Link, { shouldForwardProp: (prop) => prop !== 'tone' })",
    (true, false)
)]
#[case::tag_always_true("styled('div', { shouldForwardProp: () => true })", (true, true))]
#[case::component_always_true("styled(Link, { shouldForwardProp: () => true })", (true, true))]
#[serial]
fn filters_runtime_transients_when_forwarding_styled_rest(
    #[case] factory: &str,
    #[case] forwarded: (bool, bool),
    #[values("", ".attrs((p) => ({ ...p.extra, id: p.$attrs }))")] attrs: &str,
) {
    // Given: transient keys supplied only at execution time, never in styled source.
    let source = format!(
        "import {{styled}} from '@devup-ui/core'\n\
         const Subject = {factory}{attrs}`color: ${{(p) => p.$tone}}; background: ${{(p) => p.tone}};`"
    );
    let code = readable_code(&source);
    for runtime_key in ["$runtimeOnly", "$🦀", "$fromAttrs"] {
        assert!(
            !code.contains(runtime_key),
            "runtime key enumerated: {code}"
        );
    }
    let component = executable_component(code);
    let script = format!(
        r"
        const Link = 'component';
        {component}
        const input = {{
            $runtimeOnly: 'secret', '$': 1, '$🦀': 2,
            $tone: 'orchid', $attrs: 'attrs-id', tone: 'navy',
            theme: {{ marker: true }}, id: 'caller-id',
            'data-ok': 'yes', 'aria-label': 'label',
            customPayload: 42, children: 'child',
            className: 'caller', style: {{ opacity: 0.25 }},
            forwardedAs: 'forwarded-target',
            extra: {{ $fromAttrs: 'secret', title: 'attrs-title' }}
        }};
        const before = JSON.stringify(input);
        const rendered = [Subject.render(input, undefined), Subject.render({{ ...input, as: 'override' }}, undefined)];
        JSON.stringify({{
            transients: rendered.map((r) => Object.keys(r.props).filter((k) => k.startsWith('$'))),
            styleValues: rendered.map((r) => Object.values(r.props.style)),
            targets: rendered.map((r) => r.target),
            preserved: rendered.map((r) => [
                r.props.id, r.props['data-ok'], r.props['aria-label'],
                r.props.customPayload, r.props.children, r.props.as,
                r.props.className.includes('caller'), r.props.style.opacity,
                'theme' in r.props, 'tone' in r.props, r.props.title || null
            ]),
            unchanged: before === JSON.stringify(input)
        }})
        "
    );

    // When: execute the Rust-produced component, including attrs and both as paths.
    let value = Context::default()
        .eval(Source::from_bytes(&script))
        .expect("generated component must execute");
    let result: serde_json::Value = serde_json::from_str(
        &value
            .as_string()
            .expect("JSON result")
            .to_std_string_escaped(),
    )
    .unwrap();

    // Then: no transient reaches either target, while style reads retain its value.
    assert_eq!(result["transients"], serde_json::json!([[], []]));
    for values in result["styleValues"].as_array().unwrap() {
        assert!(
            values
                .as_array()
                .unwrap()
                .contains(&serde_json::json!("orchid"))
        );
        assert!(
            values
                .as_array()
                .unwrap()
                .contains(&serde_json::json!("navy"))
        );
    }
    let target = if factory.starts_with("styled(Link") {
        "component"
    } else {
        "div"
    };
    assert_eq!(result["targets"], serde_json::json!([target, "override"]));
    let (id, title) = if attrs.is_empty() {
        ("caller-id", None)
    } else {
        ("attrs-id", Some("attrs-title"))
    };
    let preserved = serde_json::json!([
        id,
        "yes",
        "label",
        42,
        "child",
        "forwarded-target",
        true,
        0.25,
        forwarded.0,
        forwarded.1,
        title
    ]);
    assert_eq!(
        result["preserved"],
        serde_json::json!([preserved, preserved])
    );
    assert_eq!(result["unchanged"], true);
}

#[rstest]
#[case::component("styled(Link)")]
#[case::tag_always_true("styled('div', { shouldForwardProp: () => true })")]
#[serial]
fn filters_runtime_transients_when_no_props_are_statically_withheld(#[case] factory: &str) {
    // Given: no style or attrs reads to populate the named withholding list.
    let component = executable_component(readable_code(&format!(
        "import {{styled}} from '@devup-ui/core'\nconst Subject = {factory}({{ color: 'red' }})"
    )));
    let script = format!(
        r"
        const Link = 'component';
        {component}
        const input = {{ $runtimeOnly: 1, '$': 2, '$🦀': 3, id: 'kept', 'x$y': 4, '': 5 }};
        const before = JSON.stringify(input);
        const props = Subject.render(input, undefined).props;
        JSON.stringify({{
            transients: Object.keys(props).filter((k) => k.startsWith('$')),
            kept: [props.id, props['x$y'], props['']],
            unchanged: before === JSON.stringify(input)
        }})
        "
    );

    // When: execute the generated component's raw-rest forwarding branch.
    let value = Context::default()
        .eval(Source::from_bytes(&script))
        .expect("generated component must execute");
    let result: serde_json::Value = serde_json::from_str(
        &value
            .as_string()
            .expect("JSON result")
            .to_std_string_escaped(),
    )
    .unwrap();

    // Then: only prefix-$ keys are omitted and caller input is untouched.
    assert_eq!(
        result,
        serde_json::json!({
            "transients": [], "kept": ["kept", 4, 5], "unchanged": true
        })
    );
}

#[test]
#[serial]
fn keeps_ordinary_jsx_spreads_when_no_styled_rest_is_generated() {
    // Given: an ordinary JSX spread, not a styled component definition.
    let source = "export const node = <div {...props} />;";

    // When: extract the ordinary JSX module.
    let code = readable_code(source);

    // Then: forwarding is unchanged and no transient filter is generated.
    assert!(code.contains("<div {...props} />"), "{code}");
    assert!(!code.contains("Object.fromEntries"), "{code}");
}
