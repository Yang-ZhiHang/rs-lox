use crate::{chunk::Value, heap::Heap, object::ObjData};

pub fn clock_native(_arg_count: usize, _args: &[Option<Value>], _heap: &Heap) -> Value {
    use std::time::{SystemTime, UNIX_EPOCH};
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    Value::Number(since_the_epoch.as_secs_f64())
}

pub fn print_native(arg_count: usize, args: &[Option<Value>], heap: &Heap) -> Value {
    for i in 0..arg_count {
        if let Some(val) = args[i] {
            match val {
                Value::Object(obj_idx) => match heap.get(obj_idx) {
                    ObjData::String(obj_string) => print!("{}", obj_string),
                    other => print!("{}", other),
                },
                _ => print!("{}", val),
            }
        }
    }
    Value::Nil
}

pub fn println_native(arg_count: usize, args: &[Option<Value>], heap: &Heap) -> Value {
    print_native(arg_count, args, heap);
    println!();
    Value::Nil
}
