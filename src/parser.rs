use std::u16;

#[cfg(debug_assertions)]
use crate::common::disassemble;
use crate::{
    chunk::{Chunk, IntoU8, OpCode, Value},
    heap::Heap,
    object::ObjId,
    tokenizer::{Token, TokenType, Tokenizer},
};

#[allow(unused)]
/// Local variable.
pub struct Local {
    token: Token,
    /// The scope level of local variable.
    ///
    /// It will be `None` if hasn't been initialized yet.
    depth: Option<usize>,
    is_captured: bool,
}

impl Local {
    pub fn new(token: Token, depth: Option<usize>, is_captured: bool) -> Self {
        Self {
            token,
            depth,
            is_captured,
        }
    }
}

pub const MAX_LOCAL_SIZE: usize = 256;

/// The context for storing local variable, function, closure.
pub struct Context {
    /// Stack array stores local variables.
    locals: [Option<Local>; MAX_LOCAL_SIZE],
    /// Amount of local variables.
    local_count: usize,
    /// The depth of current code block.
    scope_depth: usize,
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

impl Context {
    /// Create `Context` with empty local variable array.
    ///
    /// `scope_depth` defaults to 0.
    pub fn new() -> Self {
        Self {
            locals: [const { None }; MAX_LOCAL_SIZE],
            local_count: 0,
            scope_depth: 0,
        }
    }
}

/// Function pointer type for prefix/infix parse functions.
type ParseFn<'src, 'heap> = fn(&mut Parser<'src, 'heap>, bool);

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
            Self::Primary    => Self::Primary,
        }
    }
}

