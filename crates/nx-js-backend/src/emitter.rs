use crate::config::CompilerConfig;
use crate::runtime_helpers::get_runtime_helpers;
use nx_ast::*;
use std::collections::HashSet;

pub struct JsEmitter {
    config: CompilerConfig,
    classes: HashSet<String>,
    functions: HashSet<String>,
    indent_level: usize,
    target_counter: usize,
    has_main: bool,
    output: String,
}

impl JsEmitter {
    pub fn new(config: CompilerConfig) -> Self {
        Self {
            config,
            classes: HashSet::new(),
            functions: HashSet::new(),
            indent_level: 0,
            target_counter: 0,
            has_main: false,
            output: String::new(),
        }
    }

    pub fn emit_program(mut self, program: &Program) -> String {
        // Collect class and function names first
        for stmt in &program.statements {
            match stmt {
                Stmt::ClassDecl(c) => {
                    self.classes.insert(c.name.name.clone());
                }
                Stmt::FunctionDecl(f) => {
                    self.functions.insert(f.name.name.clone());
                    if f.name.name == "main" {
                        self.has_main = true;
                    }
                }
                _ => {}
            }
        }

        // Emit header comment
        self.output
            .push_str("// Compiled by NextLang JavaScript Compiler (nx-js-backend)\n");
        self.output.push_str("\"use strict\";\n\n");

        // Emit runtime helpers if enabled
        if self.config.emit_runtime_helpers {
            self.output
                .push_str(&get_runtime_helpers(self.config.target));
            self.output.push('\n');
        }

        // Emit top-level statements
        for stmt in &program.statements {
            self.emit_stmt(stmt);
        }

        // Emit automatic main() call if present and configured
        if self.has_main && self.config.auto_invoke_main {
            self.output.push_str("\n// Automatic main entry point\n");
            self.output
                .push_str("if (typeof main === 'function') {\n    main();\n}\n");
        }

        self.output
    }

    fn indent_str(&self) -> String {
        "    ".repeat(self.indent_level)
    }

