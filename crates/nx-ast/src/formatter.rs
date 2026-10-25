use crate::*;
use std::fmt::Write;

/// Canonical Source Code Formatter for NextLang (`nx fmt`).
/// Produces deterministic, idiomatic, indented NextLang source code from an AST.
pub struct SourceFormatter {
    indent_level: usize,
    output: String,
}

impl Default for SourceFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl SourceFormatter {
    pub fn new() -> Self {
        Self {
            indent_level: 0,
            output: String::new(),
        }
    }

    pub fn format_program(mut self, program: &Program) -> String {
        for (i, stmt) in program.statements.iter().enumerate() {
            if i > 0
                && matches!(
                    stmt,
                    Stmt::FunctionDecl(_)
                        | Stmt::ClassDecl(_)
                        | Stmt::InterfaceDecl(_)
                        | Stmt::StructDecl(_)
                        | Stmt::ExternBlock(_)
                        | Stmt::TestDecl(_)
                )
            {
                self.output.push('\n');
            }
            self.format_stmt(stmt);
        }
        if !self.output.ends_with('\n') {
            self.output.push('\n');
        }
        self.output
    }

    fn indent_str(&self) -> String {
        "    ".repeat(self.indent_level)
    }

    fn format_stmt(&mut self, stmt: &Stmt) {
        let ind = self.indent_str();
        match stmt {
            Stmt::VarDecl(decl) => {
                let kw = decl.kind.as_str();
                let init_str = if let Some(ref init) = decl.initializer {
                    format!(" = {}", self.format_expr(init))
                } else {
                    String::new()
                };
                let type_str = if let Some(ref ty) = decl.type_annotation {
                    format!(": {}", ty)
                } else {
                    String::new()
                };
                let _ = writeln!(
                    self.output,
                    "{}{} {}{}{};",
                    ind, kw, decl.name.name, type_str, init_str
                );
            }
            Stmt::Assignment(assign) => {
                let target = self.format_expr(&assign.target);
                let val = self.format_expr(&assign.value);
                let _ = writeln!(self.output, "{}{} {} {};", ind, target, assign.op, val);
            }
            Stmt::Update(up) => {
                let target = self.format_expr(&up.target);
                let op = match up.op {
                    UpdateOp::Increment => "++",
                    UpdateOp::Decrement => "--",
                };
                if up.is_prefix {
                    let _ = writeln!(self.output, "{}{}{};", ind, op, target);
                } else {
                    let _ = writeln!(self.output, "{}{}{};", ind, target, op);
                }
            }
            Stmt::FunctionDecl(f) => {
                let ret = match &f.return_type {
                    Some(t) => format!(" -> {}", t),
                    None => " -> Void".to_string(),
                };
                let params: Vec<String> = f
                    .params
                    .iter()
                    .map(|p| format!("{}: {}", p.name.name, p.type_annotation))
                    .collect();
                let async_prefix = if f.is_async { "async " } else { "" };
                let _ = writeln!(
                    self.output,
                    "{}{}fn {}({}){} {{",
                    ind,
                    async_prefix,
                    f.name.name,
                    params.join(", "),
                    ret
                );
                self.indent_level += 1;
                for s in &f.body.statements {
                    self.format_stmt(s);
                }
                self.indent_level -= 1;
                let _ = writeln!(self.output, "{}}}", ind);
            }
            Stmt::Return(ret) => {
                if let Some(ref val) = ret.value {
                    let _ = writeln!(self.output, "{}return {};", ind, self.format_expr(val));
                } else {
                    let _ = writeln!(self.output, "{}return;", ind);
                }
            }
            Stmt::If(if_stmt) => {
                let cond = self.format_expr(&if_stmt.condition);
                let _ = writeln!(self.output, "{}if {} {{", ind, cond);
                self.indent_level += 1;
                for s in &if_stmt.then_branch.statements {
                    self.format_stmt(s);
                }
                self.indent_level -= 1;

                for ow in &if_stmt.otherwise_branches {
                    let ow_cond = self.format_expr(&ow.condition);
                    let _ = writeln!(self.output, "{}}} otherwise {} {{", ind, ow_cond);
                    self.indent_level += 1;
                    for s in &ow.body.statements {
                        self.format_stmt(s);
                    }
                    self.indent_level -= 1;
                }

                if let Some(ref else_block) = if_stmt.else_branch {
                    let _ = writeln!(self.output, "{}}} else {{", ind);
                    self.indent_level += 1;
                    for s in &else_block.statements {
                        self.format_stmt(s);
                    }
                    self.indent_level -= 1;
                }
                let _ = writeln!(self.output, "{}}}", ind);
            }
            Stmt::AsLoop(as_loop) => {
                let cond = self.format_expr(&as_loop.condition);
                let _ = writeln!(self.output, "{}As {} {{", ind, cond);
                self.indent_level += 1;
                for s in &as_loop.body.statements {
                    self.format_stmt(s);
                }
                self.indent_level -= 1;
                let _ = writeln!(self.output, "{}}}", ind);
            }
            Stmt::ForLoop(fr_loop) => {
                let init_str = if let Some(ref init) = fr_loop.init {
                    let mut tmp = SourceFormatter::new();
                    tmp.format_stmt(init);
                    tmp.output.trim_end_matches([';', '\n', ' ']).to_string()
                } else {
                    String::new()
                };
                let cond_str = if let Some(ref cond) = fr_loop.condition {
                    self.format_expr(cond)
                } else {
                    String::new()
                };
                let up_str = if let Some(ref update) = fr_loop.update {
                    let mut tmp = SourceFormatter::new();
                    tmp.format_stmt(update);
                    tmp.output.trim_end_matches([';', '\n', ' ']).to_string()
                } else {
                    String::new()
                };

                let _ = writeln!(
                    self.output,
                    "{}fr<{} : {} : {}> {{",
                    ind, init_str, cond_str, up_str
                );
                self.indent_level += 1;
                for s in &fr_loop.body.statements {
                    self.format_stmt(s);
                }
                self.indent_level -= 1;
                let _ = writeln!(self.output, "{}}}", ind);
            }
            Stmt::DoAsLoop(do_as) => {
                let _ = writeln!(self.output, "{}Do {{", ind);
                self.indent_level += 1;
                for s in &do_as.body.statements {
                    self.format_stmt(s);
                }
                self.indent_level -= 1;
                let cond = self.format_expr(&do_as.condition);
                let _ = writeln!(self.output, "{}}} As {};", ind, cond);
            }
            Stmt::Block(block) => {
                let _ = writeln!(self.output, "{}{{", ind);
                self.indent_level += 1;
                for s in &block.statements {
                    self.format_stmt(s);
                }
                self.indent_level -= 1;
                let _ = writeln!(self.output, "{}}}", ind);
            }
            Stmt::Expr(expr) => {
                let _ = writeln!(self.output, "{}{};", ind, self.format_expr(expr));
            }
            Stmt::ClassDecl(c) => {
                let abst = if c.is_abstract { "abstract " } else { "" };
                let fin = if c.is_final { "final " } else { "" };
                let ext = if let Some(ref p) = c.parent {
                    format!(" extends {}", p.name)
                } else {
                    String::new()
                };
                let imp = if !c.implements.is_empty() {
                    let ifaces: Vec<String> = c.implements.iter().map(|i| i.name.clone()).collect();
                    format!(" implements {}", ifaces.join(", "))
                } else {
                    String::new()
                };
                let _ = writeln!(
                    self.output,
                    "{}{}{}class {}{}{} {{",
                    ind, abst, fin, c.name.name, ext, imp
                );
                self.indent_level += 1;
                for m in &c.members {
                    match m {
                        ClassMember::Field(f) => {
                            let stat = if f.is_static { "static " } else { "" };
                            let init_str = if let Some(ref init) = f.var_decl.initializer {
                                format!(" = {}", self.format_expr(init))
                            } else {
                                String::new()
                            };
                            let type_str = if let Some(ref ty) = f.var_decl.type_annotation {
                                format!(": {}", ty)
                            } else {
                                String::new()
                            };
                            let _ = writeln!(
                                self.output,
                                "{}{} {}{} {}{}{};",
                                self.indent_str(),
                                f.access,
                                stat,
                                f.var_decl.kind.as_str(),
                                f.var_decl.name.name,
                                type_str,
                                init_str
                            );
                        }
                        ClassMember::Method(m) => {
                            let stat = if m.is_static { "static " } else { "" };
                            let abst = if m.is_abstract { "abstract " } else { "" };
                            let fin = if m.is_final { "final " } else { "" };
                            let asyn = if m.is_async || m.function.is_async {
                                "async "
                            } else {
                                ""
                            };
                            let ret = match &m.function.return_type {
                                Some(t) => format!(" -> {}", t),
                                None => " -> Void".to_string(),
                            };
                            let params: Vec<String> = m
                                .function
                                .params
                                .iter()
                                .map(|p| format!("{}: {}", p.name.name, p.type_annotation))
                                .collect();
                            if m.is_abstract {
                                let _ = writeln!(
                                    self.output,
                                    "{}{} {}{}{}{}fn {}({}){};",
                                    self.indent_str(),
                                    m.access,
                                    stat,
                                    abst,
                                    fin,
                                    asyn,
                                    m.function.name.name,
                                    params.join(", "),
                                    ret
                                );
                            } else {
                                let _ = writeln!(
                                    self.output,
                                    "{}{} {}{}{}{}fn {}({}){} {{",
                                    self.indent_str(),
                                    m.access,
                                    stat,
                                    abst,
                                    fin,
                                    asyn,
                                    m.function.name.name,
                                    params.join(", "),
                                    ret
                                );
                                self.indent_level += 1;
                                for s in &m.function.body.statements {
                                    self.format_stmt(s);
                                }
                                self.indent_level -= 1;
                                let _ = writeln!(self.output, "{}}}", self.indent_str());
                            }
                        }
                    }
                }
                self.indent_level -= 1;
                let _ = writeln!(self.output, "{}}}", ind);
            }
            Stmt::InterfaceDecl(iface) => {
                let _ = writeln!(self.output, "{}interface {} {{", ind, iface.name.name);
                self.indent_level += 1;
                for m in &iface.methods {
                    let ret = match &m.return_type {
                        Some(t) => format!(" -> {}", t),
                        None => " -> Void".to_string(),
                    };
                    let params: Vec<String> = m
                        .params
                        .iter()
                        .map(|p| format!("{}: {}", p.name.name, p.type_annotation))
                        .collect();
                    let _ = writeln!(
                        self.output,
                        "{}fn {}({}){};",
                        self.indent_str(),
                        m.name.name,
                        params.join(", "),
                        ret
                    );
                }
                self.indent_level -= 1;
                let _ = writeln!(self.output, "{}}}", ind);
            }
            Stmt::StructDecl(s) => {
                let _ = writeln!(self.output, "{}struct {} {{", ind, s.name.name);
                self.indent_level += 1;
                for (i, f) in s.fields.iter().enumerate() {
                    let comma = if i + 1 < s.fields.len() { "," } else { "" };
                    let _ = writeln!(
                        self.output,
                        "{}{}: {}{}",
                        self.indent_str(),
                        f.name.name,
                        f.field_type,
                        comma
                    );
                }
                self.indent_level -= 1;
                let _ = writeln!(self.output, "{}}}", ind);
            }
            Stmt::Unsafe(u) => {
                let _ = writeln!(self.output, "{}unsafe {{", ind);
                self.indent_level += 1;
                for s in &u.body.statements {
                    self.format_stmt(s);
                }
                self.indent_level -= 1;
                let _ = writeln!(self.output, "{}}}", ind);
            }
            Stmt::ExternBlock(e) => {
                let _ = writeln!(self.output, "{}extern \"{}\" {{", ind, e.abi);
                self.indent_level += 1;
                for f in &e.functions {
                    let ret = match &f.return_type {
                        Some(t) => format!(" -> {}", t),
                        None => " -> Void".to_string(),
                    };
                    let params: Vec<String> = f
                        .params
                        .iter()
                        .map(|p| format!("{}: {}", p.name.name, p.type_annotation))
                        .collect();
                    let _ = writeln!(
                        self.output,
                        "{}fn {}({}){};",
                        self.indent_str(),
                        f.name.name,
                        params.join(", "),
                        ret
                    );
                }
                self.indent_level -= 1;
                let _ = writeln!(self.output, "{}}}", ind);
            }
            Stmt::Match(m) => {
                let target = self.format_expr(&m.target);
                let _ = writeln!(self.output, "{}match {} {{", ind, target);
                self.indent_level += 1;
                for case in &m.cases {
                    let pat = self.format_pattern(&case.pattern);
                    let guard_str = if let Some(ref g) = case.guard {
                        format!(" if {}", self.format_expr(g))
                    } else {
                        String::new()
                    };
                    let _ = writeln!(
                        self.output,
                        "{}case {}{} => {{",
                        self.indent_str(),
                        pat,
                        guard_str
                    );
                    self.indent_level += 1;
                    for s in &case.body.statements {
                        self.format_stmt(s);
                    }
                    self.indent_level -= 1;
                    let _ = writeln!(self.output, "{}}}", self.indent_str());
                }
                self.indent_level -= 1;
                let _ = writeln!(self.output, "{}}}", ind);
            }
            Stmt::Import(imp) => {
                if imp.path.contains('/') || imp.path.contains('.') {
                    let _ = writeln!(self.output, "{}import \"{}\";", ind, imp.path);
                } else {
                    let _ = writeln!(self.output, "{}import {};", ind, imp.path);
                }
            }
            Stmt::TestDecl(t) => {
                let _ = writeln!(self.output, "{}test \"{}\" {{", ind, t.name);
                self.indent_level += 1;
                for s in &t.body.statements {
                    self.format_stmt(s);
                }
                self.indent_level -= 1;
                let _ = writeln!(self.output, "{}}}", ind);
            }
        }
    }

