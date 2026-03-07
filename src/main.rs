use std::env;

pub mod chunk;
#[cfg(debug_assertions)]
pub mod common;
pub mod constant;
pub mod file;
pub mod macros;
pub mod tokenizer;
pub mod vm;

use crate::{file::read_file, tokenizer::Tokenizer, vm::VM};

/// Compile source code into byte code.
pub fn compile(source: &str) -> &[u8] {
    let mut tokenizer = Tokenizer::new(source);
    let tokens = tokenizer.scan_tokens();
    todo!()
}

pub fn run_file(vm: &VM, path: &str) {
    // Read source code
    let source = read_file(path);
    let byte_code = compile(&source);
}

/// Read-Eval-Print-Loop.
/// A type of console interaction.
pub fn repl(vm: &VM) {
    let mut line = String::new();
    loop {
        print!(">>> ");
        match std::io::stdin().read_line(&mut line) {
            Ok(_) => {
                todo!()
            }
            Err(error) => println!("Error reading input: {}", error),
        }
    }
}

fn main() {
    let mut vm = VM::new();
    let args: Vec<String> = env::args().collect();
    match args.len() {
        1 => run_file(&mut vm, &args[1]),
        2 => repl(&mut vm),
        _ => println!("Usage: lox [PATH]\n"),
    }
}
