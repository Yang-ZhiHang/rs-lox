#[cfg(debug_assertions)]
use crate::common::disassemble;
use crate::{
    chunk::{Chunk, IntoU8, OpCode, Value},
    heap::Heap,
    object::ObjId,
    tokenizer::{Token, TokenType, Tokenizer},
};

/// Function pointer type for prefix/infix parse functions.
type ParseFn<'src, 'heap> = fn(&mut Parser<'src, 'heap>);

#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub enum Precedence {
    None,
    Assignment, // =
    Or,         // or
    And,        // and
    Equality,   // == !=
    Comparison, // < > <= >=
    Term,       // + -
    Factor,     // * /
    Unary,      // ! -
    Call,       // . ()
    Primary,
}

impl Precedence {
    #[rustfmt::skip]
    /// Returns the next higher precedence level.
    /// Used in infix parsing to enforce left-associativity.
    pub fn next(self) -> Self {
        match self {
            Self::None       => Self::Assignment,
            Self::Assignment => Self::Or,
            Self::Or         => Self::And,
            Self::And        => Self::Equality,
            Self::Equality   => Self::Comparison,
            Self::Comparison => Self::Term,
            Self::Term       => Self::Factor,
            Self::Factor     => Self::Unary,
            Self::Unary      => Self::Call,
            Self::Call       => Self::Primary,
            // Primary is already the highest, return self to avoid going out of bounds.
            Self::Primary => Self::Primary,
        }
    }
}

pub struct ParseRule<'src, 'heap> {
    pub prefix: Option<ParseFn<'src, 'heap>>,
    pub infix: Option<ParseFn<'src, 'heap>>,
    pub precedence: Precedence,
}

impl<'src, 'heap> ParseRule<'src, 'heap> {
    fn new(
        prefix: Option<ParseFn<'src, 'heap>>,
        infix: Option<ParseFn<'src, 'heap>>,
        precedence: Precedence,
    ) -> Self {
        Self {
            prefix,
            infix,
            precedence,
        }
    }
}

/// Returns the parse rule for the given token type.
/// Using a `match` instead of a static array avoids lifetime and fn-pointer coercion
/// issues that arise from `Parser<'src>` carrying a lifetime parameter.
#[rustfmt::skip]
pub fn get_rule<'src, 'heap>(tt: TokenType) -> ParseRule<'src, 'heap> {
    match tt {
        TokenType::LeftParen    => ParseRule::new(Some(Parser::grouping), None,                 Precedence::None),
        TokenType::RightParen   => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::LeftBrace    => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::RightBrace   => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::Comma        => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::Dot          => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::Minus        => ParseRule::new(Some(Parser::unary),    Some(Parser::binary), Precedence::Term),
        TokenType::Plus         => ParseRule::new(None,                   Some(Parser::binary), Precedence::Term),
        TokenType::Semicolon    => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::Star         => ParseRule::new(None,                   Some(Parser::binary), Precedence::Factor),
        TokenType::Bang         => ParseRule::new(Some(Parser::unary),    None,                 Precedence::None),
        TokenType::Slash        => ParseRule::new(None,                   Some(Parser::binary), Precedence::Factor),
        TokenType::BangEqual    => ParseRule::new(None,                   Some(Parser::binary), Precedence::Equality),
        TokenType::LessEqual    => ParseRule::new(None,                   Some(Parser::binary), Precedence::Comparison),
        TokenType::GreaterEqual => ParseRule::new(None,                   Some(Parser::binary), Precedence::Comparison),
        TokenType::EqualEqual   => ParseRule::new(None,                   Some(Parser::binary), Precedence::Comparison),
        TokenType::Less         => ParseRule::new(None,                   Some(Parser::binary), Precedence::Comparison),
        TokenType::Greater      => ParseRule::new(None,                   Some(Parser::binary), Precedence::Comparison),
        TokenType::Equal        => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::String       => ParseRule::new(Some(Parser::string),  None,                 Precedence::None),
        TokenType::Identifier   => ParseRule::new(Some(Parser::variable),                   None,                 Precedence::None),
        TokenType::Number       => ParseRule::new(Some(Parser::number),   None,                 Precedence::None),
        TokenType::And          => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::Class        => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::Else         => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::For          => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::Fun          => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::If           => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::Or           => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::Print        => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::Return       => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::Super        => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::This         => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::True         => ParseRule::new(Some(Parser::literal),  None,                 Precedence::None),
        TokenType::False        => ParseRule::new(Some(Parser::literal),  None,                 Precedence::None),
        TokenType::Nil          => ParseRule::new(Some(Parser::literal),  None,                 Precedence::None),
        TokenType::Var          => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::While        => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::Error(_)     => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::EOF          => ParseRule::new(None,                   None,                 Precedence::None),
    }
}