    fn format_pattern(&self, pat: &Pattern) -> String {
        match pat {
            Pattern::Literal(lit) => self.format_literal(lit),
            Pattern::Binding(id) => id.name.clone(),
            Pattern::Wildcard(_) => "_".to_string(),
            Pattern::Alternative(alts, _) => {
                let parts: Vec<String> = alts.iter().map(|p| self.format_pattern(p)).collect();
                parts.join(" | ")
            }
        }
    }

    fn format_literal(&self, lit: &Literal) -> String {
        match lit {
            Literal::Integer { value, .. } => format!("{}", value),
            Literal::Float { raw, .. } => raw.clone(),
            Literal::String { value, .. } => format!("\"{}\"", value),
            Literal::Char { value, .. } => format!("'{}'", value),
            Literal::Bool { value, .. } => format!("{}", value),
            Literal::Null { .. } => "null".to_string(),
        }
    }

    pub fn format_expr(&self, expr: &Expr) -> String {
        match expr {
            Expr::Literal(lit) => self.format_literal(lit),
            Expr::Identifier(id) => id.name.clone(),
            Expr::Unary { op, operand, .. } => {
                let op_str = match op {
                    UnaryOp::Neg => "-",
                    UnaryOp::Pos => "+",
                    UnaryOp::Not => "not ",
                    UnaryOp::BitNot => "~",
                };
                format!("{}{}", op_str, self.format_expr(operand))
            }
            Expr::Binary {
                op, left, right, ..
            } => {
                let l = self.format_expr(left);
                let r = self.format_expr(right);
                if *op == BinaryOp::Range {
                    format!("{}..{}", l, r)
                } else {
                    format!("{} {} {}", l, op.as_str(), r)
                }
            }
            Expr::Update {
                op,
                target,
                is_prefix,
                ..
            } => {
                let t = self.format_expr(target);
                let op_str = match op {
                    UpdateOp::Increment => "++",
                    UpdateOp::Decrement => "--",
                };
                if *is_prefix {
                    format!("{}{}", op_str, t)
                } else {
                    format!("{}{}", t, op_str)
                }
            }
            Expr::Call { callee, args, .. } => {
                let c = self.format_expr(callee);
                let arg_strs: Vec<String> = args.iter().map(|a| self.format_expr(a)).collect();
                format!("{}({})", c, arg_strs.join(", "))
            }
            Expr::Index { target, index, .. } => {
                let t = self.format_expr(target);
                let i = self.format_expr(index);
                format!("{}[{}]", t, i)
            }
            Expr::PropertyAccess {
                target, property, ..
            } => {
                let t = self.format_expr(target);
                format!("{}.{}", t, property.name)
            }
            Expr::ListLiteral { elements, .. } => {
                let elem_strs: Vec<String> = elements.iter().map(|e| self.format_expr(e)).collect();
                format!("[{}]", elem_strs.join(", "))
            }
            Expr::MapLiteral { entries, .. } => {
                let entry_strs: Vec<String> = entries
                    .iter()
                    .map(|(k, v)| format!("{}: {}", self.format_expr(k), self.format_expr(v)))
                    .collect();
                format!("{{{}}}", entry_strs.join(", "))
            }
            Expr::Group { expr, .. } => {
                format!("({})", self.format_expr(expr))
            }
            Expr::Borrow { expr, is_mut, .. } => {
                format!(
                    "&{}{}",
                    if *is_mut { "mut " } else { "" },
                    self.format_expr(expr)
                )
            }
            Expr::Alloc { expr, .. } => {
                format!("alloc({})", self.format_expr(expr))
            }
            Expr::Free { target, .. } => {
                format!("free({})", self.format_expr(target))
            }
            Expr::StructLiteral { name, fields, .. } => {
                let field_strs: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k.name, self.format_expr(v)))
                    .collect();
                format!("{} {{ {} }}", name.name, field_strs.join(", "))
            }
            Expr::Await { expr, .. } => {
                format!("await {}", self.format_expr(expr))
            }
        }
    }
}

