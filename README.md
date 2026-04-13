# Lox Interpreter

The lox interpreter is implemented by `Rust` lang.

Different from Tree-Walking interpreter, lox interpreter is a Single-pass interpreter. This interpreter is also a virtual machine based interpreter.

## Declaration

For this interpreter learning project, we don't strictly obey the official grammar standard. I made some different design choices or some additional features from the original one in `Crafting Interpreters` book. As the following:

1. Change `var` to `let` for variable declaration.
2. Add `&&` and `||` logical operators.
3. Add `switch` statement.
4. Add `+=`、`-=`、`*=`、`/=` assignment operators.

As using raw pointer in `Rust` is unsafe, so we use `ObjIndex` to represent the index of object in the heap. This is a safe way to implement the virtual machine based interpreter.

## Features

- [x] Single-pass interpreter which performs better than tree-walk one.
- [x] Virtual machine based interpreter.
- [x] Instruction set based on stack (A better performance one is based on register).
- [x] Support advanced features of closure.

## Grammar

Here, we support:

- variable declaration & assignment
- logical operators
- control flow (if, while, for, switch)
- function statement

### Variable declaration & assignment

```
let a = 1;
let b = 2;
let c = a + b;
print c + 1; // 4
```

### Logical operators

```
let a = 2;
if a > 1 and a < 3 {
  print "a is greater than 1 and b is less than 3";
}
```

### Control flow

#### If statement

```
let a = 0;
if (a > 1) {
  print "a is greater than 1";
} else if (a < 1) {
  print "a is less than 1";
} else {
  print "a is equal to 1";
}
```

#### While statement

```
let a = 0;
while (a < 10) {
  print a;
  a = a + 1;
}
```

#### For statement

```
for (let i = 0; i < 10; i = i + 1) {
  print i;
}
```

#### Switch statement

```
let a = 1;
switch a {
  case 1:
    print "a is 1";
  case 2:
    print "a is 2";
  default:
    print "a is not 1 or 2";
}
```

### Function statement

```
fun fibonacci(n) {
  if (n <= 1) return n;
  return fibonacci(n - 1) + fibonacci(n - 2);
}
print fibonacci(10); // 55
```

### Closure

```
fun makeCounter() {
  let count = 0;
  fun counter() {
    count = count + 1;
    return count;
  }
  return counter;
}

let counter = makeCounter();
print counter(); // 1
print counter(); // 2
print counter(); // 3
```

## References

- [Crafting Interpreters](https://craftinginterpreters.com): Follow the step of Robert Nystrom to make your own programming language (Implemented by Java and C).
- [Pratt Parsers](https://journal.stuffwithstuff.com/2011/03/19/pratt-parsers-expression-parsing-made-easy/): A better way to parse expression.