use std::fmt::Display;

use crate::{
    heap::Heap,
    object::{ObjData, ObjIndex},
};

// Use strum to automatically distribute number for enum member. It's useful when we
// read bytes data and detect it is opcode or index.
#[rustfmt::skip]
#[derive(Clone, Copy, Debug, strum::Display, strum::FromRepr)]
#[repr(u8)]
pub enum OpCode {
    Return, Print, Pop, Call,
    /// Condition
    JumpIfFalse, Jump, Loop,
    /// Variable
    DefineGlobal, GetGlobal, SetGlobal,
    GetLocal, SetLocal,
    /// Literal
    Constant, Nil, True, False,
    /// Unary
    Negate, Not,
    /// Binary
    Add, Sub, Mul, Div, Less, Greater, Equal,
}

/// A trait for types that can be written into the chunk as a single byte.
/// We can't use `Into<u8>` directly because the orphan rule forbids implementing
/// `From<usize> for u8` (both types are from std).
pub trait IntoU8 {
    fn into_u8(self) -> u8;
}

impl IntoU8 for OpCode {
    fn into_u8(self) -> u8 {
        self as u8
    }
}

impl IntoU8 for usize {
    fn into_u8(self) -> u8 {
        self as u8
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Value {
    Nil,
    Bool(bool),
    Number(f64),
    /// A heap-allocated Lox object, referenced by index.
    ///
    /// `ObjId` (an index into the VM's `Heap`) is stored rather than
    /// `ObjData` so that `Value` remains `Copy` trait.
    Object(ObjIndex),
}

impl Value {
    /// Return `f64` if the value is `Value::Number` else error.
    pub fn as_number(&self) -> Result<f64, &'static str> {
        match self {
            Value::Number(n) => Ok(*n),
            _ => Err("Operand must be a number."),
        }
    }

    /// Return a muttable reference `f64` if the value is `Value::Number`
    /// else error.
    pub fn as_number_mut(&mut self) -> Result<&mut f64, &'static str> {
        match self {
            Value::Number(n) => Ok(n),
            _ => Err("Operand must be a number."),
        }
    }

    /// Return a copy of `String` if the `Value::Object` is `ObjString`
    /// else error.
    pub fn as_string(&self, heap: &Heap) -> Result<String, &'static str> {
        if let Value::Object(obj_idx) = self
            && let ObjData::String(obj) = heap.get(*obj_idx)
        {
            let s = String::from(&obj.value);
            return Ok(s);
        };
        Err("Operand must be a string.")
    }

    /// Return `bool` if the value is `Value::Bool` else error.
    pub fn as_bool(&self) -> Result<bool, &'static str> {
        match self {
            Value::Bool(b) => Ok(*b),
            _ => Err("Operand must be a bool."),
        }
    }

    /// Return true if the value is `Value::Number` else false.
    pub fn is_number(&self) -> bool {
        matches!(self, Value::Number(_))
    }

    /// Return true if the value is `Value::Object` else false.
    pub fn is_string(&self, heap: &Heap) -> bool {
        match self {
            Value::Object(obj_idx) => {
                if let ObjData::String(_) = heap.get(*obj_idx) {
                    return true;
                };
                false
            }
            _ => false,
        }
    }

    /// Return true if the value is `Value::Bool` else false.
    pub fn is_bool(&self) -> bool {
        matches!(self, Value::Bool(_))
    }

    /// Return true if the value is `Value::Nil` else false.
    pub fn is_nil(&self) -> bool {
        matches!(self, Value::Nil)
    }

    /// Return true if the result of the expression is truth else false.
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Nil => false,
            Value::Bool(b) => *b,
            Value::Number(n) => *n != 0.0,
            Value::Object(_) => true,
        }
    }

    /// Return true if the result of the expression is false else false.
    pub fn is_falsey(&self) -> bool {
        match self {
            Value::Nil => true,
            Value::Bool(b) => !b,
            Value::Number(n) => *n == 0.0,
            Value::Object(_) => false,
        }
    }

    /// Convert `Value` into `String`.
    ///
    /// `Heap` is needed to get real value of object such as string.
    pub fn to_string(&self, heap: &Heap) -> String {
        match self {
            Value::Nil => "nil".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => n.to_string(),
            Value::Object(obj_idx) => heap.get(*obj_idx).to_string(),
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Nil => write!(f, "nil"),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Number(n) => write!(f, "{}", n),
            Value::Object(obj_idx) => write!(f, "<obj {}>", obj_idx.val),
        }
    }
}

/// `Chunk` is used to store loads of `OpCode`.
/// All of the member of `Chunk` is private, because the members are related to each
/// other (instead of pure data container), which will cause chaos if make them public.
pub struct Chunk {
    /// Code area (code segment)
    /// We use `u8` to be the element type instead of `OpCode` because there might
    /// be constant value index which the type is not `OpCode` but usize.
    code: Vec<u8>,
    /// Constant area (BSS or heap).
    constants: Vec<Value>,
    /// Container to stored the line number of each code.
    /// fmt: (line number, count)
    /// We use this format (RLE) instead of making line number to be index and count to be
    /// value, because we shouldn't store empty line.
    line: Vec<(usize, usize)>,
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new()
    }
}

impl Chunk {
    /// Create a empty chunk object.
    pub fn new() -> Self {
        Self {
            code: vec![],
            constants: vec![],
            line: vec![],
        }
    }

    /// Getter of member `code`.
    pub fn code(&self) -> &[u8] {
        &self.code
    }

    /// Get a muttable reference of member `code`.
    pub fn code_mut(&mut self) -> &mut [u8] {
        &mut self.code
    }

    /// Getter of member `constants`.
    pub fn constants(&self) -> &[Value] {
        &self.constants
    }

    /// Getter of member `line`.
    pub fn line(&self) -> &[(usize, usize)] {
        &self.line
    }

    /// Get the line number of opcode in given offset.
    pub fn get_line(&self, offset: usize) -> usize {
        let mut acc = 0;
        for pair in self.line.iter() {
            acc += pair.1;
            if acc > offset {
                return pair.0;
            }
        }
        // panic!("Unavailable offset {}.", offset);
        0
    }

    /// Write a byte to the chunk.
    pub fn write(&mut self, byte: impl IntoU8, line: usize) {
        self.code.push(byte.into_u8());
        match self.line.last_mut() {
            // Increase line number count if the line number already exists.
            Some(pair) if pair.0 == line => pair.1 += 1,
            // Push a new space to line list if it's empty or new line number.
            _ => self.line.push((line, 1)),
        }
    }

    /// Write a constant value to the constant area and return the value index
    /// in the constant area.
    pub fn write_constant(&mut self, v: Value) -> usize {
        self.constants.push(v);
        self.constants.len() - 1
    }
}
