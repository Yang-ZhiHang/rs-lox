#[cfg(debug_assertions)]
use crate::common::disassemble;
use crate::{
    chunk::{Chunk, IntoU8, OpCode, Value},
    constant::{MAX_LOCAL_SIZE, MAX_UPVALUE_SIZE},
    heap::Heap,
    object::{FunctionType, ObjData, ObjFunction, ObjIndex},
    tokenizer::{Token, TokenType, Tokenizer, token_cmp},
};

/// Local variable.
#[allow(unused)]
#[derive(Clone, Copy)]
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

#[derive(Clone, Copy, Debug)]
pub struct Upvalue {
    idx: usize,
    is_local: bool,
}

impl Upvalue {
    pub fn new(idx: usize, is_local: bool) -> Self {
        Self { idx, is_local }
    }
}

/// The context for storing local variable, function, closure.
#[derive(Clone)]
#[allow(unused)]
pub struct Context {
    /// The context of caller function (current one is callee).
    caller: Option<Box<Context>>,
    /// Use ObjId instead of reference to make the `func` a unsafe reference.
    func_obj_idx: ObjIndex,
    /// The current function type.
    func_type: FunctionType,
    /// Stack array stores local variables.
    locals: [Option<Local>; MAX_LOCAL_SIZE],
    /// Amount of local variables.
    local_count: usize,
    /// Stack array stores upvalues.
    upvalues: [Option<Upvalue>; MAX_LOCAL_SIZE],
    /// The depth of current code block.
    scope_depth: usize,
}

impl Context {
    /// Create `Context` with empty local variable array.
    ///
    /// `scope_depth` defaults to 0.
    pub fn new(heap: &mut Heap, name_idx: ObjIndex, func_type: FunctionType, depth: usize) -> Self {
        // Temporarily set arity to zero. We will update it in `function()` after parsing parameter list.
        let func_obj_idx = heap.write_func(name_idx, 0);
        let mut ctx = Self {
            caller: None,
            func_obj_idx,
            func_type,
            locals: [None; MAX_LOCAL_SIZE],
            local_count: 0,
            upvalues: [None; MAX_UPVALUE_SIZE],
            scope_depth: depth,
        };
        // The slot 0 is used for storing function calling object.
        ctx.locals[0] = Some(Local::new(Token::default(), Some(0), false));
        ctx.local_count += 1;
        ctx
    }
}

/// Function pointer type for prefix/infix parse functions.
type ParseFn<'heap> = fn(&mut Parser<'heap>, bool);

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
pub struct ParseRule<'heap> {
    pub prefix: Option<ParseFn<'heap>>,
    pub infix: Option<ParseFn<'heap>>,
    pub precedence: Precedence,
}

