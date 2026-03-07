#[macro_export]
/// Apply a binary operation on the top two values of the VM stack.
/// Usage: binary_op!(self, +)
macro_rules! binary_op {
    ($vm:expr, $op:tt) => {{
        // Pop b, then mutate a in-place at the new stack top.
        // This avoids a redundant pop+push pair compared to the naive approach.
        let b = $vm.pop();
        let a = $vm.stack_top_mut();
        // allow op pattern to avoid clippy warning
        #[allow(clippy::assign_op_pattern)]
        { *a = *a $op b; }
    }};
}
