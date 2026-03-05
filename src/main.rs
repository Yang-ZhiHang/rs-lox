use std::env;

pub mod chunk;
pub mod tokenizer;

use chunk::{Chunk, OpCode};

fn main() {
    let mut chunk = Chunk::new();
    chunk.write(OpCode::OpReturn);
    chunk.disassemble("test");
}