impl<'heap> ParseRule<'heap> {
    fn new(
        prefix: Option<ParseFn<'heap>>,
        infix: Option<ParseFn<'heap>>,
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
/// issues that arise from `Parser` carrying a lifetime parameter.
/// Perf: Maybe we can make `match` to table search according to `TokenType` enum which has a better performance
/// and cache consistency.
#[rustfmt::skip]
pub fn get_rule<'heap>(tt: TokenType) -> ParseRule<'heap> {
    match tt {
        TokenType::LeftParen    => ParseRule::new(Some(Parser::grouping), Some(Parser::call),   Precedence::Call),
        TokenType::RightParen   => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::LeftBrace    => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::RightBrace   => ParseRule::new(None,                   None,                 Precedence::None),

        TokenType::Comma        => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::Dot          => ParseRule::new(None,                   None,                 Precedence::None),

        TokenType::Minus        => ParseRule::new(Some(Parser::unary),    Some(Parser::binary), Precedence::Term),
        TokenType::Plus         => ParseRule::new(None,                   Some(Parser::binary), Precedence::Term),
        TokenType::Star         => ParseRule::new(None,                   Some(Parser::binary), Precedence::Factor),
        TokenType::Slash        => ParseRule::new(None,                   Some(Parser::binary), Precedence::Factor),
        TokenType::MinusEqual   => ParseRule::new(None,                   None,                 Precedence::Term),
        TokenType::PlusEqual    => ParseRule::new(None,                   None,                 Precedence::Term),
        TokenType::MulEqual     => ParseRule::new(None,                   None,                 Precedence::Term),
        TokenType::DivEqual     => ParseRule::new(None,                   None,                 Precedence::Term),

        TokenType::Colon        => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::Semicolon    => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::Bang         => ParseRule::new(Some(Parser::unary),    None,                 Precedence::None),
        TokenType::BangEqual    => ParseRule::new(None,                   Some(Parser::binary), Precedence::Equality),
        TokenType::LessEqual    => ParseRule::new(None,                   Some(Parser::binary), Precedence::Comparison),
        TokenType::GreaterEqual => ParseRule::new(None,                   Some(Parser::binary), Precedence::Comparison),
        TokenType::EqualEqual   => ParseRule::new(None,                   Some(Parser::binary), Precedence::Comparison),
        TokenType::Less         => ParseRule::new(None,                   Some(Parser::binary), Precedence::Comparison),
        TokenType::Greater      => ParseRule::new(None,                   Some(Parser::binary), Precedence::Comparison),
        TokenType::Equal        => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::Let          => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::String       => ParseRule::new(Some(Parser::string),   None,                 Precedence::None),
        TokenType::Identifier   => ParseRule::new(Some(Parser::variable), None,                 Precedence::None),
        TokenType::Number       => ParseRule::new(Some(Parser::number),   None,                 Precedence::None),
        TokenType::And          => ParseRule::new(None,                   Some(Parser::and),    Precedence::And),
        TokenType::Or           => ParseRule::new(None,                   Some(Parser::or),     Precedence::Or),
        TokenType::Print        => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::Return       => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::True         => ParseRule::new(Some(Parser::literal),  None,                 Precedence::None),
        TokenType::False        => ParseRule::new(Some(Parser::literal),  None,                 Precedence::None),
        TokenType::Nil          => ParseRule::new(Some(Parser::literal),  None,                 Precedence::None),
        TokenType::If           => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::Else         => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::While        => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::For          => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::Switch       => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::Case         => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::Default      => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::Fun          => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::Class        => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::Super        => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::This         => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::Error(_)     => ParseRule::new(None,                   None,                 Precedence::None),
        TokenType::EOF          => ParseRule::new(None,                   None,                 Precedence::None),
    }
}

pub enum CompileError {
    SyntaxError,
}

pub struct Parser<'heap> {
    tokenizer: Tokenizer,
    heap: &'heap mut Heap,
    /// Why we need a prev and cur token? Why only two tokens?
    prev: Token,
    cur: Token,
    /// Judge if there had error occurs.
    had_error: bool,
    /// Set true to avoid error cascade.
    panic_mode: bool,
    /// Context to store information of local variable, function and enclosure.
    /// The context will be `None` after the `end_compile` in global scope.
    ctx: Option<Context>,
}

impl<'heap> Parser<'heap> {
    pub fn new(tokenizer: Tokenizer, heap: &'heap mut Heap) -> Self {
        let name_idx = heap.write_string("<Global>");
        let ctx = Context::new(heap, name_idx, FunctionType::Global, 0);
        Self {
            tokenizer,
            heap,
            prev: Token::default(),
            cur: Token::default(),
            had_error: false,
            panic_mode: false,
            ctx: Some(ctx),
        }
    }

    /// Return a immutable reference of context.
    pub fn ctx(&self) -> &Context {
        self.ctx.as_ref().unwrap()
    }

