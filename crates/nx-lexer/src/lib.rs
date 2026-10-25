use nx_diagnostics::{DiagnosticBag, Span};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    // Keywords
    Var,
    Const,
    Tem,
    Fn,
    Return,
    If,
    Otherwise,
    Else,
    As,
    Do,
    Fr,
    Not,
    True,
    False,
    Null,

    // OOP Keywords
    Class,
    Interface,
    Extends,
    Implements,
    Public,
    Private,
    Protected,
    Static,
    Abstract,
    Final,
    Self_,
    Super,

    // Phase 5: Pattern Matching & Ownership
    Match,
    Case,
    Alloc,
    Free,
    Mut,
    Underscore,

    // Phase 8: Systems Programming & Low-Level Control
    Unsafe,
    Struct,
    Extern,

    // Phase 9: Standard Library, Modules & Testing
    Import,
    Test,

    // Identifiers and Literals
    Ident(String),
    Int(i128),
    Float(String),
    Str(String),
    Char(char),

    // Operators
    Plus,       // +
    Minus,      // -
    Star,       // *
    Slash,      // /
    Percent,    // %
    PlusPlus,   // ++
    MinusMinus, // --
    Eq,         // =
    PlusEq,     // +=
    MinusEq,    // -=
    StarEq,     // *=
    SlashEq,    // /=
    PercentEq,  // %=
    EqEq,       // ==
    BangEq,     // !=
    Lt,         // <
    LtEq,       // <=
    Gt,         // >
    GtEq,       // >=
    Pipe,       // |
    Ampersand,  // &
    Caret,      // ^
    Tilde,      // ~
    LtLt,       // <<
    GtGt,       // >>
    DotDot,     // ..

    // Punctuation
    Colon,        // :
    Semicolon,    // ;
    Comma,        // ,
    Dot,          // .
    OpenParen,    // (
    CloseParen,   // )
    OpenBrace,    // {
    CloseBrace,   // }
    OpenBracket,  // [
    CloseBracket, // ]

    // Comments & Special
    Comment(String),
    Eof,
}

impl TokenKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            TokenKind::Var => "var",
            TokenKind::Const => "const",
            TokenKind::Tem => "tem",
            TokenKind::Fn => "fn",
            TokenKind::Return => "return",
            TokenKind::If => "if",
            TokenKind::Otherwise => "otherwise",
            TokenKind::Else => "else",
            TokenKind::As => "As",
            TokenKind::Do => "Do",
            TokenKind::Fr => "fr",
            TokenKind::Not => "not",
            TokenKind::True => "true",
            TokenKind::False => "false",
            TokenKind::Null => "null",
            TokenKind::Class => "class",
            TokenKind::Interface => "interface",
            TokenKind::Extends => "extends",
            TokenKind::Implements => "implements",
            TokenKind::Public => "public",
            TokenKind::Private => "private",
            TokenKind::Protected => "protected",
            TokenKind::Static => "static",
            TokenKind::Abstract => "abstract",
            TokenKind::Final => "final",
            TokenKind::Self_ => "self",
            TokenKind::Super => "super",
            TokenKind::Match => "match",
            TokenKind::Case => "case",
            TokenKind::Alloc => "alloc",
            TokenKind::Free => "free",
            TokenKind::Mut => "mut",
            TokenKind::Underscore => "_",
            TokenKind::Unsafe => "unsafe",
            TokenKind::Struct => "struct",
            TokenKind::Extern => "extern",
            TokenKind::Import => "import",
            TokenKind::Test => "test",
            TokenKind::Ident(_) => "identifier",
            TokenKind::Int(_) => "integer",
            TokenKind::Float(_) => "float",
            TokenKind::Str(_) => "string",
            TokenKind::Char(_) => "char",
            TokenKind::Plus => "+",
            TokenKind::Minus => "-",
            TokenKind::Star => "*",
            TokenKind::Slash => "/",
            TokenKind::Percent => "%",
            TokenKind::PlusPlus => "++",
            TokenKind::MinusMinus => "--",
            TokenKind::Eq => "=",
            TokenKind::PlusEq => "+=",
            TokenKind::MinusEq => "-=",
            TokenKind::StarEq => "*=",
            TokenKind::SlashEq => "/=",
            TokenKind::PercentEq => "%=",
            TokenKind::EqEq => "==",
            TokenKind::BangEq => "!=",
            TokenKind::Lt => "<",
            TokenKind::LtEq => "<=",
            TokenKind::Gt => ">",
            TokenKind::GtEq => ">=",
            TokenKind::Pipe => "|",
            TokenKind::Ampersand => "&",
            TokenKind::Caret => "^",
            TokenKind::Tilde => "~",
            TokenKind::LtLt => "<<",
            TokenKind::GtGt => ">>",
            TokenKind::DotDot => "..",
            TokenKind::Colon => ":",
            TokenKind::Semicolon => ";",
            TokenKind::Comma => ",",
            TokenKind::Dot => ".",
            TokenKind::OpenParen => "(",
            TokenKind::CloseParen => ")",
            TokenKind::OpenBrace => "{",
            TokenKind::CloseBrace => "}",
            TokenKind::OpenBracket => "[",
            TokenKind::CloseBracket => "]",
            TokenKind::Comment(_) => "comment",
            TokenKind::Eof => "<EOF>",
        }
    }
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Ident(s) => write!(f, "identifier '{}'", s),
            TokenKind::Int(n) => write!(f, "integer '{}'", n),
            TokenKind::Float(s) => write!(f, "float '{}'", s),
            TokenKind::Str(s) => write!(f, "string \"{}\"", s),
            TokenKind::Char(c) => write!(f, "char '{}'", c),
            TokenKind::Comment(c) => write!(f, "comment '{}'", c),
            _ => write!(f, "'{}'", self.as_str()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    pub text: String,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span, text: impl Into<String>) -> Self {
        Self {
            kind,
            span,
            text: text.into(),
        }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:<16} {:<20} [{}]",
            self.kind.as_str(),
            format!("{:?}", self.text),
            self.span
        )
    }
}

