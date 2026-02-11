use crate::{
    COLOR_HASH, F_SPACE_RE, ZERO_RE,
    constant::{
        DOT_ZERO_RE, F_DOT_RE, F_RGB_RE, F_RGBA_RE, INNER_TRIM_RE, NUM_TRIM_RE, RM_MINUS_ZERO_RE,
        ZERO_PERCENT_FUNCTION,
    },
};

pub fn optimize_value(value: &str) -> String {
    let trimmed = value.trim();
    let mut ret = String::with_capacity(trimmed.len() + 8);
    ret.push_str(trimmed);

    // Wrap CSS custom property names in var() when used as values
    // e.g., "--var-0" becomes "var(--var-0)"
    if ret.starts_with("--") && !ret.contains(' ') && !ret.contains(',') {
        ret.insert_str(0, "var(");
        ret.push(')');
    }

    // Use Cow-aware replacement: only allocate when regex matches
    let replaced = INNER_TRIM_RE.replace_all(&ret, "(${1})");
    if let std::borrow::Cow::Owned(s) = replaced {
        ret = s;
    }

    // Skip RM_MINUS_ZERO_RE for values containing CSS custom property references
    // to preserve names like --var-0 (the -0 should not be converted to 0)
    if !ret.contains("--") {
        let replaced = RM_MINUS_ZERO_RE.replace_all(&ret, "0${1}");
        if let std::borrow::Cow::Owned(s) = replaced {
            ret = s;
        }
    }
    let replaced = NUM_TRIM_RE.replace_all(&ret, "${1} ${3}");
    if let std::borrow::Cow::Owned(s) = replaced {
        ret = s;
    }

    if ret.contains(',') {
        let replaced = F_SPACE_RE.replace_all(&ret, ",");
        if let std::borrow::Cow::Owned(s) = replaced {
            ret = s;
        }
    }
    let replaced = F_RGBA_RE.replace_all(&ret, |c: &regex::Captures| {
        let r = c[1].parse::<i32>().unwrap();
        let g = c[2].parse::<i32>().unwrap();
        let b = c[3].parse::<i32>().unwrap();
        let a = c[4].parse::<f32>().unwrap();
        format!(
            "#{:02X}{:02X}{:02X}{:02X}",
            r,
            g,
            b,
            (a * 255.0).round() as i32
        )
    });
    if let std::borrow::Cow::Owned(s) = replaced {
        ret = s;
    }
    let replaced = F_RGB_RE.replace_all(&ret, |c: &regex::Captures| {
        let r = c[1].parse::<i32>().unwrap();
        let g = c[2].parse::<i32>().unwrap();
        let b = c[3].parse::<i32>().unwrap();
        format!("#{r:02X}{g:02X}{b:02X}")
    });
    if let std::borrow::Cow::Owned(s) = replaced {
        ret = s;
    }
    if ret.contains('#') {
        let replaced = COLOR_HASH.replace_all(&ret, |c: &regex::Captures| optimize_color(&c[1]));
        if let std::borrow::Cow::Owned(s) = replaced {
            ret = s;
        }
    }
    if ret.contains('0') {
        let replaced = DOT_ZERO_RE.replace_all(&ret, "${1}0${2}");
        if let std::borrow::Cow::Owned(s) = replaced {
            ret = s;
        }
        let replaced = F_DOT_RE.replace_all(&ret, "${1}.${2}");
        if let std::borrow::Cow::Owned(s) = replaced {
            ret = s;
        }
        let replaced = ZERO_RE.replace_all(&ret, "${1}0");
        if let std::borrow::Cow::Owned(s) = replaced {
            ret = s;
        }

        for f in ZERO_PERCENT_FUNCTION.iter() {
            let tmp = ret.to_lowercase();
            if tmp.contains(f) {
                let index = tmp.find(f).unwrap() + f.len();
                let mut zero_idx = Vec::with_capacity(4);
                let mut depth = 0;
                let chars: Vec<char> = tmp.chars().collect();
                let byte_indices: Vec<usize> = tmp.char_indices().map(|(i, _)| i).collect();

                for (char_idx, &ch) in chars.iter().enumerate().skip(index) {
                    if ch == '(' {
                        depth += 1;
                    } else if ch == ')' {
                        depth -= 1;
                    } else if ch == '0'
                        && (char_idx == 0 || !chars[char_idx - 1].is_ascii_digit())
                        && (char_idx + 1 >= chars.len() || !chars[char_idx + 1].is_ascii_digit())
                        && depth == 0
                    {
                        zero_idx.push(byte_indices[char_idx]);
                    }
                }
                // In-place replacement: replace each '0' with '0%' from back to front
                for i in zero_idx.iter().rev() {
                    ret.replace_range(*i..*i + 1, "0%");
                }
            }
        }
    }
    // remove ; from dynamic value
    // Check suffix patterns directly without format! allocation
    for str_symbol in ["", "`", "\"", "'"] {
        let suffix_with_paren = if str_symbol.is_empty() {
            ";)".to_string()
        } else {
            let mut s = String::with_capacity(str_symbol.len() + 2);
            s.push(';');
            s.push_str(str_symbol);
            s.push(')');
            s
        };
        let suffix_without_paren = if str_symbol.is_empty() {
            ";".to_string()
        } else {
            let mut s = String::with_capacity(str_symbol.len() + 1);
            s.push(';');
            s.push_str(str_symbol);
            s
        };
        if ret.ends_with(&suffix_without_paren) {
            let base = ret[..ret.len() - suffix_without_paren.len()].trim_end_matches(';');
            let mut new_ret = String::with_capacity(base.len() + str_symbol.len());
            new_ret.push_str(base);
            new_ret.push_str(str_symbol);
            ret = new_ret;
        } else if ret.ends_with(&suffix_with_paren) {
            let base = ret[..ret.len() - suffix_with_paren.len()].trim_end_matches(';');
            let mut new_ret = String::with_capacity(base.len() + str_symbol.len() + 1);
            new_ret.push_str(base);
            new_ret.push_str(str_symbol);
            new_ret.push(')');
            ret = new_ret;
        }
    }

    if ret.contains('(') || ret.contains(')') {
        let mut depth: i32 = 0;
        for ch in ret.chars() {
            if ch == '(' {
                depth += 1;
            } else if ch == ')' {
                depth -= 1;
            }
        }
        if depth < 0 {
            for _ in 0..(-depth) {
                ret.insert(0, '(');
            }
        }
        if depth > 0 {
            for _ in 0..depth {
                ret.push(')');
            }
        }
    }
    ret
}

