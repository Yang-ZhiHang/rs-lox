/// common.rs: In this file, we made some disassemble tools to debug.
/// We'd better make the file not to compile in release mode.
/// Use #[cfg(debug_assertions)] when import this module.
use crate::{
    chunk::{Chunk, OpCode, Value},
    heap::Heap,
    object::ObjId,
};

/// Just print the opcode name to the console.
pub fn simple_instruction(_chunk: &Chunk, offset: usize, opcode: OpCode) -> usize {
    println!("{}", opcode);
    offset + 1
}

/// Print the constant opcode value to the console.
pub fn constant_instruction(chunk: &Chunk, heap: &Heap, offset: usize, opcode: OpCode) -> usize {
    let val = chunk.constants()[chunk.code()[offset + 1] as usize];
    match val {
        Value::Object(ObjId(idx)) => {
            println!("{}\t\"{}\"", opcode, heap.get(idx));
        }
        _ => {
            println!("{}\t{}", opcode, val);
        }
    }
    offset + 2
}

/// Disassemble chunk.
pub fn disassemble(chunk: &Chunk, heap: &Heap, name: &str) {
    // Print the name title so that we know which chunk we are looking.
    println!("Disassemble '{}' result:", name);
    println!("Offset\tLine\tOpcode");
    let mut offset = 0;
    // Execute each instruction (the size of instruction may be different).
    while offset < chunk.code().len() {
        offset = disassemble_instruction(chunk, heap, offset);
    }
}

/// Disassemble and execute instruction with an offset in the chunk.
pub fn disassemble_instruction(chunk: &Chunk, heap: &Heap, offset: usize) -> usize {
    // Print the offset, line number and opcode.
    // fmt: 000000 0001 OpReturn
    if offset > 0 && chunk.get_line(offset) == chunk.get_line(offset - 1) {
        // Here we use `:>4` instead of `:04` because string "-" is not number.
        print!("{:06}\t{:>4}\t", offset, "-");
    } else {
        print!("{:06}\t{:04}\t", offset, chunk.get_line(offset));
    }
    let byte = chunk.code()[offset];
    match OpCode::from_repr(byte) {
        Some(opcode) => match opcode {
            OpCode::Constant | OpCode::DefineGlobal | OpCode::GetGlobal => {
                constant_instruction(chunk, heap, offset, opcode)
            }
            _ => simple_instruction(chunk, offset, opcode),
        },
        None => {
            println!("Unknown opcode: {}", byte);
            offset + 1
        }
    }
}
