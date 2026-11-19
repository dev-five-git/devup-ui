use insta::assert_snapshot;
use rstest::rstest;

use super::errors::{Problem, RuntimeFeature};
use super::props::Declaration;
use super::sheet::{Sheet, key_name};
use super::style::{Choice, style_code};
use super::values::{Literal, NativeValue, css_text, number_or_text, quoted_list};
use super::{NativePlatform, NativeTarget};
use crate::{ExtractOption, ExtractTarget, extract, extract_without_source_map};

const IMPORT: &str =
    "import { Box, Button, Center, Flex, Image, Input, Text, VStack } from '@devup-ui/react'\n";

fn option(platform: NativePlatform, runtime: bool) -> ExtractOption {
    ExtractOption {
        target: ExtractTarget::Native(NativeTarget { platform, runtime }),
        ..ExtractOption::default()
    }
}

/// `code` compiled for `platform`, or its build errors
fn compile_for(platform: NativePlatform, runtime: bool, code: &str) -> String {
    match extract("test.tsx", code, option(platform, runtime)) {
        Ok(output) => output.code,
        Err(error) => format!("error: {error}"),
    }
}

/// `code` after the import of every component, compiled for iOS
fn compile(code: &str) -> String {
    compile_for(NativePlatform::Ios, true, &format!("{IMPORT}{code}"))
}

#[rstest]
#[case::components(
    "components",
    r#"export const App = () => (
  <VStack gap={2}>
    <Box w={10} />
    <Flex />
    <Center />
    <Text color="red">Hi</Text>
    <Image src="https://example.com/a.png" alt="a" />
    <Image src={`https://example.com/b.png`} />
    <Image src={logo} />
    <Image src={(images.logo)} />
    <Input placeholder="name" />
    <Button onPress={go}><Text>Go</Text></Button>
  </VStack>
)"#
)]
#[case::shorthands_and_units(
    "shorthands_and_units",
    r#"<Box p={1} m="2" px={2} py="3px" w="50%" h="auto" maxW="1rem" minH="2em" boxSize={4}
  fontSize="16px" letterSpacing="-0.5px" gap="4px 8px" rowGap={1} borderRadius={2} opacity={0.5}
  zIndex={10} flex={1} flexGrow="2" aspectRatio="16 / 9" fontWeight={700} lineHeight="24px"
  bgColor={`#fff`} color="red" display="none" pos="absolute" top={-1} positioning="top-left"
  textDecoration="underline" transform="rotate(45deg)" boxShadow="0 1px 2px #0003" />"#
)]
#[case::box_sides(
    "box_sides",
    r#"<Box m="1px 2px" p="1px 2px 3px" mx="1px" />;
<Box m="1px 2px 3px 4px" />"#
)]
#[case::backgrounds_and_borders(
    "backgrounds_and_borders",
    r#"<Box bg="linear-gradient(red, blue)" border="1px solid rgba(0, 0, 0, 0.5)" />;
<Box bg="red" border="none" />;
<Box bgImage="radial-gradient(red, blue)" border="2px dashed" />"#
)]
#[case::dynamic_values(
    "dynamic_values",
    "<Box bg={color} px={space} m={(a, b)} opacity={props.opacity} p={pad} top={-y} />"
)]
#[case::conditional_values(
    "conditional_values",
    r"<Box bg={on ? 'red' : 'blue'} p={big && 4} m={a > 1 ? 2 : undefined}
  color={a ? 'red' : b ? c : null} w={x ?? 1} h={off ? null : 2} opacity={false} />"
)]
#[case::merge_order(
    "merge_order",
    r#"<Box bg={on ? 'red' : 'blue'} p={1} bg="green" m={2} bg="black" />;
<Box px={1} pl={2} />;
<Flex flexDir="column" />"#
)]
#[case::given_styles(
    "given_styles",
    r#"<Box {...props} p={1} {...a.b} style={s} />;
<Box {...props} style={s} />;
<Box style={s} key="k" />"#
)]
#[case::shared_objects(
    "shared_objects",
    r#"<Box p={1} />;
<Box p="4px" />;
<Box p={on ? 1 : 2} />"#
)]
#[case::retagged(
    "retagged",
    r"<Box as={Pressable} p={1}>text</Box>;
<Flex as={Animated.View} />;
<Button as={Link}>Go</Button>"
)]
#[case::nested_elements(
    "nested_elements",
    r#"<Box p={1} icon={<Text color="red">!</Text>} render={() => <Box m={1} />}>
  {items.map((item) => <Text key={item}>{item}</Text>)}
  {/* a comment */}
