use insta::assert_snapshot;
use rstest::rstest;

use super::element::units_declaration;
use super::errors::Problem;
use super::props::Declaration;
use super::sheet::{Sheet, key_name};
use super::style::{Choice, State, Vars, cascade, in_theme, style_code, varied, when};
use super::values::{Literal, NativeValue, css_text, number_or_text, quoted_list};
use super::{NativePlatform, NativeTarget, NativeTheme, TokenLevels, TypographyProperties};
use crate::{ExtractOption, ExtractTarget, extract, extract_without_source_map};

const VARS: Vars<'static> = Vars {
    theme: "t",
    breakpoint: "b",
    landscape: "l",
    pressed: "p",
    hovered: "h",
    focused: "f",
    disabled: "d",
};

const IMPORT: &str =
    "import { Box, Button, Center, Flex, Image, Input, Text, VStack } from '@devup-ui/react'\n";

fn option(platform: NativePlatform, runtime: bool) -> ExtractOption {
    themed_option(platform, runtime, NativeTheme::default())
}

fn themed_option(platform: NativePlatform, runtime: bool, theme: NativeTheme) -> ExtractOption {
    ExtractOption {
        target: ExtractTarget::Native(NativeTarget {
            platform,
            runtime,
            reanimated: true,
            theme: std::sync::Arc::new(theme),
        }),
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
#[case::dynamic_groups(
    "dynamic_groups",
    r"<Box w={w} h={h} maxW={mw} gap={g} rowGap={rg} borderRadius={r} borderWidth={bw}
  fontSize={fs} letterSpacing={ls} top={t} inset={i} flex={f} flexGrow={fg} zIndex={z}
  aspectRatio={ar} lineHeight={lh} fontWeight={fw} color={c} display={d} boxShadow={bs} />"
)]
#[case::dynamic_in_conditions(
    "dynamic_in_conditions",
    "<Box p={on ? 2 : pad} m={big && space} bg={on ? 'red' : color} />"
)]
#[case::dynamic_with_sheet_name_taken(
    "dynamic_with_sheet_name_taken",
    "const __u = 1;
<Box p={pad} m={1} />"
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
        (vec![], vec!["given".to_string()]),
        &mut sheet,
        &VARS,
    )
    .map(|style| style.code);
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
    assert!(
        style_code(
            vec![Choice::Empty],
            (vec![], vec!["given".to_string()]),
            &mut sheet,
            &VARS
        )
        .is_none()
    );
    assert_eq!(
        style_code(
            vec![Choice::Empty],
            (vec!["class".to_string()], vec!["given".to_string()]),
            &mut sheet,
            &VARS
        )
        .map(|style| style.code)
        .as_deref(),
        Some("[class, given]")
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
}

#[rstest]
#[case("2, 4", "8")]
#[case("-1.5, 4", "-6")]
#[case("0.5, 1", "0.5")]
#[case("\"2\", 4", "8")]
#[case("\" .5 \", 1", "0.5")]
#[case("\"3px\", 4", "3")]
#[case("\"-3px\", 1", "-3")]
#[case("\"1rem\", 4", "16")]
#[case("\"0.5em\", 1", "8")]
#[case("\"50%\", 4", "50%")]
#[case("\"auto\", 4", "auto")]
#[case("\"1vw\", 4", "1vw")]
#[case("\"16 / 9\", 1", "16 / 9")]
#[case("null, 4", "null")]
#[case("undefined, 4", "undefined")]
fn runtime_values_convert_as_literals_do(#[case] arguments: &str, #[case] expected: &str) {
    // Given: the function the build declares for values read at runtime
    let code = format!("{}String(u({arguments}))", units_declaration("u"));
    // When
    let mut context = boa_engine::Context::default();
    let value = context
        .eval(boa_engine::Source::from_bytes(&code))
        .and_then(|value| value.to_string(&mut context))
        .map(|text| text.to_std_string_escaped());
    // Then
    assert_eq!(value.ok().as_deref(), Some(expected), "{code}");
}

/// A token with one value a theme, `None` where the theme lacks it
fn per_theme(values: &[Option<&str>]) -> Vec<Option<TokenLevels>> {
    values
        .iter()
        .map(|value| value.map(|value| vec![Some(value.to_string())]))
        .collect()
}

fn typography_entry(properties: &[(&str, &str)]) -> TypographyProperties {
    properties
        .iter()
        .map(|(property, value)| ((*property).to_string(), (*value).to_string()))
        .collect()
}