    fn emit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::VarDecl(decl) => {
                let kw = if decl.kind == VarKind::Const {
                    "const"
                } else {
                    "let"
                };
                let indent = self.indent_str();
                if let Some(ref init) = decl.initializer {
                    let init_js = self.emit_expr_to_string(init);
                    self.output.push_str(&format!(
                        "{}{} {} = {};\n",
                        indent, kw, decl.name.name, init_js
                    ));
                } else {
                    self.output
                        .push_str(&format!("{}{} {} = null;\n", indent, kw, decl.name.name));
                }
            }
            Stmt::Assignment(assign) => {
                let indent = self.indent_str();
                let target_js = self.emit_expr_to_string(&assign.target);
                let val_js = self.emit_expr_to_string(&assign.value);
                let op_str = match assign.op {
                    AssignOp::Assign => "=",
                    AssignOp::AddAssign => "+=",
                    AssignOp::SubAssign => "-=",
                    AssignOp::MulAssign => "*=",
                    AssignOp::DivAssign => "/=",
                    AssignOp::ModAssign => "%=",
                };
                self.output
                    .push_str(&format!("{}{} {} {};\n", indent, target_js, op_str, val_js));
            }
            Stmt::Update(up) => {
                let indent = self.indent_str();
                let target_js = self.emit_expr_to_string(&up.target);
                let op_str = match up.op {
                    UpdateOp::Increment => "++",
                    UpdateOp::Decrement => "--",
                };
                if up.is_prefix {
                    self.output
                        .push_str(&format!("{}{}{};\n", indent, op_str, target_js));
                } else {
                    self.output
                        .push_str(&format!("{}{}{};\n", indent, target_js, op_str));
                }
            }
            Stmt::FunctionDecl(f) => {
                self.emit_function(f);
            }
            Stmt::Return(ret) => {
                let indent = self.indent_str();
                if let Some(ref val) = ret.value {
                    let val_js = self.emit_expr_to_string(val);
                    self.output
                        .push_str(&format!("{}return {};\n", indent, val_js));
                } else {
                    self.output.push_str(&format!("{}return;\n", indent));
                }
            }
            Stmt::If(if_stmt) => {
                let indent = self.indent_str();
                let cond_js = self.emit_expr_to_string(&if_stmt.condition);
                self.output
                    .push_str(&format!("{}if ({}) {{\n", indent, cond_js));
                self.indent_level += 1;
                for s in &if_stmt.then_branch.statements {
                    self.emit_stmt(s);
                }
                self.indent_level -= 1;

                for otherwise in &if_stmt.otherwise_branches {
                    let ow_cond = self.emit_expr_to_string(&otherwise.condition);
                    self.output
                        .push_str(&format!("{}}} else if ({}) {{\n", indent, ow_cond));
                    self.indent_level += 1;
                    for s in &otherwise.body.statements {
                        self.emit_stmt(s);
                    }
                    self.indent_level -= 1;
                }

                if let Some(ref else_b) = if_stmt.else_branch {
                    self.output.push_str(&format!("{}}} else {{\n", indent));
                    self.indent_level += 1;
                    for s in &else_b.statements {
                        self.emit_stmt(s);
                    }
                    self.indent_level -= 1;
                }
                self.output.push_str(&format!("{}}}\n", indent));
            }
            Stmt::AsLoop(as_loop) => {
                let indent = self.indent_str();
                let cond_js = self.emit_expr_to_string(&as_loop.condition);
                self.output
                    .push_str(&format!("{}while ({}) {{\n", indent, cond_js));
                self.indent_level += 1;
                for s in &as_loop.body.statements {
                    self.emit_stmt(s);
                }
                self.indent_level -= 1;
                self.output.push_str(&format!("{}}}\n", indent));
            }
            Stmt::ForLoop(for_loop) => {
                let indent = self.indent_str();
                let init_js = if let Some(ref init) = for_loop.init {
                    let mut tmp = self.stmt_to_for_header(init);
                    if tmp.ends_with(';') {
                        tmp.pop();
                    }
                    tmp
                } else {
                    String::new()
                };
                let cond_js = if let Some(ref cond) = for_loop.condition {
                    self.emit_expr_to_string(cond)
                } else {
                    String::new()
                };
                let update_js = if let Some(ref up) = for_loop.update {
                    let mut tmp = self.stmt_to_for_header(up);
                    if tmp.ends_with(';') {
                        tmp.pop();
                    }
                    tmp
                } else {
                    String::new()
                };

                self.output.push_str(&format!(
                    "{}for ({}; {}; {}) {{\n",
                    indent, init_js, cond_js, update_js
                ));
                self.indent_level += 1;
                for s in &for_loop.body.statements {
                    self.emit_stmt(s);
                }
                self.indent_level -= 1;
                self.output.push_str(&format!("{}}}\n", indent));
            }
            Stmt::DoAsLoop(do_as) => {
                let indent = self.indent_str();
                let cond_js = self.emit_expr_to_string(&do_as.condition);
                self.output.push_str(&format!("{}do {{\n", indent));
                self.indent_level += 1;
                for s in &do_as.body.statements {
                    self.emit_stmt(s);
                }
                self.indent_level -= 1;
                self.output
                    .push_str(&format!("{}}} while ({});\n", indent, cond_js));
            }
            Stmt::Block(b) => {
                let indent = self.indent_str();
                self.output.push_str(&format!("{}{{\n", indent));
                self.indent_level += 1;
                for s in &b.statements {
                    self.emit_stmt(s);
                }
                self.indent_level -= 1;
                self.output.push_str(&format!("{}}}\n", indent));
            }
            Stmt::Expr(e) => {
                let indent = self.indent_str();
                let e_js = self.emit_expr_to_string(e);
                self.output.push_str(&format!("{}{};\n", indent, e_js));
            }
            Stmt::ClassDecl(c) => {
                self.emit_class(c);
            }
            Stmt::InterfaceDecl(_) => {
                // Interfaces are compile-time type contracts in NextLang; no JavaScript emission required
            }
            Stmt::Match(m) => {
                self.emit_match(m);
            }
            Stmt::StructDecl(s) => {
                let indent = self.indent_str();
                self.output
                    .push_str(&format!("{}class {} {{\n", indent, s.name.name));
                self.output.push_str(&format!(
                    "{}  constructor(fields = {{}}) {{ Object.assign(this, fields); }}\n",
                    indent
                ));
                self.output.push_str(&format!("{}}}\n", indent));
            }
            Stmt::Unsafe(u) => {
                let indent = self.indent_str();
                self.output.push_str(&format!("{}{{\n", indent));
                self.indent_level += 1;
                for s in &u.body.statements {
                    self.emit_stmt(s);
                }
                self.indent_level -= 1;
                self.output.push_str(&format!("{}}}\n", indent));
            }
            Stmt::ExternBlock(_) => {
                // Foreign C declarations; no-op in JS emitter
            }
            Stmt::Import(_) => {
                // Imports are resolved before JS emission
            }
            Stmt::TestDecl(t) => {
                let indent = self.indent_str();
                self.output
                    .push_str(&format!("{}// test: {}\n", indent, t.name));
                self.output.push_str(&format!("{}{{\n", indent));
                self.indent_level += 1;
                for s in &t.body.statements {
                    self.emit_stmt(s);
                }
                self.indent_level -= 1;
                self.output.push_str(&format!("{}}}\n", indent));
            }
        }
    }

    fn stmt_to_for_header(&mut self, stmt: &Stmt) -> String {
        match stmt {
            Stmt::VarDecl(decl) => {
                let kw = if decl.kind == VarKind::Const {
                    "const"
                } else {
                    "let"
                };
                if let Some(ref init) = decl.initializer {
                    format!(
                        "{} {} = {}",
                        kw,
                        decl.name.name,
                        self.emit_expr_to_string(init)
                    )
                } else {
                    format!("{} {} = null", kw, decl.name.name)
                }
            }
            Stmt::Assignment(assign) => {
                let target = self.emit_expr_to_string(&assign.target);
                let val = self.emit_expr_to_string(&assign.value);
                format!("{} = {}", target, val)
            }
            Stmt::Update(up) => {
                let target = self.emit_expr_to_string(&up.target);
                let op = match up.op {
                    UpdateOp::Increment => "++",
                    UpdateOp::Decrement => "--",
                };
                if up.is_prefix {
                    format!("{}{}", op, target)
                } else {
                    format!("{}{}", target, op)
                }
            }
            Stmt::Expr(e) => self.emit_expr_to_string(e),
            _ => String::new(),
        }
    }

    fn emit_function(&mut self, f: &FunctionDecl) {
        let indent = self.indent_str();
        let params: Vec<String> = f.params.iter().map(|p| p.name.name.clone()).collect();
        let params_str = params.join(", ");

        self.output.push_str(&format!(
            "{}function {}({}) {{\n",
            indent, f.name.name, params_str
        ));
        self.indent_level += 1;
        for s in &f.body.statements {
            self.emit_stmt(s);
        }
        self.indent_level -= 1;
        self.output.push_str(&format!("{}}}\n\n", indent));
    }

    fn emit_class(&mut self, c: &ClassDecl) {
        let indent = self.indent_str();
        let heritage = if let Some(ref parent) = c.parent {
            format!(" extends {}", parent.name)
        } else {
            String::new()
        };

        self.output
            .push_str(&format!("{}class {}{} {{\n", indent, c.name.name, heritage));
        self.indent_level += 1;

        // Collect fields and constructor
        let mut fields = Vec::new();
        let mut constructor: Option<MethodDecl> = None;
        let mut methods = Vec::new();

        for member in &c.members {
            match member {
                ClassMember::Field(f) => fields.push(f.clone()),
                ClassMember::Method(m) => {
                    if m.function.name.name == "init" || m.function.name.name == c.name.name {
                        constructor = Some(m.clone());
                    } else if !m.is_abstract {
                        methods.push(m.clone());
                    }
                }
            }
        }

        // Emit constructor
        let ctor_indent = self.indent_str();
        let ctor_params = if let Some(ref ctor) = constructor {
            ctor.function
                .params
                .iter()
                .map(|p| p.name.name.clone())
                .collect::<Vec<_>>()
                .join(", ")
        } else {
            String::new()
        };

        self.output
            .push_str(&format!("{}constructor({}) {{\n", ctor_indent, ctor_params));
        self.indent_level += 1;

        // If class has a parent, invoke super() first
        if c.parent.is_some() {
            let inner_indent = self.indent_str();
            self.output.push_str(&format!("{}super();\n", inner_indent));
        }

        // Initialize default instance fields
        for f in &fields {
            if !f.is_static {
                let inner_indent = self.indent_str();
                let init_val = if let Some(ref init) = f.var_decl.initializer {
                    self.emit_expr_to_string(init)
                } else {
                    "null".to_string()
                };
                self.output.push_str(&format!(
                    "{}this.{} = {};\n",
                    inner_indent, f.var_decl.name.name, init_val
                ));
            }
        }

        // Execute constructor body statements
        if let Some(ref ctor) = constructor {
            for s in &ctor.function.body.statements {
                self.emit_stmt(s);
            }
        }

        self.indent_level -= 1;
        self.output.push_str(&format!("{}}}\n", ctor_indent));

        // Emit methods
        for m in &methods {
            let m_indent = self.indent_str();
            let prefix = if m.is_static { "static " } else { "" };
            let params = m
                .function
                .params
                .iter()
                .map(|p| p.name.name.clone())
                .collect::<Vec<_>>()
                .join(", ");

            self.output.push_str(&format!(
                "{}{}{}({}) {{\n",
                m_indent, prefix, m.function.name.name, params
            ));
            self.indent_level += 1;
            for s in &m.function.body.statements {
                self.emit_stmt(s);
            }
            self.indent_level -= 1;
            self.output.push_str(&format!("{}}}\n", m_indent));
        }

        // Emit static fields
        for f in &fields {
            if f.is_static {
                let f_indent = self.indent_str();
                let init_val = if let Some(ref init) = f.var_decl.initializer {
                    self.emit_expr_to_string(init)
                } else {
                    "null".to_string()
                };
                self.output.push_str(&format!(
                    "{}static {} = {};\n",
                    f_indent, f.var_decl.name.name, init_val
                ));
            }
        }

        self.indent_level -= 1;
        self.output.push_str(&format!("{}}}\n\n", indent));
    }

    fn emit_match(&mut self, m: &MatchStmt) {
        let indent = self.indent_str();
        let target_var = format!("__target_{}", self.target_counter);
        self.target_counter += 1;

        let target_expr = self.emit_expr_to_string(&m.target);
        self.output.push_str(&format!("{}{{\n", indent));
        self.indent_level += 1;
        let inner_indent = self.indent_str();
        self.output.push_str(&format!(
            "{}const {} = {};\n",
            inner_indent, target_var, target_expr
        ));

        let mut first = true;
        for case in &m.cases {
            let case_indent = self.indent_str();
            let cond_str = self.case_condition(case, &target_var);

            let branch_hdr = if first {
                if cond_str == "true" {
                    format!("{}if (true) {{\n", case_indent)
                } else {
                    format!("{}if ({}) {{\n", case_indent, cond_str)
                }
            } else if cond_str == "true" {
                format!("{}}} else {{\n", case_indent)
            } else {
                format!("{}}} else if ({}) {{\n", case_indent, cond_str)
            };

            self.output.push_str(&branch_hdr);
            self.indent_level += 1;

            // Bind pattern variable if it's an identifier
            if let Pattern::Binding(ref id) = case.pattern {
                let bind_indent = self.indent_str();
                self.output.push_str(&format!(
                    "{}const {} = {};\n",
                    bind_indent, id.name, target_var
                ));
            }

            for s in &case.body.statements {
                self.emit_stmt(s);
            }
            self.indent_level -= 1;
            first = false;
        }

        if !first {
            self.output.push_str(&format!("{}}}\n", inner_indent));
        }

        self.indent_level -= 1;
        self.output.push_str(&format!("{}}}\n", indent));
    }

    fn case_condition(&mut self, case: &MatchCase, target_var: &str) -> String {
        let pat_cond = self.pattern_to_condition(&case.pattern, target_var);
        if let Some(ref guard) = case.guard {
            if let Pattern::Binding(ref id) = case.pattern {
                let guard_js = self.emit_expr_to_string(guard);
                format!(
                    "((() => {{ const {} = {}; return {}; }})())",
                    id.name, target_var, guard_js
                )
            } else {
                let guard_js = self.emit_expr_to_string(guard);
                if pat_cond == "true" {
                    format!("({})", guard_js)
                } else {
                    format!("({} && ({}))", pat_cond, guard_js)
                }
            }
        } else {
            pat_cond
        }
    }

    fn pattern_to_condition(&self, pattern: &Pattern, target_var: &str) -> String {
        match pattern {
            Pattern::Literal(lit) => {
                let lit_str = self.literal_to_string(lit);
                format!("{} === {}", target_var, lit_str)
            }
            Pattern::Alternative(alts, _) => {
                let parts: Vec<String> = alts
                    .iter()
                    .map(|p| self.pattern_to_condition(p, target_var))
                    .collect();
                parts.join(" || ")
            }
            Pattern::Wildcard(_) => "true".to_string(),
            Pattern::Binding(_) => "true".to_string(),
        }
    }

    fn literal_to_string(&self, lit: &Literal) -> String {
        match lit {
            Literal::Integer { value, .. } => format!("{}", value),
            Literal::Float { raw, .. } => raw.clone(),
            Literal::String { value, .. } => format!("{:?}", value),
            Literal::Bool { value, .. } => format!("{}", value),
            Literal::Char { value, .. } => format!("{:?}", value.to_string()),
            Literal::Null { .. } => "null".to_string(),
        }
    }

    pub fn emit_expr_to_string(&mut self, expr: &Expr) -> String {
        match expr {
            Expr::Literal(lit) => self.literal_to_string(lit),
            Expr::Identifier(id) => {
                if id.name == "self" {
                    "this".to_string()
                } else {
                    id.name.clone()
                }
            }
            Expr::Unary { op, operand, .. } => {
                let inner = self.emit_expr_to_string(operand);
                match op {
                    UnaryOp::Not => format!("!({})", inner),
                    UnaryOp::Neg => format!("-({})", inner),
                    UnaryOp::Pos => format!("+({})", inner),
                    UnaryOp::BitNot => format!("~({})", inner),
                }
            }
            Expr::Binary {
                op, left, right, ..
            } => {
                let l = self.emit_expr_to_string(left);
                let r = self.emit_expr_to_string(right);
                match op {
                    BinaryOp::Add => format!("({} + {})", l, r),
                    BinaryOp::Sub => format!("({} - {})", l, r),
                    BinaryOp::Mul => format!("({} * {})", l, r),
                    BinaryOp::Div => format!("({} / {})", l, r),
                    BinaryOp::Mod => format!("({} % {})", l, r),
                    BinaryOp::Eq => format!("({} === {})", l, r),
                    BinaryOp::NotEq => format!("({} !== {})", l, r),
                    BinaryOp::Lt => format!("({} < {})", l, r),
                    BinaryOp::LtEq => format!("({} <= {})", l, r),
                    BinaryOp::Gt => format!("({} > {})", l, r),
                    BinaryOp::GtEq => format!("({} >= {})", l, r),
                    BinaryOp::And => format!("({} && {})", l, r),
                    BinaryOp::Or => format!("({} || {})", l, r),
                    BinaryOp::BitXor => format!("({} ^ {})", l, r),
                    BinaryOp::Shl => format!("({} << {})", l, r),
                    BinaryOp::Shr => format!("({} >> {})", l, r),
                    BinaryOp::Range => {
                        format!(
                            "Array.from({{length: ({} - {} + 1)}}, (_, i) => {} + i)",
                            r, l, l
                        )
                    }
                }
            }
            Expr::Update {
                op,
                target,
                is_prefix,
                ..
            } => {
                let t = self.emit_expr_to_string(target);
                let op_str = match op {
                    UpdateOp::Increment => "++",
                    UpdateOp::Decrement => "--",
                };
                if *is_prefix {
                    format!("({}{})", op_str, t)
                } else {
                    format!("({}{})", t, op_str)
                }
            }
            Expr::Call { callee, args, .. } => {
                let arg_strs: Vec<String> =
                    args.iter().map(|a| self.emit_expr_to_string(a)).collect();
                let args_joined = arg_strs.join(", ");

                if let Expr::Identifier(ident) = callee.as_ref() {
                    if ident.name == "vout" {
                        return format!("console.log({})", args_joined);
                    }
                    if ident.name == "len" {
                        return format!("__nx_len({})", args_joined);
                    }
                    if ident.name == "to_int" {
                        return format!("__nx_to_int({})", args_joined);
                    }
                    if ident.name == "to_float" {
                        return format!("__nx_to_float({})", args_joined);
                    }
                    if ident.name == "input" {
                        return format!("__nx_input({})", args_joined);
                    }
                    if ident.name == "alloc" {
                        return format!("__nx_alloc({})", args_joined);
                    }
                    if ident.name == "free" {
                        return format!("__nx_free({})", args_joined);
                    }
                    if self.classes.contains(&ident.name) {
                        return format!("new {}({})", ident.name, args_joined);
                    }
                }

                let callee_str = self.emit_expr_to_string(callee);
                format!("{}({})", callee_str, args_joined)
            }
            Expr::PropertyAccess {
                target, property, ..
            } => {
                let t = self.emit_expr_to_string(target);
                format!("{}.{}", t, property.name)
            }
            Expr::Index { target, index, .. } => {
                let t = self.emit_expr_to_string(target);
                let i = self.emit_expr_to_string(index);
                format!("{}[{}]", t, i)
            }
            Expr::ListLiteral { elements, .. } => {
                let elems: Vec<String> = elements
                    .iter()
                    .map(|e| self.emit_expr_to_string(e))
                    .collect();
                format!("[{}]", elems.join(", "))
            }
            Expr::MapLiteral { entries, .. } => {
                let kvs: Vec<String> = entries
                    .iter()
                    .map(|(k, v)| {
                        let k_str = self.emit_expr_to_string(k);
                        let v_str = self.emit_expr_to_string(v);
                        format!("[{}]: {}", k_str, v_str)
                    })
                    .collect();
                format!("({{ {} }})", kvs.join(", "))
            }
            Expr::Group { expr, .. } => {
                format!("({})", self.emit_expr_to_string(expr))
            }
            Expr::Borrow { expr, .. } => self.emit_expr_to_string(expr),
            Expr::Alloc { expr, .. } => {
                let inner = self.emit_expr_to_string(expr);
                format!("__nx_alloc({})", inner)
            }
            Expr::Free { target, .. } => {
                let inner = self.emit_expr_to_string(target);
                format!("__nx_free({})", inner)
            }
            Expr::StructLiteral { fields, .. } => {
                let f_strs: Vec<String> = fields
                    .iter()
                    .map(|(fname, fval)| {
                        format!("{}: {}", fname.name, self.emit_expr_to_string(fval))
                    })
                    .collect();
                format!("({{ {} }})", f_strs.join(", "))
            }
        }
    }
}
