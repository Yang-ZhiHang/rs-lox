use lox::chunk::{Chunk, OpCode};

// ==================== write & code ====================

mod write {
    use super::*;

    #[test]
    fn test_write_preserves_order() {
        let mut chunk = Chunk::new();
        chunk.write(OpCode::Constant, 1);
        chunk.write(0_usize, 1);
        chunk.write(OpCode::UnaryNegate, 1);
        chunk.write(OpCode::Return, 1);
        let code = chunk.code();
        assert_eq!(code[0], OpCode::Constant as u8);
        assert_eq!(code[1], 0_u8);
        assert_eq!(code[2], OpCode::UnaryNegate as u8);
        assert_eq!(code[3], OpCode::Return as u8);
    }
}

// ==================== line & RLE ====================

mod line {
    use super::*;

    #[test]
    fn test_line_mixed_opcode_and_operand() {
        let mut chunk = Chunk::new();
        // line 1: Constant opcode + its operand index, both count as separate bytes
        chunk.write(OpCode::Constant, 1);
        chunk.write(0_usize, 1);
        // line 2: Return
        chunk.write(OpCode::Return, 2);
        assert_eq!(chunk.get_line(0), 1);
        assert_eq!(chunk.get_line(1), 1);
        assert_eq!(chunk.get_line(2), 2);
    }

    #[test]
    fn test_line_skipped_lines() {
        let mut chunk = Chunk::new();
        // Jump from line 1 to line 100, no instructions in between
        chunk.write(OpCode::Return, 1);
        chunk.write(OpCode::Return, 100);
        assert_eq!(chunk.get_line(0), 1);
        assert_eq!(chunk.get_line(1), 100);
    }

    #[test]
    #[should_panic(expected = "Unavailable offset.")]
    fn test_line_out_of_bounds_panics() {
        let mut chunk = Chunk::new();
        chunk.write(OpCode::Return, 1);
        chunk.get_line(1);
    }

    #[test]
    fn test_rle_same_line_collapses_to_single_entry() {
        let mut chunk = Chunk::new();
        chunk.write(OpCode::Return, 5);
        chunk.write(OpCode::Return, 5);
        chunk.write(OpCode::Return, 5);
        assert_eq!(chunk.line().len(), 1);
        assert_eq!(chunk.line()[0], (5, 3));
    }

    #[test]
    fn test_rle_skipped_lines_do_not_produce_empty_entries() {
        let mut chunk = Chunk::new();
        chunk.write(OpCode::Return, 1);
        chunk.write(OpCode::Return, 10);
        assert_eq!(chunk.line().len(), 2);
    }

    #[test]
    fn test_rle_count_is_correct() {
        let mut chunk = Chunk::new();
        chunk.write(OpCode::Return, 1);
        chunk.write(OpCode::Return, 1);
        chunk.write(OpCode::Return, 2);
        chunk.write(OpCode::Return, 2);
        chunk.write(OpCode::Return, 2);
        assert_eq!(chunk.line()[0], (1, 2));
        assert_eq!(chunk.line()[1], (2, 3));
    }
}

// ==================== constants ====================

mod constants {
    use super::*;

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
    fn test_constant_instruction_roundtrip() {
        let mut chunk = Chunk::new();
        let index = chunk.write_constant(42.0);
        chunk.write(OpCode::Constant, 1);
        chunk.write(index, 1);
        let stored_index = chunk.code()[1] as usize;
        assert_eq!(chunk.constants()[stored_index], 42.0);
    }

    #[test]
    fn test_multiple_constant_instructions_roundtrip() {
        let mut chunk = Chunk::new();
        let values = [1.0, 2.0, 3.0];
        for (line, &v) in values.iter().enumerate() {
            let index = chunk.write_constant(v);
            chunk.write(OpCode::Constant, line as u32 + 1);
            chunk.write(index, line as u32 + 1);
        }
        // Each instruction is 2 bytes: opcode + index
        for (i, &expected) in values.iter().enumerate() {
            let index = chunk.code()[i * 2 + 1] as usize;
            assert_eq!(chunk.constants()[index], expected);
        }
    }
}

// ==================== opcodes ====================

mod opcodes {
    use super::*;

    #[test]
    fn test_each_opcode_encodes_to_correct_byte() {
        let opcodes = [
            (OpCode::Return, 0_u8),
            (OpCode::Constant, 1_u8),
            (OpCode::UnaryNegate, 2_u8),
            (OpCode::BinaryAdd, 3_u8),
            (OpCode::BinarySubtract, 4_u8),
            (OpCode::BinaryMultiple, 5_u8),
            (OpCode::BinaryDivide, 6_u8),
        ];
        let mut chunk = Chunk::new();
        for &(op, _) in &opcodes {
            chunk.write(op, 1);
        }
        for (i, &(_, expected_byte)) in opcodes.iter().enumerate() {
            assert_eq!(chunk.code()[i], expected_byte, "mismatch at index {i}");
        }
    }

    #[test]
    fn test_opcodes_are_contiguous_from_zero() {
        let last_known = OpCode::BinaryDivide as u8;
        for byte in 0..=last_known {
            assert!(
                OpCode::from_repr(byte).is_some(),
                "expected opcode at discriminant {byte} but from_repr returned None"
            );
        }
        assert!(OpCode::from_repr(last_known + 1).is_none());
    }

    #[test]
    fn test_opcode_from_repr_invalid_byte_returns_none() {
        assert!(OpCode::from_repr(255).is_none());
    }
}
