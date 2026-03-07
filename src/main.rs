pub mod chunk;
#[cfg(debug_assertions)]
pub mod common;
pub mod constant;
pub mod tokenizer;
pub mod vm;
pub mod macros;

use chunk::{Chunk, OpCode};

#[cfg(debug_assertions)]
use crate::common::disassemble;
use crate::vm::VM;

fn main() {
    let mut vm = VM::new();
    let mut chunk = Chunk::new();

    let constant_index = chunk.write_constant(1.2);
    chunk.write(OpCode::Constant, 1);
    chunk.write(constant_index, 1);

    let constant_index = chunk.write_constant(3.4);
    chunk.write(OpCode::Constant, 1);
    chunk.write(constant_index, 1);

    chunk.write(OpCode::BinaryAdd, 1);

    let constant_index = chunk.write_constant(5.6);
    chunk.write(OpCode::Constant, 1);
    chunk.write(constant_index, 1);

    chunk.write(OpCode::BinaryDivide, 1);
    chunk.write(OpCode::UnaryNegate, 1);
    chunk.write(OpCode::Return, 1);

    #[cfg(debug_assertions)]
    disassemble(&chunk, "test chunk");
    vm.interpret(&chunk);
}
