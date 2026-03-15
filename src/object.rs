use std::{
    fmt::Display,
    hash::{DefaultHasher, Hash, Hasher},
};

#[derive(Clone, Copy, PartialEq, Debug)]
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
            }
            #[allow(unreachable_patterns)]
            _ => {
                unreachable!()
            }
        }
    }
}

#[derive(Hash, PartialEq, Clone)]
pub struct ObjString {
    pub value: String,
    /// Pre-store hash value to avoid runtime overhead.
    pub hash: u64,
}

impl Display for ObjString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl ObjString {
    /// Allocates a new `ObjString`, coping the characters from `s`.
    pub fn new(s: &str) -> Self {
        let mut h = DefaultHasher::new();
        s.hash(&mut h);
        Self {
            value: String::from(s),
            hash: h.finish(),
        }
    }
}
