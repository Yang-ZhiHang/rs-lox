use crate::{
    chunk::{Chunk, OpCode, Value},
    heap::Heap,
    object::ObjId,
    table::HashTable,
};

macro_rules! binary_op {
    ($vm:expr, number, $op:tt) => {{
        // Pop b, then mutate a in-place at the new stack top.
        // This avoids a redundant pop+push pair compared to the naive approach.
        let b = $vm.pop();
        let a = $vm.stack_top_mut();
        match (a.as_number_mut(), b.as_number()) {
            (Ok(a), Ok(b)) => {
                #[allow(clippy::assign_op_pattern)]
                { *a = *a $op b; }
            }
            (Err(e), _) | (_, Err(e)) => {
                eprintln!("{}", e);
                return InterpretResult::RuntimeError;
            }
        }
    }};
    ($vm:expr, bool, $op:tt) => {{
        let b = $vm.pop();
        let a = $vm.pop();
        match (a.as_number(), b.as_number()) {
            (Ok(a), Ok(b)) => $vm.push(Value::Bool(a $op b)),
            (Err(e), _) | (_, Err(e)) => {
                eprintln!("{}", e);
                return InterpretResult::RuntimeError;
            }
        }
    }};
}

const STACK_SIZE: usize = 256;

pub enum InterpretResult {
    Ok,
    CompileError,
    RuntimeError,
}