/// A theme named `names`, each token with its values in them, and the
/// tokens and typographies every test reads
fn theme(names: &[&str], dark: Option<usize>, tokens: &[(&str, &[Option<&str>])]) -> NativeTheme {
    let mut all: std::collections::BTreeMap<String, Vec<Option<TokenLevels>>> = tokens
        .iter()
        .map(|(name, values)| ((*name).to_string(), per_theme(values)))
        .collect();
    all.insert(
        "gutter".to_string(),
        vec![Some(vec![Some("4px".to_string()), None, Some("8px".to_string())]); names.len()],
    );
    all.insert(
        "trailing".to_string(),
        vec![Some(vec![Some("2px".to_string()), None]); names.len()],
    );
    all.insert(
        "loopA".to_string(),
        per_theme(&vec![Some("$loopB"); names.len()]),
    );
    all.insert(
        "loopB".to_string(),
        per_theme(&vec![Some("$loopA"); names.len()]),
    );
    NativeTheme {
        names: names.iter().map(|name| (*name).to_string()).collect(),
        dark,
        tokens: all,
        typography: [
            (
                "body",
                vec![Some(typography_entry(&[
                    ("font-family", "Inter"),
                    ("font-size", "16px"),
                    ("font-weight", "500"),
                    ("line-height", "1.5"),
                    ("letter-spacing", "-0.02em"),
                ]))],
            ),
            (
                "caption",
                vec![
                    Some(typography_entry(&[
                        ("font-size", "$captionSize"),
                        ("line-height", "150%"),
                        ("letter-spacing", "1px"),
                        ("text-transform", "uppercase"),
                    ])),
                    None,
                ],
            ),
            (
                "loose",
                vec![Some(typography_entry(&[
                    ("line-height", "normal"),
                    ("letter-spacing", "normal"),
                    ("font-style", "italic"),
                ]))],
            ),
            (
                "relative",
                vec![Some(typography_entry(&[("line-height", "2em")]))],
            ),
            (
                "darkFont",
                vec![Some(typography_entry(&[("font-family", "$onlyDark")]))],
            ),
            (
                "rounded",
                vec![Some(typography_entry(&[
                    ("font-size", "24px"),
                    ("line-height", "1.3"),
                ]))],
            ),
            (
                "heading",
                vec![
                    Some(typography_entry(&[("font-size", "24px")])),
                    None,
                    Some(typography_entry(&[("font-size", "32px")])),
                ],
            ),
        ]
        .into_iter()
        .map(|(name, levels)| (name.to_string(), levels))
        .collect(),
        ..NativeTheme::default()
    }
}

fn one_theme() -> NativeTheme {
    theme(
        &["default"],
        None,
        &[
            ("primary", &[Some("#5a44ff")]),
            ("text", &[Some("#111")]),
            ("containerX", &[Some("16px")]),
            ("card", &[Some("0 1px 2px $shadow")]),
            ("shadow", &[Some("#0003")]),
            ("captionSize", &[Some("12px")]),
        ],
    )
}

fn two_themes() -> NativeTheme {
    theme(
        &["light", "dark"],
        Some(1),
        &[
            ("primary", &[Some("#5a44ff"), Some("#8c7bff")]),
            ("text", &[Some("#111"), Some("#eee")]),
            ("surface", &[Some("#fff"), Some("#1a1a2e")]),
            ("same", &[Some("red"), Some("red")]),
            ("containerX", &[Some("16px"), Some("16px")]),
            ("card", &[Some("0 1px 2px #0003"), Some("0 1px 2px #0009")]),
            ("onlyDark", &[None, Some("#000")]),
            ("captionSize", &[Some("12px"), Some("14px")]),
            ("primary-100", &[Some("#eef"), Some("#223")]),
        ],
    )
}

fn three_themes() -> NativeTheme {
    theme(
        &["light", "dark", "sepia"],
        Some(1),
        &[
            (
                "primary",
                &[Some("#5a44ff"), Some("#8c7bff"), Some("#704214")],
            ),
            ("text", &[Some("#111"), Some("#eee"), Some("#3b2a1a")]),
            ("captionSize", &[Some("12px"), Some("12px"), Some("12px")]),
        ],
    )
}

/// `code` after the import of every component, compiled for iOS with `theme`
fn compile_themed(theme: NativeTheme, runtime: bool, code: &str) -> String {
    match extract(
        "test.tsx",
        &format!("{IMPORT}{code}"),
        themed_option(NativePlatform::Ios, runtime, theme),
    ) {
        Ok(output) => output.code,
        Err(error) => format!("error: {error}"),
    }
}

const ONE_THEME_ELEMENTS: &str = r#"export const Card = ({ on }) => (
  <Box bg="$primary" w="$containerX" boxShadow="$card" border="1px solid $primary" _themeDefault={{ p: 1 }}>
    <Text typography="body" color={on ? '$primary' : '$text'} />
    <Text typography={on ? 'caption' : 'loose'} />
    <Text typography="caption" m="$trailing" />
    <Text typography="rounded" />
  </Box>
)"#;

const TWO_THEME_ELEMENTS: &str = r#"export function Card({ on, color }) {
  return (
    <Box bg="$surface" p={4} borderRadius="8px" boxShadow="$card" w="$containerX">
      <Text typography="body" color={on ? '$primary' : '$text'}>Hi</Text>
      <Text typography={on ? 'caption' : 'body'} color="$same" borderColor="$primary.100" />
      <Box bg={color} border="1px solid $primary" m={on && 2} _themeDark={{ opacity: 0.5, p: on ? 1 : 2 }} />
      <Box _themeLight={{ bg: '$primary', _themeLight: { m: 1 }, _themeDark: { m: 2 } }} color="$onlyDark" />
      <Text typography="darkFont" />
      <Box _themeDark={{ m: 1 }} m={2} />
      <Box _themeDark={{ p: on ? 1 : 2 }} m={1} />
    </Box>
  )
}"#;

const THREE_THEME_ELEMENTS: &str = r#"export function Card() {
  return <Box bg="$primary" color="$text" _themeSepia={{ p: 2 }} _themeDark={{ _themeDark: { m: 1 } }} />
}"#;

#[rstest]
#[case::one_theme("theme_one", one_theme(), ONE_THEME_ELEMENTS)]
#[case::two_themes("theme_two", two_themes(), TWO_THEME_ELEMENTS)]
#[case::three_themes("theme_three", three_themes(), THREE_THEME_ELEMENTS)]
fn compiles_themes(#[case] name: &str, #[case] theme: NativeTheme, #[case] code: &str) {
    assert_snapshot!(name, compile_themed(theme, true, code));
}

