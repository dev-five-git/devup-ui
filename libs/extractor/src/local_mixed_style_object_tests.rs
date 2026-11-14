use super::*;
use crate::extract_style::extract_static_style::ExtractStaticStyle;
use css::style_selector::StyleSelector;
use oxc_span::ContentEq;

#[rstest]
#[case("_hover={inner} {...inner}")]
#[case("{...inner} _hover={inner}")]
#[serial]
fn selector_and_spread_read_one_exact_local_record(#[case] attributes: &str) {
    for single_css in [false, true] {
        let source = format!(
            "import {{Box}} from '@devup-ui/react';\nexport function f(){{const inner={{color:'red'}};return <Box {attributes}/>;}}"
        );
        reset_class_map();
        reset_file_map();
        let output = extract(
            "local-mixed.tsx",
            &source,
            ExtractOption {
                single_css,
                ..ExtractOption::default()
            },
        )
        .unwrap_or_else(|error| panic!("{error}"));
        let expected: rustc_hash::FxHashSet<_> = [
            ExtractStyleValue::Static(ExtractStaticStyle::new("color", "red", 0, None)),
            ExtractStyleValue::Static(ExtractStaticStyle::new(
                "color",
                "red",
                0,
                Some(StyleSelector::Selector("&:hover".into())),
            )),
        ]
        .into_iter()
        .collect();
        assert_eq!(output.styles, expected);
        reset_class_map();
        reset_file_map();
        let direct = extract(
            "local-mixed.tsx",
            "import {Box} from '@devup-ui/react';\nexport function f(){const inner={color:'red'};return <Box color='red' _hover={{color:'red'}}/>;}",
            ExtractOption { single_css, ..ExtractOption::default() },
        ).unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(output.styles, direct.styles);
        assert_eq!(output.css_file, direct.css_file);
        let arena = oxc_allocator::Allocator::default();
        let actual =
            oxc_parser::Parser::new(&arena, &output.code, oxc_span::SourceType::tsx()).parse();
        let direct =
            oxc_parser::Parser::new(&arena, &direct.code, oxc_span::SourceType::tsx()).parse();
        assert!(!actual.fatal_error && actual.diagnostics.is_empty());
        assert!(!direct.fatal_error && direct.diagnostics.is_empty());
        assert!(
            actual.program.content_eq(&direct.program),
            "{}",
            output.code
        );
    }
}

#[rstest]
#[case(
    "const inner={color:'red'};",
    "<Box _hover={inner} {...inner} id={inner}/>"
)]
#[case(
    "const inner={color:'red'};",
    "<Box ref={inner} {...inner} _hover={inner}/>"
)]
#[case(
    "const inner={color:'red'};",
    "<Box _hover={inner} data-rules={inner} {...inner}/>"
)]
#[case(
    "const inner={color:'red'};console.log(inner);",
    "<Box {...inner} _hover={inner}/>"
)]
#[case(
    "const inner={color:'red'};inner.color='blue';",
    "<Box _hover={inner} {...inner}/>"
)]
#[case(
    "let inner={color:'red'};inner={color:'blue'};",
    "<Box {...inner} _hover={inner}/>"
)]
#[case("const inner=unknown;", "<Box _hover={inner} {...inner}/>")]
#[case(
    "const inner={get color(){return 'red'}};",
    "<Box {...inner} _hover={inner}/>"
)]
#[case(
    concat!("const inner=", "{", "color:tone", "}", ";const tone='red';"),
    "<Box _hover={inner} {...inner}/>"
)]
#[case(
    "const inner={color:'red'};",
    "<Box as={inner} _hover={inner} {...inner}/>"
)]
#[case(
    "const inner={color:'red'};",
    "<Box onClick={inner} {...inner} _hover={inner}/>"
)]
#[case(
    "const inner={color:'red'};const outer=<div data-rules={inner}/>;",
    "<Box _hover={inner} {...inner}/>"
)]
#[serial]
fn selector_and_spread_do_not_admit_identity_escapes_or_unproven_records(
    #[case] declaration: &str,
    #[case] element: &str,
) {
    for single_css in [false, true] {
        reset_class_map();
        reset_file_map();
        let source = format!(
            "import {{Box}} from '@devup-ui/react';\nexport function f(){{{declaration}return {element};}}"
        );
        let error = extract(
            "local-mixed.tsx",
            &source,
            ExtractOption {
                single_css,
                ..ExtractOption::default()
            },
        )
        .err()
        .unwrap_or_else(|| panic!("expected located refusal: {source}"));
        assert!(
            error.to_string().starts_with("local-mixed.tsx:2:"),
            "{source}: {error}"
        );
    }
}
