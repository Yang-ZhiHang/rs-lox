// Use strum to automatically distribute number for enum member. It's useful when we
// read bytes data and detect it is opcode or index.
#[derive(Clone, Copy, Debug, strum::Display, strum::FromRepr)]
#[repr(u8)]
pub enum OpCode {
    Return,
    /// Literal
    // There is still one byte of space after `OpConstant` for storing the constant index.
    Constant,
    Nil,
    True,
    False,
    /// Unary
    UnaryNegate,
    UnaryNot,
    /// Binary
    BinaryAdd,
    BinarySubtract,
    BinaryMultiply,
    BinaryDivide,
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Value {
    Nil,
    Bool(bool),
    Number(f64),
}

impl Value {
    /// Unwrap the `Value` struct and return f64 if the value is `Number`.
    /// Otherwise, panic.
    /// `is_number` function is always called before this function.
    pub fn as_number(&self) -> f64 {
        match self {
            Value::Number(n) => *n,
            _ => panic!("Operand must be a number."),
        }
    }

    /// Unwrap the `Value` struct and return the mutable reference f64 if
    /// the value is `Number`. Otherwise, panic.
    /// `is_number` function is always called before this function.
    pub fn as_number_mut(&mut self) -> Option<&mut f64> {
        match self {
            Value::Number(n) => Some(n),
            _ => None,
        }
    }

    /// Unwrap the `Value` struct and return bool if the value is `Bool`.
    /// Otherwise, panic.
    /// `is_bool` function is always called before this function.
    pub fn as_bool(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            _ => panic!("Operand must be a bool."),
        }
    }

    /// Check if the `Value` struct is f64 and return bool.
    pub fn is_number(&self) -> bool {
        matches!(self, Value::Number(_))
    }

    /// Check if the `Value` struct is a bool value and return bool.
    pub fn is_bool(&self) -> bool {
        matches!(self, Value::Bool(_))
    }

    /// Check if the `Value` struct is nil and return bool.
    pub fn is_nil(&self) -> bool {
        matches!(self, Value::Nil)
    }

    /// Check if the `Value` struct is truth or not and return bool.
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Nil => false,
            Value::Bool(b) => *b,
            Value::Number(_) => true,
        }
    }

    /// Check if the `Value` struct is false and return bool.
    pub fn is_falsey(&self) -> bool {
        self.is_nil()
            || (self.is_bool() && !self.as_bool())
            || (self.is_number() && self.as_number() == 0f64)
    }
    
    /// Print the inner field value to console.
    pub fn print(&self) {
        match self {
            Value::Nil => println!("nil"),
            Value::Bool(b) => println!("{}", b),
            Value::Number(n) => println!("{}", n),
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
    line: Vec<(u32, u32)>,
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

    /// Getter of member `constants`.
    pub fn constants(&self) -> &[Value] {
        &self.constants
    }

    /// Getter of member `line`.
    pub fn line(&self) -> &[(u32, u32)] {
        &self.line
    }

    /// Get the line number of opcode in given offset.
    pub fn get_line(&self, offset: usize) -> u32 {
        let mut acc = 0;
        for pair in self.line.iter() {
            acc += pair.1;
            if acc > offset as u32 {
                return pair.0;
            }
        }
        panic!("Unavailable offset.")
    }

    /// Write a byte to the chunk.
    pub fn write(&mut self, byte: impl IntoU8, line: u32) {
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
    pub fn write_constant(&mut self, value: Value) -> usize {
        self.constants.push(value);
        self.constants.len() - 1
    }
}
