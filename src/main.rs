use std::env;

pub mod chunk;
#[cfg(debug_assertions)]
pub mod common;
pub mod file;
pub mod macros;
pub mod parser;
pub mod tokenizer;
pub mod vm;

use crate::{file::read_file, parser::Parser, tokenizer::Tokenizer, vm::VM};

pub fn run_file(vm: &mut VM, path: &str) {
    // Read source code
    let source = read_file(path);
    let tokenizer = Tokenizer::new(&source);
    let parser = Parser::new(tokenizer);
    match parser.compile() {
        Ok(chunk) => {
            vm.interpret(&chunk);
        }
        Err(_) => {
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
        1 => repl(&mut vm),
        2 => run_file(&mut vm, &args[1]),
        _ => println!("Usage: lox [PATH]\n"),
    }
}