</Box>"#
)]
#[case::no_styles(
    "no_styles",
    r#"<Box {...props} onLayout={f} testID="box" disabled />"#
)]
#[case::empty_values("empty_values", "<Box bg={undefined} p={null} m={true} />")]
#[case::namespaced_prop("namespaced_prop", "<Box xml:lang=\"en\" p={1} />")]
fn compiles(#[case] name: &str, #[case] code: &str) {
    assert_snapshot!(name, compile(code));
}

#[rstest]
#[case(NativePlatform::Ios)]
#[case(NativePlatform::Android)]
#[case(NativePlatform::Web)]
fn platform_blocks(#[case] platform: NativePlatform) {
    let code = format!(
        "{IMPORT}<Box p={{1}} _ios={{{{ bg: 'red', _android: {{ m: 1 }} }}}} _android={{{{ bg: 'green', opacity: on ? 1 : 0 }}}} _web={{{{ 'bg': 'blue', cursor: 'pointer' }}}} />"
    );
    assert_snapshot!(
        format!("platform_blocks_{platform}"),
        compile_for(platform, true, &code)
    );
}

#[rstest]
#[case::aliases_and_types(
    "aliases_and_types",
    r#"import { Box as B, type DevupProps, DevupTheme } from "@devup-ui/react"
import type { DevupThemeTypography } from '@devup-ui/react'
export const C = (props: DevupProps) => <B p={1} />"#
)]
#[case::name_collisions(
    "name_collisions",
    r"import { StyleSheet, View } from 'react-native'
import { Box, Text } from '@devup-ui/react'
const __s = StyleSheet.create({ a: { flex: 1 } })
export const C = () => <View style={__s.a}><Box p={1} /><Text>t</Text></View>"
)]
#[case::no_components(
    "no_components",
    r"import type { DevupProps } from '@devup-ui/react'
export const value: DevupProps = {}"
)]
#[case::unused_import("unused_import", "import { Box } from '@devup-ui/react'\n")]
#[case::types_read(
    "types_read",
    r"import { Box } from '@devup-ui/react'
type P = React.ComponentProps<typeof Box>
export const C = (props: P) => <Box {...props} />"
)]
#[case::without_package("without_package", "export const value = 1\n")]
#[case::sheet_after_imports(
    "sheet_after_imports",
    r"'use client'
import { Box } from '@devup-ui/react'
import { a } from './a'
const element = <Box p={1} />
import { b } from './b'"
)]
fn compiles_files(#[case] name: &str, #[case] code: &str) {
    assert_snapshot!(name, compile_for(NativePlatform::Android, true, code));
}

#[rstest]
#[case::responsive("responsive", "<Box p={[1, 2]} />")]
#[case::responsive_in_condition("responsive_in_condition", "<Box p={on ? [1, 2] : 3} />")]
#[case::orientation("orientation", "<Box _portrait={{ p: 1 }} _landscape={{ p: 2 }} />")]
#[case::theme_token("theme_token", "<Box bg=\"$primary\" />")]
#[case::theme_selector("theme_selector", "<Box _themeDark={{ bg: 'black' }} />")]
#[case::typography("typography", "<Text typography=\"heading\" />")]
#[case::pressed(
    "pressed",
    "<Button _pressed={{ bg: 'red' }} _active={{ bg: 'blue' }} />"
)]
#[case::selectors(
    "selectors",
    "<Box _hover={{ bg: 'red' }} _focus={{ bg: 'blue' }} selectors={{ '&:hover': {} }} />"
)]
#[case::web_only_props(
    "web_only_props",
    "<Box className=\"a\" styleOrder={1} styleVars={{ a: 1 }} />"
)]
#[case::unknown_properties(
    "unknown_properties",
    "<Box gridTemplateColumns=\"1fr\" transition=\"all 1s\" float=\"left\" borderTop=\"1px solid\" />"
)]
#[case::unsupported_values(
    "unsupported_values",
    r#"<Box display="grid" position="fixed" w="10vw" h="calc(100% - 4px)" maxW="var(--w)"
  minW="1px !important" lineHeight={1.5} opacity="half" m="1px 2px 3px 4px 5px"
  bg="url(a.png)" positioning={1} flexDir="row" maxH="1ch" />"#
)]
#[case::missing_and_object_values(
    "missing_and_object_values",
    "<Box flex bg={{ a: 1 }} p=<b /> />"
)]
#[case::dynamic_splits("dynamic_splits", "<Box border={b} positioning={p} gridArea={g} />")]
#[case::tags(
    "tags",
    "<Box as=\"section\" />;\n<Box as={on ? A : B} />;\n<Box as={tag} />"
)]
#[case::style_text("style_text", "<Box style=\"color: red\" />")]
#[case::image_source("image_source", "<Image src={images[0]} />")]
#[case::platform_shapes(
    "platform_shapes",
    r"<Box _ios={styles} _android={{ ...base, [key]: 1, onPress: f, _hover: {}, _pressed: {}, _web: styles, as: A, bg: [1] }} />"
)]
#[case::text_children(
    "text_children",
    "<Box>Hello</Box>;\n<Button>{'Go'}</Button>;\n<Flex>{`a`}</Flex>;
