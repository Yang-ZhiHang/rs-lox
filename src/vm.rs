/// vm.rs: We call this LVM (Lox Virtual Machine).
use crate::{
    binary_op,
    chunk::{Chunk, OpCode, Value},
};

const STACK_SIZE: usize = 256;

pub enum InterpretResult {
    Ok,
    CompileError,
    RuntimeError,
}

pub struct VM {
    /// We don't use chunk as member of vm in rust to avoid series of problems cause
    /// by `Option`.
    // chunk: Option<Chunk>
    /// Program counter, use to represent where has the code been executed.
    /// TODO: Is pointer better than counter in rust?
    pc: usize,
    /// The stack to stored temporary value in expression.
    /// TODO: wheater to make it dynamic vector or just static?
    stack: Vec<Value>,
    /// The index of next element.
    stack_top: usize,
}

impl Default for VM {
    fn default() -> Self {
        Self::new()
    }
}

impl VM {
    /// Create a empty virtual machine.
    /// chunk should be pass in when we call `interpret` function.
    pub fn new() -> Self {
        Self {
            pc: 0,
            stack: vec![Value::Nil; STACK_SIZE],
            stack_top: 0,
        }
    }

    /// Interpret the given byte chunk.
    pub fn interpret(&mut self, chunk: &Chunk) -> InterpretResult {
        self.run(chunk)
    }

    /// Run the opcode from the byte chunk.
    /// `chunk` is passed in as a parameter instead of stored in self,
    /// so that self is free to be mutably borrowed for push/pop inside the loop.
    pub fn run(&mut self, chunk: &Chunk) -> InterpretResult {
        while self.pc < chunk.code().len() {
            let opcode = Self::read_byte(chunk, &mut self.pc);
            match OpCode::from_repr(opcode) {
                Some(opcode) => match opcode {
                    OpCode::Constant => {
                        let value = Self::read_constant(chunk, &mut self.pc);
                        self.push(value);
                    }
                    OpCode::Return => {
                        let val = self.pop();
                        val.print();
                    }
                    OpCode::Negate => {
                        let val = &mut self.stack[self.stack_top - 1];
                        match val {
                            Value::Number(v) => *v = -*v,
                            _ => {
                                self.runtime_error(chunk, "Operand must be a number.");
                                return InterpretResult::RuntimeError;
                            }
                        }
                    }
                    OpCode::Add => binary_op!(self, number, +),
                    OpCode::Subtract => binary_op!(self, number, -),
                    OpCode::Multiply => binary_op!(self, number, *),
                    OpCode::Divide => binary_op!(self, number, /),
                    OpCode::Not => {
                        let val = &mut self.stack[self.stack_top - 1];
                        *val = Value::Bool(val.is_falsey());
                    }
                    OpCode::True => self.push(Value::Bool(true)),
                    OpCode::False => self.push(Value::Bool(false)),
                    OpCode::Nil => self.push(Value::Nil),
                    OpCode::Less => binary_op!(self, bool, <),
                    OpCode::Greater => binary_op!(self, bool, >),
                    OpCode::Equal => {
                        let b = self.pop();
                        let a = self.pop();
                        self.push(Value::Bool(a == b));
                    }
                },
                None => {
                    self.runtime_error(chunk, &format!("Unknown opcode: {}", opcode));
                    return InterpretResult::CompileError;
                }
            }
        }
        InterpretResult::Ok
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
    pub fn push(&mut self, value: Value) {
        self.stack[self.stack_top] = value;
        self.stack_top += 1;
    }

    /// Pop a value from the stack of vm.
    pub fn pop(&mut self) -> Value {
        self.stack_top -= 1;
        self.stack[self.stack_top]
    }

    /// Return a mutable reference to the current top value of the stack.
    /// Used by binary_op! to mutate the top value in-place without an ex
    /// tra modify on `stack_top`.
    pub fn stack_top_mut(&mut self) -> &mut Value {
        &mut self.stack[self.stack_top - 1]
    }

    /// Return the value away `n` from top element of the stack.
    pub fn peek(&self, n: usize) -> Value {
        self.stack[self.stack_top - 1 - n]
    }

    pub fn runtime_error(&mut self, chunk: &Chunk, msg: &str) {
        let line = chunk.get_line(self.pc);
        println!("{}:{}: {}", line, self.pc, msg);
        self.reset_stack();
    }

    /// Reset the stack of vm.
    pub fn reset_stack(&mut self) {
        self.stack = vec![Value::Nil; STACK_SIZE];
        self.stack_top = 0;
    }
}
