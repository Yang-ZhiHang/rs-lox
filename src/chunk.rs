use crate::constant::Constant;

#[derive(Clone, Copy, Debug, strum::FromRepr)]
#[repr(u8)]
pub enum OpCode {
    OpReturn,
    OpConstant,
}

/// A trait for types that can be written into the chunk as a single byte.
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

/// `Chunk` is used to store loads of `OpCode`.
pub struct Chunk {
    /// Code area (code segment)
    /// We use `u8` to be the element type instead of `OpCode` because there might
    /// be constant value index which the type is not `OpCode` but usize.
    code: Vec<u8>,
    /// Constant area (BSS or heap).
    constants: Constant,
    /// Container to stored the line number of each code.
    line: Vec<u32>,
}

impl Chunk {
    /// Create a empty chunk object.
    pub fn new() -> Self {
        Self {
            code: vec![],
            constants: Constant::new(),
            line: vec![],
        }
    }

    /// Write a byte to the chunk.
    pub fn write(&mut self, byte: impl IntoU8, line: u32) {
        self.code.push(byte.into_u8());
        self.line.push(line);
    }

    /// Disassemble chunk.
    pub fn disassemble(&self, name: &str) {
        // Print the name title so that we know which chunk we are looking.
        println!("== {} ==", name);
        println!("offset line opcode");
        let mut offset = 0;
        // Execute each instruction (the size of instruction may be different).
        while offset < self.code.len() {
            offset = self.disassemble_instruction(offset);
        }
    }

    /// Disassemble and execute instruction with an offset in the chunk.
    pub fn disassemble_instruction(&self, offset: usize) -> usize {
        // Print the offset, line number and opcode.
        // fmt: 000000 0001 OpReturn
        if offset > 0 && self.line[offset] == self.line[offset - 1] {
            print!("{:06} {:04} ", offset, "-");
        } else {
            print!("{:06} {:04} ", offset, self.line[offset]);
        }
        let byte = self.code[offset];
        match OpCode::from_repr(byte) {
            Some(opcode) => match opcode {
                OpCode::OpReturn => self.simple_instruction(offset, opcode),
                OpCode::OpConstant => self.constant_instruction(offset, opcode),
            },
            None => {
                println!("Unknown opcode: {}", byte);
                offset + 1
            }
        }
    }

    /// Write a constant value to the constant area and return the value index
    /// in the constant area.
    pub fn write_constant(&mut self, value: f64) -> usize {
        self.constants.write(value)
    }

    /// Just print the opcode name to the console.
    pub fn simple_instruction(&self, offset: usize, opcode: OpCode) -> usize {
        println!("{:?}", opcode);
        offset + 1
    }

    /// Print the constant opcode value to the console.
    pub fn constant_instruction(&self, offset: usize, opcode: OpCode) -> usize {
        let val = self.constants.values(self.code[offset + 1] as usize);
        println!("{:?} {}", opcode, val);
        offset + 2
    }
}
