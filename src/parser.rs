use crate::tokenizer::{Token, TokenType, Tokenizer};

pub struct Parser {
    prev: Token,
    cur: Token,
}

impl Parser {
    // pub fn new() -> Self {
    //     Self { prev: (), cur: () }
    // }

    /// Compile source code into byte code.
    pub fn compile(&self, source: &str) -> &[u8] {
        let mut tokenizer = Tokenizer::new(source);
        let tokens = tokenizer.scan_tokens();
        // `advance` is the function which start the process of compiling.
        // advance();
        // expression();
        // consume(TokenType::EOF, "Expected end of expression.");
        todo!()
    }

    /// Start the process of scanning token and detect errors.
    pub fn advance(&mut self, tokenizer: &mut Tokenizer) {
        self.prev = self.cur;
        while !tokenizer.is_at_end() {
            let token = tokenizer.scan_token();
            if let TokenType::Error(msg) = token.token_type {
                error_at_current(msg);
            }
            break;
        }
    }
    
    /// Consume a token which matches the given token type. if don't, call error handling
    /// function with error message.
    pub fn consume(&self, tt: TokenType, err_msg: &str) {
        if (self.cur.token_type == tt) {
            self.advance(tokenizer);
        }
    }

    pub fn error_at_current(&self, msg: &str) {
        self.error_at()
    }
    
    // pub fn error_at(&self, )
}