pub struct VM {
    /// The heap that stores objects (dynamic length).
    pub heap: Heap,
    /// Program counter, use to represent where has the code been executed.
    /// Q: Is pointer better than counter in rust?
    pc: usize,
    /// The stack to stored temporary value in expression.
    /// Q: wheater to make it dynamic vector or just static?
    stack: [Value; STACK_SIZE],
    /// The index of next element.
    stack_top: usize,
    /// The hash table to store identifier.
    strings: HashTable,
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
            heap: Heap::new(),
            pc: 0,
            stack: [Value::Nil; STACK_SIZE],
            stack_top: 0,
            strings: HashTable::new(),
        }
    }

    /// Interpret the given byte chunk.
    pub fn interpret(&mut self, chunk: &Chunk) -> InterpretResult {
        self.run(chunk)
    }

    /// Run the opcode from the byte chunk.
    /// `chunk` is passing in as a parameter instead of storing at self,
    /// so that self is free to be mutably borrowed for push/pop inside the loop.
    pub fn run(&mut self, chunk: &Chunk) -> InterpretResult {
        while self.pc < chunk.code().len() {
            let opcode = Self::read_byte(chunk, &mut self.pc);
            match OpCode::from_repr(opcode) {
                Some(opcode) => match opcode {
                    OpCode::Constant => {
                        let val = Self::read_constant(chunk, &mut self.pc);
                        self.push(val);
                    }
                    OpCode::Print => {
                        let val = self.pop();
                        println!("{}", val.to_string(&self.heap));
                    }
                    OpCode::Return => {
                        return InterpretResult::Ok;
                    }
                    OpCode::Pop => {
                        self.pop();
                    }
                    OpCode::Loop => {
                        let offset = Self::read_short(chunk, &mut self.pc);
                        self.pc -= offset;
                    }
                    OpCode::JumpIfFalse => {
                        let offset = Self::read_short(chunk, &mut self.pc);
                        self.pc += if self.peek(0).is_falsey() { offset } else { 0 };
                    }
                    OpCode::Jump => {
                        let offset = Self::read_short(chunk, &mut self.pc);
                        self.pc += offset;
                    }
                    OpCode::DefineGlobal => {
                        if let Value::Object(obj_id) = Self::read_constant(chunk, &mut self.pc) {
                            let v = self.pop();
                            self.strings.set(obj_id, v);
                        }
                    }
                    OpCode::GetGlobal => {
                        if let Value::Object(obj_id) = Self::read_constant(chunk, &mut self.pc) {
                            match self.strings.get(&obj_id) {
                                Some(e) => {
                                    let v = e.v;
                                    self.push(v);
                                }
                                None => {
                                    self.runtime_error(chunk, "Undefined variable.");
                                    return InterpretResult::RuntimeError;
                                }
                            }
                        }
                    }
                    OpCode::SetGlobal => {
                        if let Value::Object(obj_id) = Self::read_constant(chunk, &mut self.pc) {
                            let v = self.peek(0);
                            match &mut self.strings.get_mut(&obj_id) {
                                Some(e) => {
                                    e.v = v;
                                }
                                None => {
                                    self.runtime_error(chunk, "Undefined variable.");
                                    return InterpretResult::RuntimeError;
                                }
                            }
                        }
                    }
                    OpCode::GetLocal => {
                        let slot = Self::read_byte(chunk, &mut self.pc);
                        self.push(self.stack[slot as usize]);
                    }
                    OpCode::SetLocal => {
                        let slot = Self::read_byte(chunk, &mut self.pc);
                        self.stack[slot as usize] = self.peek(0);
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
                    OpCode::Add => {
                        let b = self.pop();
                        let a = self.pop();
                        if let (Ok(a), Ok(b)) = (a.as_string(&self.heap), b.as_string(&self.heap)) {
                            self.concatenate(&a, &b);
                        } else if let (Ok(a), Ok(b)) = (a.as_number(), b.as_number()) {
                            self.push(Value::Number(a + b));
                        } else {
                            self.runtime_error(
                                chunk,
                                "Operands must be two numbers or two strings.",
                            );
                            return InterpretResult::RuntimeError;
                        }
                    }
                    OpCode::Subtract => binary_op!(self, number, -),
                    OpCode::Multiply => binary_op!(self, number, *),
                    OpCode::Divide => binary_op!(self, number, /),
                    OpCode::Less => binary_op!(self, bool, <),
                    OpCode::Greater => binary_op!(self, bool, >),
                    OpCode::Not => {
                        let val = &mut self.stack[self.stack_top - 1];
                        *val = Value::Bool(val.is_falsey());
                    }
                    OpCode::True => self.push(Value::Bool(true)),
                    OpCode::False => self.push(Value::Bool(false)),
                    OpCode::Nil => self.push(Value::Nil),
                    OpCode::Equal => {
                        let b = self.pop();
                        let a = self.pop();
                        self.push(Value::Bool(a == b));
                    }
                },
                None => {
                    self.runtime_error(chunk, &format!("Unknown opcode: {}", opcode));
                    return InterpretResult::RuntimeError;
                }
            }
        }
        InterpretResult::Ok
    }

    /// Read a byte data from given chunk and increase pc.
    ///
    /// We pass chunk into the function so that `read_byte` doesn't need to pay attention
    /// to unwrap the chunk.
    pub fn read_byte(chunk: &Chunk, pc: &mut usize) -> u8 {
        let byte = chunk.code()[*pc];
        *pc += 1;
        byte
    }

    /// Read two byte data from given chunk and increase pc by two.
    pub fn read_short(chunk: &Chunk, pc: &mut usize) -> usize {
        *pc += 2;
        let h = chunk.code()[*pc - 2] as usize;
        let l = chunk.code()[*pc - 1] as usize;
        h << 8 | l
    }

    /// Read a constant value from given chunk and increase pc.
    pub fn read_constant(chunk: &Chunk, pc: &mut usize) -> Value {
        let index = Self::read_byte(chunk, pc);
        chunk.constants()[index as usize]
    }

    /// Push a value to the stack of vm.
    pub fn push(&mut self, value: Value) {
        if self.stack_top == STACK_SIZE {
            panic!("Stack overflow!");
        }
        self.stack[self.stack_top] = value;
        self.stack_top += 1;
    }

    /// Pop a value from the stack of vm.
    pub fn pop(&mut self) -> Value {
        if self.stack_top == 0 {
            panic!("Empty stack cannot be returned!");
        }
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
        if n > self.stack_top - 1 {
            panic!("Cannot peek index under zero.")
        }
        self.stack[self.stack_top - 1 - n]
    }

    /// Print runtime error to console output.
    pub fn runtime_error(&mut self, chunk: &Chunk, msg: &str) {
        let line = chunk.get_line(self.pc);
        println!("line {}: Runtime error: {}", line, msg);
        self.reset_stack();
    }

    /// Reset the stack of vm.
    pub fn reset_stack(&mut self) {
        self.stack = [Value::Nil; STACK_SIZE];
        self.stack_top = 0;
    }

    /// Concatenate two string slices `a`, `b` and push to the stack.
    pub fn concatenate(&mut self, a: &str, b: &str) {
        let s = &format!("{}{}", a, b);
        let obj_idx = self.heap.write_string(s);
        self.push(Value::Object(ObjId::new(obj_idx)));
    }
}