#[rstest]
#[case::arrow_expression("arrow_expression", "export const A = () => <Box bg=\"$primary\" />")]
#[case::arrow_parenthesized(
    "arrow_parenthesized",
    "export const A = () => (\n  <Box bg=\"$primary\" />\n)"
)]
#[case::arrow_block(
    "arrow_block",
    "export const A = () => {\n  \"use memo\"\n  return <Box bg=\"$primary\" />\n}"
)]
#[case::function_declaration(
    "function_declaration",
    "function A() {\n  const x = 1\n  return <><Box bg=\"$primary\" /><Box bg=\"$text\" /></>\n}"
)]
#[case::wrapped(
    "wrapped",
    "export const A = memo(forwardRef((props, ref) => <Box ref={ref} bg=\"$primary\" />));\nexport const B = React.memo(function () { return <Box bg=\"$text\" /> });\nexport const C = (wrap)(() => <Box bg=\"$text\" />);\nexport const D = wrap()(() => <Box bg=\"$text\" />);\nexport const E = 1"
)]
#[case::export_default_arrow(
    "export_default_arrow",
    "export default () => <Box bg=\"$primary\" />"
)]
#[case::export_default_function(
    "export_default_function",
    "export default function () {\n  return <Box bg=\"$primary\" />\n}"
)]
#[case::export_default_value("export_default_value", "export default <Box bg=\"$primary\" />")]
#[case::hook(
    "hook",
    "export function useCard() {\n  return <Box bg=\"$primary\" />\n}\nexport const use = () => <Box bg=\"$text\" />;\nexport const user = () => <Box bg=\"$text\" />"
)]
#[case::callback_in_component(
    "callback_in_component",
    "export function List({ items }) {\n  return <Box>{items.map((item) => <Text key={item} color=\"$text\">{item}</Text>)}</Box>\n}"
)]
#[case::nested_components(
    "nested_components",
    "export function Outer() {\n  const Inner = () => <Box bg=\"$text\" />\n  return <Box bg=\"$primary\"><Inner /></Box>\n}"
)]
#[case::module_scope(
    "module_scope",
    "const element = <Box bg=\"$primary\" p={1} />;\nexport const icon = <Flex as={Animated.View} bg=\"$text\" key=\"k\">\n  <Text>t</Text>\n</Flex>"
)]
#[case::helper_function(
    "helper_function",
    "function renderItem({ item }) {\n  return <Text color=\"$text\">{item}</Text>\n}\nconst lower = function () { return <Box _themeDark={{ p: 1 }} /> }\nconst { Destructured } = { Destructured: () => <Box bg=\"$text\" /> }"
)]
#[case::class_component(
    "class_component",
    "export class A extends Component {\n  render() {\n    return <Box bg=\"$primary\" />\n  }\n}"
)]
#[case::static_in_component(
    "static_in_component",
    "export function A() {\n  return <Box bg=\"$same\" p={1} />\n}"
)]
#[case::declared_function(
    "declared_function",
    "declare function Typed(): void\nexport function A() {\n  return <Box bg=\"$primary\" />\n}"
)]
#[case::name_collisions(
    "theme_name_collisions",
    "const __t = 1, __useThemeIndex = 2, __Themed = 3, __defineThemes = 4;\nexport const A = () => <Box bg=\"$primary\" />;\nexport const b = <Box bg=\"$primary\" />"
)]
fn places_the_theme_hook(#[case] name: &str, #[case] code: &str) {
    assert_snapshot!(name, compile_themed(two_themes(), true, code));
}

const THEME_API: &str = r"import { Box, Text, useTheme, setTheme as set, getTheme, type DevupTheme } from '@devup-ui/react'
export function Toggle() {
  const theme = useTheme()
  return <Box onPress={() => set(getTheme() === 'dark' ? 'light' : 'dark')}><Text>{theme}</Text></Box>
}";

#[rstest]
#[case::two_themes("theme_api_two", two_themes(), true)]
#[case::one_theme("theme_api_one", one_theme(), true)]
#[case::no_theme("theme_api_none", NativeTheme::default(), true)]
#[case::runtime_off("theme_api_runtime_off", two_themes(), false)]
fn compiles_theme_api(#[case] name: &str, #[case] theme: NativeTheme, #[case] runtime: bool) {
    let output = match extract(
        "test.tsx",
        THEME_API,
        themed_option(NativePlatform::Ios, runtime, theme),
    ) {
        Ok(output) => output.code,
        Err(error) => format!("error: {error}"),
    };
    assert_snapshot!(name, output);
}

#[test]
fn theme_api_alone_defines_the_themes_at_the_end() {
    let output = extract(
        "test.tsx",
        "import { setTheme } from '@devup-ui/react'\n",
        themed_option(NativePlatform::Ios, true, two_themes()),
    )
    .map(|output| output.code);
    assert_eq!(
        output.ok().as_deref(),
        Some(
            "import { setTheme } from \"@devup-ui/react/native\";\nimport { defineThemes as __defineThemes } from \"@devup-ui/react/native\";\n__defineThemes([\"light\", \"dark\"], 1);\n"
        )
    );
}

const RUNTIME_OFF_ELEMENTS: &str = r#"<Box bg="$primary" w="$containerX" color="$same" _themeDark={{ p: 1 }} />;
<Text typography="body" />"#;

#[rstest]
#[case::one_theme("runtime_off_one_theme", theme(&["dark"], Some(0), &[("primary", &[Some("#000")]), ("containerX", &[Some("8px")]), ("same", &[Some("red")])]))]
#[case::two_themes("runtime_off_two_themes", two_themes())]
fn runtime_off_keeps_single_values(#[case] name: &str, #[case] theme: NativeTheme) {
    assert_snapshot!(name, compile_themed(theme, false, RUNTIME_OFF_ELEMENTS));
}