pub enum CompileError {
    SyntaxError,
}

pub struct Parser<'src, 'heap> {
    tokenizer: Tokenizer<'src>,
    heap: &'heap mut Heap,
    /// The container to store byte code when compiling.
    pub chunk: Chunk,
    /// Why we need a prev and cur token? Why only two tokens?
    prev: Token,
    cur: Token,
    had_error: bool,
    panic_mode: bool,
}

impl<'src, 'heap> Parser<'src, 'heap> {
    pub fn new(tokenizer: Tokenizer<'src>, heap: &'heap mut Heap) -> Self {
        Self {
            tokenizer,
            heap,
            chunk: Chunk::new(),
            prev: Token::default(),
            cur: Token::default(),
            had_error: false,
            panic_mode: false,
        }
    }

    /// Start single-step token scanning and detect error.
    /// If the token is valid, the scanning will stop. Else it will continue to detect
    /// error.
    pub fn advance(&mut self) {
        self.prev = self.cur;
        loop {
            self.tokenizer.skip_ignore_character();
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

    /// Compile source code into byte code.
    pub fn compile(mut self) -> Result<Chunk, CompileError> {
        self.advance();
        // self.expression();
        // self.consume(TokenType::EOF, "Expected end of expression.");
        while !self.next(TokenType::EOF) {
            self.declaration();
        }
        self.end_compile();
        if self.had_error {
            Err(CompileError::SyntaxError)
        } else {
            Ok(self.chunk)
        }
    }

    /// make an advance and return true if the current token matches the given
    /// token type else false.
    pub fn next(&mut self, tt: TokenType) -> bool {
        if !self.check(tt) {
            return false;
        }
        self.advance();
        true
    }

    /// Check if the current token matches the given token type.
    pub fn check(&self, tt: TokenType) -> bool {
        self.cur.token_type == tt
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

        print!("{}:{}: Error", token.line, token.col);
        if token.token_type == TokenType::EOF {
            print!(" at end")
        } else if let TokenType::Error(_) = token.token_type {
            // The error message of `TokenType::Error` is passed-in parameter `msg`.
        } else {
            let token =
                str::from_utf8(&self.tokenizer.source()[token.start..token.start + token.len])
                    .unwrap();
            print!(" at '{}'", token);
        }
        println!(": {}", msg);
        self.had_error = true;
    }

    pub fn declaration(&mut self) {
        if self.next(TokenType::Var) {
            self.var_declaration();
        } else if self.next(TokenType::Fun) {
            self.fun_declaration();
        } else {
            self.statement();
        }
        if self.panic_mode {
            self.synchronize();
        }
    }

    pub fn var_declaration(&mut self) {
        // 1. parse variable name
        let global = self.parse_variable("Expected variable name.");
        // 2. parse '='
        if self.next(TokenType::Equal) {
            // 3. parse expression
            self.expression();
            // 4. parse end ';'
            self.consume(TokenType::Semicolon, "Expected ';' of expression");
        } else {
            self.emit_byte(OpCode::Nil, self.cur.line);
        }
        self.define_variable(global);
    }

    pub fn fun_declaration(&self) {
        unimplemented!()
    }

    pub fn statement(&mut self) {
        if self.next(TokenType::Print) {
            self.print_statement();
        } else {
            self.expression_statement();
        }
    }

    /// In print statement, we finally emit print opcode.
    pub fn print_statement(&mut self) {
        self.expression();
        self.consume(TokenType::Semicolon, "Expected ';' of expression.");
        self.emit_byte(OpCode::Print, self.cur.line);
    }

    /// In expression statement, we finally emit a pop like rust.
    pub fn expression_statement(&mut self) {
        self.expression();
        self.consume(TokenType::Semicolon, "Expected ';' of expression.");
        self.emit_byte(OpCode::Pop, self.cur.line);
    }

    pub fn parse_variable(&mut self, msg: &str) -> usize {
        self.consume(TokenType::Identifier, msg);
        self.identifier_constant(self.prev)
    }

    /// Advance to a identifier indicating an end after a panic error.
    pub fn synchronize(&mut self) {
        while !self.check(TokenType::EOF) {
            if self.prev.token_type == TokenType::Semicolon {
                break;
            }
            match self.cur.token_type {
                TokenType::Class
                | TokenType::Fun
                | TokenType::Var
                | TokenType::For
                | TokenType::If
                | TokenType::While
                | TokenType::Print
                | TokenType::Return => {
                    break;
                }
                _ => {}
            }
            self.advance();
        }
    }

    pub fn emit_byte(&mut self, byte: impl IntoU8, line: u16) {
        self.chunk.write(byte, line);
    }

    /// Write operation code and constant value index (which from constant area) to the chunk.
    pub fn emit_constant(&mut self, constant: Value, line: u16) {
        let idx = self.chunk.write_constant(constant);
        self.emit_bytes(OpCode::Constant, idx, line);
    }

    /// Write two bytes to the chunk.
    /// Used to write opcode with it's immediate operand.
    pub fn emit_bytes(&mut self, byte1: impl IntoU8, byte2: impl IntoU8, line: u16) {
        self.emit_byte(byte1, line);
        self.emit_byte(byte2, line);
    }

    /// Write return operation code to chunk.
    pub fn emit_return(&mut self) {
        self.emit_byte(OpCode::Return, self.cur.line);
    }

    /// Write return operation code to chunk.
    pub fn end_compile(&mut self) {
        self.emit_return();
        #[cfg(debug_assertions)]
        if !self.had_error {
            disassemble(&self.chunk, self.heap, "code");
        }
    }

    /// Add a identifier into the heap and get a object index.
    ///
    /// Return the index in constant area (Which stores the object index).
    pub fn identifier_constant(&mut self, t: Token) -> usize {
        let slice = &self.tokenizer.source()[t.start..t.start + t.len];
        let s = std::str::from_utf8(slice).unwrap();
        let obj_idx = self.heap.write_string(s);
        self.chunk.write_constant(Value::Object(ObjId(obj_idx)))
    }

    /// Define a global variable
    pub fn define_variable(&mut self, global: impl IntoU8) {
        self.emit_bytes(OpCode::DefineGlobal, global, self.cur.line);
    }

    /// The unary operator handling unit of Pratt parser.
    pub fn unary(&mut self) {
        let tt = self.prev.token_type;
        // Save the operator's line before expression() advances `prev`
        // to the operand, which would cause emit_byte to record the wrong
        // line number for the unary opcode.
        let line = self.prev.line;
        // Compile the operand.
        self.parse_precedence(Precedence::Unary);
        // Emit the operator instruction.
        match tt {
            TokenType::Minus => {
                self.chunk.write(OpCode::Negate, line);
            }
            TokenType::Bang => {
                self.chunk.write(OpCode::Not, line);
            }
            _ => {}
        }
    }

    /// The binary operator handling unit of Pratt parser.
    pub fn binary(&mut self) {
        let tt = self.prev.token_type;
        // Save the operator's line number before `parse_precedence` advances `prev`.
        // This ensures the emitted bytecode has the correct line number for the oper
        // -ator, even if the expression spans multiple lines.
        let line = self.prev.line;
        let rule = get_rule(tt);
        self.parse_precedence(rule.precedence.next());
        #[rustfmt::skip]
        match tt {
            TokenType::Plus         => self.emit_byte(OpCode::Add, line),
            TokenType::Minus        => self.emit_byte(OpCode::Subtract, line),
            TokenType::Star         => self.emit_byte(OpCode::Multiply, line),
            TokenType::Slash        => self.emit_byte(OpCode::Divide, line),
            TokenType::Equal        => self.emit_byte(OpCode::Equal, line),
            TokenType::BangEqual    => self.emit_bytes(OpCode::Equal, OpCode::Not, line),
            TokenType::Less         => self.emit_byte(OpCode::Less, line),
            TokenType::Greater      => self.emit_byte(OpCode::Greater, line),
            TokenType::LessEqual    => self.emit_bytes(OpCode::Less, OpCode::Not, line),
            TokenType::GreaterEqual => self.emit_bytes(OpCode::Greater, OpCode::Not, line),
            TokenType::EqualEqual   => self.emit_byte(OpCode::Equal, line),
            _ => {},
        };
    }

    /// The number handling unit of Pratt parser.
    pub fn number(&mut self) {
        let source = self.tokenizer.source();
        let slice = &source[self.prev.start..self.prev.start + self.prev.len];
        let val: f64 = std::str::from_utf8(slice)
            .expect("Number token should be valid UTF-8.")
            .parse()
            .expect("Number token should be a valid float.");
        self.emit_constant(Value::Number(val), self.prev.line);
    }

    /// The parenthesis handling unit of Pratt parser.
    pub fn grouping(&mut self) {
        self.expression();
        self.consume(TokenType::RightParen, "Expected ')' after expression.");
    }

    #[rustfmt::skip]
    /// The literal handling unit of Pratt parser.
    pub fn literal(&mut self) {
        match self.prev.token_type {
            TokenType::True   => self.emit_byte(OpCode::True, self.prev.line),
            TokenType::False  => self.emit_byte(OpCode::False, self.prev.line),
            TokenType::Nil    => self.emit_byte(OpCode::Nil, self.prev.line),
            _ => {
                unreachable!()
            }
        }
    }

    /// The string handling unit of Pratt parser.
    pub fn string(&mut self) {
        let slice =
            &self.tokenizer.source()[self.prev.start + 1..self.prev.start + self.prev.len - 1];
        let s = std::str::from_utf8(slice).unwrap();
        let idx = self.heap.write_string(s);
        self.emit_constant(Value::Object(ObjId(idx)), self.prev.line);
    }

    /// The variable handling unit of Pratt parser.
    pub fn variable(&mut self) {
        let idx = self.identifier_constant(self.prev);
        self.emit_bytes(OpCode::GetGlobal, idx, self.prev.line);
    }

    /// Parse the precedence of previous token.
    pub fn parse_precedence(&mut self, precedence: Precedence) {
        self.advance();
        if let Some(prefix_rule) = get_rule(self.prev.token_type).prefix {
            prefix_rule(self);
            while precedence <= get_rule(self.cur.token_type).precedence {
                self.advance();
                if let Some(infix_rule) = get_rule(self.prev.token_type).infix {
                    infix_rule(self);
                }
            }
        } else {
            println!("Expected expression.");
        }
    }

    /// Compile the previous token.
    pub fn expression(&mut self) {
        // Temporarily use assginment percedence to parse the whole expression.
        self.parse_precedence(Precedence::Assignment);
    }
}