<Box>{\" \"}</Box>"
)]
#[case::references(
    "references",
    "const C = Box;\nexport { Text };\nstyled(Flex);\n<Box.Item />"
)]
#[case::opaque_spread("opaque_spread", "<Box {...getProps()} p={1} />")]
fn errors(#[case] name: &str, #[case] code: &str) {
    assert_snapshot!(name, compile(code));
}

#[rstest]
#[case::grid("grid", "import { Grid } from '@devup-ui/react'\n")]
#[case::apis(
    "apis",
    "import { css, styled, globalCss, ThemeScript, initTheme, keyframes, stylex, useTheme, getTheme, setTheme } from '@devup-ui/react'\n"
)]
#[case::namespace("namespace", "import * as D from '@devup-ui/react'\n")]
#[case::default_import("default_import", "import D, { Box } from '@devup-ui/react'\n<Box />")]
#[case::parse_error("parse_error", "import { Box } from '@devup-ui/react'\n<Box p={1}>\n")]
fn import_errors(#[case] name: &str, #[case] code: &str) {
    assert_snapshot!(name, compile_for(NativePlatform::Ios, false, code));
}

#[rstest]
#[case(true)]
#[case(false)]
fn runtime_errors_name_the_option(#[case] runtime: bool) {
    let code = format!("{IMPORT}<Box p={{[1, 2]}} _themeDark={{{{}}}} _pressed={{{{}}}} />");
    assert_snapshot!(
        format!("runtime_{runtime}"),
        compile_for(NativePlatform::Ios, runtime, &code)
    );
}

#[test]
fn unsupported_file_extension_is_an_error() {
    let error = extract(
        "test.vue",
        "import { Box } from '@devup-ui/react'",
        option(NativePlatform::Ios, true),
    )
    .err()
    .map_or_else(String::new, |error| error.to_string());
    assert!(error.starts_with("test.vue:1:1: "), "{error}");
}

#[test]
fn source_map_follows_the_request() {
    let code = format!("{IMPORT}<Box p={{1}} />");
    let with = extract("test.tsx", &code, option(NativePlatform::Ios, true)).map(|o| o.map);
    let without = extract_without_source_map("test.tsx", &code, option(NativePlatform::Ios, true))
        .map(|o| o.map);
    assert!(matches!(with, Ok(Some(map)) if map.contains("test.tsx")));
    assert!(matches!(without, Ok(None)));
}

#[test]
fn native_output_has_no_css() {
    let code = format!("{IMPORT}<Box p={{1}} />");
    let output = extract("test.tsx", &code, option(NativePlatform::Ios, true));
    assert!(matches!(
        output,
        Ok(output) if output.styles.is_empty() && output.css_file.is_none() && output.dependencies.is_empty()
    ));
}

#[test]
fn web_stays_the_default_target() {
    assert_eq!(ExtractOption::default().target, ExtractTarget::Web);
    assert_eq!(ExtractTarget::default(), ExtractTarget::Web);
}

#[rstest]
#[case("ios", Some(NativePlatform::Ios))]
#[case("android", Some(NativePlatform::Android))]
#[case("web", Some(NativePlatform::Web))]
#[case("IOS", None)]
#[case("windows", None)]
fn platforms_parse(#[case] text: &str, #[case] expected: Option<NativePlatform>) {
    assert_eq!(text.parse::<NativePlatform>().ok(), expected);
    if let Some(platform) = expected {
        assert_eq!(platform.to_string(), text);
    }
}

#[rstest]
#[case(0, "a")]
#[case(25, "z")]
#[case(26, "aa")]
#[case(27, "ab")]
#[case(701, "zz")]
#[case(702, "aaa")]
fn key_names(#[case] index: usize, #[case] expected: &str) {
    assert_eq!(key_name(index), expected);
}

