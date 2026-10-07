use super::*;
use serial_test::serial;

const CARD: &str =
    "import { Box, styled } from '@devup-ui/react';\nconst Card = styled(Box, { bg: 'white' });\n";

fn card(element: &str) -> ExtractOutput {
    output(&format!("{CARD}export const a = (props) => {element};"))
}

#[test]
#[serial]
fn a_styled_component_renders_the_props_it_is_given_before_its_own_styles() {
    let rendered = code(
        "import { styled } from '@devup-ui/react';\nconst Card = styled('div', { color: 'red' });\nexport const a = (rest) => <Card {...rest} />;",
    );

    let spread = rendered
        .find("{...")
        .unwrap_or_else(|| panic!("{rendered}"));
    let own = rendered
        .find("className=")
        .unwrap_or_else(|| panic!("{rendered}"));
    assert!(spread < own, "{rendered}");
}

#[test]
#[serial]
fn an_explicit_style_after_a_spread_compiles_into_the_component() {
    let output = card("<Card {...props} color=\"red\" />");

    assert_eq!(slots(&output).len(), 0);
    assert!(
        static_styles(&output).contains(&("color".to_string(), "red".to_string())),
        "{}",
        output.code
    );
    assert!(
        output.code.split_whitespace().collect::<Vec<_>>().join(" ").contains(
            "<Card {...(({ \"styleOrder\": __devupOrder, \"style-order\": __devupKebabOrder, ...__devupProps }) => __devupProps)({ __proto__: null, ...__devupSpread0 })} className="
        ),
        "{}",
        output.code
    );
    assert!(!output.code.contains("color=\"red\""), "{}", output.code);
    assert!(!output.code.contains("<div"), "{}", output.code);
}

#[test]
#[serial]
fn an_explicit_style_before_a_spread_gives_way_to_it_in_the_component() {
    let output = card("<Card color=\"red\" {...props} />");
    let found = slots(&output);

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].property, "color");
    assert_eq!(found[0].fallback, "red");
    assert_eq!(
        read(&found[0].identifier, "const props = { color: 'blue' };"),
        "\"blue\""
    );
    assert_eq!(
        read(&found[0].identifier, "const props = { color: undefined };"),
        "undefined"
    );
    assert!(output.code.contains("<Card "), "{}", output.code);
    assert!(output.code.contains("...props"), "{}", output.code);
    assert!(!output.code.contains("color=\"red\""), "{}", output.code);
    assert!(!output.code.contains("<div"), "{}", output.code);
}

#[test]
#[serial]
fn the_component_keeps_rendering_its_theme_attrs_and_ref() {
    let output = card("<Card color=\"red\" {...props} />");

    assert!(
        output
            .code
            .contains("const Card = __devupForwardRef((__devupRefProps, __devupRef) =>"),
        "{}",
        output.code
    );
    assert!(output.code.contains("ref: __devupRef"), "{}", output.code);
    assert!(
        output.code.contains("\"theme\": __devupOmit0"),
        "{}",
        output.code
    );
    assert!(
        output.code.contains("background-0-white--255"),
        "{}",
        output.code
    );
}

#[test]
#[serial]
fn repeated_spreads_give_the_last_own_key_to_a_styled_component() {
    let output = card("<Card color=\"red\" {...props} color=\"blue\" {...props.more} />");
    let found = slots(&output);

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].fallback, "blue");
    assert_eq!(
        read(
            &found[0].identifier,
            "const props = { color: 'x', more: {} };"
        ),
        "undefined"
    );
    assert_eq!(
        read(
            &found[0].identifier,
            "const props = { more: { color: 'y' } };"
        ),
        "\"y\""
    );
}

#[test]
#[serial]
fn what_the_component_configures_or_reads_stays_an_attribute() {
    let rendered = code(
        "import { Box, styled } from '@devup-ui/react';\nconst Card = styled(Box)`color: ${(p) => p.tone};`;\nexport const a = (props) => <Card as=\"a\" tone=\"x\" theme={props.theme} p={1} {...props} />;",
    );

    assert!(rendered.contains("as=\"a\""), "{rendered}");
    assert!(rendered.contains("tone=\"x\""), "{rendered}");
    assert!(
        rendered.contains("theme={__devupValue") && rendered.contains("props.theme"),
        "{rendered}"
    );
    assert!(!rendered.contains("p={1}"), "{rendered}");
}

#[test]
#[serial]
fn a_component_extending_a_styled_one_takes_the_style_props_too() {
    let output = output(&format!(
        "{CARD}const Wide = styled(Card, {{ w: '100%' }});\nexport const a = (props) => <Wide color=\"red\" {{...props}} />;"
    ));

    assert_eq!(slots(&output).len(), 1);
    assert!(!output.code.contains("color=\"red\""), "{}", output.code);
}

#[test]
#[serial]
fn a_styled_component_of_a_tag_keeps_passing_what_it_always_did() {
    let rendered = code(
        "import { styled } from '@devup-ui/react';\nconst Card = styled('div', { bg: 'white' });\nexport const a = (props) => <Card id=\"x\" color=\"red\" {...props} />;",
    );

    assert!(rendered.contains("id=\"x\""), "{rendered}");
    assert!(rendered.contains("color=\"red\""), "{rendered}");
}
