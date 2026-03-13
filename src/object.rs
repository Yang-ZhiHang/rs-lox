use std::fmt::Display;

#[derive(Clone, Copy, PartialEq)]
pub struct ObjId(pub usize);

pub enum ObjData {
    String(ObjString),
    // Function,
    // Closure,
}

impl Display for ObjData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObjData::String(obj) => {
                let s = obj.value.as_str();
                write!(f, "{}", s)
            } // _ => {
              //     // Unreachable
              //     write!(f, "To be implemented.")
              // }
        }
    }
}

pub struct ObjString {
    pub value: String,
}

impl ObjString {
    /// Allocates a new `ObjString`, coping the characters from `s`.
    pub fn new(s: &str) -> Self {
        Self {
            value: String::from(s),
        }
    }
}
