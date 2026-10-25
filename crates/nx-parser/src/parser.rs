use nx_ast::*;
use nx_diagnostics::{DiagnosticBag, Span};
use nx_lexer::{Token, TokenKind};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Precedence {
    None = 0,
    Assignment = 1,
    Or = 2,         // |
    Xor = 3,        // ^
    And = 4,        // &
    Equality = 5,   // ==, !=
    Relational = 6, // <, <=, >, >=
    Shift = 7,      // <<, >>
    Range = 8,      // ..
    Term = 9,       // +, -
    Factor = 10,    // *, /, %
    Unary = 11,     // not, -, +, ~
    Postfix = 12,   // ++, --, call, index, property
}

impl Precedence {
    fn for_binary(op: &TokenKind, allow_gt: bool) -> Option<Precedence> {
        match op {
            TokenKind::Pipe => Some(Precedence::Or),
            TokenKind::Caret => Some(Precedence::Xor),
            TokenKind::Ampersand => Some(Precedence::And),
            TokenKind::EqEq | TokenKind::BangEq => Some(Precedence::Equality),
            TokenKind::Lt | TokenKind::LtEq | TokenKind::GtEq => Some(Precedence::Relational),
            TokenKind::Gt => {
                if allow_gt {
                    Some(Precedence::Relational)
                } else {
                    None
                }
            }
            TokenKind::LtLt | TokenKind::GtGt => Some(Precedence::Shift),
            TokenKind::DotDot => Some(Precedence::Range),
            TokenKind::Plus | TokenKind::Minus => Some(Precedence::Term),
            TokenKind::Star | TokenKind::Slash | TokenKind::Percent => Some(Precedence::Factor),
            _ => None,
        }
    }
}

