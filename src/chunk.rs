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
    Negate,
    Not,
    /// Binary
    Add,
    Subtract,
    Multiply,
    Divide,
    Less,
    Greater,
    Equal,
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
            Value::Number(_) => true,
        }
    }

    /// Return true if the result of the expression is false else false.
    pub fn is_falsey(&self) -> bool {
        match self {
            Value::Nil => true,
            Value::Bool(b) => !b,
            Value::Number(n) => *n == 0.0,
        }
    }

    /// Print the inner field value to the console.
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
    line: Vec<(u16, u16)>,
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
    pub fn line(&self) -> &[(u16, u16)] {
        &self.line
    }

    /// Get the line number of opcode in given offset.
    pub fn get_line(&self, offset: usize) -> u16 {
        let mut acc = 0;
        for pair in self.line.iter() {
            acc += pair.1;
            if acc > offset as u16 {
                return pair.0;
            }
        }
        panic!("Unavailable offset.")
    }

    /// Write a byte to the chunk.
    pub fn write(&mut self, byte: impl IntoU8, line: u16) {
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
