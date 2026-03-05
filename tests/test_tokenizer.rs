use lox::tokenizer::{Token, TokenType, Tokenizer};

/// Extract token types from token list.
fn token_types(tokens: &[Token]) -> Vec<TokenType> {
    tokens.iter().map(|t| t.token_type).collect()
}

fn assert_ends_with_eof(tokens: &[Token]) {
    assert_eq!(
        tokens.last().unwrap().token_type,
        TokenType::EOF,
        "The last token should be EOF"
    );
}

#[test]
fn test_empty_source() {
    let mut tokenizer = Tokenizer::new("");
    let tokens = tokenizer.scan_tokens();
    assert_eq!(tokens.len(), 1);
    assert_ends_with_eof(&tokens);
}

#[test]
fn test_single_char_tokens() {
    let mut tokenizer = Tokenizer::new("(){},.-+;*");
    let tokens = tokenizer.scan_tokens();
    assert_eq!(
        token_types(&tokens),
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
            TokenType::EOF,
        ]
    );
}

#[test]
fn test_two_char_tokens() {
    let mut tokenizer = Tokenizer::new("!= <= >=");
    let tokens = tokenizer.scan_tokens();
    assert_eq!(
        token_types(&tokens),
        vec![
            TokenType::BangEqual,
            TokenType::LessEqual,
            TokenType::GreaterEqual,
            TokenType::EOF,
        ]
    );
}

#[test]
fn test_single_char_vs_two_char() {
    let mut tokenizer = Tokenizer::new("! < >");
    let tokens = tokenizer.scan_tokens();
    assert_eq!(
        token_types(&tokens),
        vec![
            TokenType::Bang,
            TokenType::Less,
            TokenType::Greater,
            TokenType::EOF,
        ]
    );
}

#[test]
fn test_slash_token() {
    let mut tokenizer = Tokenizer::new("/");
    let tokens = tokenizer.scan_tokens();
    assert_eq!(token_types(&tokens), vec![TokenType::Slash, TokenType::EOF]);
}

#[test]
fn test_comment_is_ignored_1() {
    // The line starts with '//' should be ignored.
    let mut tokenizer = Tokenizer::new("// this is a comment\n+");
    let tokens = tokenizer.scan_tokens();
    assert_eq!(token_types(&tokens), vec![TokenType::Plus, TokenType::EOF]);
}

#[test]
fn test_comment_is_ignored_2() {
    // The line starts with '//' should be ignored.
    let mut tokenizer = Tokenizer::new("/* this is a comment */+");
    let tokens = tokenizer.scan_tokens();
    assert_eq!(token_types(&tokens), vec![TokenType::Plus, TokenType::EOF]);
}

#[test]
fn test_comment_unclosed() {
    // The line starts with '//' should be ignored.
    let mut tokenizer = Tokenizer::new("/* this is a unclosed comment");
    let tokens = tokenizer.scan_tokens();
    assert_eq!(token_types(&tokens), vec![TokenType::EOF]);
}

#[test]
fn test_whitespace_is_ignored() {
    let mut tokenizer = Tokenizer::new("  +  \t  -  \r  ");
    let tokens = tokenizer.scan_tokens();
    assert_eq!(
        token_types(&tokens),
        vec![TokenType::Plus, TokenType::Minus, TokenType::EOF]
    );
}

#[test]
fn test_newline_increments_line() {
    let mut tokenizer = Tokenizer::new("+\n+\n+");
    let tokens = tokenizer.scan_tokens();
    assert_eq!(tokens[0].line, 1);
    assert_eq!(tokens[1].line, 2);
    assert_eq!(tokens[2].line, 3);
}

#[test]
fn test_token_start_and_len() {
    let mut tokenizer = Tokenizer::new("!=");
    let tokens = tokenizer.scan_tokens();
    let t = tokens[0];
    assert_eq!(t.token_type, TokenType::BangEqual);
    assert_eq!(t.start, 0);
    assert_eq!(t.len, 2);
    assert_eq!(t.line, 1);
}

#[test]
fn test_string() {
    let mut tokenizer = Tokenizer::new("\"Hello, world\n\"");
    let tokens = tokenizer.scan_tokens();
    assert_eq!(
        token_types(&tokens),
        vec![TokenType::String, TokenType::EOF]
    );
    let t = tokens[0];
    assert_eq!(t.start, 1);
    assert_eq!(t.len, 15);
    assert_eq!(t.line, 2);
}
