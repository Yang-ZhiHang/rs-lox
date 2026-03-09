#[macro_export]
/// Apply a binary operation on the top two values of the VM stack.
/// Usage: binary_op!(self, +)
macro_rules! binary_op {
    ($vm:expr, $op:tt) => {{
        // Pop b, then mutate a in-place at the new stack top.
        // This avoids a redundant pop+push pair compared to the naive approach.
        let b = $vm.pop();
        let a = $vm.stack_top_mut();
        match (a.as_number_mut(), b.as_number()) {
            (Some(a), b) => {
                #[allow(clippy::assign_op_pattern)]
                { *a = *a $op b; }
            }
            _ => {
                eprintln!("Operands must be numbers.");
                return InterpretResult::RuntimeError;
            }
        }
    }};
}
