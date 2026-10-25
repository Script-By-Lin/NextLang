use nx_ast::*;
use nx_diagnostics::{DiagnosticBag, Span};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Var,
    Const,
    Tem,
    Param,
    Function,
    Builtin,
    Class,
    Interface,
    Struct,
}

impl SymbolKind {
    pub fn is_constant(&self) -> bool {
        matches!(
            self,
            SymbolKind::Const | SymbolKind::Function | SymbolKind::Builtin | SymbolKind::Struct
        )
    }
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub span: Span,
}

pub struct ScopeChecker<'a> {
    scopes: Vec<HashMap<String, Symbol>>,
    diagnostics: &'a mut DiagnosticBag,
}

impl<'a> ScopeChecker<'a> {
    pub fn new(diagnostics: &'a mut DiagnosticBag) -> Self {
        let mut root_scope = HashMap::new();
        for builtin in [
            "vout",
            "len",
            "input",
            "to_int",
            "to_float",
            "file_open",
            "file_read_all",
            "file_read_str",
            "file_write",
            "file_close",
            "file_exists",
            "file_remove",
            "env_get",
            "env_set",
            "get_cwd",
            "get_pid",
            "buffer_new",
            "buffer_free",
            "buffer_size",
            "buffer_get_u8",
            "buffer_set_u8",
            "buffer_get_u32_le",
            "buffer_set_u32_le",
            "ptr_offset",
            "ptr_read_u8",
            "ptr_write_u8",
            // Phase 9 Standard Library builtins
            "math_abs",
            "math_sqrt",
            "math_pow",
            "math_floor",
            "math_ceil",
            "math_min",
            "math_max",
            "math_sin",
            "math_cos",
            "math_random",
            "int_abs",
            "str_len",
            "str_upper",
            "str_lower",
            "str_trim",
            "str_sub",
            "str_contains",
            "str_starts_with",
            "str_ends_with",
            "time_now_ms",
            "time_sleep_ms",
            "hex_encode",
            "hex_decode",
            "base64_encode",
            "base64_decode",
            "json_parse",
            "json_stringify",
            "assert",
            "assert_eq",
        ] {
            root_scope.insert(
                builtin.to_string(),
                Symbol {
                    name: builtin.to_string(),
                    kind: SymbolKind::Builtin,
                    span: Span::empty(0),
                },
            );
        }

        Self {
            scopes: vec![root_scope],
            diagnostics,
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn insert_symbol(&mut self, symbol: Symbol) {
        let current_scope = self.scopes.last_mut().expect("at least one scope exists");
        if let Some(existing) = current_scope.get(&symbol.name) {
            self.diagnostics.error_with_help(
                symbol.span,
                format!(
                    "identifier '{}' is already declared in this scope",
                    symbol.name
                ),
                format!("previously declared here at {}", existing.span),
            );
        } else {
            current_scope.insert(symbol.name.clone(), symbol);
        }
    }

    fn lookup_symbol(&self, name: &str) -> Option<&Symbol> {
        for scope in self.scopes.iter().rev() {
            if let Some(sym) = scope.get(name) {
                return Some(sym);
            }
        }
        None
    }

    pub fn check_program(&mut self, program: &Program) {
        for stmt in &program.statements {
            self.check_stmt(stmt);
        }
    }

    fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::VarDecl(decl) => {
                let kind = match decl.kind {
                    VarKind::Var => SymbolKind::Var,
                    VarKind::Const => SymbolKind::Const,
                    VarKind::Tem => SymbolKind::Tem,
                };
                if let Some(ref init) = decl.initializer {
                    self.check_expr(init);
                }
                self.insert_symbol(Symbol {
                    name: decl.name.name.clone(),
                    kind,
                    span: decl.name.span,
                });
            }
            Stmt::Assignment(assign) => {
                self.check_assignment_target(&assign.target);
                self.check_expr(&assign.value);
            }
            Stmt::Update(up) => {
                self.check_update_target(&up.target);
            }
            Stmt::FunctionDecl(f) => {
                self.insert_symbol(Symbol {
                    name: f.name.name.clone(),
                    kind: SymbolKind::Function,
                    span: f.name.span,
                });
                self.push_scope();
                for param in &f.params {
                    self.insert_symbol(Symbol {
                        name: param.name.name.clone(),
                        kind: SymbolKind::Param,
                        span: param.name.span,
                    });
                }
                for s in &f.body.statements {
                    self.check_stmt(s);
                }
                self.pop_scope();
            }
            Stmt::Return(ret) => {
                if let Some(ref val) = ret.value {
                    self.check_expr(val);
                }
            }
            Stmt::If(if_stmt) => {
                self.check_expr(&if_stmt.condition);
                self.check_block(&if_stmt.then_branch);
                for ow in &if_stmt.otherwise_branches {
                    self.check_expr(&ow.condition);
                    self.check_block(&ow.body);
                }
                if let Some(ref else_block) = if_stmt.else_branch {
                    self.check_block(else_block);
                }
            }
            Stmt::AsLoop(as_loop) => {
                self.check_expr(&as_loop.condition);
                self.check_block(&as_loop.body);
            }
            Stmt::ForLoop(for_loop) => {
                self.push_scope();
                if let Some(ref init) = for_loop.init {
                    self.check_stmt(init);
                }
                if let Some(ref cond) = for_loop.condition {
                    self.check_expr(cond);
                }
                if let Some(ref update) = for_loop.update {
                    self.check_stmt(update);
                }
                for s in &for_loop.body.statements {
                    self.check_stmt(s);
                }
                self.pop_scope();
            }
            Stmt::DoAsLoop(do_as) => {
                self.check_block(&do_as.body);
                self.check_expr(&do_as.condition);
            }
            Stmt::Block(block) => {
                self.check_block(block);
            }
            Stmt::Expr(expr) => {
                self.check_expr(expr);
            }
            Stmt::ClassDecl(c) => {
                self.insert_symbol(Symbol {
                    name: c.name.name.clone(),
                    kind: SymbolKind::Class,
                    span: c.name.span,
                });
                for member in &c.members {
                    match member {
                        ClassMember::Field(f) => {
                            if let Some(ref init) = f.var_decl.initializer {
                                self.check_expr(init);
                            }
                        }
                        ClassMember::Method(m) => {
                            self.push_scope();
                            if !m.is_static {
                                self.insert_symbol(Symbol {
                                    name: "self".to_string(),
                                    kind: SymbolKind::Param,
                                    span: m.span,
                                });
                                if c.parent.is_some() {
                                    self.insert_symbol(Symbol {
                                        name: "super".to_string(),
                                        kind: SymbolKind::Param,
                                        span: m.span,
                                    });
                                }
                            }
                            for param in &m.function.params {
                                self.insert_symbol(Symbol {
                                    name: param.name.name.clone(),
                                    kind: SymbolKind::Param,
                                    span: param.name.span,
                                });
                            }
                            for s in &m.function.body.statements {
                                self.check_stmt(s);
                            }
                            self.pop_scope();
                        }
                    }
                }
            }
            Stmt::InterfaceDecl(i) => {
                self.insert_symbol(Symbol {
                    name: i.name.name.clone(),
                    kind: SymbolKind::Interface,
                    span: i.name.span,
                });
            }
            Stmt::Match(m) => {
                self.check_expr(&m.target);
                for case in &m.cases {
                    self.push_scope();
                    self.bind_pattern(&case.pattern);
                    if let Some(ref guard) = case.guard {
                        self.check_expr(guard);
                    }
                    for s in &case.body.statements {
                        self.check_stmt(s);
                    }
                    self.pop_scope();
                }
            }
            Stmt::StructDecl(s) => {
                self.insert_symbol(Symbol {
                    name: s.name.name.clone(),
                    kind: SymbolKind::Struct,
                    span: s.name.span,
                });
            }
            Stmt::Unsafe(u) => {
                self.check_block(&u.body);
            }
            Stmt::ExternBlock(e) => {
                for f in &e.functions {
                    self.insert_symbol(Symbol {
                        name: f.name.name.clone(),
                        kind: SymbolKind::Function,
                        span: f.name.span,
                    });
                }
            }
            Stmt::Import(_) => {}
            Stmt::TestDecl(t) => {
                self.check_block(&t.body);
            }
        }
    }

