use std::{
    cell::RefCell,
    fmt::Display,
    hash::{DefaultHasher, Hash, Hasher},
    rc::Rc,
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
    Upvalue(Value),
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

#[derive(Clone, Copy, PartialEq)]
pub enum FunctionType {
    /// The type used to represent the global scope
    Global,
    Function,
}

pub struct ObjFunction {
    /// The object index of identifier of the function.
    pub name: ObjIndex,

    /// The byte chunk of function body.
    ///
    /// Use `Rc` to unbind the reference from the heap in runtime, Making the cost of copying avoided. In
    /// addition, we don't use `RefCell` because at compile time, the chunk only references by its function,
    /// which will not lead to cow.
    pub chunk: Rc<Chunk>,

    /// The number of function parameters.
    pub arity: usize,

    /// The number of upvalues the function uses.
    pub upvalues_count: usize,
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
            chunk: Rc::new(Chunk::new()),
            arity,
            upvalues_count: 0,
        }
    }
}

#[derive(Clone, Copy)]
pub enum UpvalueState {
    Open(usize),
    Closed(ObjIndex),
}

impl Display for UpvalueState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpvalueState::Open(idx) => write!(f, "<Local {}>", idx),
            UpvalueState::Closed(obj_idx) => write!(f, "<Closed {}>", obj_idx),
        }
    }
}

impl UpvalueState {
    /// Create a open upvalue object.
    pub fn open(idx: usize) -> Self {
        Self::Open(idx)
    }

    /// Create a closed upvalue object.
    pub fn closed(obj_val_idx: ObjIndex) -> Self {
        Self::Closed(obj_val_idx)
    }

    /// Check if the upvalue is open.
    pub fn is_open(&self) -> bool {
        matches!(self, Self::Open(_))
    }

    /// Check if the upvalue is closed.
    pub fn is_closed(&self) -> bool {
        matches!(self, Self::Closed(_))
    }

    /// Return the local variable index of the upvalue, only if it's open.
    pub fn as_idx(&self) -> usize {
        match self {
            Self::Open(idx) => *idx,
            _ => unreachable!(),
        }
    }

    /// Return a immutable reference to the value of the upvalue, only if it's closed.
    pub fn as_val(&self, heap: &Heap) -> Value {
        match self {
            Self::Closed(obj_idx) => match heap.get(*obj_idx) {
                ObjData::Upvalue(val) => *val,
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }

    /// Set the value of the upvalue, only if it's closed.
    pub fn set(&mut self, heap: &mut Heap, val: Value) {
        match self {
            Self::Closed(obj_idx) => {
                let upval = heap.get_upvalue_mut(*obj_idx);
                *upval = val;
            }
            _ => unreachable!(),
        }
    }
}

pub struct ObjClosure {
    /// The object index of function object.
    pub func: ObjIndex,
    /// List of upvalues.
    pub upvalues: [Option<Rc<RefCell<UpvalueState>>>; MAX_UPVALUE_SIZE],
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
            upvalues: [const { None }; MAX_UPVALUE_SIZE],
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