/// The parse rule of Pratt parser.
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
        TokenType::String       => ParseRule::new(Some(Parser::string),   None,                 Precedence::None),
        TokenType::Identifier   => ParseRule::new(Some(Parser::variable), None,                 Precedence::None),
        TokenType::Number       => ParseRule::new(Some(Parser::number),   None,                 Precedence::None),
        TokenType::And          => ParseRule::new(None,                   Some(Parser::and),    Precedence::And),
        TokenType::Or           => ParseRule::new(None,                   Some(Parser::or),     Precedence::Or),
        TokenType::Class        => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::Else         => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::For          => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::Fun          => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::If           => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::Print        => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::Return       => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::Super        => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::This         => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::True         => ParseRule::new(Some(Parser::literal),  None,                 Precedence::None),
        TokenType::False        => ParseRule::new(Some(Parser::literal),  None,                 Precedence::None),
        TokenType::Nil          => ParseRule::new(Some(Parser::literal),  None,                 Precedence::None),
        TokenType::Let          => ParseRule::new(None,                   None,                 Precedence::None),
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
    /// Judge if there had error occurs.
    had_error: bool,
    /// Set true to avoid error cascade.
    panic_mode: bool,
    /// Context to store information of local variable, function and enclosure.
    ctx: Context,
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
            ctx: Context::new(),
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
            let token = str::from_utf8(token.name(self.tokenizer.source())).unwrap();
            print!(" at '{}'", token);
        }
        println!(": {}", msg);
        self.had_error = true;
    }

    pub fn declaration(&mut self) {
        if self.next(TokenType::Let) {
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
        } else {
            self.write_byte(OpCode::Nil);
        }
        // 4. parse end ';'
        self.consume(TokenType::Semicolon, "Expected ';' of expression");
        self.define_variable(global);
    }

    /// Declare a local variable. Return if current context is global.
    pub fn declare_variable(&mut self) {
        // We only declare local variable.
        if self.ctx.scope_depth == 0 {
            return;
        }
        self.add_local(self.prev);
    }

    /// Add a local variable to the local variable list.
    pub fn add_local(&mut self, token: Token) {
        // Temporarily set `is_captured` to false.
        let local = Local::new(token, None, false);
        let idx = self.ctx.local_count;
        if idx >= MAX_LOCAL_SIZE {
            self.error_at_current("Too many local variables.");
        }
        self.ctx.locals[idx] = Some(local);
        self.ctx.local_count += 1;
    }

    pub fn fun_declaration(&self) {
        unimplemented!()
    }

    pub fn statement(&mut self) {
        if self.next(TokenType::Print) {
            self.print_statement();
        } else if self.next(TokenType::LeftBrace) {
            // Blocks are a kind of statement.
            self.begin_scope();
            self.block();
            self.end_scope();
        } else if self.next(TokenType::If) {
            self.if_statement();
        } else if self.next(TokenType::While) {
            self.while_statement();
        } else if self.next(TokenType::For) {
            unimplemented!()
        } else {
            self.expression_statement();
        }
    }

    /// Parse a print statement, which finally emit print operation code.
    pub fn print_statement(&mut self) {
        self.expression();
        self.consume(TokenType::Semicolon, "Expected ';' of expression.");
        self.write_byte(OpCode::Print);
    }

    /// Parse an expression statement which end with `;` character, we finally emit a pop to return a value.
    pub fn expression_statement(&mut self) {
        self.expression();
        self.consume(TokenType::Semicolon, "Expected ';' of expression.");
        self.write_byte(OpCode::Pop);
    }

    /// Parse an if statement.
    pub fn if_statement(&mut self) {
        // 1. parse condition
        self.consume(TokenType::LeftParen, "Expected '(' of condition.");
        self.expression();
        self.consume(TokenType::RightParen, "Expected ')' of condition.");
        // 2. parse code block
        let else_branch = self.write_jump(OpCode::JumpIfFalse);
        // pop the value after judgement of if branch to avoid runtime stack overflow.
        self.write_byte(OpCode::Pop);
        self.statement();
        let if_end = self.write_jump(OpCode::Jump);
        // Program will jump here if condition is false.
        self.patch_jump(else_branch);
        self.write_byte(OpCode::Pop);
        if self.next(TokenType::Else) {
            self.statement();
        }
        // Program will execute statement and jump here if condition is true.
        self.patch_jump(if_end);
    }

    /// Re-write jump offset to given index in byte code chunk.
    pub fn patch_jump(&mut self, idx: usize) {
        let offset = self.chunk.code().len() - idx - 2;
        if offset > u16::MAX as usize {
            panic!("Jump body too large.");
        }
        self.chunk.code_mut()[idx] = (offset >> 8) as u8;
        self.chunk.code_mut()[idx + 1] = offset as u8;
    }

    /// Parse a while statement.
    pub fn while_statement(&mut self) {
        let loop_start = self.chunk.code().len();
        self.consume(TokenType::LeftParen, "Expected '(' of condition.");
        self.expression();
        self.consume(TokenType::RightParen, "Expected ')' of condition.");
        let while_end = self.write_jump(OpCode::JumpIfFalse);
        self.write_byte(OpCode::Pop);
        self.statement();
        self.write_loop(loop_start);
        self.patch_jump(while_end);
    }

    /// Uses after enter a new function scope.
    ///
    /// Call this function after consumed token `{`.
    pub fn begin_scope(&mut self) {
        self.ctx.scope_depth += 1;
    }

    /// Uses before leave a new function scope.
    ///
    /// Call this function after consumed token `}`.
    pub fn end_scope(&mut self) {
        self.ctx.scope_depth -= 1;
        // Pop the local variable from current stack frame.
        while self.ctx.local_count > 0
            && self.ctx.locals[self.ctx.local_count - 1]
                .as_ref()
                // Value will not be `None` because index is ranged between 0 and `local_count` - 1.
                .unwrap()
                .depth
                // Entering the function `end_scope`, all local variable would already been initialized.
                // Which means `depth` will not be `None`.
                .unwrap()
                > self.ctx.scope_depth
        {
            self.write_byte(OpCode::Pop);
            self.ctx.local_count -= 1;
        }
    }

    /// Uses in code block.
    pub fn block(&mut self) {
        while !self.check(TokenType::RightBrace) && !self.check(TokenType::EOF) {
            self.declaration();
        }
        self.consume(TokenType::RightBrace, "Expect '}' after block.");
    }

    pub fn parse_variable(&mut self, err_msg: &str) -> usize {
        self.consume(TokenType::Identifier, err_msg);
        self.declare_variable();
        // Avoid writing data into constant area of chunk if the identifier is local variable.
        // Also avoided in following context: at `define_variable()`.
        if self.ctx.scope_depth > 0 {
            return 0;
        }
        self.identifier_constant(self.prev)
    }

    /// Advance to a identifier indicating an end after a panic error and set `panic_mode` to false.
    ///
    /// Call this function in panic mode to avoid error cascade between the location where the error
    /// occurred and the end identifier.
    pub fn synchronize(&mut self) {
        self.panic_mode = false;
        while !self.check(TokenType::EOF) {
            if self.prev.token_type == TokenType::Semicolon {
                break;
            }
            match self.cur.token_type {
                TokenType::Class
                | TokenType::Fun
                | TokenType::Let
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

    /// Write a byte data to the chunk.
    pub fn write_byte(&mut self, byte: impl IntoU8) {
        self.chunk.write(byte, self.prev.line);
    }

    /// Write two bytes to the chunk.
    /// Used to write opcode with it's immediate operand.
    pub fn write_bytes(&mut self, byte1: impl IntoU8, byte2: impl IntoU8) {
        self.write_byte(byte1);
        self.write_byte(byte2);
    }

    /// Write return operation code to chunk.
    pub fn write_return(&mut self) {
        self.write_byte(OpCode::Return);
    }

    /// Write operation code and constant value index (which from constant area) to the chunk.
    pub fn write_constant(&mut self, constant: Value) {
        let idx = self.chunk.write_constant(constant);
        self.write_bytes(OpCode::Constant, idx);
    }

    /// Write jump operation code.
    ///
    /// Returning the start index of jump offset (occupies two bytes) which should be used in `patch_jump` function.
    pub fn write_jump(&mut self, tt: OpCode) -> usize {
        self.write_byte(tt);
        self.write_byte(0xff);
        self.write_byte(0xff);
        self.chunk.code().len() - 2
    }

    /// Write loop operation code.
    pub fn write_loop(&mut self, start: usize) {
        self.write_byte(OpCode::Loop);
        let offset = self.chunk.code().len() - start + 2;
        if offset > u16::MAX as usize {
            panic!("Loop body too large.");
        }
        self.write_byte(offset >> 8);
        self.write_byte(offset);
    }

    /// Write return operation code to chunk.
    pub fn end_compile(&mut self) {
        self.write_return();
        #[cfg(debug_assertions)]
        if !self.had_error {
            disassemble(&self.chunk, self.heap, "dev");
        }
    }

    /// Add a identifier into the heap and get a object index.
    ///
    /// Return the index in constant area (Which stores the object index).
    pub fn identifier_constant(&mut self, t: Token) -> usize {
        let slice = t.name(self.tokenizer.source());
        let s = std::str::from_utf8(slice).unwrap();
        let obj_idx = self.heap.write_string(s);
        self.chunk
            .write_constant(Value::Object(ObjId::new(obj_idx)))
    }

    /// Define a global variable
    pub fn define_variable(&mut self, global: impl IntoU8) {
        // Avoid emit `DefineGlobal` into code area of chunk if the identifier is local variable.
        // Also avoided in previous context (`var_declaration()`): at `parse_variable()`.
        if self.ctx.scope_depth > 0 {
            self.mark_initialized();
            return;
        }
        self.write_bytes(OpCode::DefineGlobal, global);
    }

    /// Initialize the scope level `depth` of local variable.
    ///
    /// This function should be called after an assignment.
    pub fn mark_initialized(&mut self) {
        let idx = self.ctx.local_count - 1;
        let depth = self.ctx.scope_depth;
        self.ctx.locals[idx].as_mut().unwrap().depth = Some(depth);
    }

    /// The unary operator handling unit of Pratt parser.
    pub fn unary(&mut self, _assignable: bool) {
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
    pub fn binary(&mut self, _assignable: bool) {
        let tt = self.prev.token_type;
        // Save the operator's line number before `parse_precedence` advances `prev`. This ensures the emitted bytecode
        // has the correct line number for the operator, even if the expression spans multiple lines.
        // let line = self.prev.line;
        let rule = get_rule(tt);
        self.parse_precedence(rule.precedence.next());
        #[rustfmt::skip]
        match tt {
            TokenType::Plus         => self.write_byte(OpCode::Add),
            TokenType::Minus        => self.write_byte(OpCode::Subtract),
            TokenType::Star         => self.write_byte(OpCode::Multiply),
            TokenType::Slash        => self.write_byte(OpCode::Divide),
            TokenType::Equal        => self.write_byte(OpCode::Equal),
            TokenType::BangEqual    => self.write_bytes(OpCode::Equal, OpCode::Not),
            TokenType::Less         => self.write_byte(OpCode::Less),
            TokenType::Greater      => self.write_byte(OpCode::Greater),
            TokenType::LessEqual    => self.write_bytes(OpCode::Less, OpCode::Not),
            TokenType::GreaterEqual => self.write_bytes(OpCode::Greater, OpCode::Not),
            TokenType::EqualEqual   => self.write_byte(OpCode::Equal),
            _ => {},
        };
    }

    /// The number handling unit of Pratt parser.
    pub fn number(&mut self, _assignable: bool) {
        let source = self.tokenizer.source();
        let slice = &source[self.prev.start..self.prev.start + self.prev.len];
        let val: f64 = std::str::from_utf8(slice)
            .expect("Number token should be valid UTF-8.")
            .parse()
            .expect("Number token should be a valid float.");
        self.write_constant(Value::Number(val));
    }

    /// The parenthesis handling unit of Pratt parser.
    pub fn grouping(&mut self, _assignable: bool) {
        self.expression();
        self.consume(TokenType::RightParen, "Expected ')' after expression.");
    }

    /// The literal handling unit of Pratt parser.
    pub fn literal(&mut self, _assignable: bool) {
        #[rustfmt::skip]
        match self.prev.token_type {
            TokenType::True   => self.write_byte(OpCode::True),
            TokenType::False  => self.write_byte(OpCode::False),
            TokenType::Nil    => self.write_byte(OpCode::Nil),
            _ => {
                unreachable!()
            }
        };
    }

    /// The string handling unit of Pratt parser.
    pub fn string(&mut self, _assignable: bool) {
        let slice = self.prev.name(self.tokenizer.source());
        let s = std::str::from_utf8(slice).unwrap();
        let obj_idx = self.heap.write_string(s);
        self.write_constant(Value::Object(ObjId::new(obj_idx)));
    }

    /// The variable handling unit of Pratt parser.
    pub fn variable(&mut self, assignable: bool) {
        let token = self.prev;
        let idx;
        let (op_get, op_set) = match self.get_local_idx(&token) {
            Some(local_idx) => {
                idx = local_idx;
                (OpCode::GetLocal, OpCode::SetLocal)
            }
            None => {
                idx = self.identifier_constant(self.prev);
                (OpCode::GetGlobal, OpCode::SetGlobal)
            }
        };
        if assignable && self.next(TokenType::Equal) {
            self.expression();
            self.write_bytes(op_set, idx);
        } else {
            self.write_bytes(op_get, idx);
        }
    }

    /// Get the local variable index according to token name.
    ///
    /// Returning the local idx in `Option` if exists else `None`.
    pub fn get_local_idx(&mut self, token: &Token) -> Option<usize> {
        if self.ctx.scope_depth == 0 {
            return None;
        }
        for (i, v) in self.ctx.locals.iter().enumerate() {
            if let Some(e) = v
                && self.token_cmp(&e.token, token)
            {
                // Only higher level scope are available.
                if let Some(depth) = e.depth
                    && depth <= self.ctx.scope_depth
                {
                    return Some(i);
                }
                return None;
            }
        }
        None
    }

    /// Compare if the name of two tokens are same.
    ///
    /// This comparison method is a byte by byte comparison which will sacrifice some performance.
    pub fn token_cmp(&self, t1: &Token, t2: &Token) -> bool {
        t1.name(self.tokenizer.source()) == t2.name(self.tokenizer.source())
    }

    /// The and handling unit of Pratt parser.
    /// `a and b` equals to `if a { b } else {}`.
    pub fn and(&mut self, _assignable: bool) {
        let if_end = self.write_jump(OpCode::JumpIfFalse);
        self.write_byte(OpCode::Pop);
        self.parse_precedence(Precedence::And);
        self.patch_jump(if_end);
    }

    /// The or handling unit of Pratt parser.
    /// `a or b` equals to `if a { } else { b }`.
    pub fn or(&mut self, _assignable: bool) {
        let else_branch = self.write_jump(OpCode::JumpIfFalse);
        let if_end = self.write_jump(OpCode::Jump);
        self.patch_jump(else_branch);
        self.write_byte(OpCode::Pop);
        self.parse_precedence(Precedence::Or);
        self.patch_jump(if_end);
    }

    /// Parse the precedence of previous token.
    pub fn parse_precedence(&mut self, precedence: Precedence) {
        self.advance();
        if let Some(prefix_rule) = get_rule(self.prev.token_type).prefix {
            // Use assignable flag to skip assignment in `variable` unit.
            let assignable = precedence <= Precedence::Assignment;
            prefix_rule(self, assignable);
            while precedence <= get_rule(self.cur.token_type).precedence {
                self.advance();
                if let Some(infix_rule) = get_rule(self.prev.token_type).infix {
                    infix_rule(self, assignable);
                }
            }
            // Essential error handling for '=' in expression.
            if assignable && self.next(TokenType::Equal) {
                self.error_at_current("Invalid assignment target.");
            }
        } else {
            self.error_at_current("Expected expression.");
        }
    }

    /// Compile an expression without end character `;`.
    pub fn expression(&mut self) {
        // Temporarily use assginment percedence to parse the whole expression.
        self.parse_precedence(Precedence::Assignment);
    }
}