/// Convenience function to format a NextLang AST Program into canonical source code.
pub fn format_source(program: &Program) -> String {
    let formatter = SourceFormatter::new();
    formatter.format_program(program)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_import_and_test() {
        let program = Program {
            statements: vec![
                Stmt::Import(ImportStmt {
                    path: "math".to_string(),
                    span: Span::new(0, 0),
                }),
                Stmt::TestDecl(TestDecl {
                    name: "basic addition".to_string(),
                    body: Block {
                        statements: vec![Stmt::VarDecl(VarDecl {
                            kind: VarKind::Var,
                            name: Ident::new("x", Span::new(0, 0)),
                            type_annotation: Some(Type::Named {
                                name: "INT".to_string(),
                                span: Span::new(0, 0),
                            }),
                            initializer: Some(Expr::Literal(Literal::Integer {
                                value: 42,
                                raw: "42".to_string(),
                                span: Span::new(0, 0),
                            })),
                            span: Span::new(0, 0),
                        })],
                        span: Span::new(0, 0),
                    },
                    span: Span::new(0, 0),
                }),
            ],
            span: Span::new(0, 0),
        };

        let formatted = format_source(&program);
        assert!(formatted.contains("import math;"));
        assert!(formatted.contains("test \"basic addition\" {"));
        assert!(formatted.contains("var x: INT = 42;"));
    }
}
