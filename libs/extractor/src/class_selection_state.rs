#[derive(Clone, Copy, Debug)]
pub enum Control {
    False,
    True,
    Opaque,
}

impl Control {
    pub const fn truthy(self) -> bool {
        match self {
            Self::False => false,
            Self::True | Self::Opaque => true,
        }
    }

    pub const fn javascript(self) -> &'static str {
        match self {
            Self::False => "false",
            Self::True => "true",
            Self::Opaque => "'opaque'",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum External {
    Null,
    Undefined,
    Zero,
    Text(&'static str),
}

impl External {
    pub fn javascript(self) -> String {
        match self {
            Self::Null => "null".into(),
            Self::Undefined => "undefined".into(),
            Self::Zero => "0".into(),
            Self::Text(text) => serde_json::to_string(text)
                .unwrap_or_else(|error| panic!("external string fixture: {error}")),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct State {
    pub on: Control,
    pub other: bool,
    pub ext: External,
    pub size: &'static str,
    pub idx: u8,
}

pub fn states(case: usize) -> Vec<State> {
    let mut result = Vec::new();
    for on in [Control::False, Control::True, Control::Opaque] {
        let base = State {
            on,
            other: false,
            ext: External::Text("external"),
            size: "a",
            idx: 0,
        };
        match case {
            12 => {
                for ext in [
                    External::Text(""),
                    External::Text("external"),
                    External::Text("external extra"),
                ] {
                    result.push(State { ext, ..base });
                }
            }
            19 | 20 | 28 => {
                for size in ["a", "b", "missing"] {
                    result.push(State { size, ..base });
                }
            }
            22 => {
                for ext in [
                    External::Null,
                    External::Undefined,
                    External::Text("external"),
                    External::Zero,
                ] {
                    result.push(State { ext, ..base });
                }
            }
            29 => {
                for other in [false, true] {
                    result.push(State { other, ..base });
                }
            }
            31 => {
                for idx in [0, 1, 9] {
                    result.push(State { idx, ..base });
                }
            }
            _ => result.push(base),
        }
    }
    result
}