    fn bind_pattern(&mut self, pattern: &Pattern) {
        match pattern {
            Pattern::Binding(ident) => {
                self.insert_symbol(Symbol {
                    name: ident.name.clone(),
                    kind: SymbolKind::Var,
                    span: ident.span,
                });
            }
            Pattern::Alternative(alts, _) => {
                for alt in alts {
                    self.bind_pattern(alt);
                }
            }
            _ => {}
        }
    }

    fn check_block(&mut self, block: &Block) {
        self.push_scope();
        for stmt in &block.statements {
            self.check_stmt(stmt);
        }
        self.pop_scope();
    }

    fn check_assignment_target(&mut self, target: &Expr) {
        match target {
            Expr::Identifier(ident) => match self.lookup_symbol(&ident.name) {
                Some(sym) => {
                    if sym.kind.is_constant() {
                        self.diagnostics.error_with_help(
                            ident.span,
                            format!("cannot reassign constant '{}'", ident.name),
                            format!(
                                "'{}' was declared as constant and cannot be reassigned",
                                ident.name
                            ),
                        );
                    }
                }
                None => {
                    self.diagnostics.error_with_help(
                            ident.span,
                            format!("undeclared variable '{}'", ident.name),
                            format!(
                                "variable '{}' must be declared with 'var', 'const', or 'tem' before assignment",
                                ident.name
                            ),
                        );
                }
            },
            Expr::Index { target, index, .. } => {
                self.check_expr(target);
                self.check_expr(index);
            }
            Expr::PropertyAccess { target, .. } => {
                self.check_expr(target);
            }
            other => {
                self.diagnostics
                    .error(other.span(), "invalid assignment target");
            }
        }
    }

