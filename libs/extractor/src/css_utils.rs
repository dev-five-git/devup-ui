use std::collections::BTreeMap;

use css::{
    optimize_multi_css_value::{check_multi_css_optimize, optimize_mutli_css_value},
    rm_css_comment::rm_css_comment,
    style_selector::StyleSelector,
};

use crate::extract_style::extract_static_style::ExtractStaticStyle;

pub fn css_to_style<'a>(
    css: &str,
    level: u8,
    selector: &Option<StyleSelector>,
) -> Vec<ExtractStaticStyle> {
    let mut styles = vec![];
    let mut input = css;

    if input.contains("@media") {
        let media_inputs = input
            .split("@media")
            .flat_map(|s| {
                let s = s.trim();
                if s.is_empty() {
                    return None;
                }
                Some(format!("@media{s}"))
            })
            .collect::<Vec<_>>();
        if media_inputs.len() > 1 {
            for media_input in media_inputs {
                styles.extend(css_to_style(&media_input, level, selector));
            }
            return styles;
        }
    }

    if input.contains('{') {
        while let Some(start) = input.find('{') {
            let rest = &input[start + 1..];

            let end = if selector.is_none() {
                rest.rfind('}').unwrap()
            } else {
                rest.find('}').unwrap()
            };
            let block = &rest[..end];
            let sel = &if let Some(StyleSelector::Media { query, .. }) = selector {
                let local_sel = input[..start].trim().to_string();
                Some(StyleSelector::Media {
                    query: query.clone(),
                    selector: if local_sel == "&" {
                        None
                    } else {
                        Some(local_sel)
                    },
                })
            } else {
                let sel = input[..start].trim().to_string();
                if sel.starts_with("@media") {
                    Some(StyleSelector::Media {
                        query: sel.replace(" ", "").replace("and(", "and (")["@media".len()..]
                            .to_string(),
                        selector: None,
                    })
                } else {
                    Some(StyleSelector::Selector(sel))
                }
            };
            let block = if block.contains('{') {
                css_to_style(block, level, sel)
            } else {
                css_to_style_block(block, level, sel)
            };
            let input_end = input.rfind('}').unwrap() + 1;

            input = &input[start + end + 2..input_end];
            styles.extend(block);
        }
    } else {
        styles.extend(css_to_style_block(input, level, selector));
    }

    styles.sort_by_key(|a| a.property().to_string());
    styles
}

fn css_to_style_block<'a>(
    css: &str,
    level: u8,
    selector: &Option<StyleSelector>,
) -> Vec<ExtractStaticStyle> {
    rm_css_comment(css)
        .split(";")
        .filter_map(|s| {
            let s = s.trim();
            if s.is_empty() {
                None
            } else {
                let mut iter = s.split(":").map(|s| s.trim());
                let property = iter.next().unwrap();
                let value = iter.next().unwrap();
                let value = if check_multi_css_optimize(property) {
                    optimize_mutli_css_value(value)
                } else {
                    value.to_string()
                };
                Some(ExtractStaticStyle::new(
                    property,
                    &value,
                    level,
                    selector.clone(),
                ))
            }
        })
        .collect()
}

pub fn keyframes_to_keyframes_style<'a>(
    keyframes: &str,
) -> BTreeMap<String, Vec<ExtractStaticStyle>> {
    let mut map = BTreeMap::new();
    let mut input = keyframes;

    while let Some(start) = input.find('{') {
        let key = input[..start].trim().to_string();
        let rest = &input[start + 1..];
        if let Some(end) = rest.find('}') {
            let block = &rest[..end];
            let mut styles = css_to_style(block, 0, &None);

            styles.sort_by_key(|a| a.property().to_string());
            map.insert(key, styles);
            input = &rest[end + 1..];
        } else {
            break;
        }
    }
    map
}