#[rstest]
#[case::unknown_token(
    "theme_unknown_token",
    "<Box bg=\"$missing\" color=\"$primary.200\" />"
)]
#[case::responsive_tokens("theme_responsive_tokens", "<Box p=\"$gutter\" m=\"$trailing\" />")]
#[case::typographies(
    "theme_typographies",
    "<Text typography=\"missing\" />;\n<Text typography=\"heading\" />;\n<Text typography={props.typography} />;\n<Text typography=\"relative\" />;\n<Text typography={1} />"
)]
#[case::unknown_theme("theme_unknown_theme", "<Box _themeSepia={{ p: 1 }} _themeDark={1} />")]
#[case::token_loop("theme_token_loop", "<Box bg=\"$loopA\" />")]
#[case::bad_values(
    "theme_bad_values",
    "<Text typography=\"caption\" lineHeight=\"$primary\" fontSize=\"$same\" />"
)]
fn theme_errors(#[case] name: &str, #[case] code: &str) {
    assert_snapshot!(name, compile_themed(two_themes(), true, code));
}

#[rstest]
#[case("$primary", Some(("", "primary", "")))]
#[case("1px solid $gray.100, red", Some(("1px solid ", "gray.100", ", red")))]
#[case("$a-b_c $d", Some(("", "a-b_c", " $d")))]
#[case("$-a $b", Some(("$-a ", "b", "")))]
#[case("cost $", None)]
#[case("no tokens", None)]
fn finds_tokens(#[case] text: &str, #[case] expected: Option<(&str, &str, &str)>) {
    assert_eq!(super::theme::next_token(text), expected);
}

#[rstest]
#[case("_themeDark", "dark")]
#[case("_themeHighContrast", "highContrast")]
#[case("_themeSepia2", "sepia2")]
fn names_selector_themes(#[case] prop: &str, #[case] expected: &str) {
    assert_eq!(super::theme::selector_theme(prop), expected);
}

#[test]
fn themed_choices_collapse_when_every_theme_agrees() {
    let red = declarations(&[("opacity", 1.0)]);
    assert_eq!(varied(vec![vec![]]), Choice::Empty);
    assert_eq!(varied(vec![vec![vec![], vec![]]]), Choice::Empty);
    assert_eq!(
        varied(vec![vec![red.clone(), red.clone()]]),
        Choice::Static(red.clone())
    );
    assert_eq!(
        in_theme(0, 1, Choice::Static(red.clone())),
        Choice::Static(red.clone())
    );
    assert_eq!(in_theme(1, 2, Choice::Empty), Choice::Empty);
    assert_eq!(
        in_theme(1, 2, Choice::Static(red.clone())),
        Choice::Varied(vec![vec![vec![], red]])
    );
}

const RESPONSIVE_ELEMENTS: &str = r#"export function Layout({ on, x }) {
  return (
    <VStack p={[1, null, 3]} m={2}>
      <Box p={[1, 2, 3, 4, 5, 6, 7, 8]} />
      <Box p={[null, 2]} w={[undefined, false, , "50%"]} />
      <Box p={[1, 1, 2]} bg="red" />
      <Box m={["1px 2px", "3px"]} p={["1px 2px", null, "3px 4px"]} />
      <Box p={[x, 2]} />
      <Box p={[on ? 1 : 2, null, 3]} m={[(on && 1), x]} />
      <Box p={on ? [1, 2] : 3} m={on && [1, null, 2]} />
      <Box p={[4]} m={[2, 2]} />
      <Box _ios={{ p: [1, 2] }} _android={{ p: [3, 4] }} />
      <Box p={[1, 2]} m={[3, null, 4]} gap={[1, 2]} />
      <Box px={[1, 2]} py={3} />
      <Box p={[1, 2, 3]} p={4} />
      <Box p={4} p={[1, 2]} />
      <Box p={on ? [x, 2] : 3} p={1} />
    </VStack>
  )
}"#;

const ORIENTATION_ELEMENTS: &str = r"export function Screen({ on }) {
  return (
    <Flex _portrait={{ flexDir: 'column', p: [1, 2] }} _landscape={{ p: on ? 3 : 4, _ios: { m: 1 } }}>
      <Box _landscape={{ _landscape: { p: 1 } }} p={2} />
    </Flex>
  )
}";

#[rstest]
#[case::responsive("responsive_arrays", RESPONSIVE_ELEMENTS)]
#[case::orientation("orientation_blocks", ORIENTATION_ELEMENTS)]
#[case::hooks_in_arrow(
    "responsive_hooks_in_arrow",
    "export const A = () => <Box p={[1, 2]} bg=\"$primary\" _landscape={{ m: 1 }} />;\nexport const B = () => <Box _portrait={{ m: 1 }} />;\nexport const c = <Box p={[1, 2]} _landscape={{ m: 1 }} />;\nexport const d = <Box bg=\"$primary\" p={[1, 2]} />"
)]
#[case::name_collisions(
    "responsive_name_collisions",
    "const __b = 1, __l = 2, __useBreakpoint = 3, __useLandscape = 4;\nexport const A = () => <Box p={[1, 2]} _landscape={{ m: 1 }} />"
)]
fn compiles_responsive(#[case] name: &str, #[case] code: &str) {
    assert_snapshot!(name, compile_themed(two_themes(), true, code));
}

/// Token values at each breakpoint
fn levels(values: &[Option<&str>]) -> TokenLevels {
    values
        .iter()
        .map(|value| value.map(str::to_string))
        .collect()
}

