use std::{
    fmt::Display,
    hash::{DefaultHasher, Hash, Hasher},
};

use crate::{
    chunk::{Chunk, Value},
    constant::MAX_UPVALUE_SIZE,
    heap::Heap,
};

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

impl Display for ObjIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<obj {:06}>", self.val)
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

pub enum ObjData {
    String(ObjString),
    Function(ObjFunction),
    Native(ObjNative),
    /// `ObjClosure` have a large static stack, in order to make the size of `ObjData` smaller, we put
    /// it in heap (Using `Box`).
    Closure(Box<ObjClosure>),
    Upvalue(ObjUpvalue),
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
                write!(f, "{}", obj_closure)
            }
            ObjData::Upvalue(obj_upval) => {
                write!(f, "{}", obj_upval)
            }
            ObjData::Native(_) => {
                write!(f, "<native fn>")
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
    /// The object index of identifier of the function.
    pub name: ObjIndex,
    /// The byte chunk of function body.
    pub chunk: Chunk,
    /// The number of function parameters.
    pub arity: usize,
    /// The number of upvalues the function uses.
    pub upvalues_count: usize,
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
            upvalues_count: 0,
        }
    }
}

#[derive(Clone, Copy)]
pub enum UpvalueState {
    Location(usize),
    Closed(Value),
}

impl Display for UpvalueState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpvalueState::Location(idx) => write!(f, "<Local {}>", idx),
            UpvalueState::Closed(val) => write!(f, "Closed({})", val),
        }
    }
}

#[derive(Clone, Copy)]
/// Make a upvalue object to manage closed upvalues.
/// The upvalue is `Location` if it's open, else closed.
pub struct ObjUpvalue {
    val: UpvalueState,
}

impl Display for ObjUpvalue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.val)
    }
}

impl ObjUpvalue {
    /// Create a open upvalue object.
    pub fn location(idx: usize) -> Self {
        Self {
            val: UpvalueState::Location(idx),
        }
    }

    /// Create a closed upvalue object.
    pub fn closed(val: Value) -> Self {
        Self {
            val: UpvalueState::Closed(val),
        }
    }

    /// Check if the upvalue is open.
    pub fn is_open(&self) -> bool {
        matches!(self.val, UpvalueState::Location(_))
    }

    /// Check if the upvalue is closed.
    pub fn is_closed(&self) -> bool {
        matches!(self.val, UpvalueState::Closed(_))
    }

    pub fn as_location(&self) -> usize {
        match self.val {
            UpvalueState::Location(idx) => idx,
            _ => unreachable!(),
        }
    }

    /// Return a immutable reference to the value of the upvalue, only if it's closed.
    pub fn as_val(&self) -> &Value {
        match self.val {
            UpvalueState::Closed(ref val) => val,
            _ => unreachable!(),
        }
    }

    /// Return a mutable reference to the value of the upvalue, only if it's closed.
    pub fn as_val_mut(&mut self) -> &mut Value {
        match self.val {
            UpvalueState::Closed(ref mut val) => val,
            _ => unreachable!(),
        }
    }
}

pub struct ObjClosure {
    /// The object index of function object.
    pub func: ObjIndex,
    /// List of upvalues.
    pub upvalues: [Option<ObjIndex>; MAX_UPVALUE_SIZE],
    /// The amount of upvalues.
    pub upvalue_count: usize,
}

impl Display for ObjClosure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.func)
    }
}

impl ObjClosure {
    pub fn new(func_obj_idx: ObjIndex, upvalue_count: usize) -> Self {
        Self {
            func: func_obj_idx,
            upvalues: [None; MAX_UPVALUE_SIZE],
            upvalue_count,
        }
    }
}

type NativeFn = fn(argc: usize, args: &[Option<Value>], heap: &Heap) -> Value;

pub struct ObjNative {
    pub func: NativeFn,
}

impl ObjNative {
    pub fn new(native_fn: NativeFn) -> Self {
        Self { func: native_fn }
    }
}
