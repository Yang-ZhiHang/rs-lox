use crate::{
    chunk::{Chunk, IntoU8, OpCode},
    tokenizer::{Token, TokenType, Tokenizer},
};

pub struct Parser<'a> {
    tokenizer: Tokenizer<'a>,
    /// The container to store byte code when compiling.
    chunk: Chunk,
    /// Why we need a prev and cur token? Why only two tokens?
    prev: Token,
    cur: Token,
    had_error: bool,
    panic_mode: bool,
}

impl<'a> Parser<'a> {
    pub fn new(tokenizer: Tokenizer<'a>) -> Self {
        Self {
            tokenizer,
            chunk: Chunk::new(),
            prev: Token::default(),
            cur: Token::default(),
            had_error: false,
            panic_mode: false,
        }
    }

    /// Compile source code into byte code.
    pub fn compile(&mut self, source: &str) -> bool {
        self.advance();
        // self.expression();
        self.consume(TokenType::EOF, "Expected end of expression.");
        !self.had_error
    }

    /// Start single-step token scanning and detect error.
    /// If the token is valid, the scanning will stop. Else it will continue to detect
    /// error.
    pub fn advance(&mut self) {
        self.prev = self.cur;
        loop {
            self.cur = self.tokenizer.scan_token();
            if let TokenType::Error(msg) = self.cur.token_type {
                self.error_at_current(msg);
                continue;
            }
            break;
        }
    }

    /// Consume a token which matches the given token type. if don't, call error handling
    /// function with error message.
    pub fn consume(&mut self, tt: TokenType, msg: &str) {
        if self.cur.token_type == tt {
            self.advance();
            return;
        }
        self.error_at_current(msg);
    }

    /// Print error message at current scanning token.
    pub fn error_at_current(&mut self, msg: &str) {
        self.error_at(self.cur, msg);
    }

    /// Print error message for given token.
    pub fn error_at(&mut self, token: Token, msg: &str) {
        // Use panic mode flag to avoid cascade error message.
        if self.panic_mode {
            return;
        }
        self.panic_mode = true;

        print!("[line {}] Error", token.line);
        if token.token_type == TokenType::EOF {
            print!(" at end")
        } else if let TokenType::Error(_) = token.token_type {
            // The error message of `TokenType::Error` is passed-in parameter `msg`.
        } else {
            let token = unsafe {
                str::from_utf8_unchecked(
                    &self.tokenizer.source()[token.start..token.start + token.len],
                )
            };
            print!(" at '{}'", token);
        }
        print!(": {}", msg);
        self.had_error = true;
    }

    pub fn emit_byte(&mut self, byte: impl IntoU8) {
        self.chunk.write(byte, self.prev.line);
    }

    pub fn emit_bytes(&mut self, byte1: impl IntoU8, byte2: impl IntoU8) {
        self.emit_byte(byte1);
        self.emit_byte(byte2);
    }

    pub fn emit_return(&mut self) {
        self.emit_byte(OpCode::Return);
    }

    pub fn end_compile(&mut self) {
        self.emit_return();
    }
}