/// Two themes whose tokens and typographies differ by breakpoint
fn responsive_themes() -> NativeTheme {
    let mut theme = two_themes();
    theme.tokens.insert(
        "space".to_string(),
        vec![
            Some(levels(&[Some("4px"), None, Some("8px")])),
            Some(levels(&[Some("6px"), None, Some("12px")])),
        ],
    );
    theme.tokens.insert(
        "edge".to_string(),
        vec![
            Some(levels(&[
                Some("1px solid $primary"),
                Some("2px solid $primary")
            ]));
            2
        ],
    );
    theme.tokens.insert(
        "late".to_string(),
        vec![Some(levels(&[None, Some("2px")])); 2],
    );
    theme.typography.insert(
        "display".to_string(),
        vec![
            Some(typography_entry(&[
                ("font-family", "Inter"),
                ("font-size", "$captionSize"),
                ("line-height", "1.5"),
            ])),
            None,
            Some(typography_entry(&[("font-size", "40px")])),
        ],
    );
    theme
}

const THEMED_RESPONSIVE_ELEMENTS: &str = r#"export function Card({ on }) {
  return (
    <Box bg={["$primary", "$text"]} p="$space" m="$gutter" borderColor="$primary">
      <Text typography="heading" color="$text" />
      <Text typography="display" />
      <Text typography={["body", null, "heading"]} />
      <Box _themeDark={{ p: [1, 2], bg: "$primary" }} />
      <Box p={["$space", null, 1]} m="$late" />
      <Box bg={on ? ["$primary", null, "red"] : "$text"} />
      <Box border="$edge" />
      <Box _landscape={{ bg: "$primary", p: "$space" }} />
      <Box bg="$primary" bg={[null, "$text"]} />
    </Box>
  )
}"#;

#[rstest]
#[case::two_themes("responsive_themes", responsive_themes(), THEMED_RESPONSIVE_ELEMENTS)]
#[case::custom_breakpoints(
    "responsive_custom_breakpoints",
    NativeTheme {
        breakpoints: vec![0, 600],
        ..responsive_themes()
    },
    "export const A = () => <Box p={[1, 2, 3]} m=\"$space\" />;\nexport const b = <Box p={[1, 2]} />"
)]
#[case::no_breakpoints(
    "responsive_no_breakpoints",
    NativeTheme {
        breakpoints: vec![],
        ..responsive_themes()
    },
    "export const A = () => <Box p={[1, 2, 3]} m=\"$space\" />"
)]
fn compiles_responsive_themes(#[case] name: &str, #[case] theme: NativeTheme, #[case] code: &str) {
    assert_snapshot!(name, compile_themed(theme, true, code));
}

#[rstest]
#[case::arrays(
    "runtime_off_arrays",
    "<Box p={[1, 2]} />;\n<Box p={[x, 2]} />;\n<Box p={[1]} m={[2, 2]} w={[null, 1]} />"
)]
#[case::tokens(
    "runtime_off_responsive_tokens",
    "<Box p=\"$gutter\" />;\n<Text typography=\"heading\" />"
)]
#[case::orientation(
    "runtime_off_orientation",
    "<Box _landscape={{ p: 1 }} _portrait={{ p: 2 }} />"
)]
fn runtime_off_rejects_responsive_values(#[case] name: &str, #[case] code: &str) {
    assert_snapshot!(name, compile_themed(one_theme(), false, code));
}

#[rstest]
#[case::spread("responsive_spread", "<Box p={[1, ...rest]} />")]
#[case::nested("responsive_nested", "<Box p={[x, [1, 2]]} />")]
#[case::orientation_value(
    "orientation_value",
    "<Box _landscape={1} _portrait={{ p: [1, { a: 1 }] }} />"
)]
fn responsive_errors(#[case] name: &str, #[case] code: &str) {
    assert_snapshot!(name, compile_themed(two_themes(), true, code));
}

#[rstest]
#[case("margin", "marginTop", true)]
#[case("margin", "margin", false)]
#[case("margin", "padding", false)]
#[case("padding", "paddingVertical", true)]
#[case("inset", "insetBlock", true)]
#[case("inset", "top", true)]
#[case("inset", "inset", false)]
#[case("inset", "width", false)]
#[case("marginVertical", "marginTop", true)]
#[case("marginVertical", "marginLeft", false)]
#[case("paddingVertical", "paddingBottom", true)]
#[case("marginHorizontal", "marginStart", true)]
#[case("marginHorizontal", "marginTop", false)]
#[case("paddingHorizontal", "paddingRight", true)]
#[case("gap", "rowGap", true)]
#[case("gap", "gap", false)]
#[case("borderWidth", "borderTopWidth", true)]
#[case("borderWidth", "borderWidth", false)]
#[case("borderColor", "borderLeftColor", true)]
#[case("borderRadius", "borderTopLeftRadius", true)]
#[case("borderRadius", "outlineRadius", false)]
#[case("width", "minWidth", false)]
fn cascade_replaces_covered_longhands(
    #[case] shorthand: &str,
    #[case] longhand: &str,
    #[case] replaced: bool,
) {
    // Given: a narrower breakpoint declaring `longhand`
    let mut declared = declarations(&[(longhand, 1.0), ("opacity", 1.0)]);
    // When: a wider one declares `shorthand`
    cascade(&mut declared, declarations(&[(shorthand, 2.0)]));
    // Then
    let kept = declared.iter().any(|(key, value)| {
        key == longhand && (longhand == shorthand || *value == NativeValue::Number(1.0))
    });
    assert_eq!(!kept, replaced, "{declared:?}");
    assert!(declared.iter().any(|(key, _)| key == "opacity"));
}

