#[derive(Clone, Copy, Debug)]
pub struct Token {
    pub token_type: TokenType,
    pub start: usize,
    pub len: usize,
    pub line: u16,
    pub col: u16,
}

impl Default for Token {
    fn default() -> Self {
        Self {
            token_type: TokenType::EOF,
            start: 0,
            len: 0,
            line: 1,
            col: 1,
        }
    }
}

impl Token {
    pub fn new(tt: TokenType, start: usize, len: usize, line: u16, col: u16) -> Self {
        Self {
            token_type: tt,
            start,
            len,
            line,
            col,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TokenType {
    // Pair
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    // Single character
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
    EqualEqual,
    // Literal
    String,
    Identifier,
    Number,
    // Keywords
    And,
    Class,
    Else,
    False,
    For,
    Fun,
    If,
    Nil,
    Or,
    Print,
    Return,
    Super,
    This,
    True,
    Var,
    While,
    // Others
    Error(&'static str),
    EOF,
}

/// The lifetime of `source` as same as the tokenizer.
pub struct Tokenizer<'a> {
    /// The source code string.
    src: &'a [u8],
    /// The array that stores each token in `source`.
    /// TODO: unused tokens and scan_tokens.
    tokens: Vec<Token>,
    /// The start index of current token. (Index start from 1)
    start: usize,
    /// The index of ready character.
    current: usize,
    /// The current line.
    line: u16,
    /// The current column.
    /// TODO: complete the logic of computing column number.
    col: u16,
}

impl<'a> Tokenizer<'a> {
    /// Create a tokenizer in initial state.
    pub fn new(source: &'a str) -> Self {
        Self {
            src: source.as_bytes(),
            tokens: vec![],
            start: 0,
            current: 0,
            line: 1,
            col: 0,
        }
    }

    /// Getter of member `source`.
    pub fn source(&self) -> &'a [u8] {
        self.src
    }

    /// Scan each character and return a token.
    pub fn scan_token(&mut self) -> Token {
        self.start = self.current;
        // Return end of file token type if reach the end (Currently, we don't use `scan_tokens()`).
        if self.is_at_end() {
            // Make `col` the last column.
            self.col = (self.current + 1) as u16;
            return self.make_token(TokenType::EOF);
        }
        let c = self.advance();
        if c.is_ascii_digit() {
            return self.number();
        } else if c.is_ascii_alphabetic() || c == '_' {
            return self.identifier();
        }
        match c {
            '(' => self.make_token(TokenType::LeftParen),
            ')' => self.make_token(TokenType::RightParen),
            '{' => self.make_token(TokenType::LeftBrace),
            '}' => self.make_token(TokenType::RightBrace),
            ',' => self.make_token(TokenType::Comma),
            '.' => self.make_token(TokenType::Dot),
            '-' => self.make_token(TokenType::Minus),
            '+' => self.make_token(TokenType::Plus),
            ';' => self.make_token(TokenType::Semicolon),
            '*' => self.make_token(TokenType::Star),
            '!' => {
                let t = if self.next('=') {
                    TokenType::BangEqual
                } else {
                    TokenType::Bang
                };
                self.make_token(t)
            }
            '=' => {
                let t = if self.next('=') {
                    TokenType::EqualEqual
                } else {
                    TokenType::Equal
                };
                self.make_token(t)
            }
            '<' => {
                let t = if self.next('=') {
                    TokenType::LessEqual
                } else {
                    TokenType::Less
                };
                self.make_token(t)
            }
            '>' => {
                let t = if self.next('=') {
                    TokenType::GreaterEqual
                } else {
                    TokenType::Greater
                };
                self.make_token(t)
            }
            '/' => self.make_token(TokenType::Slash),
            '"' => self.string(),
            _ => self.error_token("Unexpected character"),
        }
    }

    /// Call this function when scanning tokens, it will consume ignore character and
    /// automatically increase `current`.
    pub fn skip_ignore_character(&mut self) {
        loop {
            let c = self.peek(0);
            match c {
                ' ' | '\r' | '\t' => {
                    self.advance();
                }
                '\n' => {
                    self.line += 1;
                    self.col = 1;
                    self.advance();
                }
                '/' => {
                    if self.peek(1) == '/' {
                        self.line_comment();
                    } else {
                        return;
                    }
                }
                _ => return,
            }
        }
    }

    /// Scan the source code and return list of tokens.
    pub fn scan_tokens(&mut self) -> Vec<Token> {
        while !self.is_at_end() {
            self.skip_ignore_character();
            if self.is_at_end() {
                break;
            }
            let token = self.scan_token();
            self.tokens.push(token);
        }
        self.tokens.push(self.make_token(TokenType::EOF));
        // Use `to_vec()` to make copy a new array and return.
        // Because `Vec` not support `Copy` trait.
        self.tokens.to_vec()
    }

    /// Return a `Token` struct according to token type.
    /// The information of `Token` (start index, length, line number) will be automatically
    /// supplied from tokenizer.
    pub fn make_token(&self, tt: TokenType) -> Token {
        Token::new(
            tt,
            self.start,
            self.current - self.start,
            self.line,
            self.col,
        )
    }

    /// Generate a error token with error message.
    /// The error token is used to report error in scanning. Error message was passed
    /// to `TokenType::Error`. The way to get the error message is to match the token
    /// type.
    pub fn error_token(&self, message: &'static str) -> Token {
        Token::new(
            TokenType::Error(message),
            self.start,
            self.current - self.start,
            self.line,
            self.col,
        )
    }

    /// Judge if we have scanned to the last character of the source code.
    pub fn is_at_end(&self) -> bool {
        self.current >= self.src.len()
    }

    /// `current` will be at the next index and return the character at the former index.
    pub fn advance(&mut self) -> char {
        self.current += 1;
        self.col += 1;
        self.src[self.current - 1] as char
    }

    /// Add token to the token list.
    pub fn add_token(&mut self, token: TokenType) {
        self.tokens.push(Token::new(
            token,
            self.start,
            self.current - self.start,
            self.line,
            self.col,
        ));
    }

    /// Judge if the next token equals to variable `c`. If equals, `current` will increase.
    pub fn next(&mut self, c: char) -> bool {
        if self.is_at_end() {
            return false;
        }
        if self.src[self.current] as char == c {
            self.advance();
            return true;
        }
        false
    }

    /// Get the character behind `current` in `n` indexes. `current` will not increase.
    pub fn peek(&self, n: usize) -> char {
        let idx = self.current + n;
        if idx >= self.src.len() {
            return '\0';
        }
        self.src[idx] as char
    }

    /// Skip a `//` line comment, consuming until end of line.
    fn line_comment(&mut self) {
        while !self.is_at_end() && self.peek(0) != '\n' {
            self.advance();
        }
    }

    /// Call this function when scanning meets `"`.
    /// Keep scanning (including line feed) until meets the close character `"`.
    pub fn string(&mut self) -> Token {
        while self.peek(0) != '"' && !self.is_at_end() {
            if self.peek(0) == '\n' {
                self.line += 1
            };
            self.advance();
        }
        if self.is_at_end() {
            return self.error_token("Unclosed string");
        }
        // Consume the closing `"`.
        self.advance();
        self.make_token(TokenType::String)
    }

    /// Call this function when scanning meets digit.
    /// Keep scanning until meets non-digital character.
    /// While scanning, it only allow `.` to appear once.
    pub fn number(&mut self) -> Token {
        while self.peek(0).is_ascii_digit() {
            self.advance();
        }
        if self.peek(0) == '.' && self.peek(1).is_ascii_digit() {
            // Consume dot
            self.advance();
            // Then, consume the rest of digit
            while self.peek(0).is_ascii_digit() {
                self.advance();
            }
        }
        self.make_token(TokenType::Number)
    }

    /// Call this function when scanning meets alpha.
    /// Judging weather the identifier is keyword.
    pub fn identifier(&mut self) -> Token {
        match self.src[self.start] as char {
            'a' => self.check_keyword(1, 2, "nd", TokenType::And),
            'c' => self.check_keyword(1, 4, "lass", TokenType::Class),
            'e' => self.check_keyword(1, 3, "lse", TokenType::Else),
            'f' => match self.src[self.start + 1] as char {
                'a' => self.check_keyword(2, 3, "lse", TokenType::False),
                'o' => self.check_keyword(2, 1, "r", TokenType::For),
                'u' => self.check_keyword(2, 1, "n", TokenType::Fun),
                _ => self.make_token(TokenType::Identifier),
            },
            'i' => self.check_keyword(1, 1, "f", TokenType::If),
            'n' => self.check_keyword(1, 2, "il", TokenType::Nil),
            'o' => self.check_keyword(1, 1, "r", TokenType::Or),
            'p' => self.check_keyword(1, 4, "rint", TokenType::Print),
            'r' => self.check_keyword(1, 5, "eturn", TokenType::Return),
            's' => self.check_keyword(1, 4, "uper", TokenType::Super),
            't' => match self.src[self.start + 1] as char {
                'h' => self.check_keyword(2, 2, "is", TokenType::This),
                'r' => self.check_keyword(2, 2, "ue", TokenType::True),
                _ => self.make_token(TokenType::Identifier),
            },
            'v' => self.check_keyword(1, 2, "ar", TokenType::Var),
            'w' => self.check_keyword(1, 4, "hile", TokenType::While),
            _ => self.make_token(TokenType::Identifier),
        }
    }

    /// Check if the scanning token is keyword, else if return normal identifier token type.
    pub fn check_keyword(
        &mut self,
        start: usize,
        len: usize,
        pattern: &str,
        tt: TokenType,
    ) -> Token {
        if self.start + start + len < self.src.len()
            && &self.src[self.start + start..self.start + start + len] == pattern.as_bytes()
        {
            self.current = self.start + start + len;
            self.make_token(tt)
        } else {
            self.make_token(TokenType::Identifier)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    macro_rules! test_tokenizer {
        ($name:ident, $cases:expr) => {
            #[test]
            fn $name() {
                for (src, expected) in $cases {
                    let mut tokenizer = Tokenizer::new(src);
                    let tokens = tokenizer.scan_tokens();
                    assert_eq!(token_types(&tokens), expected);
                }
            }
        };
    }

    /// Extract token types from token list.
    fn token_types(tokens: &[Token]) -> Vec<TokenType> {
        tokens.iter().map(|t| t.token_type).collect()
    }

    test_tokenizer!(
        test_tokens,
        [
            // Empty source should produce only EOF token.
            ("", vec![TokenType::EOF]),
            // Single character tokens.
            (
                "(){},.-+;*=<>!",
                vec![
                    TokenType::LeftParen,
                    TokenType::RightParen,
                    TokenType::LeftBrace,
                    TokenType::RightBrace,
                    TokenType::Comma,
                    TokenType::Dot,
                    TokenType::Minus,
                    TokenType::Plus,
                    TokenType::Semicolon,
                    TokenType::Star,
                    TokenType::Equal,
                    TokenType::Less,
                    TokenType::Greater,
                    TokenType::Bang,
                    TokenType::EOF,
                ]
            ),
            // Two character tokens.
            (
                "!= <= >=",
                vec![
                    TokenType::BangEqual,
                    TokenType::LessEqual,
                    TokenType::GreaterEqual,
                    TokenType::EOF
                ]
            ),
            // String token.
            (
                "\"Hello, world\n\"",
                vec![TokenType::String, TokenType::EOF]
            ),
            // Comment should be ignored.
            ("// this is a comment\n", vec![TokenType::EOF])
        ]
    );
}
