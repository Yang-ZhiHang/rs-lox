use std::{
    fmt::Display,
    hash::{DefaultHasher, Hash, Hasher},
};

use crate::{chunk::Chunk, heap::Heap};

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ObjId {
    pub val: usize,
    pub hash: u64,
}

impl ObjId {
    pub fn new(id: usize) -> Self {
        let mut h = DefaultHasher::new();
        id.hash(&mut h);
        Self {
            val: id,
            hash: h.finish(),
        }
    }
}

impl Display for ObjId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.val)
    }
}

pub enum ObjData {
    String(ObjString),
    Function(ObjFunction),
    // Closure,
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
    pub name: ObjId,
    pub chunk: Chunk,
    pub arity: usize,
}

pub enum FunctionType {
    /// The type used to represent the global scope
    Script,
    Function,
}

impl Display for ObjFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<fn {}>", self.name)
    }
}

impl ObjFunction {
    pub fn new(name: ObjId, arity: usize) -> Self {
        Self {
            name,
            chunk: Chunk::new(),
            arity,
        }
    }
}
