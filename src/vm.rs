use crate::chunk::{Chunk, OpCode};

pub struct VM {
    /// The byte chunk virutal machine need to interpret.
    chunk: Option<Chunk>,
    /// Program counter, use to represent where has the code been executed.
    /// TODO: Is pointer better than counter in rust?
    pc: usize,
}

impl VM {
    /// Create a empty virtual machine.
    /// chunk should be pass in when we call `interpret` function.
    pub fn new() -> Self {
        Self { chunk: None, pc: 0 }
    }

    /// Interpret the given byte chunk.
    pub fn interpret(&mut self, chunk: Chunk) {
        self.chunk = Some(chunk);
        self.run();
    }

    /// Run the opcode from the byte chunk.
    pub fn run(&mut self) {
        if let Some(chunk) = &self.chunk {
            while self.pc < chunk.code().len() {
                let opcode = Self::read_byte(chunk, &mut self.pc);
                match OpCode::from_repr(opcode) {
                    Some(opcode) => match opcode {
                        OpCode::OpConstant => {
                            let index = Self::read_byte(chunk, &mut self.pc);
                            let val = chunk.constants()[index as usize];
                            println!("{}", val);
                        }
                        OpCode::OpReturn => {
                            println!("Return");
                        }
                    },
                    None => {
                        println!("Unknown opcode: {}", opcode);
                    }
                }
            }
        }
    }

    /// Read a byte data from given chunk and increase pc.
    /// We pass chunk into the function so that `read_byte` doesn't need to pay attention
    /// to unwrap the chunk.
    pub fn read_byte(chunk: &Chunk, pc: &mut usize) -> u8 {
        let byte = chunk.code()[*pc];
        *pc += 1;
        byte
    }
}