fn optimize_color(value: &str) -> String {
    let mut ret = value.to_uppercase();

    if ret.len() == 6 {
        let ch = ret.chars().collect::<Vec<char>>();
        if ch[0] == ch[1] && ch[2] == ch[3] && ch[4] == ch[5] {
            ret = format!("{}{}{}", ch[0], ch[2], ch[4]);
        }
    } else if ret.len() == 8 {
        let ch = ret.chars().collect::<Vec<char>>();
        if ch[0] == ch[1] && ch[2] == ch[3] && ch[4] == ch[5] && ch[6] == ch[7] {
            ret = format!("{}{}{}{}", ch[0], ch[2], ch[4], ch[6]);
            if ret.ends_with("F") {
                ret = ret[..ret.len() - 1].to_string();
            }
        } else if ret.ends_with("FF") {
            ret = ret[..ret.len() - 2].to_string();
        }
    } else if ret.len() == 4 && ret.ends_with("F") {
        ret = ret[..ret.len() - 1].to_string();
    }

    format!("#{ret}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case("0px", "0")]
    #[case("0.12px", ".12px")]
    #[case("-0.12px", "-.12px")]
    #[case("0.0px", "0")]
    #[case("0.0em", "0")]
    #[case("0.0rem", "0")]
    #[case("0.0vh", "0")]
    #[case("0.0vw", "0")]
    #[case("0.0%", "0")]
    #[case("0.0dvh", "0")]
    #[case("0.0dvw", "0")]
    #[case("1.3s", "1.3s")]
    #[case("0.3s", ".3s")]
    #[case("0.3s ease-in-out", ".3s ease-in-out")]
    #[case("0em", "0")]
    #[case("0rem", "0")]
    #[case("0vh", "0")]
    #[case("0vw", "0")]
    #[case("0%", "0")]
    #[case("0dvh", "0")]
    #[case("0dvw", "0")]
    #[case("0px 0px", "0 0")]
    #[case("-0px -0px", "0 0")]
    #[case("0.0px   -0px", "0 0")]
    #[case("0em 0em", "0 0")]
    #[case("0rem 0rem", "0 0")]
    #[case("0vh 0vh", "0 0")]
    #[case("0vw 0vw", "0 0")]
    #[case("-0vw -0vw", "0 0")]
    #[case("-0.2em", "-.2em")]
    #[case("-0.02em", "-.02em")]
    #[case("scale(0px)", "scale(0)")]
    #[case("scale(-0px)", "scale(0)")]
    #[case("scale(-0px);", "scale(0)")]
    #[case("rgba(255,12,12,0.5)", "#FF0C0C80")]
    #[case("rgba(255,12,12,.5)", "#FF0C0C80")]
    #[case("rgba(255,12,12,1)", "#FF0C0C")]
    #[case("rgba(255, 0, 0,    0.5)", "#FF000080")]
    #[case("rgba(255, 255, 255,   0.8  )", "#FFFC")]
    #[case("rgb(255,12,12)", "#FF0C0C")]
    #[case("rgb(255, 0, 0)", "#F00")]
    #[case("rgb(255, 255, 255)", "#FFF")]
    #[case("red;", "red")]
    #[case("translate(0px)", "translate(0)")]
    #[case("translate(-0px,0px)", "translate(0,0)")]
    #[case("translate(-0px, 0px)", "translate(0,0)")]
    #[case("translate(0px, 0px)", "translate(0,0)")]
    #[case("translate(10px, 0px)", "translate(10px,0)")]
    #[case("translate(     10px  , 0px   )", "translate(10px,0)")]
    #[case("translate(     0px  , 0px   )", "translate(0,0)")]
    #[case("         translate(     0px  , 0px   )         ", "translate(0,0)")]
    #[case("clamp(0, 10px, 10px)", "clamp(0%,10px,10px)")]
    #[case("clamp(10px, 0, 10px)", "clamp(10px,0%,10px)")]
    #[case("clamp(10px, 10px, 0)", "clamp(10px,10px,0%)")]
    #[case("clamp(0px, 10px, 0px)", "clamp(0%,10px,0%)")]
    #[case("min(0, 10px)", "min(0%,10px)")]
    #[case("max(0, 10px)", "max(0%,10px)")]
    #[case("min(10px, 0)", "min(10px,0%)")]
    #[case("max(10px, 0)", "max(10px,0%)")]
    #[case("max(some(0), 0)", "max(some(0),0%)")]
    #[case("max(some(0), -0)", "max(some(0),0%)")]
    #[case("translate(0, min(0, 10px))", "translate(0,min(0%,10px))")]
    #[case("\"red\"", "\"red\"")]
    #[case("'red'", "'red'")]
    #[case("`red`", "`red`")]
    #[case("\"red;\"", "\"red\"")]
    #[case("'red;'", "'red'")]
    #[case("`red;`", "`red`")]
    #[case("(\"red;\")", "(\"red\")")]
    #[case("(`red;`)", "(`red`)")]
    #[case("('red;')", "('red')")]
    #[case("('red') + 'blue;'", "('red') + 'blue'")]
    #[case("translateX(0px) translateY(0px)", "translateX(0) translateY(0)")]
    // recovery case
    #[case("max(10px, 0", "max(10px,0%)")]
    #[case("max(10px, calc(0", "max(10px,calc(0%))")]
    #[case("max(10px, any(0", "max(10px,any(0))")]
    #[case("10px, any(0))", "(10px,any(0))")]
    #[case("scale(0deg, 0deg)", "scale(0,0)")]
    #[case(
        "scaleX(0deg) scaleY(0deg) scaleZ(0deg)",
        "scaleX(0) scaleY(0) scaleZ(0)"
    )]
    #[case("scaleX(0deg)", "scaleX(0)")]
    #[case("scaleY(0deg)", "scaleY(0)")]
    #[case("scaleZ(0deg)", "scaleZ(0)")]
    #[case("translate(0px) scale(0deg)", "translate(0) scale(0)")]
    #[case("translate(-0px) scale(-0deg)", "translate(0) scale(0)")]
    #[case("translate(-10px) scale(-10deg)", "translate(-10px) scale(-10deg)")]
    fn test_optimize_value(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(optimize_value(input), expected);
    }

    #[rstest]
    #[case("#ff0000", "#F00")]
    #[case("#123456", "#123456")]
    #[case("#ff0000ff", "#F00")]
    #[case("#f00", "#F00")]
    #[case("#f00f", "#F00")]
    #[case("red", "red")]
    #[case("blue", "blue")]
    #[case("transparent", "transparent")]
    fn test_optimize_color(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(optimize_value(input), expected);
    }

    #[rstest]
    #[case("--var-0", "var(--var-0)")]
    #[case("--my-custom-prop", "var(--my-custom-prop)")]
    #[case("--primary-color", "var(--primary-color)")]
    #[case("var(--var-0)", "var(--var-0)")] // Already wrapped, don't double wrap
    #[case("--a --b", "--a --b")] // Contains space, don't wrap
    #[case("--a, --b", "--a,--b")] // Contains comma, don't wrap (spaces after commas are removed)
    fn test_css_custom_property_wrapping(#[case] input: &str, #[case] expected: &str) {
        assert_eq!(optimize_value(input), expected);
    }
}
