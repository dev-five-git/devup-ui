use std::fmt::Write;

pub(crate) fn encode_selector(selector: &str) -> String {
    let mut result = String::with_capacity(selector.len() + 8);
    for character in selector.chars() {
        let encoded = match character {
            '&' => "_a_",
            ':' => "_c_",
            '(' => "_lp_",
            ')' => "_rp_",
            '[' => "_lb_",
            ']' => "_rb_",
            '=' => "_eq_",
            '>' => "_gt_",
            '<' => "_lt_",
            '~' => "_tl_",
            '+' => "_pl_",
            ' ' => "_s_",
            '*' => "_st_",
            '.' => "_d_",
            '#' => "_h_",
            ',' => "_cm_",
            '"' => "_dq_",
            '\'' => "_sq_",
            '/' => "_sl_",
            '\\' => "_bs_",
            '%' => "_pc_",
            '^' => "_cr_",
            '$' => "_dl_",
            '|' => "_pp_",
            '@' => "_at_",
            '!' => "_ex_",
            '?' => "_qm_",
            ';' => "_sc_",
            '{' => "_lc_",
            '}' => "_rc_",
            character if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') => {
                result.push(character);
                continue;
            }
            character => {
                let _ = write!(result, "_u{:04x}_", u32::from(character));
                continue;
            }
        };
        result.push_str(encoded);
    }
    result
}

pub(crate) fn write_u8(output: &mut String, value: u8) {
    if value >= 100 {
        output.push(char::from(b'0' + value / 100));
        output.push(char::from(b'0' + (value / 10) % 10));
    } else if value >= 10 {
        output.push(char::from(b'0' + value / 10));
    }
    output.push(char::from(b'0' + value % 10));
}
