pub use crate::class_selection_state::{Control, External, State, states};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Declaration {
    pub property: String,
    pub value: String,
    pub level: u8,
    pub selector: Option<String>,
    pub layer: Option<String>,
    pub order: u8,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Meaning {
    Declaration(Declaration),
    Typography(String),
    Keyframes(String),
    Opaque(String),
}

pub fn declaration(property: &str, value: &str) -> Meaning {
    Meaning::Declaration(atom(property, value))
}

fn atom(property: &str, value: &str) -> Declaration {
    Declaration {
        property: property.into(),
        value: value.into(),
        level: 0,
        selector: None,
        layer: None,
        order: 255,
    }
}

pub fn expected(case: usize, state: State, single_css: bool) -> Vec<Meaning> {
    let on = state.on.truthy();
    let mut result = Vec::new();
    let color = match case {
        1 | 5 | 6 | 7 | 12 | 23 | 25 | 33 => "azure",
        2 | 3 | 19 | 20 | 24 | 26 | 28 | 30 | 31 | 34 => "yellow",
        4 => "green",
        8 => {
            if on {
                "azure"
            } else {
                "yellow"
            }
        }
        9 | 13 | 21 => {
            if on {
                "yellow"
            } else {
                "azure"
            }
        }
        10 => {
            if on {
                "pink"
            } else {
                "yellow"
            }
        }
        11 => {
            if on {
                "pink"
            } else {
                "teal"
            }
        }
        14 | 15 => "a1",
        16 => "navy",
        17 => "",
        18 => "gold",
        22 => match state.ext {
            External::Null | External::Undefined => "azure",
            External::Zero | External::Text(_) => "yellow",
        },
        27 => {
            if on {
                "a"
            } else {
                "b"
            }
        }
        29 => {
            if on {
                if state.other { "a" } else { "b" }
            } else {
                "yellow"
            }
        }
        32 => "pink",
        _ => panic!("unregistered original case {case}"),
    };
    if !color.is_empty() {
        let mut plain = atom("color", color);
        plain.order = match case {
            16 => 3,
            27 => 2,
            _ => 255,
        };
        result.push(Meaning::Declaration(plain));
    }
    if !matches!(case, 14 | 15 | 17 | 25) && (case != 9 || on) {
        let mut hover = atom("color", "red");
        hover.selector = Some("&:hover".into());
        result.push(Meaning::Declaration(hover));
    }
    if color == "azure" || matches!(case, 2 | 14 | 18) {
        result.push(declaration("background", "black"));
    }
    match case {
        12 => match state.ext {
            External::Text(text) => result.extend(
                text.split_whitespace()
                    .map(|token| Meaning::Opaque(token.into())),
            ),
            External::Null | External::Undefined | External::Zero => {
                unreachable!("text-only fixture states")
            }
        },
        13 if on => result.push(Meaning::Opaque("plain".into())),
        14 | 15 => {
            let mut responsive = atom("color", if case == 14 { "b1" } else { "x2" });
            responsive.level = 1;
            result.push(Meaning::Declaration(responsive));
        }
        17 => result.push(Meaning::Typography("body".into())),
        19 => match state.size {
            "a" => result.push(declaration("color", "red")),
            "b" => result.push(declaration("color", "blue")),
            _ => {}
        },
        20 if on && state.size == "a" => result.push(declaration("color", "red")),
        21 | 26 => match state.on {
            Control::Opaque => result.push(Meaning::Opaque("opaque".into())),
            Control::False if case == 26 => result.push(Meaning::Opaque("plain".into())),
            Control::False | Control::True => {}
        },
        22 => match state.ext {
            External::Text(text) => result.push(Meaning::Opaque(text.into())),
            External::Null | External::Undefined | External::Zero => {}
        },
        27 => {
            for (level, value) in [(0, "4px"), (1, "8px")] {
                let mut margin = atom("margin", value);
                margin.level = level;
                margin.order = 2;
                result.push(Meaning::Declaration(margin));
            }
        }
        28 | 31 => {
            let lookup = match case {
                28 if state.size == "a" => Some("x"),
                31 if state.idx == 0 => Some("x"),
                31 if state.idx == 1 => Some("y"),
                _ => None,
            };
            if let Some(color) = lookup {
                let mut lookup = atom("color", color);
                lookup.order = 2;
                result.push(Meaning::Declaration(lookup));
            }
        }
        30 => result.push(Meaning::Keyframes(
            if single_css { "a" } else { "a-a" }.into(),
        )),
        _ => {}
    }
    result.sort();
    result
}