#[rstest]
#[case(&[], "")]
#[case(&["a"], "`a`")]
#[case(&["a", "b"], "`a` or `b`")]
#[case(&["a", "b", "c"], "`a`, `b` or `c`")]
fn quoted_lists(#[case] values: &[&str], #[case] expected: &str) {
    assert_eq!(quoted_list(values), expected);
}

#[rstest]
#[case(NativeValue::Number(4.0), "4")]
#[case(NativeValue::Number(-0.5), "-0.5")]
#[case(NativeValue::Text("a\"b".to_string()), "\"a\\\"b\"")]
fn native_value_code(#[case] value: NativeValue, #[case] expected: &str) {
    assert_eq!(value.code(), expected);
}

#[rstest]
#[case("padding", Literal::Number(1.0), "4px")]
#[case("padding", Literal::Text("2".to_string()), "8px")]
#[case("padding", Literal::Text("2px".to_string()), "2px")]
#[case("opacity", Literal::Number(0.5), "0.5")]
#[case("opacity", Literal::Text("1".to_string()), "1")]
fn web_css_text(#[case] property: &str, #[case] literal: Literal, #[case] expected: &str) {
    assert_eq!(css_text(property, &literal), expected);
}

#[rstest]
#[case("2", NativeValue::Number(2.0))]
#[case(" 16 / 9 ", NativeValue::Text("16 / 9".to_string()))]
fn ratios(#[case] text: &str, #[case] expected: NativeValue) {
    assert_eq!(number_or_text(text), expected);
}

fn declarations(pairs: &[(&str, f64)]) -> Vec<Declaration> {
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_string(), NativeValue::Number(*value)))
        .collect()
}

#[test]
fn sheet_shares_equal_objects() {
    let mut sheet = Sheet::new("s".to_string());
    assert!(sheet.is_empty());
    let a = sheet.reference(&declarations(&[("padding", 4.0)]));
    let b = sheet.reference(&declarations(&[("margin", 4.0)]));
    let again = sheet.reference(&declarations(&[("padding", 4.0)]));
    assert_eq!(
        (a.as_str(), b.as_str(), again.as_str()),
        ("s.a", "s.b", "s.a")
    );
    assert_eq!(
        sheet.declaration("SS"),
        "const s = SS.create({ a: { padding: 4 }, b: { margin: 4 } });"
    );
}

#[test]
fn style_code_orders_and_merges_choices() {
    let mut sheet = Sheet::new("s".to_string());
    let condition = |test: &str, consequent: Choice, alternate: Choice| Choice::Condition {
        test: test.to_string(),
        consequent: Box::new(consequent),
        alternate: Box::new(alternate),
    };
    let dynamic =
        |key: &str, code: &str| Choice::Dynamic(vec![(key.to_string(), code.to_string())]);
    let code = style_code(
        vec![
            Choice::Empty,
            dynamic("margin", "m"),
            Choice::Static(declarations(&[("padding", 1.0)])),
            Choice::Static(declarations(&[("margin", 2.0)])),
            Choice::Static(declarations(&[("margin", 3.0), ("width", 1.0)])),
            condition(
                "a.b",
                condition("c > 1", dynamic("width", "w"), Choice::Empty),
                Choice::Static(declarations(&[("width", 2.0)])),
            ),
            condition(
                "!d",
                Choice::Empty,
                Choice::Static(declarations(&[("height", 1.0)])),
            ),
            Choice::Static(declarations(&[("padding", 5.0)])),
        ],
        vec!["given".to_string()],
        &mut sheet,
    );
    assert_eq!(
        code.as_deref(),
        Some(
            "[s.a, { margin: m }, s.b, a.b ? ((c > 1) && { width: w }) : s.c, (!d) ? null : s.d, given]"
        )
    );
    assert_eq!(
        sheet.declaration("SS"),
        "const s = SS.create({ a: { padding: 5 }, b: { margin: 3, width: 1 }, c: { width: 2 }, d: { height: 1 } });"
    );
    assert_eq!(
        style_code(vec![Choice::Empty], vec!["given".to_string()], &mut sheet),
        None
    );
}

#[test]
fn problems_format_for_props_and_imports() {
    let problem = Problem::new("why", "how");
    assert_eq!(
        problem.on("Box", "p"),
        "`<Box>` cannot use `p` on React Native: why. Fix: how"
    );
    assert_eq!(
        problem.import("css", "@devup-ui/react"),
        "`css` from `@devup-ui/react` cannot be used on React Native: why. Fix: how"
    );
    let target = NativeTarget {
        platform: NativePlatform::Ios,
        runtime: true,
    };
    assert!(
        RuntimeFeature::Pressed
            .problem(target)
            .fix
            .contains("Pressable")
    );
}
