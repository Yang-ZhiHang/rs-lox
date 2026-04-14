/// common.rs: In this file, we made some disassemble tools to debug.
/// We'd better make the file not to compile in release mode.
/// Use #[cfg(debug_assertions)] when import this module.
use crate::{
    chunk::{Chunk, OpCode, Value},
    heap::Heap,
    object::ObjData,
};

/// Just print the operation code name to the console.
pub fn simple_instruction(_chunk: &Chunk, offset: usize, opcode: OpCode) -> usize {
    println!("{}", opcode);
    offset + 1
}

/// Print the constant operation code with value to the console.
pub fn constant_instruction(
    chunk: &Chunk,
    heap: &Heap,
    mut offset: usize,
    opcode: OpCode,
) -> usize {
    let val = chunk.constants()[chunk.code()[offset + 1] as usize];
    match val {
        Value::Object(obj_idx) => match heap.get(obj_idx) {
            ObjData::String(obj_string) => {
                println!("{:<8}\t\"{}\"", opcode, obj_string);
            }
            ObjData::Function(obj_func) => {
                println!("{:<8}\t<fn {}>", opcode, heap.get_string(obj_func.name));
                let upvalues_count = obj_func.upvalues_count;
                for _ in 0..upvalues_count {
                    let is_local = if chunk.code()[offset + 2] == 1 {
                        "local"
                    } else {
                        "upvalue"
                    };
                    let idx = chunk.code()[offset + 2];
                    println!("\t\t{:<8}\t<index {}>", is_local, idx);
                    offset += 2;
                }
            }
            ObjData::Closure(_) | ObjData::Upvalue(_) | ObjData::Native(_) => {
                unreachable!()
            }
        },
        _ => {
            println!("{}\t{}", opcode, val);
        }
    }
    offset + 2
}

/// Print operation code with index to the console.
pub fn index_instruction(chunk: &Chunk, offset: usize, opcode: OpCode) -> usize {
    let idx = chunk.code()[offset + 1];
    match opcode {
        OpCode::Call => {
            println!("{:<8}\targc({})", opcode, idx);
        }
        _ => {
            println!("{:<8}\t<index {}>", opcode, idx);
        }
    }
    offset + 2
}

/// Print operation code with jump offset to the console.
pub fn jump_instruction(chunk: &Chunk, offset: usize, opcode: OpCode) -> usize {
    let h = chunk.code()[offset + 1] as usize;
    let l = chunk.code()[offset + 2] as usize;
    let jump_offset = h << 8 | l;
    // {:<8} to avoid the alignment problem of output information caused by overly short opcode characters like `Jump`.
    println!("{:<8}\toffset({})", opcode, jump_offset);
    offset + 3
}

/// Disassemble chunk.
pub fn disassemble(chunk: &Chunk, heap: &Heap, name: &str) {
    // Print the name title so that we know which chunk we are looking.
    println!("Disassemble '{}':", name);
    println!("Offset\tLine\tOpcode");
    let mut offset = 0;
    // Execute each instruction (the size of instruction may be different).
    while offset < chunk.code().len() {
        offset = disassemble_instruction(chunk, heap, offset);
    }
    println!()
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
            OpCode::Constant
            | OpCode::DefineGlobal
            | OpCode::GetGlobal
            | OpCode::SetGlobal
            | OpCode::Closure => constant_instruction(chunk, heap, offset, opcode),
            OpCode::JumpIfFalse | OpCode::Jump | OpCode::Loop => {
                jump_instruction(chunk, offset, opcode)
            }
            OpCode::GetLocal
            | OpCode::SetLocal
            | OpCode::Call
            | OpCode::GetUpvalue
            | OpCode::SetUpvalue => index_instruction(chunk, offset, opcode),
            _ => simple_instruction(chunk, offset, opcode),
        },
        None => {
            println!("Unknown opcode: {}", byte);
            offset + 1
        }
    }
}
