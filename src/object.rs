use std::{
    fmt::Display,
    hash::{DefaultHasher, Hash, Hasher},
};

use crate::chunk::Chunk;

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ObjIndex {
    pub val: usize,
    pub hash: u64,
}

impl From<usize> for ObjIndex {
    fn from(value: usize) -> Self {
        Self::new(value)
    }
}

impl ObjIndex {
    pub fn new(id: usize) -> Self {
        let mut h = DefaultHasher::new();
        id.hash(&mut h);
        Self {
            val: id,
            hash: h.finish(),
        }
    }
}

impl Display for ObjIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:06}", self.val)
    }
}

pub enum ObjData {
    String(ObjString),
    Function(ObjFunction),
    Closure(ObjClosure),
}

impl Display for ObjData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObjData::String(obj_string) => {
                write!(f, "{}", obj_string)
            }
            ObjData::Function(obj_func) => {
                write!(f, "{}", obj_func)
            }
            ObjData::Closure(obj_closure) => {
                write!(f, "{}", obj_closure.func)
            }
        }
    }
}

#[derive(PartialEq, Clone)]
pub struct ObjString {
    pub value: String,
}

impl Display for ObjString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl ObjString {
    /// Allocates a new `ObjString`, coping the characters from `s`.
    pub fn new(s: &str) -> Self {
        Self {
            value: String::from(s),
        }
    }
}

pub struct ObjFunction {
    pub name: ObjIndex,
    pub chunk: Chunk,
    pub arity: usize,
}

#[derive(Clone, Copy, PartialEq)]
pub enum FunctionType {
    /// The type used to represent the global scope
    Global,
    Function,
}

impl Display for ObjFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<obj {}>", self.name)
    }
}

impl ObjFunction {
    pub fn new(name: ObjIndex, arity: usize) -> Self {
        Self {
            name,
            chunk: Chunk::new(),
            arity,
        }
    }
}

pub struct ObjClosure {
    pub obj: ObjIndex,
    /// The object index of function object.
    pub func: ObjIndex,
}

impl ObjClosure {
    pub fn new(func: ObjIndex) -> Self {
        Self {
            // TODO: obj not implemented.
            obj: ObjIndex::new(0),
            func,
        }
    }
}