pub fn optimize_css_block(css: &str) -> String {
    rm_css_comment(css)
        .split("{")
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
                s.trim().to_string()
            } else {
                let mut iter = s.split(":");
                let property = iter.next().unwrap().trim();
                let value = iter.next().unwrap().trim();
                let value = if check_multi_css_optimize(property.split("{").last().unwrap()) {
                    optimize_mutli_css_value(value)
                } else {
                    value.to_string()
                };
                format!("{property}:{value}")
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
        "div{
        /* comment */
        background-color: red;
        /* color: blue; */
    }",
        "div{background-color:red}"
    )]
    #[case(
        "/*div{
        background-color: red;
    }*/",
        ""
    )]
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
    #[case(
        "ul { font-family: 'Roboto',       sans-serif; }",
        "ul{font-family:Roboto,sans-serif}"
    )]
    #[case(
        "ul { font-family: \"Roboto Hello\",       sans-serif; }",
        "ul{font-family:\"Roboto Hello\",sans-serif}"
    )]
    #[case("section{  }", "section{}")]
    fn test_optimize_css_block(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(optimize_css_block(input), expected);
    }

    #[rstest]
    #[case(
        "color: red; background: blue;",
        vec![
            ("color", "red", None),
            ("background", "blue", None),
        ]
    )]
    #[case(
        "margin:0;padding:0;",
        vec![
            ("margin", "0", None),
            ("padding", "0", None),
        ]
    )]
    #[case(
        "font-size: 16px;",
        vec![
            ("font-size", "16px", None),
        ]
    )]
    #[case(
        "border: 1px solid #000; color: #fff;",
        vec![
            ("border", "1px solid #000", None),
            ("color", "#FFF", None),
        ]
    )]
    #[case(
        "",
        vec![]
    )]
    #[case(
        "@media (min-width: 768px) {
            border: 1px solid #000;
            color: #fff;
        }",
        vec![
            ("border", "1px solid #000", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("color", "#FFF", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
        ]
    )]
    #[case(
        "@media (min-width: 768px) and (max-width: 1024px) {
            border: 1px solid #000;
            color: #fff;
        }
        
        @media (min-width: 768px) {
            border: 1px solid #000;
            color: #fff;
        }",
        vec![
            ("border", "1px solid #000", Some(StyleSelector::Media {
                query: "(min-width:768px)and (max-width:1024px)".to_string(),
                selector: None,
            })),
            ("color", "#FFF", Some(StyleSelector::Media {
                query: "(min-width:768px)and (max-width:1024px)".to_string(),
                selector: None,
            })),
            ("border", "1px solid #000", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("color", "#FFF", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
        ]
    )]
    #[case(
        "@media (min-width: 768px) {
            & {
                border: 1px solid #fff;
                color: #fff;
            }
            &:hover,   &:active, &:nth-child(2) {
                border: 1px solid #000;
                color: #000;
            }
        }",
        vec![
            ("border", "1px solid #FFF", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("color", "#FFF", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("border", "1px solid #000", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: Some("&:hover,&:active,&:nth-child(2)".to_string()),
            })),
            ("color", "#000", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: Some("&:hover,&:active,&:nth-child(2)".to_string()),
            })),
        ]
    )]
    #[case(
        "@media (min-width: 768px) {
            & {
                border: 1px solid #fff;
                color: #fff;
            }
            &:hover {
                border: 1px solid #000;
                color: #000;
            }
        }",
        vec![
            ("border", "1px solid #FFF", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("color", "#FFF", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("border", "1px solid #000", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: Some("&:hover".to_string()),
            })),
            ("color", "#000", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: Some("&:hover".to_string()),
            })),
        ]
    )]
    #[case(
        "@media (min-width: 768px) {
            & {
                border: 1px solid #fff;
                color: #fff;
            }
            &:hover {
                border: 1px solid #000;
                color: #000;
            }
        }
        @media (max-width: 768px) and (min-width: 480px) {
            & {
                border: 1px solid #fff;
                color: #fff;
            }
            &:hover {
                border: 1px solid #000;
                color: #000;
            }
        }",
        vec![
            ("border", "1px solid #FFF", Some(StyleSelector::Media {
                query: "(max-width:768px)and (min-width:480px)".to_string(),
                selector: None,
            })),
            ("color", "#FFF", Some(StyleSelector::Media {
                query: "(max-width:768px)and (min-width:480px)".to_string(),
                selector: None,
            })),
            ("border", "1px solid #000", Some(StyleSelector::Media {
                query: "(max-width:768px)and (min-width:480px)".to_string(),
                selector: Some("&:hover".to_string()),
            })),
            ("color", "#000", Some(StyleSelector::Media {
                query: "(max-width:768px)and (min-width:480px)".to_string(),
                selector: Some("&:hover".to_string()),
            })),
            ("border", "1px solid #FFF", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("color", "#FFF", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("border", "1px solid #000", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: Some("&:hover".to_string()),
            })),
            ("color", "#000", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: Some("&:hover".to_string()),
            })),
        ]
    )]
    #[case(
        "@media (min-width: 768px) {
            & {
                border: 1px solid #fff;
                color: #fff;
            }
        }
        @media (max-width: 768px) and (min-width: 480px) {
            border: 1px solid #000;
            color: #000;
        }",
        vec![
            ("border", "1px solid #FFF", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("color", "#FFF", Some(StyleSelector::Media {
                query: "(min-width:768px)".to_string(),
                selector: None,
            })),
            ("border", "1px solid #000", Some(StyleSelector::Media {
                query: "(max-width:768px)and (min-width:480px)".to_string(),
                selector: None,
            })),
            ("color", "#000", Some(StyleSelector::Media {
                query: "(max-width:768px)and (min-width:480px)".to_string(),
                selector: None,
            })),
        ]
    )]
    #[case(
        "@media (min-width: 768px) {
            & {
            }
        }
        @media (max-width: 768px) and (min-width: 480px) {
        }",
        vec![]
    )]
    #[case(
        "ul { font-family: 'Roboto Hello',       sans-serif; }",
        vec![
            ("font-family", "\"Roboto Hello\",sans-serif", Some(StyleSelector::Selector("ul".to_string()))),
        ]
    )]
    fn test_css_to_style(
        #[case] input: &str,
        #[case] expected: Vec<(&str, &str, Option<StyleSelector>)>,
    ) {
        let styles = css_to_style(input, 0, &None);
        let mut result: Vec<(&str, &str, Option<StyleSelector>)> = styles
            .iter()
            .map(|prop| (prop.property(), prop.value(), prop.selector().cloned()))
            .collect();
        result.sort();
        let mut expected_sorted = expected.clone();
        expected_sorted.sort();
        assert_eq!(result, expected_sorted);
    }

    #[rstest]
    #[case(
        "to {\nbackground-color:red;\n}\nfrom {\nbackground-color:blue;\n}",
        vec![
            ("to", vec![("background-color", "red")]),
            ("from", vec![("background-color", "blue")]),
        ],
    )]
    #[case(
        "0% { opacity: 0; }\n100% { opacity: 1; }",
        vec![
            ("0%", vec![("opacity", "0")]),
            ("100%", vec![("opacity", "1")]),
        ],
    )]
    #[case(
        "from { left: 0; }\nto { left: 100px; }",
        vec![
            ("from", vec![("left", "0")]),
            ("to", vec![("left", "100px")]),
        ],
    )]
    #[case(
        "50% { color: red; background: blue; }",
        vec![
            ("50%", vec![("color", "red"), ("background", "blue")]),
        ],
    )]
    #[case(
        "",
        vec![],
    )]
    #[case(
        "50% { color: red        ; background: blue; }",
        vec![
            ("50%", vec![("color", "red"), ("background", "blue")]),
        ],
    )]
    // comment case
    #[case(
        "50% { color: red; /*background: blue;*/ }",
        vec![
            ("50%", vec![("color", "red")]),
        ],
    )]
    // error case
    #[case(
        "50% { color: red        ; background: blue ",
        vec![
        ],
    )]
    fn test_keyframes_to_keyframes_style(
        #[case] input: &str,
        #[case] expected: Vec<(&str, Vec<(&str, &str)>)>,
    ) {
        let styles = keyframes_to_keyframes_style(input);
        if styles.len() != expected.len() {
            panic!("styles.len() != expected.len()");
        }
        for (expected_key, expected_styles) in styles.iter() {
            let styles = expected_styles;
            let mut result: Vec<(&str, &str)> = styles
                .iter()
                .map(|prop| (prop.property(), prop.value()))
                .collect();
            result.sort();
            let mut expected_sorted = expected
                .iter()
                .find(|(k, _)| k == expected_key)
                .map(|(_, v)| v.clone())
                .unwrap();
            expected_sorted.sort();
            assert_eq!(result, expected_sorted);
        }
    }
}