#[test]
fn varied_choices_pick_by_breakpoint_and_theme() {
    let a = declarations(&[("padding", 1.0)]);
    let b = declarations(&[("padding", 2.0)]);
    let c = declarations(&[("padding", 3.0)]);
    let mut sheet = Sheet::new("s".to_string());
    let mut code = |choice: Choice| {
        style_code(vec![choice], (vec![], vec![]), &mut sheet, &VARS)
            .map_or_else(Default::default, |style| (style.code, style.reads))
    };
    // Three runs of breakpoints, the first without styles
    let (three, _) = code(Choice::Varied(vec![
        vec![vec![]],
        vec![a.clone()],
        vec![a.clone()],
        vec![b.clone()],
    ]));
    assert_eq!(three, "b < 1 ? null : b < 3 ? s.a : s.b");
    // Breakpoints and themes both
    let (both, reads) = code(Choice::Varied(vec![
        vec![a.clone(), b.clone()],
        vec![c.clone(), c.clone()],
    ]));
    assert_eq!(both, "b < 1 ? [s.a, s.b][t] : s.c");
    assert!(reads.theme && reads.breakpoint && !reads.landscape);
    // In a condition, and applied while a state holds
    let (condition, reads) = code(Choice::Condition {
        test: "on".to_string(),
        consequent: Box::new(Choice::Varied(vec![vec![a.clone()], vec![b.clone()]])),
        alternate: Box::new(when(State::Landscape(false), Choice::Static(c))),
    });
    assert_eq!(condition, "on ? (b < 1 ? s.a : s.b) : (!l && s.c)");
    assert!(!reads.theme && reads.breakpoint && reads.landscape);
    let (nested, _) = code(Choice::Condition {
        test: "on".to_string(),
        consequent: Box::new(Choice::Varied(vec![
            vec![declarations(&[("padding", 1.0), ("margin", 1.0)])],
            vec![declarations(&[("padding", 2.0), ("margin", 1.0)])],
        ])),
        alternate: Box::new(Choice::All(vec![
            Choice::Static(a.clone()),
            when(State::AtLeast(2), Choice::Static(b)),
        ])),
    });
    assert_eq!(
        nested,
        "on ? ([s.d, b < 1 ? s.a : s.b]) : [s.a, b >= 2 && s.b]"
    );
    assert_eq!(when(State::Theme(1), Choice::Empty), Choice::Empty);
    assert_eq!(
        in_theme(
            1,
            2,
            when(State::Landscape(true), Choice::Static(a.clone()))
        ),
        when(
            State::Theme(1),
            when(State::Landscape(true), Choice::Static(a))
        )
    );
}

const PRESSED_ELEMENTS: &str = r#"export function Buttons({ on, go, extra }) {
  return (
    <Box>
      <Button onPress={go} bg="$primary" p={2} _pressed={{ bg: '$text', opacity: 0.8 }}>
        <Text>Go</Text>
      </Button>
      <Box onPress={go} _active={{ opacity: 0.5 }} p={[1, 2]} />
      <Flex onPress={go} _pressed={{ p: on ? 1 : 2 }} style={extra}>
        <Box />
      </Flex>
      <Box _themeDark={{ _pressed: { bg: 'red' } }} />
      <Button _pressed={{}} />
      <Box _active={{}} />
    </Box>
  )
}
export const outside = <Box _pressed={{ bg: '$primary' }} />"#;

#[rstest]
#[case::pressed("pressed_styles", true, PRESSED_ELEMENTS)]
#[case::runtime_off(
    "pressed_runtime_off",
    false,
    "export const a = <Box _pressed={{ m: 1, _pressed: { p: 1 } }} bg=\"red\" />"
)]
#[case::name_collisions(
    "pressed_name_collisions",
    true,
    "const __p = 1, __Pressable = 2;\nexport const A = () => <Box _pressed={{ m: 1 }} />"
)]
#[case::errors(
    "pressed_errors",
    true,
    "<Text _pressed={{ color: 'red' }} />;\n<Image _pressed={{ opacity: 0.5 }} />;\n<Input _pressed={{ bg: 'red' }} />;\n<Box as={Animated.View} _pressed={{ bg: 'red' }} />;\n<Box _pressed={1} />"
)]
fn compiles_pressed(#[case] name: &str, #[case] runtime: bool, #[case] code: &str) {
    assert_snapshot!(name, compile_themed(two_themes(), runtime, code));
}

const CSS_ELEMENTS: &str = r"import { css as style, type DevupProps } from '@devup-ui/react'
const card = style({ p: 4, borderRadius: '8px' })
export function Card({ on, extra, children }) {
  const title = style({ color: '$text', fontSize: [14, null, 18] })
  return (
    <Box className={card} p={2} style={extra}>
      <Text className={style(title, on && { fontWeight: 700 }, on ? { color: 'red' } : null)}>{children}</Text>
      <Box className={style({ _themeDark: { bg: '$surface' } }, false, undefined, [card])} />
      <Box className={on ? card : undefined} />
      <View style={style({ m: 1 }, extra)} />
      <Box style={style({ p: 1 })} className={(0, card)} />
      <Box style={[style(), (on, extra)]} />
    </Box>
  )
}";

#[rstest]
#[case::css("css_calls", true, CSS_ELEMENTS)]
#[case::css_only(
    "css_only",
    true,
    "import { css } from '@devup-ui/react'\nexport const s = css({ m: 2 }, cond ? { p: 1 } : { p: 2 })\nexport const t = css(true, on ? undefined : { p: 1 }, on ? { m: 1 } : false, (other.css({ p: 1 })))\nexport const useU = () => css(t, { p: 1 }, { _landscape: { m: 1 } }, { m: 2 }, on ? {} : { p: 2, _landscape: { p: 3 } }, other`x`)
