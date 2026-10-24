use nx_diagnostics::DiagnosticBag;
use nx_lexer::{Lexer, TokenKind};

#[test]
fn test_keywords_and_identifiers() {
    let src =
        "var const tem fn return if otherwise else As Do fr not true false null my_var _ident";
    let mut bag = DiagnosticBag::new();
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize(&mut bag);

    assert!(!bag.has_errors());
    let kinds: Vec<_> = tokens.into_iter().map(|t| t.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::Var,
            TokenKind::Const,
            TokenKind::Tem,
            TokenKind::Fn,
            TokenKind::Return,
            TokenKind::If,
            TokenKind::Otherwise,
            TokenKind::Else,
            TokenKind::As,
            TokenKind::Do,
            TokenKind::Fr,
            TokenKind::Not,
            TokenKind::True,
            TokenKind::False,
            TokenKind::Null,
            TokenKind::Ident("my_var".into()),
            TokenKind::Ident("_ident".into()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn test_oop_keywords() {
    let src = "class interface extends implements public private protected static abstract final self super";
    let mut bag = DiagnosticBag::new();
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize(&mut bag);

    assert!(!bag.has_errors());
    let kinds: Vec<_> = tokens.into_iter().map(|t| t.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::Class,
            TokenKind::Interface,
            TokenKind::Extends,
            TokenKind::Implements,
            TokenKind::Public,
            TokenKind::Private,
            TokenKind::Protected,
            TokenKind::Static,
            TokenKind::Abstract,
            TokenKind::Final,
            TokenKind::Self_,
            TokenKind::Super,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn test_literals() {
    let src = r#"42 8000000000 3.14159 "Hello World\n" 'Z' '\t'"#;
    let mut bag = DiagnosticBag::new();
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize(&mut bag);

    assert!(!bag.has_errors());
    assert_eq!(tokens[0].kind, TokenKind::Int(42));
    assert_eq!(tokens[1].kind, TokenKind::Int(8000000000));
    assert_eq!(tokens[2].kind, TokenKind::Float("3.14159".into()));
    assert_eq!(tokens[3].kind, TokenKind::Str("Hello World\n".into()));
    assert_eq!(tokens[4].kind, TokenKind::Char('Z'));
    assert_eq!(tokens[5].kind, TokenKind::Char('\t'));
}

#[test]
fn test_operators_and_punctuation() {
    let src = "+ - * / % ++ -- = += -= *= /= %= == != < <= > >= | & .. : ; , . ( ) { } [ ]";
    let mut bag = DiagnosticBag::new();
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize(&mut bag);

    assert!(!bag.has_errors());
    let kinds: Vec<_> = tokens.into_iter().map(|t| t.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::Plus,
            TokenKind::Minus,
            TokenKind::Star,
            TokenKind::Slash,
            TokenKind::Percent,
            TokenKind::PlusPlus,
            TokenKind::MinusMinus,
            TokenKind::Eq,
            TokenKind::PlusEq,
            TokenKind::MinusEq,
            TokenKind::StarEq,
            TokenKind::SlashEq,
            TokenKind::PercentEq,
            TokenKind::EqEq,
            TokenKind::BangEq,
            TokenKind::Lt,
            TokenKind::LtEq,
            TokenKind::Gt,
            TokenKind::GtEq,
            TokenKind::Pipe,
            TokenKind::Ampersand,
            TokenKind::DotDot,
            TokenKind::Colon,
            TokenKind::Semicolon,
            TokenKind::Comma,
            TokenKind::Dot,
            TokenKind::OpenParen,
            TokenKind::CloseParen,
            TokenKind::OpenBrace,
            TokenKind::CloseBrace,
            TokenKind::OpenBracket,
            TokenKind::CloseBracket,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn test_comments_and_spans() {
    let src = "# First line comment\nvar x = 1; // Second line comment\n";
    let mut bag = DiagnosticBag::new();
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize(&mut bag);

    assert!(!bag.has_errors());
    assert_eq!(tokens[0].span.start, 0);
    assert_eq!(tokens[0].span.end, 20);
    assert_eq!(tokens[1].kind, TokenKind::Var);
    assert_eq!(tokens[1].span.start, 21);
    assert_eq!(tokens[1].span.end, 24);
}

#[test]
fn test_unclosed_string_error_no_panic() {
    let src = r#"var s = "unclosed string;"#;
    let mut bag = DiagnosticBag::new();
    let mut lexer = Lexer::new(src);
    let _tokens = lexer.tokenize(&mut bag);

    assert!(bag.has_errors());
    let msgs: Vec<_> = bag.diagnostics().iter().map(|d| &d.message).collect();
    assert!(msgs
        .iter()
        .any(|m| m.contains("unterminated string literal")));
}

#[test]
fn test_unclosed_char_error_no_panic() {
    let src = "var c = 'a;\n";
    let mut bag = DiagnosticBag::new();
    let mut lexer = Lexer::new(src);
    let _tokens = lexer.tokenize(&mut bag);

    assert!(bag.has_errors());
    let msgs: Vec<_> = bag.diagnostics().iter().map(|d| &d.message).collect();
    assert!(msgs
        .iter()
        .any(|m| m.contains("unterminated character literal")));
}

#[test]
fn test_phase5_keywords() {
    let src = "match case alloc free mut _";
    let mut bag = DiagnosticBag::new();
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize(&mut bag);

    assert!(!bag.has_errors());
    let kinds: Vec<_> = tokens.into_iter().map(|t| t.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::Match,
            TokenKind::Case,
            TokenKind::Alloc,
            TokenKind::Free,
            TokenKind::Mut,
            TokenKind::Underscore,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn test_phase8_systems_tokens() {
    let src = "unsafe struct extern ^ ~ << >>";
    let mut bag = DiagnosticBag::new();
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize(&mut bag);

    assert!(!bag.has_errors());
    let kinds: Vec<_> = tokens.into_iter().map(|t| t.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::Unsafe,
            TokenKind::Struct,
            TokenKind::Extern,
            TokenKind::Caret,
            TokenKind::Tilde,
            TokenKind::LtLt,
            TokenKind::GtGt,
            TokenKind::Eof,
        ]
    );
}
