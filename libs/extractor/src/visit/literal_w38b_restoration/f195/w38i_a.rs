use super::*;

#[rstest]
#[case("external-a")]
#[case("external-b")]
#[serial]
fn literal_mixin_when_external_keeps_ordered_red_and_one_read(#[case] external: &str) {
    // Given: the genuine local literal mixin route with an effectful external leaf.
    let _debug = DebugMode::enabled();
    let source = "import {ClassNames} from '@emotion/react';<ClassNames>{({css,cx})=>css`color:red;${state.className};style-order:2`}</ClassNames>;";
    let allocator = Allocator::default();
    let (mut visitor, mut value) = local(&allocator, source);
    let span = value.span();
    // When: local composition emits the literal's declaration and captured mixin.
    assert!(visitor.compile_class_names_call(&mut value));
    // Then: the source class and independently observed ordered-red atom both apply.
    assert_eq!(visitor.errors, Vec::<(u32, String)>::new());
    assert_eq!(value.span(), span);
    let red = visitor.styles.iter().find(|style| {
        matches!(style, ExtractStyleValue::Static(style) if style.property() == "color" && style.value() == "red" && style.style_order() == Some(2))
    }).unwrap_or_else(|| panic!("ordered red declaration"));
    let setup =
        format!("const state={{get className(){{trace.push('mixin');return '{external}';}}}}");
    let actual = evaluated(&value, &setup);
    let tokens: std::collections::BTreeSet<_> = actual[0]
        .as_str()
        .unwrap_or_else(|| panic!("class"))
        .split_whitespace()
        .collect();
    let red = class_of(red);
    assert_eq!(
        tokens,
        std::collections::BTreeSet::from([external, red.as_str()])
    );
    assert_eq!(actual[1], serde_json::json!(["mixin"]));
}

#[test]
#[serial]
fn class_names_when_mixin_is_nested_preserves_located_rejection() {
    // Given: the original public fixture puts a known mixin inside ordered hover rules.
    let _debug = DebugMode::enabled();
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    let filename = "a.tsx";
    let source = "import {css,ClassNames} from '@emotion/react'; const base=css({color:'blue',styleOrder:3}); export const view=<ClassNames>{({css,cx})=>css`&:hover{color:red;${base};style-order:2}`}</ClassNames>;";
    let option = crate::ExtractOption {
        import_aliases: std::collections::HashMap::from([(
            "@emotion/react".to_string(),
            crate::ImportAlias::NamedToNamed,
        )]),
        ..crate::ExtractOption::default()
    };
    // When: the public extractor parses and prepares the real aliased ClassNames source.
    let actual = crate::extract(filename, source, option);
    // Then: the authored base owns the exact original located error, not a successful output.
    assert_eq!(
        actual.err().map(|error| error.to_string()),
        Some(format!(
            "{filename}:1:160: `<ClassNames>` cannot use `base` at build time: a mixin must stand outside nested rules, where the parts it composes with can be split"
        ))
    );
}
