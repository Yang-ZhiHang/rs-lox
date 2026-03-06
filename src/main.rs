pub mod chunk;
#[cfg(debug_assertions)]
pub mod common;
pub mod constant;
pub mod tokenizer;
pub mod vm;

use chunk::{Chunk, OpCode};

#[cfg(debug_assertions)]
use crate::common::disassemble;
use crate::vm::VM;

fn main() {
    let mut vm = VM::new();
    let mut chunk = Chunk::new();
    chunk.write(OpCode::OpConstant, 1);
    let constant_index = chunk.write_constant(1.0);
    chunk.write(constant_index, 1);
    chunk.write(OpCode::OpConstant, 2);
    let constant_index = chunk.write_constant(1.2);
    chunk.write(constant_index, 2);
    chunk.write(OpCode::OpReturn, 3);
    #[cfg(debug_assertions)]
    disassemble(&chunk, "test chunk");
    vm.interpret(chunk);
}
