# Lox Interpreter

The lox interpreter is implemented by `Rust` lang.

Different from Tree-Walking interpreter, lox interpreter is a Single-pass interpreter. This interpreter is also a virtual machine based interpreter.

## Features

- [x] Single-pass interpreter which performs better than tree-walk one.
- [x] Virtual machine based interpreter.
- [x] Instruction set based on stack (A better performance one is based on register).

## Future work

- [ ] Refactor vm to register-based and compare the performance diff.

## References

- [Crafting Interpreters](https://craftinginterpreters.com): Follow the step of Robert Nystrom to make your own programming language (Implemented by Java and C).