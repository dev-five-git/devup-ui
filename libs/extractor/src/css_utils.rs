use css::{style_selector::StyleSelector, utils::to_camel_case};

use crate::{
    ExtractStyleProp,
    extract_style::{
        extract_static_style::ExtractStaticStyle, extract_style_value::ExtractStyleValue,
    },
};

pub fn css_to_style<'a>(
    css: &str,
    level: u8,
    selector: &Option<&StyleSelector>,
) -> Vec<ExtractStyleProp<'a>> {
    css.split(";")
        .map(|s| {
            let s = s.trim();
            if s.is_empty() {
                None
            } else {
                let mut iter = s.split(":").map(|s| s.trim());
                let property = to_camel_case(iter.next().unwrap());
                let value = iter.next().unwrap();
                Some(ExtractStyleProp::Static(ExtractStyleValue::Static(
                    ExtractStaticStyle::new(&property, value, level, selector.cloned()),
                )))
            }
        })
        .flatten()
        .collect()
}

pub fn optimize_css_block(css: &str) -> String {
    css.split("{")
        .map(|s| s.trim().to_string())
        .collect::<Vec<String>>()
        .join("{")
        .split("}")
        .map(|s| s.trim().to_string())
        .collect::<Vec<String>>()
        .join("}")
        .split(";")
        .map(|s| {
            if !s.contains(":") {
                s.to_string().trim().to_string()
            } else {
                let mut iter = s.split(":");
                let property = iter.next().unwrap().trim();
                let value = iter.next().unwrap().trim();
                format!("{}:{}", property, value)
            }
        })
        .collect::<Vec<String>>()
        .join(";")
        .trim()
        .replace(";}", "}")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    use rstest::rstest;

    #[rstest]
    #[case(
        "       img      {       background-color    :       red;      }     ",
        "img{background-color:red}"
    )]
    #[case(
        "       img      {       background-color    :       red;          color     :          blue;      }     ",
        "img{background-color:red;color:blue}"
    )]
    #[case("div{margin : 0 ; padding : 0 ; }", "div{margin:0;padding:0}")]
    #[case(
        "a { text-decoration : none ; color : black ; }",
        "a{text-decoration:none;color:black}"
    )]
    #[case("body{background: #fff;}", "body{background:#fff}")]
    #[case(
        "h1{ font-size : 2rem ; font-weight : bold ; }",
        "h1{font-size:2rem;font-weight:bold}"
    )]
    #[case("span { }", "span{}")]
    #[case("p{color:blue;}", "p{color:blue}")]
    #[case(
        "ul { list-style : none ; margin : 0 ; padding : 0 ; }",
        "ul{list-style:none;margin:0;padding:0}"
    )]
    #[case("section{  }", "section{}")]
    fn test_optimize_css_block(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(optimize_css_block(input), expected);
    }

    #[rstest]
    #[case(
        "color: red; background: blue;",
        vec![
            ("color", "red"),
            ("background", "blue"),
        ]
    )]
    #[case(
        "margin:0;padding:0;",
        vec![
            ("margin", "0"),
            ("padding", "0"),
        ]
    )]
    #[case(
        "font-size: 16px;",
        vec![
            ("fontSize", "16px"),
        ]
    )]
    #[case(
        "border: 1px solid #000; color: #fff;",
        vec![
            ("border", "1px solid #000"),
            ("color", "#FFF"),
        ]
    )]
    #[case(
        "",
        vec![]
    )]
    fn test_css_to_style(#[case] input: &str, #[case] expected: Vec<(&str, &str)>) {
        let styles = css_to_style(input, 0, &None);
        let mut result: Vec<(&str, &str)> = styles
            .iter()
            .filter_map(|prop| {
                if let crate::ExtractStyleProp::Static(crate::ExtractStyleValue::Static(st)) = prop
                {
                    Some((st.property(), st.value()))
                } else {
                    None
                }
            })
            .collect();
        result.sort();
        let mut expected_sorted = expected.clone();
        expected_sorted.sort();
        assert_eq!(result, expected_sorted);
    }
}
