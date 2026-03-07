# Lox Interpreter

The lox interpreter is implemented by `Rust` lang.

Different from Tree-Walking interpreter, lox interpreter is virtual machine based interpreter which first interpret language to byte code and execute by virtual machine.

## Features

- [x] Virtual machine based interpreter.
- [x] Instruction set based on stack (A better performance one is based on register).

## Future work

- [ ] Refactor vm to register-based and compare the performance diff.