export const useV = () => css({ _landscape: { m: 1 } }, { p: 2 })"
)]
#[case::text(
    "css_text",
    true,
    "import { css } from '@devup-ui/react'\nexport function A({ size }) {\n  const s = css`\n    /* the card */\n    background-color: $primary;\n    padding: 8px 16px !important;\n    border-radius: ${size};\n    font-weight: ${'bold'};\n    /* unterminated`\n  return <Box className={s} />\n}\nexport const t = css``"
)]
#[case::runtime_off(
    "css_runtime_off",
    false,
    "import { css } from '@devup-ui/react'\nexport const s = css({ m: [1, 2] })"
)]
#[case::errors(
    "css_errors",
    true,
    r#"import { css } from '@devup-ui/react'
export const a = css({ bg: '$surface' })
export const b = css(...parts)
export const c = css('flex', `p-2`, cond ? { p: 1 } : 2)
export const d = css({ _hover: { p: 1 }, [key]: 1 })
export const f = css
export const g = css({ _ios: { className: 'a' } }, on ? 1 : { p: 1 })
export const h = css`&:hover { color: red }`
export const i = css`padding ${x}px; padding: ${x}px; -webkit-mask: red; color`
export function E() {
  return <Box className="p-4" _active={{ p: 1 }} style={css({ _pressed: { m: 1 } })}><Box className /></Box>
}"#
)]
fn compiles_css(#[case] name: &str, #[case] runtime: bool, #[case] code: &str) {
    assert_snapshot!(name, compile_themed(two_themes(), runtime, code));
}

#[test]
fn css_alone_drops_its_import() {
    assert_snapshot!(
        "css_alone",
        compile_for(
            NativePlatform::Android,
            false,
            "import { css } from '@devup-ui/react'\nimport { View } from 'react-native'\nexport const A = () => <View style={css({ p: 1 })} />"
        )
    );
}

#[rstest]
#[case::theme("block_order_theme", "<Box _themeDark={{ bg: 'red' }} bg=\"blue\" />")]
#[case::orientation("block_order_orientation", "<Box _landscape={{ p: 1 }} p={2} />")]
#[case::pressed("block_order_pressed", "<Box _active={{ m: 1 }} m={2} />")]
#[case::nested(
    "block_order_nested",
    "<Box _landscape={{ _active: { m: 1 }, m: 2, _themeDark: { m: 3 }, p: 4 }} p={5} />"
)]
#[case::platform("block_order_platform", "<Box _ios={{ m: 1 }} m={2} />")]
fn blocks_apply_after_the_other_styles(#[case] name: &str, #[case] code: &str) {
    let code = format!("export function A() {{ return {code} }}");
    assert_snapshot!(name, compile_themed(two_themes(), true, &code));
}

#[test]
fn a_static_style_after_one_kept_in_order_checks_it() {
    let mut sheet = Sheet::new("s".to_string());
    let code = style_code(
        vec![
            Choice::Dynamic(vec![("margin".to_string(), "m".to_string())]),
            Choice::Static(declarations(&[("margin", 2.0)])),
            Choice::Dynamic(vec![("width".to_string(), "w".to_string())]),
            Choice::Static(declarations(&[("padding", 1.0)])),
            when(
                State::Pressed,
                Choice::Static(declarations(&[("height", 1.0)])),
            ),
            Choice::Static(declarations(&[("top", 1.0)])),
        ],
        (vec![], vec![]),
        &mut sheet,
        &VARS,
    )
    .map(|style| style.code);
    assert_eq!(
        code.as_deref(),
        Some("[s.a, { margin: m }, s.b, { width: w }, p && s.c]")
    );
}

const STYLED_ELEMENTS: &str = r"import { styled, css } from '@devup-ui/react'
import { View, Animated } from 'react-native'
export const Card = styled(Box)({ bg: '$surface', p: [2, null, 4], borderRadius: '8px' })
export const Row = styled(Flex, { gap: 2 })
export const Title = styled(Text)`
  color: ${(props) => props.color};
  font-size: 18px;
`
export const Item = styled(View)({ m: 1 }, (props) => props.active && css({ opacity: 0.5 }), { w: (p) => p.width })
export const Tap = styled(Box)({ _active: { opacity: 0.6 }, _themeDark: { bg: 'black' } })
export const Plain = styled(Button)({ _pressed: { m: 1 } })
export const Empty = styled(Animated.View)()
export function App() {
  return <Card p={1}><Row /><Title color='red'>Hi</Title></Card>
}";

#[rstest]
#[case::styled("styled_components", STYLED_ELEMENTS)]
#[case::errors(
    "styled_errors",
    r"import { styled } from '@devup-ui/react'
export const A = styled('div')({ p: 1 })
export const B = styled(View)
export const C = styled(View)(...parts)
export const D = styled(View)({ _active: { p: 1 } })
export const E = styled(Text)({ _active: { p: 1 } })
export const F = styled(View)((p) => ({ bg: p.color }), (p) => (p.on ? null : { m: 1 }), function (p) { return p.style })
export const G = styled(Box, { p: 1 }, extra)
export const H = styled"
)]
fn compiles_styled(#[case] name: &str, #[case] code: &str) {
    assert_snapshot!(name, compile_themed(two_themes(), true, code));
}

