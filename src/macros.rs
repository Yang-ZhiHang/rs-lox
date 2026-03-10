#[macro_export]
/// Apply a binary operation on the top two values of the VM stack.
/// Usage: binary_op!(self, +)
macro_rules! binary_op {
    ($vm:expr, number, $op:tt) => {{
        // Pop b, then mutate a in-place at the new stack top.
        // This avoids a redundant pop+push pair compared to the naive approach.
        let b = $vm.pop();
        let a = $vm.stack_top_mut();
        match (a.as_number_mut(), b.as_number()) {
            (Ok(a), Ok(b)) => {
                #[allow(clippy::assign_op_pattern)]
                { *a = *a $op b; }
            }
            (Err(e), _) | (_, Err(e)) => {
                eprintln!("{}", e);
                return InterpretResult::RuntimeError;
            }
        }
    }};
    ($vm:expr, bool, $op:tt) => {{
        let b = $vm.pop();
        let a = $vm.pop();
        match (a.as_number(), b.as_number()) {
            (Ok(a), Ok(b)) => $vm.push(Value::Bool(a $op b)),
            (Err(e), _) | (_, Err(e)) => {
                eprintln!("{}", e);
                return InterpretResult::RuntimeError;
            }
        }
    }};
}
