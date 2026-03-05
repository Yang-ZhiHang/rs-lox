use std::env;

pub mod tokenizer;

fn run_file(path: String) {
    println!("Running file: {}", path);
}

fn run_prompt() {
    println!("Running in interactive mode");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() > 2 {
        println!("Usage: lox [script]");
    } else if args.len() == 2 {
        run_file(args[1].clone());
    } else {
        run_prompt();
    }
}