const STATE_ELEMENTS: &str = r"export function Form({ busy, go, value }) {
  return (
    <Box>
      <Button disabled={busy} onPress={go} bg='$primary' _hover={{ opacity: 0.9 }} _focus={{ borderColor: 'blue' }} _disabled={{ opacity: 0.4 }} _active={{ opacity: 0.7 }} />
      <Box _hover={{ bg: 'red' }} _disabled={{ m: 1 }} />
      <Input _focus={{ borderColor: '$primary' }} value={value} onFocus={go} />
      <Input _focus={{ borderColor: 'red' }} disabled _disabled={{ opacity: 0.5 }} />
      <Input _focus={{}} />
    </Box>
  )
}
export const outside = <Input _focus={{ bg: '$primary', p: [1, 2] }} />
export const plain = <Input _focus={{ m: 1 }} />";

#[rstest]
#[case::states("interaction_states", STATE_ELEMENTS)]
#[case::styled(
    "interaction_styled",
    "import { styled } from '@devup-ui/react'\nexport const A = styled(Button)({ _hover: { m: 1 }, _disabled: { opacity: 0.5 } })"
)]
#[case::errors(
    "interaction_errors",
    "import { css } from '@devup-ui/react'\nexport const B = css({ _disabled: { m: 1 } })\nexport const C = css({ _hover: { m: 1 } });\n<Text _hover={{ m: 1 }} />;\n<Input as={X} _focus={{ m: 1 }} />;\n<Input _hover={{ m: 1 }} />;
<Box _ios={{ _before: { m: 1 } }} />"
)]
fn compiles_interaction_states(#[case] name: &str, #[case] code: &str) {
    assert_snapshot!(name, compile_themed(two_themes(), true, code));
}

const ANIMATIONS: &str = r#"import { keyframes, css, styled } from '@devup-ui/react'
const fade = keyframes({ from: { opacity: 0, transform: 'scale(0.9)' }, '50%': { opacity: 0.5, bg: '$same' }, to: { opacity: 1 } })
export const pulse = keyframes({ 0: { w: [size], h: null, _ios: { p: 1 } }, [mid]: { opacity: 0.5 }, 100: { opacity: 1, opacity: 0.8 }, 99: {} })
export function Card({ on, d }) {
  return (
    <Box animationName={fade} animationDuration="300ms" animationIterationCount="infinite" animationDirection="alternate" animationFillMode="both" animationTimingFunction="ease-in-out" animationDelay={d} animationPlayState={on ? 'running' : 'paused'} p={2}>
      <Text transition="opacity 0.3s, background-color 1s ease, all" opacity={on ? 1 : 0.5}>Hi</Text>
      <Input transitionProperty="border-color" transitionDuration={200} borderColor={on ? 'red' : 'blue'} _focus={{ borderColor: 'green' }} />
      <Input transitionProperty="opacity, background-color, all" transitionDuration={200} transitionTimingFunction="linear" transitionDelay="1s" />
      <Button animationName={on ? fade : pulse} animationDuration={300} />
      <Button transitionDuration={300} />
      <Image transitionProperty="opacity" transitionBehavior="allow-discrete" src="a.png" />
      <Box as={Animated.ScrollView} animationName={fade} />
      <Box _ios={{ animationDuration: '1s' }} bg="$primary" />
      <Box icon={<Text transitionDuration="1s" />} _active={{ opacity: 0.5 }} />
    </Box>
  )
}
export const outside = <Box animationName={fade} bg="$primary" />
export const Pulse = styled(Box)({ animationName: pulse, animationDuration: '1s' })
export const Label = styled(Input)({ transition: 'opacity 1s' })
export const Spin = styled(Animated.View)`animation-name: ${fade}; animation-duration: 1s;`
export const spin = css({ animationName: fade, animationDuration: '1s' })
export const text = css`transition: opacity 1s;`
"#;

const ANIMATION_ERRORS: &str = r#"import { keyframes } from '@devup-ui/react'
const a = keyframes({ from: { bg: '$primary' }, to: { opacity: [0, 1] }, at: { opacity: on ? 0 : 1 } })
const b = keyframes({ from: { _active: { opacity: 0 } }, to: 1, ...rest })
const c = keyframes(frames, more)
const c2 = keyframes(frames)
const d = keyframes
export const e = <Box animationName="fade" animation="spin 1s" />
export const f = <Box _active={{ opacity: 0.5 }} transition="opacity 1s" />
export const g = <Button _hover={{ opacity: 0.5 }} animationDuration="1s" />
export const h = <Box animationDirection="sideways" />
"#;

const WITHOUT_REANIMATED: &str = r#"import { keyframes, css, styled } from '@devup-ui/react'
export const b = <Box animationDuration="1s" />
export const c = css({ transition: 'opacity 1s' })
export const D = styled(Box)({ animationDuration: '1s' })
export const E = styled(X)({ animationDuration: '1s' })
"#;

#[rstest]
#[case::animations("animations", ANIMATIONS, true)]
#[case::errors("animation_errors", ANIMATION_ERRORS, true)]
#[case::without_reanimated("animations_without_reanimated", WITHOUT_REANIMATED, false)]
fn compiles_animations(#[case] name: &str, #[case] code: &str, #[case] reanimated: bool) {
    let mut option = themed_option(NativePlatform::Ios, true, two_themes());
    if let ExtractTarget::Native(target) = &mut option.target {
        target.reanimated = reanimated;
    }
    let output = match extract("test.tsx", &format!("{IMPORT}{code}"), option) {
        Ok(output) => output.code,
        Err(error) => format!("error: {error}"),
    };
    assert_snapshot!(name, output);
}
