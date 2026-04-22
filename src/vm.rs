use std::{cell::RefCell, rc::Rc};

use crate::{
    chunk::{Chunk, OpCode, Value},
    constant::{MAX_FRAME_SIZE, MAX_STACK_SIZE, MAX_UPVALUE_SIZE},
    heap::Heap,
    object::{ObjClosure, ObjData, ObjIndex, UpvalueState},
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

pub enum InterpretResult {
    Ok,
    CompileError,
    RuntimeError,
}

#[derive(Clone, Copy)]
pub struct CallFrame {
    pc: usize,
    /// Object index of closure.
    closure_obj_idx: ObjIndex,
    /// The start index of call frame.
    slot_offset: usize,
}

impl CallFrame {
    pub fn new(closure: ObjIndex, slot_offset: usize) -> Self {
        Self {
            pc: 0,
            closure_obj_idx: closure,
            slot_offset,
        }
    }

    /// Return the value from vm's stack according to relative offset.
    pub fn get(&self, stack: &[Option<Value>], offset: usize) -> Value {
        stack[self.slot_offset + offset].unwrap()
    }

    /// Set the value in vm's stack according to relative offset.
    pub fn set(&self, stack: &mut [Option<Value>], offset: usize, v: Value) {
        stack[self.slot_offset + offset] = Some(v);
    }
}

pub struct VM {
    /// The heap that stores objects (dynamic length).
    pub heap: Heap,
    /// Function call frames.
    frames: [Option<CallFrame>; MAX_FRAME_SIZE],
    /// The depth of function call.
    frame_count: usize,
    /// The stack to stored temporary value in expression.
    /// Q: wheater to make it dynamic vector or just static?
    stack: [Option<Value>; MAX_STACK_SIZE],
    /// The index of next element.
    stack_top: usize,
    /// The hash table to store identifier.
    global_variables: HashTable,
    /// The array storing the index of upvalues in vm's heap.
    upvalues: [Option<Rc<RefCell<UpvalueState>>>; MAX_UPVALUE_SIZE],
    /// The amount of open upvalues.
    upvalues_count: usize,
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
            frames: [None; MAX_FRAME_SIZE],
            frame_count: 0,
            stack: [None; MAX_STACK_SIZE],
            stack_top: 0,
            global_variables: HashTable::new(),
            upvalues: [const { None }; MAX_UPVALUE_SIZE],
            upvalues_count: 0,
        }
    }

    /// Interpret the given byte chunk.
    pub fn interpret(&mut self, func_obj_idx: ObjIndex) -> InterpretResult {
        let closure = ObjClosure::new(func_obj_idx, 0);
        let closure_obj_idx = self.heap.write_closure(closure);
        self.push(Value::Object(closure_obj_idx));
        self.call(closure_obj_idx, 0);
        self.run()
    }

    /// Run the opcode from the byte chunk.
    /// `chunk` is passing in as a parameter instead of storing at self,
    /// so that self is free to be mutably borrowed for push/pop inside the loop.
    pub fn run(&mut self) -> InterpretResult {
        loop {
            let frame = self.frames[self.frame_count - 1].as_mut().unwrap();
            let closure = self.heap.get_closure(frame.closure_obj_idx);
            let chunk = {
                let func = self.heap.get_func(closure.func);
                func.chunk.clone()
            };
            // Perf: Make pc a local variable then assign it to `frame.pc` after the match of operation code. Which
            // will make cache locality performs better.
            let pc: &mut usize = &mut frame.pc;
            if *pc > chunk.code().len() {
                break;
            }
            let frame_slot_offset = frame.slot_offset;
            let opcode = Self::read_byte(&chunk, pc);
            match OpCode::from_repr(opcode) {
                Some(opcode) => match opcode {
                    OpCode::Constant => {
                        let val = Self::read_constant(&chunk, pc);
                        self.push(val);
                    }
                    OpCode::Return => {
                        let ret = self.pop();
                        // Close the upvalues of current function.
                        self.close_upvalue(frame_slot_offset);
                        self.frame_count -= 1;
                        if self.frame_count == 0 {
                            self.pop();
                            return InterpretResult::Ok;
                        }
                        // Destory the call frame of callee by fallback stack pointer.
                        self.stack_top = frame_slot_offset;
                        self.push(ret);
                    }
                    OpCode::Call => {
                        let arg_count = Self::read_byte(&chunk, pc) as usize;
                        if !self.call_value(arg_count) {
                            return InterpretResult::RuntimeError;
                        }
                    }
                    OpCode::Pop => {
                        self.pop();
                    }
                    OpCode::Loop => {
                        let offset = Self::read_short(&chunk, pc);
                        frame.pc -= offset;
                    }
                    OpCode::JumpIfFalse => {
                        let val = Self::peek(&self.stack, self.stack_top, 0);
                        let offset = Self::read_short(&chunk, pc);
                        frame.pc += if val.is_falsey() { offset } else { 0 };
                    }
                    OpCode::Jump => {
                        let offset = Self::read_short(&chunk, pc);
                        frame.pc += offset;
                    }
                    OpCode::DefineGlobal => {
                        if let Value::Object(obj_idx) = Self::read_constant(&chunk, pc) {
                            let v = self.pop();
                            self.global_variables.set(obj_idx, v);
                        }
                    }
                    OpCode::GetGlobal => {
                        if let Value::Object(obj_idx) = Self::read_constant(&chunk, pc) {
                            match self.global_variables.get(&obj_idx) {
                                Some(e) => {
                                    let v = e.v;
                                    self.push(v);
                                }
                                None => {
                                    let name = self.heap.get_string(obj_idx);
                                    self.runtime_error(&format!("Undefined variable '{}'.", name));
                                    return InterpretResult::RuntimeError;
                                }
                            }
                        }
                    }
                    OpCode::SetGlobal => {
                        if let Value::Object(obj_idx) = Self::read_constant(&chunk, pc) {
                            let v = Self::peek(&self.stack, self.stack_top, 0);
                            match &mut self.global_variables.get_mut(&obj_idx) {
                                Some(e) => {
                                    e.v = v;
                                }
                                None => {
                                    let name = self.heap.get_string(obj_idx);
                                    self.runtime_error(&format!("Undefined variable '{}'.", name));
                                    return InterpretResult::RuntimeError;
                                }
                            }
                        }
                    }
                    OpCode::GetLocal => {
                        let slot = Self::read_byte(&chunk, pc);
                        let val = frame.get(&self.stack, slot as usize);
                        self.push(val);
                    }
                    OpCode::SetLocal => {
                        let slot = Self::read_byte(&chunk, pc);
                        let val = Self::peek(&self.stack, self.stack_top, 0);
                        frame.set(&mut self.stack, slot as usize, val);
                    }
                    OpCode::Closure => {
                        if let Value::Object(func_obj_idx) = Self::read_constant(&chunk, pc) {
                            let func = self.heap.get_func(func_obj_idx);
                            let mut new_closure =
                                ObjClosure::new(func_obj_idx, func.upvalues_count);
                            for i in 0usize..new_closure.upvalue_count {
                                let is_local = Self::read_byte(&chunk, pc);
                                let idx = Self::read_byte(&chunk, pc) as usize;
                                // Get the upvalue index in stack (Unclosed upvalue).
                                new_closure.upvalues[i] = if is_local != 0 {
                                    let obj_upval = Self::capture_upvalue(
                                        frame.slot_offset + idx,
                                        &mut self.upvalues,
                                        &mut self.upvalues_count,
                                    );
                                    Some(obj_upval)
                                } else {
                                    let current_closure =
                                        self.heap.get_closure(frame.closure_obj_idx);
                                    current_closure.upvalues[idx].clone()
                                }
                            }
                            let closure_idx = self.heap.write_closure(new_closure);
                            self.push(Value::Object(closure_idx));
                        } else {
                            self.runtime_error("Invalid use of `Closure` operation code.");
                        }
                    }
                    OpCode::GetUpvalue => {
                        // Read the index in upvalues stack.
                        let slot = Self::read_byte(&chunk, pc);
                        let closure = self.heap.get_closure(frame.closure_obj_idx);
                        let obj_upval_ref =
                            closure.upvalues[slot as usize].as_ref().unwrap().clone();
                        let obj_upval = obj_upval_ref.borrow();
                        if obj_upval.is_open() {
                            let idx = obj_upval.as_idx();
                            let val = self.stack[idx].unwrap();
                            self.push(val);
                        } else if obj_upval.is_closed() {
                            let val = obj_upval.as_val(&self.heap);
                            self.push(val);
                        } else {
                            // TODO: Error handling
                        }
                    }
                    OpCode::SetUpvalue => {
                        let slot = Self::read_byte(&chunk, pc);
                        let mut upvalues = self
                            .heap
                            .get_closure(frame.closure_obj_idx)
                            .upvalues
                            .clone();
                        let obj_upval_ref = upvalues[slot as usize].as_mut().unwrap().clone();
                        let mut obj_upval = obj_upval_ref.borrow_mut();
                        let val = Self::peek(&self.stack, self.stack_top, 0);
                        if obj_upval.is_open() {
                            self.stack[obj_upval.as_idx()] = Some(val);
                        } else if obj_upval.is_closed() {
                            obj_upval.set(&mut self.heap, val);
                        } else {
                            // TODO: Error handling.
                        }
                    }
                    OpCode::CloseUpvalue => {
                        self.close_upvalue(self.stack_top - 1);
                    }
                    OpCode::Negate => {
                        let val = self.stack[self.stack_top - 1].as_mut().unwrap();
                        match val {
                            Value::Number(v) => *v = -*v,
                            _ => {
                                self.runtime_error("Operand must be a number.");
                                return InterpretResult::RuntimeError;
                            }
                        }
                    }
                    OpCode::Add => {
                        let b = self.pop();
                        let a = self.pop();
                        if a.is_string(&self.heap) || b.is_string(&self.heap) {
                            let a_str = a.to_string(&self.heap);
                            let b_str = b.to_string(&self.heap);
                            self.concatenate(&a_str, &b_str);
                        } else if let (Ok(a), Ok(b)) = (a.as_number(), b.as_number()) {
                            self.push(Value::Number(a + b));
                        } else {
                            self.runtime_error(
                                "Operands must be two numbers or at least one string.",
                            );
                        }
                    }
                    OpCode::Sub => binary_op!(self, number, -),
                    OpCode::Mul => binary_op!(self, number, *),
                    OpCode::Div => binary_op!(self, number, /),
                    OpCode::Mod => binary_op!(self, number, %),
                    OpCode::Less => binary_op!(self, bool, <),
                    OpCode::Greater => binary_op!(self, bool, >),
                    OpCode::Not => {
                        let val = self.stack[self.stack_top - 1].as_mut().unwrap();
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
                    self.runtime_error(&format!("Unknown opcode: {}", opcode));
                    return InterpretResult::RuntimeError;
                }
            }
        }
        InterpretResult::Ok
    }

    #[inline(always)]
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

    #[inline(always)]
    /// Push a value to the stack of vm.
    pub fn push(&mut self, value: Value) {
        if self.stack_top == MAX_STACK_SIZE {
            // TODO: don't panic.
            panic!("Stack overflow!");
        }
        self.stack[self.stack_top] = Some(value);
        self.stack_top += 1;
    }

    #[inline(always)]
    /// Pop a value from the stack of vm.
    pub fn pop(&mut self) -> Value {
        if self.stack_top == 0 {
            // TODO: don't panic.
            panic!("Empty stack cannot be returned!");
        }
        self.stack_top -= 1;
        self.stack[self.stack_top].unwrap()
    }

    /// Return a mutable reference to the current top value of the stack.
    /// Used by binary_op! to mutate the top value in-place without an ex
    /// tra modify on `stack_top`.
    pub fn stack_top_mut(&mut self) -> &mut Value {
        self.stack[self.stack_top - 1].as_mut().unwrap()
    }

    /// Return the value away `n` from top element of the stack.
    ///
    /// Here, we only passing-in `stack` instead of `self` to borrow only the member.
    pub fn peek(stack: &[Option<Value>], stack_top: usize, n: usize) -> Value {
        if n > stack_top - 1 {
            panic!("Cannot peek index under zero.")
        }
        stack[stack_top - 1 - n].unwrap()
    }

    /// Print runtime error to console output.
    pub fn runtime_error(&mut self, msg: &str) {
        println!("Runtime error: {}", msg);
        let mut frame_idx = self.frame_count - 1;
        while frame_idx != 0 {
            let frame = self.frames[frame_idx].as_ref().unwrap();
            let closure = &self.heap.get_closure(frame.closure_obj_idx);
            let func = &self.heap.get_func(closure.func);
            let line = func.chunk.get_line(frame.pc);
            let func_name = self.heap.get_string(func.name);
            println!("line {} in {}", line, func_name);
            frame_idx -= 1;
        }
        self.reset_stack();
    }

    /// Define a native function with given name and function pointer, and push the native function object to
    /// global variables.
    pub fn define_native(&mut self, name: &str, func: fn(usize, &[Option<Value>], &Heap) -> Value) {
        let name_idx = self.heap.write_string(name);
        let native_obj_idx = self.heap.write_native_func(func);
        self.global_variables
            .set(name_idx, Value::Object(native_obj_idx));
    }

    /// Reset the stack of vm.
    pub fn reset_stack(&mut self) {
        self.stack = [None; MAX_STACK_SIZE];
        self.stack_top = 0;
        self.frame_count = 0;
    }

    /// Concatenate two string slices `a`, `b` and push to the stack.
    pub fn concatenate(&mut self, a: &str, b: &str) {
        let s = &format!("{}{}", a, b);
        let obj_idx = self.heap.write_string(s);
        self.push(Value::Object(obj_idx));
    }

    /// Call the `Value` if it's `Value::Object` and the object id refers to `ObjData::Function`.
    pub fn call_value(&mut self, arg_count: usize) -> bool {
        if let Value::Object(obj_idx) = Self::peek(&self.stack, self.stack_top, arg_count) {
            match self.heap.get(obj_idx) {
                ObjData::Closure(_) => self.call(obj_idx, arg_count),
                ObjData::Native(obj_native) => {
                    let native_fn = obj_native.func;
                    let res = native_fn(
                        arg_count,
                        &self.stack[self.stack_top - arg_count..self.stack_top],
                        &self.heap,
                    );
                    self.stack_top -= arg_count + 1;
                    self.push(res);
                    true
                }
                _ => {
                    self.runtime_error("Can only call function or class.");
                    false
                }
            }
        } else {
            self.runtime_error("Can only call function or class.");
            false
        }
    }

    /// Call the function object with arguments.
    pub fn call(&mut self, closure_obj_idx: ObjIndex, arg_count: usize) -> bool {
        if self.frame_count == MAX_FRAME_SIZE {
            self.runtime_error("Stack overflow");
            return false;
        }
        let closure = self.heap.get_closure(closure_obj_idx);
        let func = self.heap.get_func(closure.func);
        if func.arity != arg_count {
            self.runtime_error(&format!(
                "Expected {} arguments but got {}",
                func.arity, arg_count
            ));
            return false;
        }
        // Create a new call frame for the function call.
        let frame = CallFrame::new(closure_obj_idx, self.stack_top - arg_count - 1);
        self.frames[self.frame_count] = Some(frame);
        self.frame_count += 1;
        true
    }

    /// Allocate a memory in heap for upvalue.
    pub fn capture_upvalue(
        idx: usize,
        upvalues: &mut [Option<Rc<RefCell<UpvalueState>>>],
        upvalues_count: &mut usize,
    ) -> Rc<RefCell<UpvalueState>> {
        // 1. Firstly search for existed upvalue object.
        for upval in upvalues.iter().flatten() {
            if let UpvalueState::Open(upval_idx) = *upval.borrow()
                && upval_idx == idx
            {
                return upval.clone();
            }
        }
        // 2. Create a new open upvalue object if it's not exist.
        let obj_upval = Rc::new(RefCell::new(UpvalueState::open(idx)));
        upvalues[*upvalues_count] = Some(obj_upval.clone());
        *upvalues_count += 1;
        obj_upval
    }

    /// Close the upvalue at the top of the stack by copying the value to heap and change the upvalue object to
    /// closed.
    pub fn close_upvalue(&mut self, offset: usize) {
        for obj_upval_ref in self.upvalues.iter_mut().flatten() {
            let mut obj_upval = obj_upval_ref.borrow_mut();
            if !obj_upval.is_closed() && obj_upval.as_idx() >= offset {
                let val = self.stack[obj_upval.as_idx()].unwrap();
                let upval_obj_idx = self.heap.write_upvalue(val);
                *obj_upval = UpvalueState::closed(upval_obj_idx);
            }
        }
    }
}
