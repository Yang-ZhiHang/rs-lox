pub mod chunk;
pub mod constant;
pub mod tokenizer;

use chunk::{Chunk, OpCode};

fn main() {
    let mut chunk = Chunk::new();
    chunk.write(OpCode::OpReturn, 1);
    chunk.write(OpCode::OpConstant, 1);
    let constant_index = chunk.write_constant(1.0);
    chunk.write(constant_index, 1);
    chunk.write(OpCode::OpConstant, 2);
    let constant_index = chunk.write_constant(1.2);
    chunk.write(constant_index, 2);
    chunk.disassemble("test chunk");
}
