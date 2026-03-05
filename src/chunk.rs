#[derive(Clone, Copy, Debug)]
pub enum OpCode {
    OpReturn,
}

/// `Chunk` is used to store loads of `OpCode`.
pub struct Chunk {
    code: Vec<OpCode>,
}

impl Chunk {
    /// Create a empty chunk object.
    pub fn new() -> Self {
        Self { code: vec![] }
    }

    /// Write a byte to the chunk.
    pub fn write(&mut self, byte: OpCode) {
        self.code.push(byte);
    }

    /// Disassemble chunk.
    pub fn disassemble(&self, name: &str) {
        // Print the name title so that we know which chunk we are looking.
        println!("===== {} =====", name);
        let mut offset = 0;
        // Execute each instruction (the size of instruction may be different).
        // TODO: Maybe we can make it a simple for loop instead of while loop.
        while offset < self.code.len() {
            offset = self.disassemble_instruction(offset);
        }
    }

    /// Disassemble and execute instruction with an offset in the chunk.
    pub fn disassemble_instruction(&self, offset: usize) -> usize {
        // Print the offset data with opcode name above.
        // fmt: 0000 OpReturn
        print!("{:04} ", offset);
        let opcode = self.code[offset];
        match opcode {
            OpCode::OpReturn => {
                println!("{:?}", opcode);
            }
            _ => {
                println!("Unknown opcode: {:?}", opcode);
            }
        }
        offset + 1
    }
}
