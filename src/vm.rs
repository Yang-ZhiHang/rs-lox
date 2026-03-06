use crate::{
    chunk::{Chunk, OpCode},
    constant::Value,
};

const STACK_SIZE: usize = 256;

pub struct VM {
    /// We don't use chunk as member of vm in rust to avoid series of problems cause
    /// by `Option`.
    // chunk: Option<Chunk>
    /// Program counter, use to represent where has the code been executed.
    /// TODO: Is pointer better than counter in rust?
    pc: usize,
    /// The stack to stored temporary value in expression.
    stack: Vec<Value>,
    /// The top index of stack.
    stack_top: usize,
}

impl VM {
    /// Create a empty virtual machine.
    /// chunk should be pass in when we call `interpret` function.
    pub fn new() -> Self {
        Self {
            pc: 0,
            stack: vec![0f64; STACK_SIZE],
            stack_top: 0,
        }
    }

    /// Interpret the given byte chunk.
    pub fn interpret(&mut self, chunk: &Chunk) {
        self.run(chunk);
    }

    /// Run the opcode from the byte chunk.
    /// `chunk` is passed in as a parameter instead of stored in self,
    /// so that self is free to be mutably borrowed for push/pop inside the loop.
    pub fn run(&mut self, chunk: &Chunk) {
        while self.pc < chunk.code().len() {
            let opcode = Self::read_byte(chunk, &mut self.pc);
            match OpCode::from_repr(opcode) {
                Some(opcode) => match opcode {
                    OpCode::OpConstant => {
                        let value = Self::read_constant(chunk, &mut self.pc);
                        self.push(value);
                    }
                    OpCode::OpReturn => {
                        let value = self.pop();
                        println!("{}", value);
                    }
                    OpCode::OpNegate => {
                        let operand = self.pop();
                        self.push(-operand);
                    }
                },
                None => {
                    println!("Unknown opcode: {}", opcode);
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

    /// Read a constant value from given chunk and increase pc.
    pub fn read_constant(chunk: &Chunk, pc: &mut usize) -> Value {
        let index = Self::read_byte(chunk, pc);
        chunk.constants()[index as usize]
    }

    /// Push a value to the stack of vm.
    /// TODO: Full error handling
    pub fn push(&mut self, value: Value) {
        self.stack[self.stack_top] = value;
        self.stack_top += 1;
    }

    /// Pop a value from the stack of vm.
    /// TODO: Empty error handling
    pub fn pop(&mut self) -> Value {
        self.stack_top -= 1;
        self.stack[self.stack_top]
    }
}
