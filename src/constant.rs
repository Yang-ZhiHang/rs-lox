pub const U8_MAX: usize = u8::MAX as usize;
pub const MAX_LOCAL_SIZE: usize = U8_MAX;
pub const MAX_UPVALUE_SIZE: usize = U8_MAX;
pub const MAX_FRAME_SIZE: usize = 64;
pub const MAX_STACK_SIZE: usize = MAX_FRAME_SIZE * u8::MAX as usize;
