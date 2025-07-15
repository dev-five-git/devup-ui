use css::{StyleSelector, to_camel_case};

use crate::{
    ExtractStyleProp,
    extract_style::{ExtractStaticStyle, ExtractStyleValue},
};
pub fn css_to_style<'a>(
    css: &str,
    level: u8,
    selector: &Option<&StyleSelector>,
) -> Vec<ExtractStyleProp<'a>> {
    let mut styles = vec![];
    for s in css.split(";") {
        let mut iter = s.split(":");
        let property = to_camel_case(iter.next().unwrap().trim());
        let value = iter.next().unwrap().trim();
        styles.push(ExtractStyleProp::Static(ExtractStyleValue::Static(
            ExtractStaticStyle::new(&property, value, level, selector.cloned()),
        )));
    }
    styles
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

    #[test]
    fn test_optimize_css_block() {
        assert_eq!(
            optimize_css_block(
                "       img      {       background-color    :       red;      }     "
            ),
            "img{background-color:red}"
        );
        assert_eq!(
            optimize_css_block(
                "       img      {       background-color    :       red;          color     :          blue;      }     "
            ),
            "img{background-color:red;color:blue}"
        );
    }
}