    fn check_update_target(&mut self, target: &Expr) {
        match target {
            Expr::Identifier(ident) => match self.lookup_symbol(&ident.name) {
                Some(sym) => {
                    if sym.kind.is_constant() {
                        self.diagnostics.error_with_help(
                            ident.span,
                            format!("cannot mutate constant '{}'", ident.name),
                            format!(
                                "'{}' was declared as constant and cannot be mutated",
                                ident.name
                            ),
                        );
                    }
                }
                None => {
                    self.diagnostics.error_with_help(
                            ident.span,
                            format!("undeclared variable '{}'", ident.name),
                            format!(
                                "variable '{}' must be declared with 'var', 'const', or 'tem' before update",
                                ident.name
                            ),
                        );
                }
            },
            Expr::Index { target, index, .. } => {
                self.check_expr(target);
                self.check_expr(index);
            }
            other => {
                self.diagnostics
                    .error(other.span(), "invalid update target");
            }
        }
    }

    fn check_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Literal(_) => {}
            Expr::Identifier(ident) => {
                if self.lookup_symbol(&ident.name).is_none() {
                    self.diagnostics.error_with_help(
                        ident.span,
                        format!("undeclared identifier '{}'", ident.name),
                        format!("'{}' has not been declared in this scope", ident.name),
                    );
                }
            }
            Expr::Unary { operand, .. } => {
                self.check_expr(operand);
            }
            Expr::Binary { left, right, .. } => {
                self.check_expr(left);
                self.check_expr(right);
            }
            Expr::Update { target, .. } => {
                self.check_update_target(target);
            }
            Expr::Call { callee, args, .. } => {
                self.check_expr(callee);
                for arg in args {
                    self.check_expr(arg);
                }
            }
            Expr::Index { target, index, .. } => {
                self.check_expr(target);
                self.check_expr(index);
            }
            Expr::PropertyAccess { target, .. } => {
                self.check_expr(target);
            }
            Expr::ListLiteral { elements, .. } => {
                for el in elements {
                    self.check_expr(el);
                }
            }
            Expr::MapLiteral { entries, .. } => {
                for (k, v) in entries {
                    self.check_expr(k);
                    self.check_expr(v);
                }
            }
            Expr::Group { expr, .. } => {
                self.check_expr(expr);
            }
            Expr::Borrow { expr, .. } => {
                self.check_expr(expr);
            }
            Expr::Alloc { expr, .. } => {
                self.check_expr(expr);
            }
            Expr::Free { target, .. } => {
                self.check_expr(target);
            }
            Expr::StructLiteral { name, fields, .. } => {
                if self.lookup_symbol(&name.name).is_none() {
                    self.diagnostics.error_with_help(
                        name.span,
                        format!("undeclared struct '{}'", name.name),
                        format!("struct '{}' has not been declared in this scope", name.name),
                    );
                }
                for (_, val) in fields {
                    self.check_expr(val);
                }
            }
        }
    }
}
