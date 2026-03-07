use lox::chunk::{Chunk, OpCode};

// ==================== write & code ====================

#[test]
fn test_write_single_opcode() {
    let mut chunk = Chunk::new();
    chunk.write(OpCode::Return, 1);
    assert_eq!(chunk.code().len(), 1);
}

#[test]
fn test_write_multiple_opcodes() {
    let mut chunk = Chunk::new();
    chunk.write(OpCode::Return, 1);
    chunk.write(OpCode::Return, 1);
    chunk.write(OpCode::Return, 2);
    assert_eq!(chunk.code().len(), 3);
}

#[test]
fn test_write_opcode_with_operand() {
    let mut chunk = Chunk::new();
    let index = chunk.write_constant(1.2);
    chunk.write(OpCode::Constant, 1);
    chunk.write(index, 1);
    // OpConstant + operand index = 2 bytes
    assert_eq!(chunk.code().len(), 2);
}

// ==================== line (RLE) ====================

#[test]
fn test_line_single_instruction() {
    let mut chunk = Chunk::new();
    chunk.write(OpCode::Return, 1);
    assert_eq!(chunk.get_line(0), 1);
}

#[test]
fn test_line_multiple_instructions_same_line() {
    let mut chunk = Chunk::new();
    chunk.write(OpCode::Return, 3);
    chunk.write(OpCode::Return, 3);
    chunk.write(OpCode::Return, 3);
    assert_eq!(chunk.get_line(0), 3);
    assert_eq!(chunk.get_line(1), 3);
    assert_eq!(chunk.get_line(2), 3);
}

#[test]
fn test_line_multiple_instructions_different_lines() {
    let mut chunk = Chunk::new();
    chunk.write(OpCode::Return, 1);
    chunk.write(OpCode::Return, 2);
    chunk.write(OpCode::Return, 3);
    assert_eq!(chunk.get_line(0), 1);
    assert_eq!(chunk.get_line(1), 2);
    assert_eq!(chunk.get_line(2), 3);
}

#[test]
fn test_line_mixed() {
    let mut chunk = Chunk::new();
    // line 1: 2 instructions
    chunk.write(OpCode::Constant, 1);
    chunk.write(0_usize, 1);
    // line 2: 1 instruction
    chunk.write(OpCode::Return, 2);
    assert_eq!(chunk.get_line(0), 1);
    assert_eq!(chunk.get_line(1), 1);
    assert_eq!(chunk.get_line(2), 2);
}

#[test]
#[should_panic(expected = "Unavailable offset.")]
fn test_line_out_of_bounds_panics() {
    let mut chunk = Chunk::new();
    chunk.write(OpCode::Return, 1);
    // offset 1 does not exist, should panic
    chunk.get_line(1);
}

// ==================== RLE structure ====================

#[test]
fn test_rle_same_line_produces_single_entry() {
    let mut chunk = Chunk::new();
    chunk.write(OpCode::Return, 5);
    chunk.write(OpCode::Return, 5);
    chunk.write(OpCode::Return, 5);
    // All three on line 5, RLE should have only 1 entry
    assert_eq!(chunk.line().len(), 1);
}

#[test]
fn test_rle_different_lines_produce_multiple_entries() {
    let mut chunk = Chunk::new();
    chunk.write(OpCode::Return, 1);
    chunk.write(OpCode::Return, 2);
    chunk.write(OpCode::Return, 3);
    // Each on a different line, RLE should have 3 entries
    assert_eq!(chunk.line().len(), 3);
}

#[test]
fn test_rle_skipped_lines_do_not_produce_empty_entries() {
    let mut chunk = Chunk::new();
    // Jump from line 1 to line 10, no instructions on lines 2-9
    chunk.write(OpCode::Return, 1);
    chunk.write(OpCode::Return, 10);
    // Should only have 2 RLE entries, no empty slots for lines 2-9
    assert_eq!(chunk.line().len(), 2);
}

// ==================== write_constant ====================

#[test]
fn test_write_constant_returns_correct_index() {
    let mut chunk = Chunk::new();
    let i0 = chunk.write_constant(1.0);
    let i1 = chunk.write_constant(2.0);
    let i2 = chunk.write_constant(3.0);
    assert_eq!(i0, 0);
    assert_eq!(i1, 1);
    assert_eq!(i2, 2);
}

#[test]
fn test_write_constant_value_is_retrievable() {
    let mut chunk = Chunk::new();
    let index = chunk.write_constant(3.14);
    assert_eq!(chunk.constants()[index], 3.14);
}

#[test]
fn test_write_multiple_constants() {
    let mut chunk = Chunk::new();
    let values = [1.1, 2.2, 3.3, 4.4];
    let indices: Vec<usize> = values.iter().map(|&v| chunk.write_constant(v)).collect();
    for (i, &idx) in indices.iter().enumerate() {
        assert_eq!(chunk.constants()[idx], values[i]);
    }
}