    /// Return a mutable reference of context.
    pub fn ctx_mut(&mut self) -> &mut Context {
        self.ctx.as_mut().unwrap()
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
    pub fn compile(mut self) -> Option<ObjIndex> {
        self.advance();
        while !self.next(TokenType::EOF) {
            self.declaration();
        }
        let func_obj_idx = self.end_compile();
        if self.had_error {
            None
        } else {
            Some(func_obj_idx)
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

    /// Parse a declaration.
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

    /// Parse a variable declaration.
    pub fn var_declaration(&mut self) {
        // 1. parse variable name
        let global = self.parse_variable("Expected variable name.");
        // 2. parse '='
        if self.next(TokenType::Equal) {
            // 3. parse expression
            self.expression();
        } else {
            self.emit_byte(OpCode::Nil);
        }
        // 4. parse end ';'
        self.consume(TokenType::Semicolon, "Expected ';' of expression");
        self.define_variable(global);
    }

    /// Parse a function declaration.
    pub fn fun_declaration(&mut self) {
        let global = self.parse_variable("Expected function name");
        self.mark_initialized();
        self.function(global);
        self.define_variable(global);
    }

    /// Parse a function body.
    pub fn function(&mut self, global: usize) {
        let name_obj_idx = if self.ctx().scope_depth == 0 {
            match self.current_chunk().constants()[global] {
                Value::Object(idx) => idx,
                other => panic!("Expected Object, got {:?}", other),
            }
        } else {
            // Perf: Maybe we can set `name_obj_idx` to `Option` instead of writing string of local function
            // to heap for reducing the usage of memory?
            self.heap.write_string(unsafe {
                std::str::from_utf8_unchecked(self.prev.name(self.tokenizer.source()))
            })
        };
        let ctx = Context::new(
            self.heap,
            name_obj_idx,
            FunctionType::Function,
            self.ctx().scope_depth,
        );
        self.update_ctx(ctx);
        // There doesn't have a corresponding `end_scope`. Because we end Compiler completely when we reach the end 
        // of the function body, there’s no need to close the lingering outermost scope.
        self.begin_scope();
        self.consume(TokenType::LeftParen, "Expected '(' after function name.");
        if !self.check(TokenType::RightParen) {
            let func_obj_idx = self.ctx().func_obj_idx;
            let mut arity = 0;
            loop {
                arity += 1;
                if arity > 255 {
                    self.error_at_current("Can't have more than 255 parameters.");
                }
                let local = self.parse_variable("Expected variable name.");
                self.define_variable(local);
                if !self.next(TokenType::Comma) {
                    break;
                }
            }
            let func: &mut ObjFunction = self.heap.get_func_mut(func_obj_idx);
            func.arity = arity;
        }
        self.consume(TokenType::RightParen, "Expected ')' after parameter list.");
        self.consume(
            TokenType::LeftBrace,
            "Expected '{' after function signature.",
        );
        self.block();
        let upvalues = self.ctx().upvalues;
        let func_obj_idx = self.end_compile();
        self.emit_with_constant_idx(OpCode::Closure, Value::Object(func_obj_idx));
        for v in upvalues.iter().flatten() {
            self.emit_bytes(v.is_local, v.idx);
        }
    }

    /// Update current context according to the passing-in one.
    pub fn update_ctx(&mut self, mut ctx: Context) {
        // Caller will never be `None` unless meets the last `end_compile`.
        let caller_ctx = self.ctx.take().unwrap();
        ctx.caller = Some(Box::new(caller_ctx));
        self.ctx = Some(ctx);
    }

    /// Declare a local variable. Return if current context is global.
    pub fn declare_variable(&mut self) {
        // We only declare local variable.
        if self.ctx().scope_depth == 0 {
            return;
        }
        self.add_local(self.prev);
    }

    /// Add a local variable to the local variable stack.
    pub fn add_local(&mut self, t: Token) {
        // Temporarily set `is_captured` to false.
        let local = Local::new(t, None, false);
        let idx = self.ctx().local_count;
        if idx >= MAX_LOCAL_SIZE {
            self.error_at_current("Too many local variables.");
            return;
        }
        self.ctx_mut().locals[idx] = Some(local);
        self.ctx_mut().local_count += 1;
    }

    /// Add a upvalue to the upvalue stack.
    pub fn add_upvalue(&mut self, idx: usize, is_local: bool) -> usize {
        let upvalue = Upvalue::new(idx, is_local);
        // TODO: Max exceed Error handling?
        // Search existing upvalues first.
        let upvalues = &self.ctx().upvalues;
        for (i, v) in upvalues.iter().flatten().enumerate() {
            if v.idx == idx && v.is_local == is_local {
                return i;
            }
        }
        let func_obj_idx = self.ctx().func_obj_idx;
        let func = self.heap.get_func_mut(func_obj_idx);
        let upvalue_count = func.upvalues_count;
        func.upvalues_count += 1;
        self.ctx_mut().upvalues[upvalue_count] = Some(upvalue);
        upvalue_count
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
            self.for_statement();
        } else if self.next(TokenType::Switch) {
            self.switch_statement();
        } else if self.next(TokenType::Return) {
            self.return_statement();
        } else {
            self.expression_statement();
        }
    }

    /// Parse a print statement, which finally emit print operation code.
    pub fn print_statement(&mut self) {
        self.expression();
        self.consume(TokenType::Semicolon, "Expected ';' of expression.");
        self.emit_byte(OpCode::Print);
    }

    /// Parse an expression statement which end with `;` character, we finally emit a pop to return a value.
    pub fn expression_statement(&mut self) {
        self.expression();
        self.consume(TokenType::Semicolon, "Expected ';' of expression.");
        self.emit_byte(OpCode::Pop);
    }

    /// Parse an if statement.
    pub fn if_statement(&mut self) {
        // 1. parse condition
        // self.consume(TokenType::LeftParen, "Expected '(' of condition.");
        self.expression();
        // self.consume(TokenType::RightParen, "Expected ')' of condition.");
        // 2. parse code block
        let else_branch = self.emit_jump(OpCode::JumpIfFalse);
        // pop the value after judgement of if branch to avoid runtime stack overflow.
        self.emit_byte(OpCode::Pop);
        self.statement();
        let if_end = self.emit_jump(OpCode::Jump);
        // Program will jump here if condition is false.
        self.patch_jump(else_branch);
        self.emit_byte(OpCode::Pop);
        if self.next(TokenType::Else) {
            self.statement();
        }
        // Program will execute statement and jump here if condition is true.
        self.patch_jump(if_end);
    }

    /// Parse a while statement.
    pub fn while_statement(&mut self) {
        let loop_start = self.current_chunk().code().len();
        // self.consume(TokenType::LeftParen, "Expected '(' of condition.");
        self.expression();
        // self.consume(TokenType::RightParen, "Expected ')' of condition.");
        let while_end = self.emit_jump(OpCode::JumpIfFalse);
        self.emit_byte(OpCode::Pop);
        self.statement();
        self.emit_loop(loop_start);
        self.patch_jump(while_end);
    }

    /// Parse a for statement.
    pub fn for_statement(&mut self) {
        self.begin_scope();
        self.consume(TokenType::LeftParen, "Expected '(' after 'for' keyword.");
        if self.next(TokenType::Semicolon) {
            // No initializer.
        } else if self.next(TokenType::Let) {
            self.var_declaration();
        } else {
            // Such as assignment clauses.
            self.expression_statement();
        }
        let mut loop_start = self.current_chunk().code().len();
        let mut exit_jump = None;
        if !self.next(TokenType::Semicolon) {
            self.expression();
            self.consume(TokenType::Semicolon, "Expected ';' after loop condition.");
            exit_jump = Some(self.emit_jump(OpCode::JumpIfFalse));
            self.emit_byte(OpCode::Pop);
        }
        if !self.next(TokenType::RightParen) {
            // for body should execute before increase clause.
            let body_jump = self.emit_jump(OpCode::Jump);
            let incr_clause = self.current_chunk().code().len();
            self.expression();
            self.emit_byte(OpCode::Pop);
            self.consume(TokenType::RightParen, "Expected ')' after increase clause.");
            self.emit_loop(loop_start);
            // reuse of `loop_start`. In the following context, it means jump offset to increase clause.
            loop_start = incr_clause;
            self.patch_jump(body_jump);
        }
        self.statement();
        self.emit_loop(loop_start);
        if let Some(offset) = exit_jump {
            self.patch_jump(offset);
            self.emit_byte(OpCode::Pop);
        }
        self.end_scope();
    }

    /// Parse a switch statement.
    pub fn switch_statement(&mut self) {
        self.begin_scope();
        self.consume(TokenType::Identifier, "Expected a variable.");
        let t = self.prev;
        self.consume(TokenType::LeftBrace, "Expected '{' after variable clause.");
        // start case clause.
        let mut next_case = None;
        let mut jump_end_list = vec![];
        while !self.check(TokenType::Default) {
            if let Some(start_idx) = next_case {
                self.patch_jump(start_idx);
                self.emit_byte(OpCode::Pop);
            };
            self.consume(TokenType::Case, "Expected 'case' caluse.");
            self.consume(TokenType::Number, "Expected a constant value.");
            self.number(false);
            self.named_variable(false, &t);
            self.consume(TokenType::Colon, "Expected ':' after case clause.");
            self.emit_byte(OpCode::Equal);
            next_case = Some(self.emit_jump(OpCode::JumpIfFalse));
            self.emit_byte(OpCode::Pop);
            while !self.check(TokenType::Case) && !self.check(TokenType::Default) {
                self.declaration();
            }
            jump_end_list.push(self.emit_jump(OpCode::Jump));
        }
        self.consume(TokenType::Default, "Expected 'default' caluse.");
        self.consume(TokenType::Colon, "Expected ':' after default clause.");
        if let Some(idx) = next_case {
            self.patch_jump(idx);
        };
        self.declaration();
        // end case clause.
        for start_idx in jump_end_list {
            self.patch_jump(start_idx);
        }
        self.consume(TokenType::RightBrace, "Expected '}' after variable clause.");
        self.end_scope();
    }

    /// Parse a return statement.
    pub fn return_statement(&mut self) {
        if self.ctx().func_type == FunctionType::Global {
            self.error_at(self.prev, "Can't return from top-level scope.");
            return;
        }
        if self.next(TokenType::Semicolon) {
            self.emit_return();
        } else {
            self.expression();
            self.consume(TokenType::Semicolon, "Expected ';'.");
            self.emit_byte(OpCode::Return);
        }
    }

    /// Re-write jump offset to given index in byte code chunk.
    pub fn patch_jump(&mut self, from: usize) {
        let offset = self.current_chunk().code().len() - from - 2;
        if offset > u16::MAX as usize {
            panic!("Jump body too large.");
        }
        self.current_chunk_mut().code_mut()[from] = (offset >> 8) as u8;
        self.current_chunk_mut().code_mut()[from + 1] = offset as u8;
    }

    /// Uses after enter a new function scope.
    ///
    /// Call this function after consumed token `{`.
    pub fn begin_scope(&mut self) {
        self.ctx_mut().scope_depth += 1;
    }

    /// Uses before leave a new function scope.
    ///
    /// Call this function after consumed token `}`.
    pub fn end_scope(&mut self) {
        self.ctx_mut().scope_depth -= 1;
        // Pop the local variable from current stack frame.
        while self.ctx().local_count > 0
            && self.ctx().locals[self.ctx().local_count - 1]
                .as_ref()
                // Value will not be `None` because index is ranged between 0 and `local_count` - 1.
                .unwrap()
                .depth
                // Entering the function `end_scope`, all local variable would already been initialized.
                // Which means `depth` will not be `None`.
                .unwrap()
                > self.ctx().scope_depth
        {
            if self.ctx().locals[self.ctx().local_count - 1]
                .as_ref()
                .unwrap()
                .is_captured
            {
                self.emit_byte(OpCode::CloseUpvalue);
            }
            self.emit_byte(OpCode::Pop);
            self.ctx_mut().local_count -= 1;
        }
    }

    /// Parse a code block.
    ///
    /// Call this function after consuming left brace.
    /// It will consume right brace.
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
        if self.ctx().scope_depth > 0 {
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
    pub fn emit_byte(&mut self, byte: impl IntoU8) {
        let line = self.prev.line;
        self.current_chunk_mut().write(byte, line);
    }

    /// Write two bytes to the chunk.
    /// Used to write opcode with it's immediate operand.
    pub fn emit_bytes(&mut self, byte1: impl IntoU8, byte2: impl IntoU8) {
        self.emit_byte(byte1);
        self.emit_byte(byte2);
    }

    /// Write return operation code to chunk.
    pub fn emit_return(&mut self) {
        self.emit_byte(OpCode::Nil);
        self.emit_byte(OpCode::Return);
    }

    /// Write operation code and constant value index (which from constant area) to the current chunk.
    pub fn emit_with_constant_idx(&mut self, op: OpCode, constant: Value) {
        let idx = self.current_chunk_mut().write_constant(constant);
        self.emit_bytes(op, idx);
    }

    /// Write jump operation code.
    ///
    /// Returning the start index of jump offset (occupies two bytes) which should be used in `patch_jump` function.
    pub fn emit_jump(&mut self, op: OpCode) -> usize {
        self.emit_byte(op);
        self.emit_byte(0xff);
        self.emit_byte(0xff);
        self.current_chunk().code().len() - 2
    }

    /// Write loop operation code.
    pub fn emit_loop(&mut self, start: usize) {
        self.emit_byte(OpCode::Loop);
        let offset = self.current_chunk().code().len() - start + 2;
        if offset > u16::MAX as usize {
            panic!("Loop body too large.");
        }
        self.emit_byte(offset >> 8);
        self.emit_byte(offset);
    }

    /// Write `Opcode::Return` to chunk and back to caller context.
    ///
    /// Returning the function object index of callee.
    pub fn end_compile(&mut self) -> ObjIndex {
        self.emit_return();
        #[cfg(debug_assertions)]
        if !self.had_error {
            let func = self.heap.get_func(self.ctx().func_obj_idx);
            let func_name = self.heap.get_string(func.name);
            disassemble(self.current_chunk(), self.heap, &func_name.value);
        }
        let func_obj_idx = self.ctx().func_obj_idx;
        let current_ctx = self.ctx.take().unwrap();
        match current_ctx.caller {
            Some(caller_ctx) => self.ctx = Some(*caller_ctx),
            None => self.ctx = None,
        }
        func_obj_idx
    }

    /// Return immutable reference of byte chunk of current function context.
    ///
    /// If the current context is function, it will return the byte chunk of function else global one.
    pub fn current_chunk(&self) -> &Chunk {
        if let ObjData::Function(obj_func) = self.heap.get(self.ctx().func_obj_idx) {
            &obj_func.chunk
        } else {
            unimplemented!()
        }
    }

    /// Return mutable reference of byte chunk of current function context.
    ///
    /// If the current context is function, it will return the byte chunk of function else global one.
    pub fn current_chunk_mut(&mut self) -> &mut Chunk {
        let func_obj_idx = self.ctx().func_obj_idx;
        if let ObjData::Function(obj_func) = self.heap.get_mut(func_obj_idx) {
            &mut obj_func.chunk
        } else {
            unimplemented!()
        }
    }

    /// Add a identifier into the heap and get a object index.
    ///
    /// Return the index in constant area (Which stores the object index).
    pub fn identifier_constant(&mut self, t: Token) -> usize {
        let slice = t.name(self.tokenizer.source());
        let s = std::str::from_utf8(slice).unwrap();
        let obj_idx = self.heap.write_string(s);
        self.current_chunk_mut()
            .write_constant(Value::Object(obj_idx))
    }

    /// Define a global variable
    pub fn define_variable(&mut self, global: impl IntoU8) {
        // Avoid emit `DefineGlobal` into code area of chunk if the identifier is local variable.
        // Also avoided in previous context (`var_declaration()`): at `parse_variable()`.
        if self.ctx().scope_depth > 0 {
            self.mark_initialized();
            return;
        }
        self.emit_bytes(OpCode::DefineGlobal, global);
    }

    /// Initialize the scope level `depth` of local variable.
    ///
    /// - Local variable: this function should be called after an initializer.
    /// - function: Just like variable, we will bind the function to global hash table if it's in the global scope.
    ///   Else mark initialized before body parsed to support recursion.
    pub fn mark_initialized(&mut self) {
        // Mark is unecessary at global scope.
        if self.ctx().scope_depth == 0 {
            return;
        }
        let idx = self.ctx().local_count - 1;
        let depth = self.ctx().scope_depth;
        self.ctx_mut().locals[idx].as_mut().unwrap().depth = Some(depth);
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
                self.current_chunk_mut().write(OpCode::Negate, line);
            }
            TokenType::Bang => {
                self.current_chunk_mut().write(OpCode::Not, line);
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
            TokenType::Plus         => self.emit_byte(OpCode::Add),
            TokenType::Minus        => self.emit_byte(OpCode::Sub),
            TokenType::Star         => self.emit_byte(OpCode::Mul),
            TokenType::Slash        => self.emit_byte(OpCode::Div),
            TokenType::BangEqual    => self.emit_bytes(OpCode::Equal, OpCode::Not),
            TokenType::Less         => self.emit_byte(OpCode::Less),
            TokenType::Greater      => self.emit_byte(OpCode::Greater),
            TokenType::LessEqual    => self.emit_bytes(OpCode::Greater, OpCode::Not),
            TokenType::GreaterEqual => self.emit_bytes(OpCode::Less, OpCode::Not),
            TokenType::EqualEqual   => self.emit_byte(OpCode::Equal),
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
        self.emit_with_constant_idx(OpCode::Constant, Value::Number(val));
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
            TokenType::True   => self.emit_byte(OpCode::True),
            TokenType::False  => self.emit_byte(OpCode::False),
            TokenType::Nil    => self.emit_byte(OpCode::Nil),
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
        self.emit_with_constant_idx(OpCode::Constant, Value::Object(obj_idx));
    }

    /// The variable handling unit of Pratt parser.
    pub fn variable(&mut self, assignable: bool) {
        let t = self.prev;
        // Extract the code block into `named_variable` to reuse it at switch clause.
        self.named_variable(assignable, &t);
    }

    /// Judge if the variable is local variable or global and emit operation code.
    pub fn named_variable(&mut self, assignable: bool, t: &Token) {
        let idx;
        let (op_get, op_set) = match self.get_local_idx(t) {
            Some(local_idx) => {
                idx = local_idx;
                (OpCode::GetLocal, OpCode::SetLocal)
            }
            None => match self.get_upvalue_idx(t) {
                Some(upvalue_idx) => {
                    idx = upvalue_idx;
                    (OpCode::GetUpvalue, OpCode::SetUpvalue)
                }
                None => {
                    idx = self.identifier_constant(*t);
                    (OpCode::GetGlobal, OpCode::SetGlobal)
                }
            },
        };
        if assignable && self.next(TokenType::Equal) {
            self.expression();
            self.emit_bytes(op_set, idx);
        } else if assignable && self.next(TokenType::PlusEqual) {
            self.emit_bytes(op_get, idx);
            self.expression();
            self.emit_byte(OpCode::Add);
            self.emit_bytes(op_set, idx);
        } else if assignable && self.next(TokenType::MinusEqual) {
            self.emit_bytes(op_get, idx);
            self.expression();
            self.emit_byte(OpCode::Sub);
            self.emit_bytes(op_set, idx);
        } else if assignable && self.next(TokenType::MulEqual) {
            self.emit_bytes(op_get, idx);
            self.expression();
            self.emit_byte(OpCode::Mul);
            self.emit_bytes(op_set, idx);
        } else if assignable && self.next(TokenType::DivEqual) {
            self.emit_bytes(op_get, idx);
            self.expression();
            self.emit_byte(OpCode::Div);
            self.emit_bytes(op_set, idx);
        } else {
            self.emit_bytes(op_get, idx);
        }
    }

    /// Get the local variable index according to token name.
    ///
    /// Returning the local index in `Option` if exists else `None`.
    pub fn get_local_idx(&mut self, t: &Token) -> Option<usize> {
        if self.ctx().scope_depth == 0 {
            return None;
        }
        for (i, v) in self.ctx().locals.iter().flatten().enumerate() {
            if token_cmp(&v.token, t, self.tokenizer.source()) {
                // The variale ought to be initialized. (`depth` is not `None`)
                if let Some(depth) = v.depth
                    // Only higher level scope are available.
                    && depth <= self.ctx().scope_depth
                {
                    return Some(i);
                }
                return None;
            }
        }
        None
    }

    /// Get the upvalue index according to token name and current context.
    ///
    /// Returning the upvalue index in `Option` if exists else `None`.
    pub fn get_upvalue_idx(&mut self, t: &Token) -> Option<usize> {
        let mut ctx = self.ctx.as_mut().unwrap();
        let depth = ctx.scope_depth;
        loop {
            // Return `None` if we recursively reach the global scope.
            ctx.caller.as_ref()?;
            let caller = ctx.caller.as_mut().unwrap();
            let locals = &mut caller.locals;
            for (i, v) in locals.iter().flatten().enumerate().skip(1) {
                if token_cmp(&v.token, t, self.tokenizer.source()) && v.depth.is_some() {
                    // Mark local variable as captured by closure.
                    locals[i].as_mut().unwrap().is_captured = true;
                    // Judge if the upvalue is from the direct caller.
                    let is_local = caller.scope_depth == depth - 1;
                    let upvalue_idx = self.add_upvalue(i, is_local);
                    return Some(upvalue_idx);
                }
            }
            ctx = caller.as_mut();
        }
    }

    /// The call handling unit of Pratt parser.
    pub fn call(&mut self, _assignable: bool) {
        let arg_count = self.arg_list();
        self.emit_bytes(OpCode::Call, arg_count);
    }

    /// Parse the argument list when calling function and return the argument count.
    pub fn arg_list(&mut self) -> usize {
        let mut arg_count = 0;
        if !self.check(TokenType::RightParen) {
            loop {
                self.expression();
                arg_count += 1;
                if arg_count > 255 {
                    self.error_at_current("Can't have more than 255 parameters.");
                }
                if !self.next(TokenType::Comma) {
                    break;
                }
            }
        }
        self.consume(TokenType::RightParen, "Missing ')'.");
        arg_count
    }

    /// The and handling unit of Pratt parser.
    /// `a and b` equals to `if a { b } else {}`.
    pub fn and(&mut self, _assignable: bool) {
        let if_end = self.emit_jump(OpCode::JumpIfFalse);
        self.emit_byte(OpCode::Pop);
        self.parse_precedence(Precedence::And);
        self.patch_jump(if_end);
    }

    /// The or handling unit of Pratt parser.
    /// `a or b` equals to `if a { } else { b }`.
    pub fn or(&mut self, _assignable: bool) {
        let else_branch = self.emit_jump(OpCode::JumpIfFalse);
        let if_end = self.emit_jump(OpCode::Jump);
        self.patch_jump(else_branch);
        self.emit_byte(OpCode::Pop);
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
            self.error_at(self.prev, "Invalid expression.");
        }
    }

    /// Compile an expression without end character `;`.
    pub fn expression(&mut self) {
        // Temporarily use assginment percedence to parse the whole expression.
        self.parse_precedence(Precedence::Assignment);
    }
}
