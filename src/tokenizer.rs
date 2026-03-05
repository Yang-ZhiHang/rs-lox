#[derive(Clone, Copy, Debug)]
pub struct Token {
    pub token_type: TokenType,
    pub start: usize,
    pub len: usize,
    pub line: usize,
}

impl Default for Token {
    fn default() -> Self {
        Self {
            token_type: TokenType::EOF,
            start: 0,
            len: 0,
            line: 1,
        }
    }
}

impl Token {
    pub fn new(tt: TokenType, start: usize, len: usize, line: usize) -> Self {
        Self {
            token_type: tt,
            start,
            len,
            line,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TokenType {
    // Single character
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Dot,
    Minus,
    Plus,
    Semicolon,
    Star,
    Bang,
    Equal,
    Less,
    Greater,
    Slash,
    // Two character
    BangEqual,
    LessEqual,
    GreaterEqual,
    //
    String,
    // Others
    EOF,
}

/// The lifetime of `source` as same as the tokenizer.
pub struct Tokenizer<'a> {
    /// The source code string.
    source: &'a str,
    /// The array that stores each token in `source`.
    tokens: Vec<Token>,
    /// The start index of current token. (Index start from 1)
    start: usize,
    /// The index of cuurent character of current token.
    current: usize,
    /// The current line of source code file.
    line: usize,
}

impl<'a> Tokenizer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            start: 0,
            current: 0,
            line: 1,
            tokens: vec![],
        }
    }

    /// Scan the source code and return list of tokens.
    pub fn scan_tokens(&mut self) -> Vec<Token> {
        while !self.is_at_end() {
            self.start = self.current;
            self.scan_token();
        }
        self.tokens.push(Token::default());
        // Use `to_vec()` to make copy a new array and return.
        // Because `Vec` not support `Copy` trait.
        self.tokens.to_vec()
    }

    /// Judge if we have scanned to the last character of the source code.
    pub fn is_at_end(&self) -> bool {
        return self.current >= self.source.len();
    }

    /// Consume `current` and return the character at the index.
    pub fn advance(&mut self) -> char {
        self.current += 1;
        return self
            .source
            .chars()
            .nth(self.current - 1)
            .expect("Index out of bound");
    }

    /// Add token to the token list.
    pub fn add_token(&mut self, token: TokenType) {
        self.tokens.push(Token::new(
            token,
            self.start,
            self.current - self.start,
            self.line,
        ));
    }

    /// Judge if the next token equals to variable `c`. If equals, `current` will increase.
    pub fn next(&mut self, p: char) -> bool {
        if let Some(c) = self.source.chars().nth(self.current)
            && c == p
        {
            self.current += 1;
            return true;
        }
        false
    }

    /// Get the character behind `current` in `n` indexes. `current` will not increase.
    pub fn peek(&self, n: usize) -> char {
        if let Some(c) = self.source.chars().nth(self.current + n) {
            return c;
        }
        '\0'
    }

    /// Skip a `//` line comment, consuming until end of line.
    fn line_comment(&mut self) {
        while self.peek(0) != '\n' && !self.is_at_end() {
            self.current += 1;
        }
    }

    /// Skip a `/* */` block comment, consuming until `*/` is found.
    fn block_comment(&mut self) {
        while !self.is_at_end() {
            if self.peek(0) == '*' && self.peek(1) == '/' {
                self.current += 2;
                return;
            }
            if self.peek(0) == '\n' {
                self.line += 1;
            }
            self.current += 1;
        }
        println!("Unclosed block comment.");
    }

    /// Judge if the token is annotation keyword `//` or `/*..*/`.
    pub fn comment(&mut self) {
        if self.next('/') {
            self.line_comment();
        } else if self.next('*') {
            self.block_comment();
        } else {
            self.add_token(TokenType::Slash);
        }
    }

    /// Consume all the character between `"` pairs.
    pub fn string(&mut self) {
        while self.peek(0) != '"' && !self.is_at_end() {
            if self.peek(0) == '\n' {
                self.line += 1
            };
            self.current += 1;
        }
        if self.is_at_end() {
            println!("Unclosed string.");
            return;
        }
        // Consume the closing `"`.
        self.advance();

        // TODO: maybe we need add string slice value to the token list?
        // let s = &self.source[self.start + 1..self.current - 1];
        self.tokens.push(Token::new(
            TokenType::String,
            self.start + 1,
            self.current - self.start,
            self.line,
        ));
    }

    /// Scan each character and add token to the token list.
    pub fn scan_token(&mut self) {
        let c = self.advance();
        match c {
            '(' => self.add_token(TokenType::LeftParen),
            ')' => self.add_token(TokenType::RightParen),
            '{' => self.add_token(TokenType::LeftBrace),
            '}' => self.add_token(TokenType::RightBrace),
            ',' => self.add_token(TokenType::Comma),
            '.' => self.add_token(TokenType::Dot),
            '-' => self.add_token(TokenType::Minus),
            '+' => self.add_token(TokenType::Plus),
            ';' => self.add_token(TokenType::Semicolon),
            '*' => self.add_token(TokenType::Star),
            '!' => {
                let t = if self.next('=') {
                    TokenType::BangEqual
                } else {
                    TokenType::Bang
                };
                self.add_token(t);
            }
            '<' => {
                let t = if self.next('=') {
                    TokenType::LessEqual
                } else {
                    TokenType::Less
                };
                self.add_token(t);
            }
            '>' => {
                let t = if self.next('=') {
                    TokenType::GreaterEqual
                } else {
                    TokenType::Greater
                };
                self.add_token(t);
            }
            '/' => self.comment(),
            '"' => self.string(),
            ' ' | '\r' | '\t' => {
                // Ignore
            }
            '\n' => self.line += 1,
            _ => {
                println!("Unexpected token: {}", c);
            }
        }
    }
}