pub struct Lexer<'a> {
    source: &'a str,
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            bytes: source.as_bytes(),
            cursor: 0,
        }
    }

    pub fn tokenize(&mut self, diagnostics: &mut DiagnosticBag) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token(diagnostics);
            let is_eof = token.kind == TokenKind::Eof;
            tokens.push(token);
            if is_eof {
                break;
            }
        }
        tokens
    }

    fn peek(&self) -> Option<u8> {
        if self.cursor < self.bytes.len() {
            Some(self.bytes[self.cursor])
        } else {
            None
        }
    }

    fn peek_next(&self) -> Option<u8> {
        if self.cursor + 1 < self.bytes.len() {
            Some(self.bytes[self.cursor + 1])
        } else {
            None
        }
    }

    fn advance(&mut self) -> Option<u8> {
        if self.cursor < self.bytes.len() {
            let b = self.bytes[self.cursor];
            self.cursor += 1;
            Some(b)
        } else {
            None
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(b) = self.peek() {
            if b.is_ascii_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    pub fn next_token(&mut self, diagnostics: &mut DiagnosticBag) -> Token {
        loop {
            self.skip_whitespace();

            let start = self.cursor;
            let Some(b) = self.advance() else {
                return Token::new(TokenKind::Eof, Span::empty(self.cursor), "");
            };

            // Comments
            if b == b'#' || (b == b'/' && self.peek() == Some(b'/')) {
                if b == b'/' {
                    self.advance(); // consume second '/'
                }
                while let Some(c) = self.peek() {
                    if c == b'\n' {
                        break;
                    }
                    self.advance();
                }
                let span = Span::new(start, self.cursor);
                let text = &self.source[start..self.cursor];
                return Token::new(TokenKind::Comment(text.to_string()), span, text);
            }

            // Identifiers / Keywords
            if b.is_ascii_alphabetic() || b == b'_' {
                while let Some(c) = self.peek() {
                    if c.is_ascii_alphanumeric() || c == b'_' {
                        self.advance();
                    } else {
                        break;
                    }
                }
                let span = Span::new(start, self.cursor);
                let ident_str = &self.source[start..self.cursor];
                let kind = match ident_str {
                    "var" => TokenKind::Var,
                    "const" => TokenKind::Const,
                    "tem" => TokenKind::Tem,
                    "fn" => TokenKind::Fn,
                    "return" => TokenKind::Return,
                    "if" => TokenKind::If,
                    "otherwise" => TokenKind::Otherwise,
                    "else" => TokenKind::Else,
                    "As" => TokenKind::As,
                    "Do" => TokenKind::Do,
                    "fr" => TokenKind::Fr,
                    "not" => TokenKind::Not,
                    "true" => TokenKind::True,
                    "false" => TokenKind::False,
                    "null" => TokenKind::Null,
                    "class" => TokenKind::Class,
                    "interface" => TokenKind::Interface,
                    "extends" => TokenKind::Extends,
                    "implements" => TokenKind::Implements,
                    "public" => TokenKind::Public,
                    "private" => TokenKind::Private,
                    "protected" => TokenKind::Protected,
                    "static" => TokenKind::Static,
                    "abstract" => TokenKind::Abstract,
                    "final" => TokenKind::Final,
                    "self" => TokenKind::Self_,
                    "super" => TokenKind::Super,
                    "match" => TokenKind::Match,
                    "case" => TokenKind::Case,
                    "alloc" => TokenKind::Alloc,
                    "free" => TokenKind::Free,
                    "mut" => TokenKind::Mut,
                    "_" => TokenKind::Underscore,
                    "unsafe" => TokenKind::Unsafe,
                    "struct" => TokenKind::Struct,
                    "extern" => TokenKind::Extern,
                    "import" => TokenKind::Import,
                    "test" => TokenKind::Test,
                    _ => TokenKind::Ident(ident_str.to_string()),
                };
                return Token::new(kind, span, ident_str);
            }

            // Numbers
            if b.is_ascii_digit() {
                // Hexadecimal literal 0x...
                if b == b'0' && (self.peek() == Some(b'x') || self.peek() == Some(b'X')) {
                    self.advance();
                    while let Some(c) = self.peek() {
                        if c.is_ascii_hexdigit() || c == b'_' {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    let span = Span::new(start, self.cursor);
                    let raw = &self.source[start + 2..self.cursor];
                    let cleaned = raw.replace('_', "");
                    match i128::from_str_radix(&cleaned, 16) {
                        Ok(val) => {
                            return Token::new(
                                TokenKind::Int(val),
                                span,
                                &self.source[start..self.cursor],
                            );
                        }
                        Err(_) => {
                            diagnostics
                                .error(span, "hexadecimal integer literal is invalid".to_string());
                            return Token::new(
                                TokenKind::Int(0),
                                span,
                                &self.source[start..self.cursor],
                            );
                        }
                    }
                }

                // Binary literal 0b...
                if b == b'0' && (self.peek() == Some(b'b') || self.peek() == Some(b'B')) {
                    self.advance();
                    while let Some(c) = self.peek() {
                        if c == b'0' || c == b'1' || c == b'_' {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    let span = Span::new(start, self.cursor);
                    let raw = &self.source[start + 2..self.cursor];
                    let cleaned = raw.replace('_', "");
                    match i128::from_str_radix(&cleaned, 2) {
                        Ok(val) => {
                            return Token::new(
                                TokenKind::Int(val),
                                span,
                                &self.source[start..self.cursor],
                            );
                        }
                        Err(_) => {
                            diagnostics
                                .error(span, "binary integer literal is invalid".to_string());
                            return Token::new(
                                TokenKind::Int(0),
                                span,
                                &self.source[start..self.cursor],
                            );
                        }
                    }
                }

                let mut has_dot = false;
                while let Some(c) = self.peek() {
                    if c.is_ascii_digit() || c == b'_' {
                        self.advance();
                    } else if c == b'.' && !has_dot {
                        // Check if following char is a digit (float) or not (range / member access)
                        if self.peek_next().is_some_and(|next| next.is_ascii_digit()) {
                            has_dot = true;
                            self.advance(); // consume '.'
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                // Check for exponent (e.g. 1e10 or 1.5e-3)
                if has_dot && (self.peek() == Some(b'e') || self.peek() == Some(b'E')) {
                    self.advance();
                    if self.peek() == Some(b'+') || self.peek() == Some(b'-') {
                        self.advance();
                    }
                    while let Some(c) = self.peek() {
                        if c.is_ascii_digit() {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }

                let span = Span::new(start, self.cursor);
                let num_str = &self.source[start..self.cursor];
                if has_dot {
                    return Token::new(TokenKind::Float(num_str.to_string()), span, num_str);
                } else {
                    let cleaned = num_str.replace('_', "");
                    match cleaned.parse::<i128>() {
                        Ok(val) => return Token::new(TokenKind::Int(val), span, num_str),
                        Err(_) => {
                            diagnostics.error(
                                span,
                                format!("integer literal '{}' is out of range", num_str),
                            );
                            return Token::new(TokenKind::Int(0), span, num_str);
                        }
                    }
                }
            }

            // String literals
            if b == b'"' {
                let mut string_val = String::new();
                let mut closed = false;
                while let Some(c) = self.advance() {
                    if c == b'"' {
                        closed = true;
                        break;
                    }
                    if c == b'\\' {
                        if let Some(esc) = self.advance() {
                            match esc {
                                b'n' => string_val.push('\n'),
                                b't' => string_val.push('\t'),
                                b'r' => string_val.push('\r'),
                                b'0' => string_val.push('\0'),
                                b'\\' => string_val.push('\\'),
                                b'"' => string_val.push('"'),
                                b'\'' => string_val.push('\''),
                                other => {
                                    diagnostics.warning(
                                        Span::new(self.cursor - 2, self.cursor),
                                        format!("unknown escape sequence '\\{}'", other as char),
                                    );
                                    string_val.push(other as char);
                                }
                            }
                        } else {
                            break;
                        }
                    } else {
                        string_val.push(c as char);
                    }
                }
                let span = Span::new(start, self.cursor);
                if !closed {
                    diagnostics.error_with_help(
                        span,
                        "unterminated string literal",
                        "add a closing quote '\"' to terminate the string",
                    );
                }
                let raw_text = &self.source[start..self.cursor];
                return Token::new(TokenKind::Str(string_val), span, raw_text);
            }

            // Character literals
            if b == b'\'' {
                let mut char_val = None;
                let mut closed = false;
                if let Some(c) = self.advance() {
                    if c == b'\\' {
                        if let Some(esc) = self.advance() {
                            let resolved = match esc {
                                b'n' => '\n',
                                b't' => '\t',
                                b'r' => '\r',
                                b'0' => '\0',
                                b'\\' => '\\',
                                b'\'' => '\'',
                                b'"' => '"',
                                other => {
                                    diagnostics.warning(
                                        Span::new(self.cursor - 2, self.cursor),
                                        format!("unknown escape sequence '\\{}'", other as char),
                                    );
                                    other as char
                                }
                            };
                            char_val = Some(resolved);
                        }
                    } else if c != b'\'' {
                        char_val = Some(c as char);
                    }
                }
                if self.peek() == Some(b'\'') {
                    self.advance();
                    closed = true;
                }
                let span = Span::new(start, self.cursor);
                if !closed {
                    diagnostics.error_with_help(
                        span,
                        "unterminated character literal",
                        "add a closing single quote '\'' to terminate the character literal",
                    );
                }
                let raw_text = &self.source[start..self.cursor];
                return Token::new(TokenKind::Char(char_val.unwrap_or('\0')), span, raw_text);
            }

            // Multi-char and single-char operators & punctuation
            let kind = match b {
                b'+' => {
                    if self.peek() == Some(b'+') {
                        self.advance();
                        TokenKind::PlusPlus
                    } else if self.peek() == Some(b'=') {
                        self.advance();
                        TokenKind::PlusEq
                    } else {
                        TokenKind::Plus
                    }
                }
                b'-' => {
                    if self.peek() == Some(b'-') {
                        self.advance();
                        TokenKind::MinusMinus
                    } else if self.peek() == Some(b'=') {
                        self.advance();
                        TokenKind::MinusEq
                    } else {
                        TokenKind::Minus
                    }
                }
                b'*' => {
                    if self.peek() == Some(b'=') {
                        self.advance();
                        TokenKind::StarEq
                    } else {
                        TokenKind::Star
                    }
                }
                b'/' => {
                    if self.peek() == Some(b'=') {
                        self.advance();
                        TokenKind::SlashEq
                    } else {
                        TokenKind::Slash
                    }
                }
                b'%' => {
                    if self.peek() == Some(b'=') {
                        self.advance();
                        TokenKind::PercentEq
                    } else {
                        TokenKind::Percent
                    }
                }
                b'=' => {
                    if self.peek() == Some(b'=') {
                        self.advance();
                        TokenKind::EqEq
                    } else {
                        TokenKind::Eq
                    }
                }
                b'!' => {
                    if self.peek() == Some(b'=') {
                        self.advance();
                        TokenKind::BangEq
                    } else {
                        diagnostics.error(
                            Span::new(start, self.cursor),
                            "unexpected character '!'; did you mean '!=' or keyword 'not'?",
                        );
                        continue;
                    }
                }
                b'<' => {
                    if self.peek() == Some(b'<') {
                        self.advance();
                        TokenKind::LtLt
                    } else if self.peek() == Some(b'=') {
                        self.advance();
                        TokenKind::LtEq
                    } else {
                        TokenKind::Lt
                    }
                }
                b'>' => {
                    if self.peek() == Some(b'>') {
                        self.advance();
                        TokenKind::GtGt
                    } else if self.peek() == Some(b'=') {
                        self.advance();
                        TokenKind::GtEq
                    } else {
                        TokenKind::Gt
                    }
                }
                b'|' => TokenKind::Pipe,
                b'&' => TokenKind::Ampersand,
                b'^' => TokenKind::Caret,
                b'~' => TokenKind::Tilde,
                b'.' => {
                    if self.peek() == Some(b'.') {
                        self.advance();
                        TokenKind::DotDot
                    } else {
                        TokenKind::Dot
                    }
                }
                b':' => TokenKind::Colon,
                b';' => TokenKind::Semicolon,
                b',' => TokenKind::Comma,
                b'(' => TokenKind::OpenParen,
                b')' => TokenKind::CloseParen,
                b'{' => TokenKind::OpenBrace,
                b'}' => TokenKind::CloseBrace,
                b'[' => TokenKind::OpenBracket,
                b']' => TokenKind::CloseBracket,
                other => {
                    let span = Span::new(start, self.cursor);
                    diagnostics.error(
                        span,
                        format!("unexpected character '{}' (ASCII {})", other as char, other),
                    );
                    continue;
                }
            };

            let span = Span::new(start, self.cursor);
            let text = &self.source[start..self.cursor];
            return Token::new(kind, span, text);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer_basic() {
        let mut bag = DiagnosticBag::new();
        let src = "var age:<INT32> = 20; # comment\nage++;\n";
        let mut lexer = Lexer::new(src);
        let tokens = lexer.tokenize(&mut bag);
        assert!(!bag.has_errors());
        assert_eq!(tokens[0].kind, TokenKind::Var);
        assert_eq!(tokens[1].kind, TokenKind::Ident("age".into()));
        assert_eq!(tokens[2].kind, TokenKind::Colon);
        assert_eq!(tokens[3].kind, TokenKind::Lt);
        assert_eq!(tokens[4].kind, TokenKind::Ident("INT32".into()));
        assert_eq!(tokens[5].kind, TokenKind::Gt);
        assert_eq!(tokens[6].kind, TokenKind::Eq);
        assert_eq!(tokens[7].kind, TokenKind::Int(20));
        assert_eq!(tokens[8].kind, TokenKind::Semicolon);
        assert!(matches!(tokens[9].kind, TokenKind::Comment(_)));
    }

    #[test]
    fn test_keywords_case() {
        let mut bag = DiagnosticBag::new();
        let src = "As Do fr as do";
        let mut lexer = Lexer::new(src);
        let tokens = lexer.tokenize(&mut bag);
        assert_eq!(tokens[0].kind, TokenKind::As);
        assert_eq!(tokens[1].kind, TokenKind::Do);
        assert_eq!(tokens[2].kind, TokenKind::Fr);
        assert_eq!(tokens[3].kind, TokenKind::Ident("as".into()));
        assert_eq!(tokens[4].kind, TokenKind::Ident("do".into()));
    }

    #[test]
    fn test_phase9_keywords() {
        let mut bag = DiagnosticBag::new();
        let src = "import math; test \"demo\" {}";
        let mut lexer = Lexer::new(src);
        let tokens = lexer.tokenize(&mut bag);
        assert!(!bag.has_errors());
        assert_eq!(tokens[0].kind, TokenKind::Import);
        assert_eq!(tokens[1].kind, TokenKind::Ident("math".into()));
        assert_eq!(tokens[2].kind, TokenKind::Semicolon);
        assert_eq!(tokens[3].kind, TokenKind::Test);
        assert_eq!(tokens[4].kind, TokenKind::Str("demo".into()));
    }
}
