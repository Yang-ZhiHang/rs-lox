set shell := ["powershell", "-c"]

test:
    cargo run examples/test.lox

test-release:
    cargo run --release examples/test.lox