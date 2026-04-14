use std::env;

use lox::{file::read_file, native::{clock_native, print_native, println_native}, parser::Parser, tokenizer::Tokenizer, vm::VM};

/// Parse the source code with streaming parser.
/// For streaming parser, it uses less memory than Tree-walking parser.
pub fn run_file(vm: &mut VM, path: &str) {
    let source = read_file(path);
    let tokenizer = Tokenizer::new(source);
    let parser = Parser::new(tokenizer, &mut vm.heap);
    match parser.compile() {
        Some(func_obj_idx) => {
            vm.interpret(func_obj_idx);
        }
        None => {
            eprintln!("Error compiling source code.");
        }
    }
}

/// Read-Eval-Print-Loop.
/// A type of console interaction.
pub fn repl(_vm: &mut VM) {
    let mut line = String::new();
    loop {
        print!(">>> ");
        match std::io::stdin().read_line(&mut line) {
            Ok(_) => {
                unimplemented!()
            }
            Err(error) => println!("Error reading input: {}", error),
        }
    }
}

fn main() {
    let mut vm = VM::new();
    vm.define_native("clock", clock_native);
    vm.define_native("print", print_native);
    vm.define_native("println", println_native);
    let args: Vec<String> = env::args().collect();
    match args.len() {
        1 => repl(&mut vm),
        2 => run_file(&mut vm, &args[1]),
        _ => println!("Usage: lox [PATH]\n"),
    }
}
