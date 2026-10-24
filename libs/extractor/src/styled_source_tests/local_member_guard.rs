use crate::{ExtractOption, ExtractStyleValue, extract};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case::member_mutation(
    "const holder={rules:{color:'red'}};holder.rules.color='blue';return css(holder.rules);"
)]
#[case::computed_mutation(
    "const holder={rules:{color:'red'}};holder['rules'].color='blue';return css(holder['rules']);"
)]
#[case::extra_read("const holder={rules:{color:'red'}};observe(holder);return css(holder.rules);")]
#[case::primitive_fold(
    "const holder={color:'red'};holder.color='blue';return css({color:holder.color});"
)]
#[case::conditional_test(
    concat!("const holder=", "{", "on:true", "}", ";holder.on=false;return css(holder.on?{color:'red'}:{color:'blue'});")
)]
#[case::logical_test(
    concat!("const holder=", "{", "on:true", "}", ";holder.on=false;return css(holder.on&&[{color:'red'}]);")
)]
#[case::nested_array(
    "const holder={rules:[{color:'red'}]};holder.rules[0].color='blue';return css(holder.rules[0]);"
)]
#[case::computed_two_reads(
    "const holder={key:'rules',rules:{color:'red'}};observe(holder);return css(holder[holder.key]);"
)]
#[serial]
fn folded_local_members_keep_the_existing_extra_use_error(
    #[case] body: &str,
    #[values(false, true)] single_css: bool,
) {
    reset_class_map();
    reset_file_map();
    let source = format!("import {{css}} from '@devup-ui/react';\nexport function f(){{{body}}}");
    let result = extract(
        "local-member.ts",
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    );
    let error = match result {
        Ok(output) => panic!("folding must not erase a non-style use: {output:?}"),
        Err(error) => error.to_string(),
    };
    assert!(error.starts_with("local-member.ts:2:"), "{error}");
    assert!(
        error.contains("Cannot read `holder` as styles at build time: it is also read elsewhere"),
        "{error}"
    );
}

#[rstest]
#[case::member("const holder={rules:{color:'red'}};return css(holder.rules);")]
#[case::computed("const holder={key:'rules',rules:{color:'red'}};return css(holder[holder.key]);")]
#[case::repeated("const holder={rules:{color:'red'}};return css(holder.rules,holder.rules);")]
#[case::conditional(
    "const holder={on:true,rules:{color:'red'},other:{color:'blue'}};return css(holder.on?holder.rules:holder.other);"
)]
#[case::logical("const holder={on:false,rules:{color:'red'}};return css(holder.on||holder.rules);")]
#[case::folded_string(
    "const holder={base:'r',suffix:'ed'};return css({color:holder.base+holder.suffix});"
)]
#[serial]
fn style_only_member_and_condition_reads_keep_static_values(
    #[case] body: &str,
    #[values(false, true)] single_css: bool,
) {
    reset_class_map();
    reset_file_map();
    let source = format!("import {{css}} from '@devup-ui/react';\nexport function f(){{{body}}}");
    let output = extract(
        "local-member-safe.ts",
        &source,
        ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("safe style-only members must compile: {error}"));
    let colors: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) if style.property() == "color" => Some(style.value()),
            _ => None,
        })
        .collect();
    assert_eq!(colors, vec!["red"]);
}

#[path = "local_member_accounting.rs"]
mod local_member_accounting;