pub struct Parser<'a> {
    tokens: Vec<Token>,
    cursor: usize,
    diagnostics: &'a mut DiagnosticBag,
    allow_gt: bool,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: Vec<Token>, diagnostics: &'a mut DiagnosticBag) -> Self {
        // Filter out comments from tokens presented to parser
        let tokens = tokens
            .into_iter()
            .filter(|t| !matches!(t.kind, TokenKind::Comment(_)))
            .collect();
        Self {
            tokens,
            cursor: 0,
            diagnostics,
            allow_gt: true,
        }
    }

    fn peek(&self) -> &Token {
        if self.cursor < self.tokens.len() {
            &self.tokens[self.cursor]
        } else {
            self.tokens.last().expect("tokens cannot be empty")
        }
    }

    fn peek_kind(&self) -> &TokenKind {
        &self.peek().kind
    }

    fn peek_offset(&self, offset: usize) -> Option<&Token> {
        let idx = self.cursor + offset;
        if idx < self.tokens.len() {
            Some(&self.tokens[idx])
        } else {
            None
        }
    }

    fn previous(&self) -> &Token {
        assert!(self.cursor > 0, "previous called at start of tokens");
        &self.tokens[self.cursor - 1]
    }

    fn is_at_end(&self) -> bool {
        matches!(self.peek_kind(), TokenKind::Eof)
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.cursor += 1;
        }
        self.previous()
    }

    fn check(&self, kind: &TokenKind) -> bool {
        if self.is_at_end() {
            false
        } else {
            std::mem::discriminant(self.peek_kind()) == std::mem::discriminant(kind)
        }
    }

    fn match_token(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: &TokenKind, msg: &str) -> Option<Token> {
        if self.check(kind) {
            Some(self.advance().clone())
        } else {
            let span = self.peek().span;
            self.diagnostics.error(span, msg);
            None
        }
    }

    fn check_gt(&self) -> bool {
        self.check(&TokenKind::Gt) || self.check(&TokenKind::GtGt)
    }

    fn expect_gt(&mut self, msg: &str) -> Option<Token> {
        if self.check(&TokenKind::Gt) {
            Some(self.advance().clone())
        } else if self.check(&TokenKind::GtGt) {
            let cur = self.cursor;
            let full_span = self.tokens[cur].span;
            let mid = full_span.start + 1;
            let first_tok = Token::new(TokenKind::Gt, Span::new(full_span.start, mid), ">");
            self.tokens[cur] = Token::new(TokenKind::Gt, Span::new(mid, full_span.end), ">");
            Some(first_tok)
        } else {
            let span = self.peek().span;
            self.diagnostics.error(span, msg);
            None
        }
    }

    fn expect_semicolon(&mut self) -> bool {
        if self.match_token(&TokenKind::Semicolon) {
            true
        } else {
            let span = if self.cursor > 0 {
                let prev_span = self.previous().span;
                Span::new(prev_span.end, prev_span.end)
            } else {
                self.peek().span
            };
            self.diagnostics
                .error_with_help(span, "expected ';'", "statements must end with ';'");
            false
        }
    }

    fn synchronize(&mut self) {
        self.advance();
        while !self.is_at_end() {
            if self.previous().kind == TokenKind::Semicolon {
                return;
            }
            match self.peek_kind() {
                TokenKind::Var
                | TokenKind::Const
                | TokenKind::Tem
                | TokenKind::Fn
                | TokenKind::If
                | TokenKind::As
                | TokenKind::Do
                | TokenKind::Fr
                | TokenKind::Return
                | TokenKind::Class
                | TokenKind::Interface
                | TokenKind::Match
                | TokenKind::Import
                | TokenKind::Test
                | TokenKind::CloseBrace => return,
                _ => {
                    self.advance();
                }
            }
        }
    }

    pub fn parse_program(&mut self) -> Program {
        let start_span = self.peek().span;
        let mut statements = Vec::new();

        while !self.is_at_end() {
            if let Some(stmt) = self.parse_statement() {
                statements.push(stmt);
            } else {
                self.synchronize();
            }
        }

        let end_span = if let Some(last) = statements.last() {
            last.span()
        } else {
            start_span
        };

        Program::new(statements, start_span.merge(end_span))
    }

    fn parse_statement(&mut self) -> Option<Stmt> {
        match self.peek_kind() {
            TokenKind::Var => self.parse_var_decl(VarKind::Var),
            TokenKind::Const => self.parse_var_decl(VarKind::Const),
            TokenKind::Tem => self.parse_var_decl(VarKind::Tem),
            TokenKind::Fn => self.parse_fn_decl(),
            TokenKind::If => self.parse_if_stmt(),
            TokenKind::As => self.parse_as_loop(),
            TokenKind::Fr => self.parse_for_loop(),
            TokenKind::Do => self.parse_do_as_loop(),
            TokenKind::Return => self.parse_return_stmt(),
            TokenKind::Match => self.parse_match_stmt(),
            TokenKind::Class => {
                let start_span = self.advance().span;
                self.parse_class_decl(false, false, start_span)
            }
            TokenKind::Interface => {
                let start_span = self.advance().span;
                self.parse_interface_decl(start_span)
            }
            TokenKind::Abstract => {
                let start_span = self.advance().span;
                if self.match_token(&TokenKind::Class) {
                    self.parse_class_decl(true, false, start_span)
                } else {
                    let span = self.peek().span;
                    self.diagnostics
                        .error(span, "expected 'class' after 'abstract'");
                    None
                }
            }
            TokenKind::Final => {
                let start_span = self.advance().span;
                if self.match_token(&TokenKind::Class) {
                    self.parse_class_decl(false, true, start_span)
                } else {
                    let span = self.peek().span;
                    self.diagnostics
                        .error(span, "expected 'class' after 'final'");
                    None
                }
            }
            TokenKind::OpenBrace => self.parse_block().map(Stmt::Block),
            TokenKind::Unsafe => self.parse_unsafe_block(),
            TokenKind::Struct => self.parse_struct_decl(),
            TokenKind::Extern => self.parse_extern_block(),
            TokenKind::Import => self.parse_import_stmt(),
            TokenKind::Test => self.parse_test_decl(),
            _ => self.parse_expr_or_assignment_stmt(),
        }
    }

    fn parse_import_stmt(&mut self) -> Option<Stmt> {
        let start_span = self.advance().span; // consume 'import'
        let (path, path_span) = match self.peek_kind() {
            TokenKind::Ident(ref s) => {
                let text = s.clone();
                let span = self.advance().span;
                (text, span)
            }
            TokenKind::Str(ref s) => {
                let text = s.clone();
                let span = self.advance().span;
                (text, span)
            }
            _ => {
                let span = self.peek().span;
                self.diagnostics.error(
                    span,
                    "expected module name or file path string after 'import'",
                );
                return None;
            }
        };

        let had_semi = self.expect_semicolon();
        let end_span = if had_semi {
            self.previous().span
        } else {
            path_span
        };

        Some(Stmt::Import(ImportStmt {
            path,
            span: start_span.merge(end_span),
        }))
    }

    fn parse_test_decl(&mut self) -> Option<Stmt> {
        let start_span = self.advance().span; // consume 'test'
        let name = match self.peek_kind() {
            TokenKind::Str(ref s) => {
                let text = s.clone();
                self.advance();
                text
            }
            TokenKind::Ident(ref s) => {
                let text = s.clone();
                self.advance();
                text
            }
            _ => {
                let span = self.peek().span;
                self.diagnostics
                    .error(span, "expected test name string or identifier after 'test'");
                return None;
            }
        };

        let body = self.parse_block()?;
        let span = start_span.merge(body.span);

        Some(Stmt::TestDecl(TestDecl { name, body, span }))
    }

    fn parse_var_decl(&mut self, kind: VarKind) -> Option<Stmt> {
        let start_span = self.advance().span; // consume var/const/tem

        let name = match self.peek_kind() {
            TokenKind::Ident(_) => {
                let token = self.advance();
                Ident::new(token.text.clone(), token.span)
            }
            _ => {
                let span = self.peek().span;
                self.diagnostics.error(
                    span,
                    format!("expected variable name after '{}'", kind.as_str()),
                );
                return None;
            }
        };

        if !self.match_token(&TokenKind::Colon) {
            let span = self.peek().span;
            self.diagnostics.error_with_help(
                span,
                format!("expected ':' type annotation for variable '{}'", name.name),
                "NextLang requires explicit type annotations: e.g. 'var x:<INT32> = ...;'",
            );
            return None;
        }

        let type_annotation = self.parse_type_annotation()?;

        let initializer = if self.match_token(&TokenKind::Eq) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        let had_semi = self.expect_semicolon();
        let end_span = if had_semi {
            self.previous().span
        } else {
            self.peek().span
        };

        Some(Stmt::VarDecl(VarDecl {
            kind,
            name,
            type_annotation,
            initializer,
            span: start_span.merge(end_span),
        }))
    }

    fn parse_type_annotation(&mut self) -> Option<Type> {
        let open_token = self.expect(&TokenKind::Lt, "expected '<' to start type annotation")?;
        let parsed_type = self.parse_type_expression()?;
        let close_token = self.expect_gt("expected '>' to close type annotation")?;

        let span = open_token.span.merge(close_token.span);
        // Update span to encompass `< ... >`
        let full_type = match parsed_type {
            Type::Named { name, .. } => Type::Named { name, span },
            Type::Generic {
                name, type_args, ..
            } => Type::Generic {
                name,
                type_args,
                span,
            },
            Type::Union { types, .. } => Type::Union { types, span },
            Type::Reference { inner, is_mut, .. } => Type::Reference {
                inner,
                is_mut,
                span,
            },
        };
        Some(full_type)
    }

    fn parse_type_expression(&mut self) -> Option<Type> {
        let first_type = self.parse_single_type()?;

        if self.check(&TokenKind::Pipe) {
            let mut types = vec![first_type];
            let start_span = types[0].span();
            while self.match_token(&TokenKind::Pipe) {
                let next_type = self.parse_single_type()?;
                types.push(next_type);
            }
            let end_span = types.last().unwrap().span();
            Some(Type::Union {
                types,
                span: start_span.merge(end_span),
            })
        } else {
            Some(first_type)
        }
    }

    fn parse_single_type(&mut self) -> Option<Type> {
        if self.match_token(&TokenKind::Ampersand) {
            let amp_span = self.previous().span;
            let is_mut = self.match_token(&TokenKind::Mut);
            let inner = self.parse_single_type()?;
            let span = amp_span.merge(inner.span());
            return Some(Type::Reference {
                inner: Box::new(inner),
                is_mut,
                span,
            });
        }

        let name_token = match self.peek_kind() {
            TokenKind::Ident(_) => self.advance().clone(),
            _ => {
                let span = self.peek().span;
                self.diagnostics.error(span, "expected type name");
                return None;
            }
        };

        // Check if generic type: e.g. LIST<INT32>, MAP<STRING, INT32>
        if self.match_token(&TokenKind::Lt) {
            let mut type_args = Vec::new();
            if !self.check_gt() {
                loop {
                    let arg = self.parse_type_expression()?;
                    type_args.push(arg);
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }
            }
            let close = self.expect_gt("expected '>' to close generic type arguments")?;
            Some(Type::Generic {
                name: name_token.text,
                type_args,
                span: name_token.span.merge(close.span),
            })
        } else {
            Some(Type::Named {
                name: name_token.text,
                span: name_token.span,
            })
        }
    }

    fn parse_fn_decl(&mut self) -> Option<Stmt> {
        let start_span = self.advance().span; // consume 'fn'

        let name = match self.peek_kind() {
            TokenKind::Ident(_) => {
                let token = self.advance();
                Ident::new(token.text.clone(), token.span)
            }
            _ => {
                let span = self.peek().span;
                self.diagnostics
                    .error(span, "expected function name after 'fn'");
                return None;
            }
        };

        self.expect(&TokenKind::OpenParen, "expected '(' after function name")?;

        let mut params = Vec::new();
        if !self.check(&TokenKind::CloseParen) {
            loop {
                let param_name = match self.peek_kind() {
                    TokenKind::Ident(_) => {
                        let token = self.advance();
                        Ident::new(token.text.clone(), token.span)
                    }
                    _ => {
                        let span = self.peek().span;
                        self.diagnostics.error(span, "expected parameter name");
                        return None;
                    }
                };

                self.expect(&TokenKind::Colon, "expected ':' after parameter name")?;
                let type_annotation = self.parse_type_annotation()?;
                let param_span = param_name.span.merge(type_annotation.span());
                params.push(Param {
                    name: param_name,
                    type_annotation,
                    span: param_span,
                });

                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
            }
        }

        self.expect(&TokenKind::CloseParen, "expected ')' after parameters")?;

        let return_type = if self.match_token(&TokenKind::Colon) {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };

        let body = self.parse_block()?;
        let span = start_span.merge(body.span);

        Some(Stmt::FunctionDecl(FunctionDecl {
            name,
            params,
            return_type,
            body,
            span,
        }))
    }

    fn parse_class_decl(
        &mut self,
        is_abstract: bool,
        is_final: bool,
        start_span: Span,
    ) -> Option<Stmt> {
        let name = match self.peek_kind() {
            TokenKind::Ident(_) => {
                let token = self.advance();
                Ident::new(token.text.clone(), token.span)
            }
            _ => {
                let span = self.peek().span;
                self.diagnostics
                    .error(span, "expected class name after 'class'");
                return None;
            }
        };

        let parent = if self.match_token(&TokenKind::Extends) {
            match self.peek_kind() {
                TokenKind::Ident(_) => {
                    let token = self.advance();
                    Some(Ident::new(token.text.clone(), token.span))
                }
                _ => {
                    let span = self.peek().span;
                    self.diagnostics
                        .error(span, "expected parent class name after 'extends'");
                    return None;
                }
            }
        } else {
            None
        };

        let mut implements = Vec::new();
        if self.match_token(&TokenKind::Implements) {
            loop {
                match self.peek_kind() {
                    TokenKind::Ident(_) => {
                        let token = self.advance();
                        implements.push(Ident::new(token.text.clone(), token.span));
                    }
                    _ => {
                        let span = self.peek().span;
                        self.diagnostics
                            .error(span, "expected interface name after 'implements'");
                        return None;
                    }
                }
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
            }
        }

        self.expect(&TokenKind::OpenBrace, "expected '{' before class body")?;

        let mut members = Vec::new();
        while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
            if let Some(member) = self.parse_class_member() {
                members.push(member);
            } else {
                self.synchronize();
            }
        }

        let close_token = self.expect(&TokenKind::CloseBrace, "expected '}' after class body")?;
        let span = start_span.merge(close_token.span);

        Some(Stmt::ClassDecl(ClassDecl {
            is_abstract,
            is_final,
            name,
            parent,
            implements,
            members,
            span,
        }))
    }

    fn parse_class_member(&mut self) -> Option<ClassMember> {
        let member_start = self.peek().span;
        let mut access = AccessModifier::Public;
        let mut is_static = false;
        let mut is_abstract = false;
        let mut is_final = false;

        loop {
            match self.peek_kind() {
                TokenKind::Public => {
                    self.advance();
                    access = AccessModifier::Public;
                }
                TokenKind::Private => {
                    self.advance();
                    access = AccessModifier::Private;
                }
                TokenKind::Protected => {
                    self.advance();
                    access = AccessModifier::Protected;
                }
                TokenKind::Static => {
                    self.advance();
                    is_static = true;
                }
                TokenKind::Abstract => {
                    self.advance();
                    is_abstract = true;
                }
                TokenKind::Final => {
                    self.advance();
                    is_final = true;
                }
                _ => break,
            }
        }

        match self.peek_kind() {
            TokenKind::Var | TokenKind::Const | TokenKind::Tem => {
                let kind = match self.peek_kind() {
                    TokenKind::Var => VarKind::Var,
                    TokenKind::Const => VarKind::Const,
                    TokenKind::Tem => VarKind::Tem,
                    _ => unreachable!(),
                };
                let stmt = self.parse_var_decl(kind)?;
                if let Stmt::VarDecl(var_decl) = stmt {
                    let span = member_start.merge(var_decl.span);
                    Some(ClassMember::Field(FieldDecl {
                        access,
                        is_static,
                        var_decl,
                        span,
                    }))
                } else {
                    None
                }
            }
            TokenKind::Fn => {
                self.advance(); // consume 'fn'
                let name = match self.peek_kind() {
                    TokenKind::Ident(_) => {
                        let token = self.advance();
                        Ident::new(token.text.clone(), token.span)
                    }
                    _ => {
                        let span = self.peek().span;
                        self.diagnostics
                            .error(span, "expected method name after 'fn'");
                        return None;
                    }
                };

                self.expect(&TokenKind::OpenParen, "expected '(' after method name")?;

                let mut params = Vec::new();
                if !self.check(&TokenKind::CloseParen) {
                    loop {
                        let param_name = match self.peek_kind() {
                            TokenKind::Ident(_) => {
                                let token = self.advance();
                                Ident::new(token.text.clone(), token.span)
                            }
                            _ => {
                                let span = self.peek().span;
                                self.diagnostics.error(span, "expected parameter name");
                                return None;
                            }
                        };

                        self.expect(&TokenKind::Colon, "expected ':' after parameter name")?;
                        let type_annotation = self.parse_type_annotation()?;
                        let param_span = param_name.span.merge(type_annotation.span());
                        params.push(Param {
                            name: param_name,
                            type_annotation,
                            span: param_span,
                        });

                        if !self.match_token(&TokenKind::Comma) {
                            break;
                        }
                    }
                }

                self.expect(&TokenKind::CloseParen, "expected ')' after parameters")?;

                let return_type = if self.match_token(&TokenKind::Colon) {
                    Some(self.parse_type_annotation()?)
                } else {
                    None
                };

                let body = if is_abstract || self.check(&TokenKind::Semicolon) {
                    if self.match_token(&TokenKind::Semicolon) {
                        is_abstract = true;
                        Block {
                            statements: Vec::new(),
                            span: name.span,
                        }
                    } else {
                        self.parse_block()?
                    }
                } else {
                    self.parse_block()?
                };

                let span = member_start.merge(body.span);

                Some(ClassMember::Method(MethodDecl {
                    access,
                    is_static,
                    is_abstract,
                    is_final,
                    function: FunctionDecl {
                        name,
                        params,
                        return_type,
                        body,
                        span,
                    },
                    span,
                }))
            }
            _ => {
                let span = self.peek().span;
                self.diagnostics.error(
                    span,
                    format!("unexpected token '{}' in class body", self.peek().kind),
                );
                None
            }
        }
    }

    fn parse_interface_decl(&mut self, start_span: Span) -> Option<Stmt> {
        let name = match self.peek_kind() {
            TokenKind::Ident(_) => {
                let token = self.advance();
                Ident::new(token.text.clone(), token.span)
            }
            _ => {
                let span = self.peek().span;
                self.diagnostics
                    .error(span, "expected interface name after 'interface'");
                return None;
            }
        };

        self.expect(&TokenKind::OpenBrace, "expected '{' before interface body")?;

        let mut methods = Vec::new();
        while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
            let method_start = self.peek().span;
            if !self.match_token(&TokenKind::Fn) {
                let span = self.peek().span;
                self.diagnostics
                    .error(span, "expected 'fn' in interface method declaration");
                self.synchronize();
                continue;
            }

            let m_name = match self.peek_kind() {
                TokenKind::Ident(_) => {
                    let token = self.advance();
                    Ident::new(token.text.clone(), token.span)
                }
                _ => {
                    let span = self.peek().span;
                    self.diagnostics
                        .error(span, "expected method name after 'fn'");
                    self.synchronize();
                    continue;
                }
            };

            if self
                .expect(&TokenKind::OpenParen, "expected '(' after method name")
                .is_none()
            {
                self.synchronize();
                continue;
            }

            let mut params = Vec::new();
            if !self.check(&TokenKind::CloseParen) {
                loop {
                    let param_name = match self.peek_kind() {
                        TokenKind::Ident(_) => {
                            let token = self.advance();
                            Ident::new(token.text.clone(), token.span)
                        }
                        _ => {
                            let span = self.peek().span;
                            self.diagnostics.error(span, "expected parameter name");
                            break;
                        }
                    };

                    if self
                        .expect(&TokenKind::Colon, "expected ':' after parameter name")
                        .is_none()
                    {
                        break;
                    }
                    let Some(type_annotation) = self.parse_type_annotation() else {
                        break;
                    };
                    let param_span = param_name.span.merge(type_annotation.span());
                    params.push(Param {
                        name: param_name,
                        type_annotation,
                        span: param_span,
                    });

                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }
            }

            if self
                .expect(&TokenKind::CloseParen, "expected ')' after parameters")
                .is_none()
            {
                self.synchronize();
                continue;
            }

            let return_type = if self.match_token(&TokenKind::Colon) {
                self.parse_type_annotation()
            } else {
                None
            };

            self.expect_semicolon();
            let end_span = self.previous().span;

            methods.push(InterfaceMethod {
                name: m_name,
                params,
                return_type,
                span: method_start.merge(end_span),
            });
        }

        let close_token =
            self.expect(&TokenKind::CloseBrace, "expected '}' after interface body")?;
        let span = start_span.merge(close_token.span);

        Some(Stmt::InterfaceDecl(InterfaceDecl {
            name,
            methods,
            span,
        }))
    }

    fn parse_unsafe_block(&mut self) -> Option<Stmt> {
        let start_span = self.advance().span; // consume 'unsafe'
        let body = self.parse_block()?;
        let span = start_span.merge(body.span);
        Some(Stmt::Unsafe(UnsafeBlock { body, span }))
    }

    fn parse_struct_decl(&mut self) -> Option<Stmt> {
        let start_span = self.advance().span; // consume 'struct'
        let name_tok = if let TokenKind::Ident(_) = self.peek_kind() {
            let tok = self.advance();
            Ident::new(tok.text.clone(), tok.span)
        } else {
            let span = self.peek().span;
            self.diagnostics
                .error(span, "expected struct name after 'struct'");
            return None;
        };
        self.expect(&TokenKind::OpenBrace, "expected '{' after struct name")?;
        let mut fields = Vec::new();
        while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
            let fname = if let TokenKind::Ident(_) = self.peek_kind() {
                let tok = self.advance();
                Ident::new(tok.text.clone(), tok.span)
            } else {
                let span = self.peek().span;
                self.diagnostics
                    .error(span, "expected field name in struct declaration");
                return None;
            };
            self.expect(&TokenKind::Colon, "expected ':' after field name")?;
            let fty = self.parse_type_annotation()?;
            let fspan = fname.span.merge(fty.span());
            fields.push(StructField {
                name: fname,
                field_type: fty,
                span: fspan,
            });
            if !self.match_token(&TokenKind::Comma) {
                self.match_token(&TokenKind::Semicolon);
            }
        }
        let close_brace = self.expect(
            &TokenKind::CloseBrace,
            "expected '}' closing struct declaration",
        )?;
        let span = start_span.merge(close_brace.span);
        Some(Stmt::StructDecl(StructDecl {
            name: name_tok,
            fields,
            span,
        }))
    }

    fn parse_extern_block(&mut self) -> Option<Stmt> {
        let start_span = self.advance().span; // consume 'extern'
        let abi = match self.peek_kind() {
            TokenKind::Str(s) => {
                let s = s.clone();
                self.advance();
                s
            }
            _ => "C".to_string(),
        };
        self.expect(&TokenKind::OpenBrace, "expected '{' in extern block")?;
        let mut functions = Vec::new();
        while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
            self.expect(&TokenKind::Fn, "expected 'fn' in extern block")?;
            let fname = if let TokenKind::Ident(_) = self.peek_kind() {
                let tok = self.advance();
                Ident::new(tok.text.clone(), tok.span)
            } else {
                let span = self.peek().span;
                self.diagnostics
                    .error(span, "expected function name after 'fn'");
                return None;
            };
            self.expect(
                &TokenKind::OpenParen,
                "expected '(' after extern function name",
            )?;
            let mut params = Vec::new();
            if !self.check(&TokenKind::CloseParen) {
                loop {
                    let pname = if let TokenKind::Ident(_) = self.peek_kind() {
                        let tok = self.advance();
                        Ident::new(tok.text.clone(), tok.span)
                    } else {
                        let span = self.peek().span;
                        self.diagnostics.error(span, "expected parameter name");
                        return None;
                    };
                    self.expect(&TokenKind::Colon, "expected ':' after parameter name")?;
                    let pty = self.parse_type_annotation()?;
                    let pspan = pname.span.merge(pty.span());
                    params.push(Param {
                        name: pname,
                        type_annotation: pty,
                        span: pspan,
                    });
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }
            }
            self.expect(&TokenKind::CloseParen, "expected ')' after parameters")?;
            let return_type = if self.match_token(&TokenKind::Colon) {
                Some(self.parse_type_annotation()?)
            } else {
                None
            };
            self.expect_semicolon();
            let fn_span = fname.span;
            let end_span = return_type.as_ref().map(|t| t.span()).unwrap_or(fn_span);
            functions.push(ExternFnDecl {
                name: fname,
                params,
                return_type,
                span: fn_span.merge(end_span),
            });
        }
        let close_brace =
            self.expect(&TokenKind::CloseBrace, "expected '}' closing extern block")?;
        let span = start_span.merge(close_brace.span);
        Some(Stmt::ExternBlock(ExternBlock {
            abi,
            functions,
            span,
        }))
    }

    fn parse_if_stmt(&mut self) -> Option<Stmt> {
        let start_span = self.advance().span; // consume 'if'
        let condition = self.parse_expression()?;
        let then_branch = self.parse_block()?;

        let mut otherwise_branches = Vec::new();
        while self.match_token(&TokenKind::Otherwise) {
            let ow_start = self.previous().span;
            let ow_cond = self.parse_expression()?;
            let ow_body = self.parse_block()?;
            let ow_span = ow_start.merge(ow_body.span);
            otherwise_branches.push(OtherwiseBranch {
                condition: ow_cond,
                body: ow_body,
                span: ow_span,
            });
        }

        let else_branch = if self.match_token(&TokenKind::Else) {
            Some(self.parse_block()?)
        } else {
            None
        };

        let end_span = if let Some(ref eb) = else_branch {
            eb.span
        } else if let Some(last_ow) = otherwise_branches.last() {
            last_ow.span
        } else {
            then_branch.span
        };

        Some(Stmt::If(IfStmt {
            condition,
            then_branch,
            otherwise_branches,
            else_branch,
            span: start_span.merge(end_span),
        }))
    }

    fn parse_match_stmt(&mut self) -> Option<Stmt> {
        let start_span = self.advance().span; // consume 'match'
        let target = self.parse_expression()?;
        self.expect(&TokenKind::OpenBrace, "expected '{' after match expression")?;

        let mut cases = Vec::new();
        while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
            if !self.match_token(&TokenKind::Case) {
                let span = self.peek().span;
                self.diagnostics
                    .error(span, "expected 'case' inside match block");
                self.synchronize();
                continue;
            }
            let case_start = self.previous().span;
            let Some(pattern) = self.parse_pattern() else {
                self.synchronize();
                continue;
            };

            let guard = if self.match_token(&TokenKind::If) {
                self.parse_expression()
            } else {
                None
            };

            let Some(body) = self.parse_block() else {
                continue;
            };

            let case_span = case_start.merge(body.span);
            cases.push(MatchCase {
                pattern,
                guard,
                body,
                span: case_span,
            });
        }

        let close_brace = self.expect(&TokenKind::CloseBrace, "expected '}' after match cases")?;
        Some(Stmt::Match(MatchStmt {
            target,
            cases,
            span: start_span.merge(close_brace.span),
        }))
    }

    fn parse_pattern(&mut self) -> Option<Pattern> {
        let first = self.parse_single_pattern()?;
        if self.check(&TokenKind::Pipe) {
            let mut alts = vec![first];
            let start_span = alts[0].span();
            while self.match_token(&TokenKind::Pipe) {
                let next = self.parse_single_pattern()?;
                alts.push(next);
            }
            let end_span = alts.last().unwrap().span();
            Some(Pattern::Alternative(alts, start_span.merge(end_span)))
        } else {
            Some(first)
        }
    }

    fn parse_single_pattern(&mut self) -> Option<Pattern> {
        let peek_kind = self.peek().kind.clone();
        match peek_kind {
            TokenKind::Underscore => {
                let span = self.advance().span;
                Some(Pattern::Wildcard(span))
            }
            TokenKind::Ident(ref s) if s == "_" => {
                let span = self.advance().span;
                Some(Pattern::Wildcard(span))
            }
            TokenKind::Ident(_) => {
                let token = self.advance();
                Some(Pattern::Binding(Ident::new(token.text.clone(), token.span)))
            }
            TokenKind::Int(val) => {
                let token = self.advance().clone();
                Some(Pattern::Literal(Literal::Integer {
                    value: val,
                    raw: token.text,
                    span: token.span,
                }))
            }
            TokenKind::Float(ref s) => {
                let token = self.advance().clone();
                Some(Pattern::Literal(Literal::Float {
                    raw: s.clone(),
                    span: token.span,
                }))
            }
            TokenKind::Str(ref s) => {
                let token = self.advance().clone();
                Some(Pattern::Literal(Literal::String {
                    value: s.clone(),
                    span: token.span,
                }))
            }
            TokenKind::Char(c) => {
                let token = self.advance().clone();
                Some(Pattern::Literal(Literal::Char {
                    value: c,
                    span: token.span,
                }))
            }
            TokenKind::True => {
                let token = self.advance().clone();
                Some(Pattern::Literal(Literal::Bool {
                    value: true,
                    span: token.span,
                }))
            }
            TokenKind::False => {
                let token = self.advance().clone();
                Some(Pattern::Literal(Literal::Bool {
                    value: false,
                    span: token.span,
                }))
            }
            TokenKind::Null => {
                let token = self.advance().clone();
                Some(Pattern::Literal(Literal::Null { span: token.span }))
            }
            _ => {
                let span = self.peek().span;
                self.diagnostics.error(
                    span,
                    format!("unexpected token '{}' in pattern", self.peek().kind),
                );
                None
            }
        }
    }

    fn parse_as_loop(&mut self) -> Option<Stmt> {
        let start_span = self.advance().span; // consume 'As'
        let condition = self.parse_expression()?;
        let body = self.parse_block()?;
        let span = start_span.merge(body.span);

        Some(Stmt::AsLoop(AsLoopStmt {
            condition,
            body,
            span,
        }))
    }

    fn parse_for_loop(&mut self) -> Option<Stmt> {
        let start_span = self.advance().span; // consume 'fr'
        self.expect(&TokenKind::Lt, "expected '<' after 'fr'")?;

        // 1. Initialization part
        let init = if self.check(&TokenKind::Colon) {
            None
        } else if matches!(
            self.peek_kind(),
            TokenKind::Var | TokenKind::Const | TokenKind::Tem
        ) {
            let kind = match self.peek_kind() {
                TokenKind::Var => VarKind::Var,
                TokenKind::Const => VarKind::Const,
                TokenKind::Tem => VarKind::Tem,
                _ => unreachable!(),
            };
            let var_start = self.advance().span;
            let name = match self.peek_kind() {
                TokenKind::Ident(_) => {
                    let token = self.advance();
                    Ident::new(token.text.clone(), token.span)
                }
                _ => {
                    let span = self.peek().span;
                    self.diagnostics
                        .error(span, "expected variable name in for-loop initialization");
                    return None;
                }
            };
            self.expect(&TokenKind::Colon, "expected ':' for type annotation")?;
            let type_annotation = self.parse_type_annotation()?;
            let initializer = if self.match_token(&TokenKind::Eq) {
                Some(self.parse_expression()?)
            } else {
                None
            };
            let var_end = if let Some(ref init_expr) = initializer {
                init_expr.span()
            } else {
                type_annotation.span()
            };
            Some(Box::new(Stmt::VarDecl(VarDecl {
                kind,
                name,
                type_annotation,
                initializer,
                span: var_start.merge(var_end),
            })))
        } else {
            // Expression or assignment in init
            let expr = self.parse_expression()?;
            if self.is_assign_op(self.peek_kind()) {
                let op = self.parse_assign_op()?;
                let val = self.parse_expression()?;
                let span = expr.span().merge(val.span());
                Some(Box::new(Stmt::Assignment(Assignment {
                    target: expr,
                    op,
                    value: val,
                    span,
                })))
            } else {
                Some(Box::new(Stmt::Expr(expr)))
            }
        };

        self.expect(
            &TokenKind::Colon,
            "expected ':' after initialization in for-loop header",
        )?;

        // 2. Condition part
        let condition = if self.check(&TokenKind::Colon) {
            None
        } else {
            Some(self.parse_expression()?)
        };

        self.expect(
            &TokenKind::Colon,
            "expected ':' after condition in for-loop header",
        )?;

        // 3. Update part
        let prev_allow_gt = self.allow_gt;
        self.allow_gt = false;
        let update = if self.check(&TokenKind::Gt) {
            None
        } else {
            let expr = self.parse_expression()?;
            if self.is_assign_op(self.peek_kind()) {
                let op = self.parse_assign_op()?;
                let val = self.parse_expression()?;
                let span = expr.span().merge(val.span());
                Some(Box::new(Stmt::Assignment(Assignment {
                    target: expr,
                    op,
                    value: val,
                    span,
                })))
            } else if let Expr::Update {
                op,
                target,
                is_prefix,
                span,
            } = expr
            {
                Some(Box::new(Stmt::Update(UpdateStmt {
                    target: *target,
                    op,
                    is_prefix,
                    span,
                })))
            } else {
                Some(Box::new(Stmt::Expr(expr)))
            }
        };
        self.allow_gt = prev_allow_gt;

        self.expect(
            &TokenKind::Gt,
            "expected '>' after update in for-loop header",
        )?;

        let body = self.parse_block()?;
        let span = start_span.merge(body.span);

        Some(Stmt::ForLoop(ForLoopStmt {
            init,
            condition,
            update,
            body,
            span,
        }))
    }

    fn parse_do_as_loop(&mut self) -> Option<Stmt> {
        let start_span = self.advance().span; // consume 'Do'
        let body = self.parse_block()?;

        self.expect(&TokenKind::As, "expected 'As' after 'Do { ... }'")?;
        let condition = self.parse_expression()?;
        self.expect_semicolon();
        let end_span = self.previous().span;

        Some(Stmt::DoAsLoop(DoAsLoopStmt {
            body,
            condition,
            span: start_span.merge(end_span),
        }))
    }

    fn parse_return_stmt(&mut self) -> Option<Stmt> {
        let start_span = self.advance().span; // consume 'return'

        let value = if !self.check(&TokenKind::Semicolon) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.expect_semicolon();
        let end_span = self.previous().span;

        Some(Stmt::Return(ReturnStmt {
            value,
            span: start_span.merge(end_span),
        }))
    }

    fn parse_block(&mut self) -> Option<Block> {
        let open_token = self.expect(&TokenKind::OpenBrace, "expected '{'")?;

        let mut statements = Vec::new();
        while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
            if let Some(stmt) = self.parse_statement() {
                statements.push(stmt);
            } else {
                self.synchronize();
            }
        }

        let close_token = self.expect(&TokenKind::CloseBrace, "expected '}' to close block")?;

        Some(Block {
            statements,
            span: open_token.span.merge(close_token.span),
        })
    }

    fn is_assign_op(&self, kind: &TokenKind) -> bool {
        matches!(
            kind,
            TokenKind::Eq
                | TokenKind::PlusEq
                | TokenKind::MinusEq
                | TokenKind::StarEq
                | TokenKind::SlashEq
                | TokenKind::PercentEq
        )
    }

    fn parse_assign_op(&mut self) -> Option<AssignOp> {
        let (kind, span) = {
            let token = self.advance();
            (token.kind.clone(), token.span)
        };
        let op = match kind {
            TokenKind::Eq => AssignOp::Assign,
            TokenKind::PlusEq => AssignOp::AddAssign,
            TokenKind::MinusEq => AssignOp::SubAssign,
            TokenKind::StarEq => AssignOp::MulAssign,
            TokenKind::SlashEq => AssignOp::DivAssign,
            TokenKind::PercentEq => AssignOp::ModAssign,
            _ => {
                self.diagnostics.error(span, "expected assignment operator");
                return None;
            }
        };
        Some(op)
    }

    fn parse_expr_or_assignment_stmt(&mut self) -> Option<Stmt> {
        let expr = self.parse_expression()?;

        // Check if assignment: expr = value;
        if self.is_assign_op(self.peek_kind()) {
            let target_span = expr.span();
            let op = self.parse_assign_op()?;
            let value = self.parse_expression()?;
            self.expect_semicolon();
            let end_span = self.previous().span;
            Some(Stmt::Assignment(Assignment {
                target: expr,
                op,
                value,
                span: target_span.merge(end_span),
            }))
        } else if let Expr::Update {
            op,
            target,
            is_prefix,
            span,
        } = expr
        {
            self.expect_semicolon();
            let end_span = self.previous().span;
            Some(Stmt::Update(UpdateStmt {
                target: *target,
                op,
                is_prefix,
                span: span.merge(end_span),
            }))
        } else {
            self.expect_semicolon();
            Some(Stmt::Expr(expr))
        }
    }

    // --- Pratt Expression Parsing ---

    pub fn parse_expression(&mut self) -> Option<Expr> {
        self.parse_precedence(Precedence::Assignment)
    }

    fn parse_precedence(&mut self, precedence: Precedence) -> Option<Expr> {
        let mut left = self.parse_prefix()?;

        while !self.is_at_end() {
            // Check for postfix ++ and --
            if precedence <= Precedence::Postfix
                && (self.check(&TokenKind::PlusPlus) || self.check(&TokenKind::MinusMinus))
            {
                let op_token = self.advance();
                let op = match op_token.kind {
                    TokenKind::PlusPlus => UpdateOp::Increment,
                    TokenKind::MinusMinus => UpdateOp::Decrement,
                    _ => unreachable!(),
                };
                let span = left.span().merge(op_token.span);
                left = Expr::Update {
                    op,
                    target: Box::new(left),
                    is_prefix: false,
                    span,
                };
                continue;
            }

            // Check for call `(...)`
            if precedence <= Precedence::Postfix && self.check(&TokenKind::OpenParen) {
                left = self.parse_call_arguments(left)?;
                continue;
            }

            // Check for indexing `[...]`
            if precedence <= Precedence::Postfix && self.check(&TokenKind::OpenBracket) {
                left = self.parse_index_expr(left)?;
                continue;
            }

            // Check for property access `.ident`
            if precedence <= Precedence::Postfix && self.check(&TokenKind::Dot) {
                left = self.parse_property_access(left)?;
                continue;
            }

            // Binary operators
            let Some(op_prec) = Precedence::for_binary(self.peek_kind(), self.allow_gt) else {
                break;
            };

            if precedence > op_prec {
                break;
            }

            let op_token = self.advance().clone();
            let op = match op_token.kind {
                TokenKind::Plus => BinaryOp::Add,
                TokenKind::Minus => BinaryOp::Sub,
                TokenKind::Star => BinaryOp::Mul,
                TokenKind::Slash => BinaryOp::Div,
                TokenKind::Percent => BinaryOp::Mod,
                TokenKind::EqEq => BinaryOp::Eq,
                TokenKind::BangEq => BinaryOp::NotEq,
                TokenKind::Lt => BinaryOp::Lt,
                TokenKind::LtEq => BinaryOp::LtEq,
                TokenKind::Gt => BinaryOp::Gt,
                TokenKind::GtEq => BinaryOp::GtEq,
                TokenKind::Pipe => BinaryOp::Or,
                TokenKind::Caret => BinaryOp::BitXor,
                TokenKind::Ampersand => BinaryOp::And,
                TokenKind::LtLt => BinaryOp::Shl,
                TokenKind::GtGt => BinaryOp::Shr,
                TokenKind::DotDot => BinaryOp::Range,
                _ => unreachable!(),
            };

            // Parse right-hand side with precedence + 1 (left-associative)
            let next_prec = match op_prec {
                Precedence::Or => Precedence::Xor,
                Precedence::Xor => Precedence::And,
                Precedence::And => Precedence::Equality,
                Precedence::Equality => Precedence::Relational,
                Precedence::Relational => Precedence::Shift,
                Precedence::Shift => Precedence::Range,
                Precedence::Range => Precedence::Term,
                Precedence::Term => Precedence::Factor,
                Precedence::Factor => Precedence::Unary,
                _ => Precedence::Postfix,
            };

            let right = self.parse_precedence(next_prec)?;
            let span = left.span().merge(right.span());
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }

        Some(left)
    }

    fn parse_prefix(&mut self) -> Option<Expr> {
        let token = self.peek().clone();
        match &token.kind {
            TokenKind::Int(val) => {
                self.advance();
                Some(Expr::Literal(Literal::Integer {
                    value: *val,
                    raw: token.text.clone(),
                    span: token.span,
                }))
            }
            TokenKind::Float(raw) => {
                self.advance();
                Some(Expr::Literal(Literal::Float {
                    raw: raw.clone(),
                    span: token.span,
                }))
            }
            TokenKind::Str(val) => {
                self.advance();
                Some(Expr::Literal(Literal::String {
                    value: val.clone(),
                    span: token.span,
                }))
            }
            TokenKind::Char(c) => {
                self.advance();
                Some(Expr::Literal(Literal::Char {
                    value: *c,
                    span: token.span,
                }))
            }
            TokenKind::True => {
                self.advance();
                Some(Expr::Literal(Literal::Bool {
                    value: true,
                    span: token.span,
                }))
            }
            TokenKind::False => {
                self.advance();
                Some(Expr::Literal(Literal::Bool {
                    value: false,
                    span: token.span,
                }))
            }
            TokenKind::Null => {
                self.advance();
                Some(Expr::Literal(Literal::Null { span: token.span }))
            }
            TokenKind::Ident(name) => {
                let id_token = self.advance().clone();
                let ident = Ident::new(name.clone(), id_token.span);
                if self.check(&TokenKind::OpenBrace) {
                    let is_struct_lit = if let Some(t1) = self.peek_offset(1) {
                        matches!(t1.kind, TokenKind::CloseBrace)
                            || (matches!(t1.kind, TokenKind::Ident(_))
                                && self
                                    .peek_offset(2)
                                    .is_some_and(|t2| matches!(t2.kind, TokenKind::Colon)))
                    } else {
                        false
                    };
                    if is_struct_lit {
                        return self.parse_struct_literal(ident);
                    }
                }
                Some(Expr::Identifier(ident))
            }
            TokenKind::Self_ => {
                self.advance();
                Some(Expr::Identifier(Ident::new("self", token.span)))
            }
            TokenKind::Super => {
                self.advance();
                Some(Expr::Identifier(Ident::new("super", token.span)))
            }
            TokenKind::Tilde => {
                let op_span = self.advance().span;
                let operand = self.parse_precedence(Precedence::Unary)?;
                let span = op_span.merge(operand.span());
                Some(Expr::Unary {
                    op: UnaryOp::BitNot,
                    operand: Box::new(operand),
                    span,
                })
            }
            TokenKind::Not => {
                let op_span = self.advance().span;
                let operand = self.parse_precedence(Precedence::Unary)?;
                let span = op_span.merge(operand.span());
                Some(Expr::Unary {
                    op: UnaryOp::Not,
                    operand: Box::new(operand),
                    span,
                })
            }
            TokenKind::Minus => {
                let op_span = self.advance().span;
                let operand = self.parse_precedence(Precedence::Unary)?;
                let span = op_span.merge(operand.span());
                Some(Expr::Unary {
                    op: UnaryOp::Neg,
                    operand: Box::new(operand),
                    span,
                })
            }
            TokenKind::Plus => {
                let op_span = self.advance().span;
                let operand = self.parse_precedence(Precedence::Unary)?;
                let span = op_span.merge(operand.span());
                Some(Expr::Unary {
                    op: UnaryOp::Pos,
                    operand: Box::new(operand),
                    span,
                })
            }
            TokenKind::PlusPlus | TokenKind::MinusMinus => {
                let op_token = self.advance().clone();
                let op = match op_token.kind {
                    TokenKind::PlusPlus => UpdateOp::Increment,
                    TokenKind::MinusMinus => UpdateOp::Decrement,
                    _ => unreachable!(),
                };
                let operand = self.parse_precedence(Precedence::Unary)?;
                let span = op_token.span.merge(operand.span());
                Some(Expr::Update {
                    op,
                    target: Box::new(operand),
                    is_prefix: true,
                    span,
                })
            }
            TokenKind::OpenParen => {
                let open_span = self.advance().span;
                let prev_allow_gt = self.allow_gt;
                self.allow_gt = true;
                let expr = self.parse_expression()?;
                self.allow_gt = prev_allow_gt;
                let close = self.expect(&TokenKind::CloseParen, "expected ')' after expression")?;
                Some(Expr::Group {
                    expr: Box::new(expr),
                    span: open_span.merge(close.span),
                })
            }
            TokenKind::Ampersand => {
                let amp_span = self.advance().span;
                let is_mut = self.match_token(&TokenKind::Mut);
                let operand = self.parse_precedence(Precedence::Unary)?;
                let span = amp_span.merge(operand.span());
                Some(Expr::Borrow {
                    expr: Box::new(operand),
                    is_mut,
                    span,
                })
            }
            TokenKind::Alloc => {
                let start_span = self.advance().span;
                self.expect(&TokenKind::OpenParen, "expected '(' after 'alloc'")?;
                let expr = self.parse_expression()?;
                let close =
                    self.expect(&TokenKind::CloseParen, "expected ')' after alloc argument")?;
                Some(Expr::Alloc {
                    expr: Box::new(expr),
                    span: start_span.merge(close.span),
                })
            }
            TokenKind::Free => {
                let start_span = self.advance().span;
                self.expect(&TokenKind::OpenParen, "expected '(' after 'free'")?;
                let target = self.parse_expression()?;
                let close =
                    self.expect(&TokenKind::CloseParen, "expected ')' after free argument")?;
                Some(Expr::Free {
                    target: Box::new(target),
                    span: start_span.merge(close.span),
                })
            }
            TokenKind::OpenBracket => self.parse_list_literal(),
            TokenKind::OpenBrace => self.parse_map_literal(),
            _ => {
                let span = self.peek().span;
                self.diagnostics.error(
                    span,
                    format!("unexpected token '{}' in expression", self.peek().kind),
                );
                None
            }
        }
    }

    fn parse_list_literal(&mut self) -> Option<Expr> {
        let open_span = self.advance().span; // consume '['
        let mut elements = Vec::new();

        if !self.check(&TokenKind::CloseBracket) {
            loop {
                let elem = self.parse_expression()?;
                elements.push(elem);
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
                if self.check(&TokenKind::CloseBracket) {
                    break;
                }
            }
        }

        let close = self.expect(&TokenKind::CloseBracket, "expected ']' after list literal")?;
        Some(Expr::ListLiteral {
            elements,
            span: open_span.merge(close.span),
        })
    }

    fn parse_map_literal(&mut self) -> Option<Expr> {
        let open_span = self.advance().span; // consume '{'
        let mut entries = Vec::new();

        if !self.check(&TokenKind::CloseBrace) {
            loop {
                let key = self.parse_expression()?;
                self.expect(&TokenKind::Colon, "expected ':' after key in map literal")?;
                let val = self.parse_expression()?;
                entries.push((key, val));
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
                if self.check(&TokenKind::CloseBrace) {
                    break;
                }
            }
        }

        let close = self.expect(&TokenKind::CloseBrace, "expected '}' after map literal")?;
        Some(Expr::MapLiteral {
            entries,
            span: open_span.merge(close.span),
        })
    }

    fn parse_struct_literal(&mut self, name: Ident) -> Option<Expr> {
        let open_brace = self.advance().span; // consume '{'
        let mut fields = Vec::new();
        while !self.check(&TokenKind::CloseBrace) && !self.is_at_end() {
            let fname = if let TokenKind::Ident(_) = self.peek_kind() {
                let tok = self.advance();
                Ident::new(tok.text.clone(), tok.span)
            } else {
                let span = self.peek().span;
                self.diagnostics.error(span, "expected struct field name");
                return None;
            };
            self.expect(&TokenKind::Colon, "expected ':' after struct field name")?;
            let fval = self.parse_expression()?;
            fields.push((fname, fval));
            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }
        let close = self.expect(
            &TokenKind::CloseBrace,
            "expected '}' closing struct literal",
        )?;
        Some(Expr::StructLiteral {
            name,
            fields,
            span: open_brace.merge(close.span),
        })
    }

    fn parse_call_arguments(&mut self, callee: Expr) -> Option<Expr> {
        self.advance(); // consume '('
        let mut args = Vec::new();

        if !self.check(&TokenKind::CloseParen) {
            loop {
                let arg = self.parse_expression()?;
                args.push(arg);
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
            }
        }

        let close = self.expect(&TokenKind::CloseParen, "expected ')' after arguments")?;
        let span = callee.span().merge(close.span);
        Some(Expr::Call {
            callee: Box::new(callee),
            args,
            span,
        })
    }

    fn parse_index_expr(&mut self, target: Expr) -> Option<Expr> {
        self.advance(); // consume '['
        let index = self.parse_expression()?;
        let close = self.expect(&TokenKind::CloseBracket, "expected ']' after index")?;
        let span = target.span().merge(close.span);
        Some(Expr::Index {
            target: Box::new(target),
            index: Box::new(index),
            span,
        })
    }

    fn parse_property_access(&mut self, target: Expr) -> Option<Expr> {
        self.advance(); // consume '.'
        let prop_name = match self.peek_kind() {
            TokenKind::Ident(_) => {
                let token = self.advance();
                Ident::new(token.text.clone(), token.span)
            }
            _ => {
                let span = self.peek().span;
                self.diagnostics
                    .error(span, "expected property name after '.'");
                return None;
            }
        };
        let span = target.span().merge(prop_name.span);
        Some(Expr::PropertyAccess {
            target: Box::new(target),
            property: prop_name,
            span,
        })
    }
}
