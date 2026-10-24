use crate::environment::Environment;
use crate::error::RuntimeError;
use crate::heap::HeapManager;
use crate::value::{ObjectInstance, Value};
use nx_ast::*;
use nx_diagnostics::Span;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Write;
use std::rc::Rc;

pub enum ControlFlow {
    None,
    Return(Value),
}

pub struct Interpreter<W: Write> {
    pub env: Environment,
    pub heap: HeapManager,
    pub functions: HashMap<String, FunctionDecl>,
    pub classes: HashMap<String, ClassDecl>,
    pub stdout: W,
    pub stdin: Box<dyn std::io::BufRead>,
    pub call_depth: usize,
    pub max_call_depth: usize,
    pub step_count: usize,
    pub max_steps: Option<usize>,
    pub main_called: bool,
}

impl<W: Write> Interpreter<W> {
    pub fn new(stdout: W) -> Self {
        Self {
            env: Environment::new(),
            heap: HeapManager::new(),
            functions: HashMap::new(),
            classes: HashMap::new(),
            stdout,
            stdin: Box::new(std::io::BufReader::new(std::io::stdin())),
            call_depth: 0,
            max_call_depth: 256,
            step_count: 0,
            max_steps: None,
            main_called: false,
        }
    }

    pub fn with_stdin(mut self, stdin: Box<dyn std::io::BufRead>) -> Self {
        self.stdin = stdin;
        self
    }

    pub fn with_max_call_depth(mut self, depth: usize) -> Self {
        self.max_call_depth = depth;
        self
    }

    pub fn with_max_steps(mut self, steps: usize) -> Self {
        self.max_steps = Some(steps);
        self
    }

    pub fn eval_program(&mut self, program: &Program) -> Result<Value, RuntimeError> {
        // Collect top-level functions and classes first
        for stmt in &program.statements {
            match stmt {
                Stmt::FunctionDecl(f) => {
                    self.functions.insert(f.name.name.clone(), f.clone());
                }
                Stmt::ClassDecl(c) => {
                    self.classes.insert(c.name.name.clone(), c.clone());
                }
                _ => {}
            }
        }

        // Execute top-level statements
        for stmt in &program.statements {
            match self.eval_stmt(stmt)? {
                ControlFlow::Return(val) => return Ok(val),
                ControlFlow::None => {}
            }
        }

        // Entry-point convention: if main() is defined and wasn't called during top-level execution,
        // execute main() as the program entry point.
        if !self.main_called {
            if let Some(main_func) = self.functions.get("main").cloned() {
                self.main_called = true;
                return self.call_function(&main_func, Vec::new());
            }
        }

        Ok(Value::Void)
    }

    pub fn eval_stmt(&mut self, stmt: &Stmt) -> Result<ControlFlow, RuntimeError> {
        self.step_count += 1;
        if let Some(max) = self.max_steps {
            if self.step_count > max {
                return Err(RuntimeError::with_span(
                    format!("execution step limit exceeded: {}", max),
                    stmt.span(),
                ));
            }
        }

        match stmt {
            Stmt::VarDecl(decl) => {
                let val = if let Some(ref init) = decl.initializer {
                    self.eval_expr(init)?
                } else {
                    Value::Null
                };
                self.env.define(decl.name.name.clone(), val);
                Ok(ControlFlow::None)
            }
            Stmt::Assignment(assign) => {
                let rhs = self.eval_expr(&assign.value)?;
                let new_val = match assign.op {
                    AssignOp::Assign => rhs,
                    AssignOp::AddAssign => {
                        let cur = self.eval_expr(&assign.target)?;
                        cur.add(&rhs)?
                    }
                    AssignOp::SubAssign => {
                        let cur = self.eval_expr(&assign.target)?;
                        cur.sub(&rhs)?
                    }
                    AssignOp::MulAssign => {
                        let cur = self.eval_expr(&assign.target)?;
                        cur.mul(&rhs)?
                    }
                    AssignOp::DivAssign => {
                        let cur = self.eval_expr(&assign.target)?;
                        cur.div(&rhs)?
                    }
                    AssignOp::ModAssign => {
                        let cur = self.eval_expr(&assign.target)?;
                        cur.rem(&rhs)?
                    }
                };
                self.assign_target(&assign.target, new_val)?;
                Ok(ControlFlow::None)
            }
            Stmt::Update(up) => {
                let cur = self.eval_expr(&up.target)?;
                let new_val = match up.op {
                    UpdateOp::Increment => cur.add(&Value::Int(1))?,
                    UpdateOp::Decrement => cur.sub(&Value::Int(1))?,
                };
                self.assign_target(&up.target, new_val)?;
                Ok(ControlFlow::None)
            }
            Stmt::FunctionDecl(_)
            | Stmt::InterfaceDecl(_)
            | Stmt::ClassDecl(_)
            | Stmt::StructDecl(_)
            | Stmt::ExternBlock(_) => Ok(ControlFlow::None),
            Stmt::Unsafe(u) => {
                self.env.push_scope();
                let res = self.eval_block(&u.body);
                self.env.pop_scope();
                res
            }
            Stmt::Return(ret) => {
                let val = if let Some(ref expr) = ret.value {
                    self.eval_expr(expr)?
                } else {
                    Value::Void
                };
                Ok(ControlFlow::Return(val))
            }
            Stmt::If(if_stmt) => self.eval_if_stmt(if_stmt),
            Stmt::AsLoop(as_loop) => self.eval_as_loop(as_loop),
            Stmt::ForLoop(for_loop) => self.eval_for_loop(for_loop),
            Stmt::DoAsLoop(do_as) => self.eval_do_as_loop(do_as),
            Stmt::Block(block) => {
                self.env.push_scope();
                let res = self.eval_block(block);
                self.env.pop_scope();
                res
            }
            Stmt::Expr(expr) => {
                self.eval_expr(expr)?;
                Ok(ControlFlow::None)
            }
            Stmt::Match(match_stmt) => self.eval_match_stmt(match_stmt),
        }
    }

    fn eval_block(&mut self, block: &Block) -> Result<ControlFlow, RuntimeError> {
        for stmt in &block.statements {
            let flow = self.eval_stmt(stmt)?;
            if let ControlFlow::Return(_) = flow {
                return Ok(flow);
            }
        }
        Ok(ControlFlow::None)
    }

    fn eval_if_stmt(&mut self, if_stmt: &IfStmt) -> Result<ControlFlow, RuntimeError> {
        let cond = self.eval_expr(&if_stmt.condition)?;
        if cond.is_truthy() {
            self.env.push_scope();
            let res = self.eval_block(&if_stmt.then_branch);
            self.env.pop_scope();
            return res;
        }

        for ow in &if_stmt.otherwise_branches {
            let ow_cond = self.eval_expr(&ow.condition)?;
            if ow_cond.is_truthy() {
                self.env.push_scope();
                let res = self.eval_block(&ow.body);
                self.env.pop_scope();
                return res;
            }
        }

        if let Some(ref else_block) = if_stmt.else_branch {
            self.env.push_scope();
            let res = self.eval_block(else_block);
            self.env.pop_scope();
            return res;
        }

        Ok(ControlFlow::None)
    }

    fn eval_as_loop(&mut self, as_loop: &AsLoopStmt) -> Result<ControlFlow, RuntimeError> {
        while self.eval_expr(&as_loop.condition)?.is_truthy() {
            self.env.push_scope();
            let res = self.eval_block(&as_loop.body)?;
            self.env.pop_scope();
            if let ControlFlow::Return(_) = res {
                return Ok(res);
            }
        }
        Ok(ControlFlow::None)
    }

    fn eval_for_loop(&mut self, for_loop: &ForLoopStmt) -> Result<ControlFlow, RuntimeError> {
        self.env.push_scope();

        if let Some(ref init) = for_loop.init {
            self.eval_stmt(init)?;
        }

        loop {
            if let Some(ref cond) = for_loop.condition {
                if !self.eval_expr(cond)?.is_truthy() {
                    break;
                }
            }

            let res = self.eval_block(&for_loop.body)?;
            if let ControlFlow::Return(_) = res {
                self.env.pop_scope();
                return Ok(res);
            }

            if let Some(ref update) = for_loop.update {
                self.eval_stmt(update)?;
            }
        }

        self.env.pop_scope();
        Ok(ControlFlow::None)
    }

    fn eval_do_as_loop(&mut self, do_as: &DoAsLoopStmt) -> Result<ControlFlow, RuntimeError> {
        loop {
            self.env.push_scope();
            let res = self.eval_block(&do_as.body)?;
            self.env.pop_scope();
            if let ControlFlow::Return(_) = res {
                return Ok(res);
            }

            if !self.eval_expr(&do_as.condition)?.is_truthy() {
                break;
            }
        }
        Ok(ControlFlow::None)
    }

    fn eval_match_stmt(&mut self, match_stmt: &MatchStmt) -> Result<ControlFlow, RuntimeError> {
        let target_val = self.eval_expr(&match_stmt.target)?;

        for case in &match_stmt.cases {
            if let Some(bindings) = self.match_pattern(&case.pattern, &target_val) {
                self.env.push_scope();
                for (name, val) in bindings {
                    self.env.define(name, val);
                }

                // Check guard if present
                let guard_passes = if let Some(ref guard) = case.guard {
                    self.eval_expr(guard)?.is_truthy()
                } else {
                    true
                };

                if guard_passes {
                    let res = self.eval_block(&case.body);
                    self.env.pop_scope();
                    return res;
                }

                self.env.pop_scope();
            }
        }

        Ok(ControlFlow::None)
    }

    fn match_pattern(&self, pattern: &Pattern, target: &Value) -> Option<Vec<(String, Value)>> {
        match pattern {
            Pattern::Literal(lit) => {
                let lit_val = match lit {
                    Literal::Integer { value, .. } => Value::Int(*value),
                    Literal::Float { raw, .. } => Value::Float(raw.parse::<f64>().unwrap_or(0.0)),
                    Literal::String { value, .. } => Value::String(value.clone()),
                    Literal::Bool { value, .. } => Value::Bool(*value),
                    Literal::Char { value, .. } => Value::Char(*value),
                    Literal::Null { .. } => Value::Null,
                };
                if target.equals(&lit_val) {
                    Some(Vec::new())
                } else {
                    None
                }
            }
            Pattern::Wildcard(_) => Some(Vec::new()),
            Pattern::Binding(ident) => Some(vec![(ident.name.clone(), target.clone())]),
            Pattern::Alternative(alts, _) => {
                for alt in alts {
                    if let Some(bindings) = self.match_pattern(alt, target) {
                        return Some(bindings);
                    }
                }
                None
            }
        }
    }

    fn assign_target(&mut self, target: &Expr, value: Value) -> Result<(), RuntimeError> {
        match target {
            Expr::Identifier(ident) => self.env.assign(&ident.name, value),
            Expr::Index {
                target,
                index,
                span,
            } => {
                let target_val = self.eval_expr(target)?;
                let index_val = self.eval_expr(index)?;
                match target_val {
                    Value::List(l) => {
                        let mut list = l.borrow_mut();
                        match index_val.unwrap_ref() {
                            Value::Int(i) => {
                                if i < 0 || i as usize >= list.len() {
                                    return Err(RuntimeError::with_span(
                                        format!(
                                            "list index out of bounds: index {}, length {}",
                                            i,
                                            list.len()
                                        ),
                                        *span,
                                    ));
                                }
                                list[i as usize] = value;
                                Ok(())
                            }
                            other => Err(RuntimeError::with_span(
                                format!("list index must be integer, found {}", other.type_name()),
                                *span,
                            )),
                        }
                    }
                    Value::Map(m) => {
                        let mut map = m.borrow_mut();
                        let key_str = format!("{}", index_val);
                        map.insert(key_str, value);
                        Ok(())
                    }
                    other => Err(RuntimeError::with_span(
                        format!("cannot index assign to type {}", other.type_name()),
                        *span,
                    )),
                }
            }
            Expr::PropertyAccess {
                target,
                property,
                span,
            } => {
                let target_val = self.eval_expr(target)?;
                match target_val {
                    Value::Object(obj) | Value::Super(obj, _) => {
                        obj.borrow_mut().fields.insert(property.name.clone(), value);
                        Ok(())
                    }
                    Value::Struct { ref fields, .. } => {
                        fields.borrow_mut().insert(property.name.clone(), value);
                        Ok(())
                    }
                    other => Err(RuntimeError::with_span(
                        format!(
                            "cannot assign property '{}' on non-object type {}",
                            property.name,
                            other.type_name()
                        ),
                        *span,
                    )),
                }
            }
            other => Err(RuntimeError::with_span(
                "invalid assignment target",
                other.span(),
            )),
        }
    }

    pub fn eval_expr(&mut self, expr: &Expr) -> Result<Value, RuntimeError> {
        match expr {
            Expr::Literal(lit) => match lit {
                Literal::Integer { value, .. } => Ok(Value::Int(*value)),
                Literal::Float { raw, .. } => Ok(Value::Float(raw.parse::<f64>().unwrap_or(0.0))),
                Literal::String { value, .. } => Ok(Value::String(value.clone())),
                Literal::Bool { value, .. } => Ok(Value::Bool(*value)),
                Literal::Char { value, .. } => Ok(Value::Char(*value)),
                Literal::Null { .. } => Ok(Value::Null),
            },
            Expr::Identifier(ident) => {
                if let Some(val) = self.env.lookup(&ident.name) {
                    Ok(val)
                } else {
                    Err(RuntimeError::with_span(
                        format!("undefined identifier '{}'", ident.name),
                        ident.span,
                    ))
                }
            }
            Expr::Unary { op, operand, span } => {
                let val = self.eval_expr(operand)?;
                match op {
                    UnaryOp::Not => Ok(Value::Bool(!val.is_truthy())),
                    UnaryOp::BitNot => match val.unwrap_ref() {
                        Value::Int(n) => Ok(Value::Int(!n)),
                        other => Err(RuntimeError::with_span(
                            format!("cannot bitwise-not type {}", other.type_name()),
                            *span,
                        )),
                    },
                    UnaryOp::Neg => match val.unwrap_ref() {
                        Value::Int(n) => Ok(Value::Int(-n)),
                        Value::Float(f) => Ok(Value::Float(-f)),
                        other => Err(RuntimeError::with_span(
                            format!("cannot negate type {}", other.type_name()),
                            *span,
                        )),
                    },
                    UnaryOp::Pos => Ok(val),
                }
            }
            Expr::Binary {
                op,
                left,
                right,
                span: _,
            } => self.eval_binary_expr(*op, left, right),
            Expr::Update {
                op,
                target,
                is_prefix,
                ..
            } => {
                let cur = self.eval_expr(target)?;
                let new_val = match op {
                    UpdateOp::Increment => cur.add(&Value::Int(1))?,
                    UpdateOp::Decrement => cur.sub(&Value::Int(1))?,
                };
                self.assign_target(target, new_val.clone())?;
                if *is_prefix {
                    Ok(new_val)
                } else {
                    Ok(cur)
                }
            }
            Expr::Group { expr, .. } => self.eval_expr(expr),
            Expr::ListLiteral { elements, .. } => {
                let mut items = Vec::new();
                for e in elements {
                    items.push(self.eval_expr(e)?);
                }
                Ok(Value::List(Rc::new(RefCell::new(items))))
            }
            Expr::MapLiteral { entries, .. } => {
                let mut map = HashMap::new();
                for (k_expr, v_expr) in entries {
                    let k = self.eval_expr(k_expr)?;
                    let v = self.eval_expr(v_expr)?;
                    map.insert(format!("{}", k), v);
                }
                Ok(Value::Map(Rc::new(RefCell::new(map))))
            }
            Expr::Index {
                target,
                index,
                span,
            } => {
                let target_val = self.eval_expr(target)?;
                let index_val = self.eval_expr(index)?;
                match target_val {
                    Value::List(l) => {
                        let list = l.borrow();
                        match index_val.unwrap_ref() {
                            Value::Int(i) => {
                                if i < 0 || i as usize >= list.len() {
                                    return Err(RuntimeError::with_span(
                                        format!(
                                            "list index out of bounds: index {}, length {}",
                                            i,
                                            list.len()
                                        ),
                                        *span,
                                    ));
                                }
                                Ok(list[i as usize].clone())
                            }
                            other => Err(RuntimeError::with_span(
                                format!("list index must be integer, found {}", other.type_name()),
                                *span,
                            )),
                        }
                    }
                    Value::Map(m) => {
                        let map = m.borrow();
                        let key_str = format!("{}", index_val);
                        Ok(map.get(&key_str).cloned().unwrap_or(Value::Null))
                    }
                    Value::String(s) => match index_val.unwrap_ref() {
                        Value::Int(i) => {
                            if i < 0 || i as usize >= s.len() {
                                return Err(RuntimeError::with_span(
                                    format!(
                                        "string index out of bounds: index {}, length {}",
                                        i,
                                        s.len()
                                    ),
                                    *span,
                                ));
                            }
                            let ch = s.chars().nth(i as usize).unwrap_or('\0');
                            Ok(Value::Char(ch))
                        }
                        other => Err(RuntimeError::with_span(
                            format!("string index must be integer, found {}", other.type_name()),
                            *span,
                        )),
                    },
                    other => Err(RuntimeError::with_span(
                        format!("cannot index into type {}", other.type_name()),
                        *span,
                    )),
                }
            }
            Expr::PropertyAccess {
                target,
                property,
                span,
            } => {
                let target_val = self.eval_expr(target)?;
                match target_val {
                    Value::Object(obj) | Value::Super(obj, _) => {
                        let inst = obj.borrow();
                        if let Some(val) = inst.fields.get(&property.name) {
                            Ok(val.clone())
                        } else {
                            Err(RuntimeError::with_span(
                                format!(
                                    "field '{}' not found on class '{}'",
                                    property.name, inst.class_name
                                ),
                                *span,
                            ))
                        }
                    }
                    Value::Struct {
                        ref name,
                        ref fields,
                    } => {
                        let map = fields.borrow();
                        if let Some(val) = map.get(&property.name) {
                            Ok(val.clone())
                        } else {
                            Err(RuntimeError::with_span(
                                format!("field '{}' not found on struct '{}'", property.name, name),
                                *span,
                            ))
                        }
                    }
                    other => Err(RuntimeError::with_span(
                        format!(
                            "cannot access property '{}' on non-object type {}",
                            property.name,
                            other.type_name()
                        ),
                        *span,
                    )),
                }
            }
            Expr::StructLiteral { name, fields, .. } => {
                let mut map = std::collections::HashMap::new();
                for (fname, fval) in fields {
                    let v = self.eval_expr(fval)?;
                    map.insert(fname.name.clone(), v);
                }
                Ok(Value::Struct {
                    name: name.name.clone(),
                    fields: std::rc::Rc::new(std::cell::RefCell::new(map)),
                })
            }
            Expr::Call { callee, args, span } => self.eval_call(callee, args, *span),
            Expr::Borrow { expr, .. } => {
                let val = self.eval_expr(expr)?;
                Ok(Value::Reference(Rc::new(RefCell::new(val))))
            }
            Expr::Alloc { expr, .. } => {
                let val = self.eval_expr(expr)?;
                let handle = self.heap.alloc(val);
                Ok(Value::Heap(handle))
            }
            Expr::Free { target, span: _ } => {
                let val = self.eval_expr(target)?;
                match val {
                    Value::Heap(h) => {
                        self.heap.free(h)?;
                        Ok(Value::Void)
                    }
                    _ => Ok(Value::Void),
                }
            }
        }
    }

    fn eval_call(
        &mut self,
        callee: &Expr,
        args: &[Expr],
        span: Span,
    ) -> Result<Value, RuntimeError> {
        // Check for method call: target.property(args...)
        if let Expr::PropertyAccess {
            target, property, ..
        } = callee
        {
            // First check if target is a class name (static method call)
            if let Expr::Identifier(ref target_ident) = target.as_ref() {
                if let Some(cls) = self.classes.get(&target_ident.name).cloned() {
                    for member in &cls.members {
                        if let ClassMember::Method(ref m) = member {
                            if m.is_static && m.function.name.name == property.name {
                                let mut arg_vals = Vec::new();
                                for a in args {
                                    arg_vals.push(self.eval_expr(a)?);
                                }
                                return self.call_function(&m.function, arg_vals);
                            }
                        }
                    }
                }
            }

            let target_val = self.eval_expr(target)?;
            let mut arg_vals = Vec::new();
            for a in args {
                arg_vals.push(self.eval_expr(a)?);
            }
            return self.call_method(target_val, &property.name, arg_vals, span);
        }

        // Identifier callee
        if let Expr::Identifier(ident) = callee {
            // super(...) constructor call
            if ident.name == "super" {
                if let Some(Value::Super(obj, parent_name)) = self.env.lookup("super") {
                    let mut arg_vals = Vec::new();
                    for a in args {
                        arg_vals.push(self.eval_expr(a)?);
                    }
                    return self.call_method(
                        Value::Super(obj, parent_name),
                        "init",
                        arg_vals,
                        span,
                    );
                }
            }

            // Builtin: vout
            if ident.name == "vout" {
                let mut first = true;
                for a in args {
                    let v = self.eval_expr(a)?;
                    if !first {
                        write!(self.stdout, " ")
                            .map_err(|e| RuntimeError::new(format!("output error: {}", e)))?;
                    }
                    first = false;
                    write!(self.stdout, "{}", v)
                        .map_err(|e| RuntimeError::new(format!("output error: {}", e)))?;
                }
                writeln!(self.stdout)
                    .map_err(|e| RuntimeError::new(format!("output error: {}", e)))?;
                let _ = self.stdout.flush();
                return Ok(Value::Void);
            }

            // Builtin: alloc
            if ident.name == "alloc" {
                if let Some(arg) = args.first() {
                    let val = self.eval_expr(arg)?;
                    let handle = self.heap.alloc(val);
                    return Ok(Value::Heap(handle));
                }
                return Ok(Value::Null);
            }

            // Builtin: free
            if ident.name == "free" {
                if let Some(arg) = args.first() {
                    let val = self.eval_expr(arg)?;
                    if let Value::Heap(h) = val {
                        self.heap.free(h)?;
                    }
                }
                return Ok(Value::Void);
            }

            // Builtin: len
            if ident.name == "len" {
                if let Some(arg) = args.first() {
                    let val = self.eval_expr(arg)?;
                    let length = match val.unwrap_ref() {
                        Value::List(l) => l.borrow().len(),
                        Value::Map(m) => m.borrow().len(),
                        Value::Set(s) => s.borrow().len(),
                        Value::String(s) => s.len(),
                        other => {
                            return Err(RuntimeError::with_span(
                                format!("cannot get length of type {}", other.type_name()),
                                span,
                            ));
                        }
                    };
                    return Ok(Value::Int(length as i128));
                }
                return Err(RuntimeError::with_span("len() requires 1 argument", span));
            }

            // Builtin: input([prompt])
            if ident.name == "input" {
                if let Some(arg) = args.first() {
                    let prompt_val = self.eval_expr(arg)?;
                    write!(self.stdout, "{}", prompt_val)
                        .map_err(|e| RuntimeError::new(format!("output error: {}", e)))?;
                    let _ = self.stdout.flush();
                }
                let mut line = String::new();
                self.stdin
                    .read_line(&mut line)
                    .map_err(|e| RuntimeError::new(format!("input error: {}", e)))?;
                if line.ends_with('\n') {
                    line.pop();
                    if line.ends_with('\r') {
                        line.pop();
                    }
                }
                return Ok(Value::String(line));
            }

            // Builtin: to_int(val)
            if ident.name == "to_int" {
                if let Some(arg) = args.first() {
                    let val = self.eval_expr(arg)?;
                    match val.unwrap_ref() {
                        Value::Int(n) => return Ok(Value::Int(n)),
                        Value::Float(f) => return Ok(Value::Int(f as i128)),
                        Value::String(s) => {
                            let trimmed = s.trim();
                            let parsed = trimmed.parse::<i128>().map_err(|_| {
                                RuntimeError::with_span(
                                    format!("invalid integer format: '{}'", s),
                                    span,
                                )
                            })?;
                            return Ok(Value::Int(parsed));
                        }
                        other => {
                            return Err(RuntimeError::with_span(
                                format!("cannot convert {} to integer", other.type_name()),
                                span,
                            ));
                        }
                    }
                }
                return Err(RuntimeError::with_span(
                    "to_int() requires 1 argument",
                    span,
                ));
            }

            // Builtin: to_float(val)
            if ident.name == "to_float" {
                if let Some(arg) = args.first() {
                    let val = self.eval_expr(arg)?;
                    match val.unwrap_ref() {
                        Value::Float(f) => return Ok(Value::Float(f)),
                        Value::Int(n) => return Ok(Value::Float(n as f64)),
                        Value::String(s) => {
                            let trimmed = s.trim();
                            let parsed = trimmed.parse::<f64>().map_err(|_| {
                                RuntimeError::with_span(
                                    format!("invalid float format: '{}'", s),
                                    span,
                                )
                            })?;
                            return Ok(Value::Float(parsed));
                        }
                        other => {
                            return Err(RuntimeError::with_span(
                                format!("cannot convert {} to float", other.type_name()),
                                span,
                            ));
                        }
                    }
                }
                return Err(RuntimeError::with_span(
                    "to_float() requires 1 argument",
                    span,
                ));
            }

            if let Some(res) = self.eval_systems_builtin(&ident.name, args)? {
                return Ok(res);
            }

            // Class constructor: ClassName(args...)
            if let Some(cls) = self.classes.get(&ident.name).cloned() {
                let mut arg_vals = Vec::new();
                for a in args {
                    arg_vals.push(self.eval_expr(a)?);
                }
                return self.instantiate_class(&cls, arg_vals, span);
            }

            // User function call
            if let Some(func) = self.functions.get(&ident.name).cloned() {
                let mut arg_vals = Vec::new();
                for a in args {
                    arg_vals.push(self.eval_expr(a)?);
                }
                return self.call_function(&func, arg_vals);
            }
        }

        Err(RuntimeError::with_span(
            "not a callable function or class",
            span,
        ))
    }

    fn call_function(
        &mut self,
        func: &FunctionDecl,
        args: Vec<Value>,
    ) -> Result<Value, RuntimeError> {
        if func.name.name == "main" {
            self.main_called = true;
        }

        if self.call_depth >= self.max_call_depth {
            return Err(RuntimeError::with_span(
                format!(
                    "call stack overflow: exceeded maximum recursion depth of {}",
                    self.max_call_depth
                ),
                func.span,
            ));
        }

        self.call_depth += 1;
        self.env.push_scope();

        for (i, param) in func.params.iter().enumerate() {
            let arg_val = args.get(i).cloned().unwrap_or(Value::Null);
            self.env.define(param.name.name.clone(), arg_val);
        }

        let res = self.eval_block(&func.body);
        self.env.pop_scope();
        self.call_depth -= 1;

        match res? {
            ControlFlow::Return(v) => Ok(v),
            ControlFlow::None => Ok(Value::Void),
        }
    }

    fn instantiate_class(
        &mut self,
        cls: &ClassDecl,
        args: Vec<Value>,
        _span: Span,
    ) -> Result<Value, RuntimeError> {
        let mut fields = HashMap::new();

        // Collect fields from parent if any
        if let Some(ref parent_name) = cls.parent {
            if let Some(parent_cls) = self.classes.get(&parent_name.name).cloned() {
                for member in &parent_cls.members {
                    if let ClassMember::Field(f) = member {
                        let init_val = if let Some(ref init) = f.var_decl.initializer {
                            self.eval_expr(init)?
                        } else {
                            Value::Null
                        };
                        fields.insert(f.var_decl.name.name.clone(), init_val);
                    }
                }
            }
        }

        // Initialize instance fields
        for member in &cls.members {
            if let ClassMember::Field(f) = member {
                if !f.is_static {
                    let init_val = if let Some(ref init) = f.var_decl.initializer {
                        self.eval_expr(init)?
                    } else {
                        Value::Null
                    };
                    fields.insert(f.var_decl.name.name.clone(), init_val);
                }
            }
        }

        let instance = Rc::new(RefCell::new(ObjectInstance {
            class_name: cls.name.name.clone(),
            fields,
        }));

        // Check if there is an explicit constructor method ("init" or matching class name)
        let constructor = self
            .find_method(&cls.name.name, "init")
            .or_else(|| self.find_method(&cls.name.name, &cls.name.name));

        if let Some((decl_class, ctor)) = constructor {
            self.env.push_scope();
            self.env
                .define("self".to_string(), Value::Object(instance.clone()));
            if let Some(ref parent_name) = decl_class.parent {
                self.env.define(
                    "super".to_string(),
                    Value::Super(instance.clone(), parent_name.name.clone()),
                );
            }
            for (i, param) in ctor.function.params.iter().enumerate() {
                let arg_val = args.get(i).cloned().unwrap_or(Value::Null);
                self.env.define(param.name.name.clone(), arg_val);
            }
            let _ = self.eval_block(&ctor.function.body)?;
            self.env.pop_scope();
        }

        Ok(Value::Object(instance))
    }

    fn call_method(
        &mut self,
        target_val: Value,
        method_name: &str,
        args: Vec<Value>,
        span: Span,
    ) -> Result<Value, RuntimeError> {
        let (obj, start_class) = match target_val {
            Value::Object(ref o) => (o.clone(), o.borrow().class_name.clone()),
            Value::Super(ref o, ref parent) => (o.clone(), parent.clone()),
            other => {
                return Err(RuntimeError::with_span(
                    format!(
                        "cannot call method '{}' on non-object type {}",
                        method_name,
                        other.type_name()
                    ),
                    span,
                ));
            }
        };

        let method = self.find_method(&start_class, method_name);

        if let Some((decl_class, method_decl)) = method {
            self.env.push_scope();
            self.env
                .define("self".to_string(), Value::Object(obj.clone()));

            if let Some(ref parent_name) = decl_class.parent {
                let super_val = Value::Super(obj.clone(), parent_name.name.clone());
                self.env.define("super".to_string(), super_val);
            }

            for (i, param) in method_decl.function.params.iter().enumerate() {
                let arg_val = args.get(i).cloned().unwrap_or(Value::Null);
                self.env.define(param.name.name.clone(), arg_val);
            }

            let res = self.eval_block(&method_decl.function.body)?;
            self.env.pop_scope();

            match res {
                ControlFlow::Return(v) => Ok(v),
                ControlFlow::None => Ok(Value::Void),
            }
        } else {
            Err(RuntimeError::with_span(
                format!(
                    "method '{}' not found on class '{}'",
                    method_name, start_class
                ),
                span,
            ))
        }
    }

    fn find_method(&self, class_name: &str, method_name: &str) -> Option<(ClassDecl, MethodDecl)> {
        let mut cur_name = Some(class_name.to_string());
        while let Some(name) = cur_name {
            if let Some(cls) = self.classes.get(&name) {
                for member in &cls.members {
                    if let ClassMember::Method(m) = member {
                        if m.function.name.name == method_name {
                            return Some((cls.clone(), m.clone()));
                        }
                    }
                }
                cur_name = cls.parent.as_ref().map(|p| p.name.clone());
            } else {
                break;
            }
        }
        None
    }

    fn eval_binary_expr(
        &mut self,
        op: BinaryOp,
        left: &Expr,
        right: &Expr,
    ) -> Result<Value, RuntimeError> {
        let l = self.eval_expr(left)?;
        if op == BinaryOp::And {
            if let Value::Int(a) = l.unwrap_ref() {
                let r = self.eval_expr(right)?;
                if let Value::Int(b) = r.unwrap_ref() {
                    return Ok(Value::Int(a & b));
                }
            }
            if !l.is_truthy() {
                return Ok(Value::Bool(false));
            }
            let r = self.eval_expr(right)?;
            return Ok(Value::Bool(r.is_truthy()));
        }
        if op == BinaryOp::Or {
            if let Value::Int(a) = l.unwrap_ref() {
                let r = self.eval_expr(right)?;
                if let Value::Int(b) = r.unwrap_ref() {
                    return Ok(Value::Int(a | b));
                }
            }
            if l.is_truthy() {
                return Ok(Value::Bool(true));
            }
            let r = self.eval_expr(right)?;
            return Ok(Value::Bool(r.is_truthy()));
        }

        let r = self.eval_expr(right)?;
        match op {
            BinaryOp::Add => l.add(&r),
            BinaryOp::Sub => l.sub(&r),
            BinaryOp::Mul => l.mul(&r),
            BinaryOp::Div => l.div(&r),
            BinaryOp::Mod => l.rem(&r),
            BinaryOp::BitXor => match (l.unwrap_ref(), r.unwrap_ref()) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a ^ b)),
                _ => Err(RuntimeError::new("bitwise ^ requires integer operands")),
            },
            BinaryOp::Shl => match (l.unwrap_ref(), r.unwrap_ref()) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a << b)),
                _ => Err(RuntimeError::new("shift << requires integer operands")),
            },
            BinaryOp::Shr => match (l.unwrap_ref(), r.unwrap_ref()) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a >> b)),
                _ => Err(RuntimeError::new("shift >> requires integer operands")),
            },
            BinaryOp::Eq => Ok(Value::Bool(l.equals(&r))),
            BinaryOp::NotEq => Ok(Value::Bool(!l.equals(&r))),
            BinaryOp::Lt => {
                let ord = l.compare(&r)?;
                Ok(Value::Bool(ord == std::cmp::Ordering::Less))
            }
            BinaryOp::LtEq => {
                let ord = l.compare(&r)?;
                Ok(Value::Bool(
                    ord == std::cmp::Ordering::Less || ord == std::cmp::Ordering::Equal,
                ))
            }
            BinaryOp::Gt => {
                let ord = l.compare(&r)?;
                Ok(Value::Bool(ord == std::cmp::Ordering::Greater))
            }
            BinaryOp::GtEq => {
                let ord = l.compare(&r)?;
                Ok(Value::Bool(
                    ord == std::cmp::Ordering::Greater || ord == std::cmp::Ordering::Equal,
                ))
            }
            BinaryOp::Range => {
                let start = match l.unwrap_ref() {
                    Value::Int(n) => n,
                    _ => 0,
                };
                let end = match r.unwrap_ref() {
                    Value::Int(n) => n,
                    _ => 0,
                };
                let items: Vec<Value> = (start..=end).map(Value::Int).collect();
                Ok(Value::List(Rc::new(RefCell::new(items))))
            }
            BinaryOp::And | BinaryOp::Or => unreachable!(),
        }
    }

    fn eval_systems_builtin(
        &mut self,
        name: &str,
        args: &[Expr],
    ) -> Result<Option<Value>, RuntimeError> {
        match name {
            "get_pid" => Ok(Some(Value::Int(std::process::id() as i128))),
            "get_cwd" => {
                let cwd = std::env::current_dir()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                Ok(Some(Value::String(cwd)))
            }
            "env_get" => {
                if let Some(arg) = args.first() {
                    let k = self.eval_expr(arg)?;
                    let v = std::env::var(format!("{}", k)).unwrap_or_default();
                    return Ok(Some(Value::String(v)));
                }
                Ok(Some(Value::String(String::new())))
            }
            "env_set" => {
                if args.len() >= 2 {
                    let k = self.eval_expr(&args[0])?;
                    let v = self.eval_expr(&args[1])?;
                    std::env::set_var(format!("{}", k), format!("{}", v));
                    return Ok(Some(Value::Bool(true)));
                }
                Ok(Some(Value::Bool(false)))
            }
            "file_exists" => {
                if let Some(arg) = args.first() {
                    let p = self.eval_expr(arg)?;
                    return Ok(Some(Value::Bool(
                        std::path::Path::new(&format!("{}", p)).exists(),
                    )));
                }
                Ok(Some(Value::Bool(false)))
            }
            "file_remove" => {
                if let Some(arg) = args.first() {
                    let p = self.eval_expr(arg)?;
                    let ok = std::fs::remove_file(format!("{}", p)).is_ok();
                    return Ok(Some(Value::Bool(ok)));
                }
                Ok(Some(Value::Bool(false)))
            }
            "file_read_all" => {
                if let Some(arg) = args.first() {
                    let p = self.eval_expr(arg)?;
                    let s = std::fs::read_to_string(format!("{}", p)).unwrap_or_default();
                    return Ok(Some(Value::String(s)));
                }
                Ok(Some(Value::String(String::new())))
            }
            "file_write" => {
                if args.len() >= 2 {
                    let p = self.eval_expr(&args[0])?;
                    let c = self.eval_expr(&args[1])?;
                    let ok = std::fs::write(format!("{}", p), format!("{}", c)).is_ok();
                    return Ok(Some(Value::Bool(ok)));
                }
                Ok(Some(Value::Bool(false)))
            }
            "buffer_new" => {
                let sz = if let Some(arg) = args.first() {
                    match self.eval_expr(arg)?.unwrap_ref() {
                        Value::Int(n) => n.max(0) as usize,
                        _ => 0,
                    }
                } else {
                    0
                };
                let buf: Vec<Value> = vec![Value::Int(0); sz];
                Ok(Some(Value::List(Rc::new(RefCell::new(buf)))))
            }
            "buffer_free" => Ok(Some(Value::Void)),
            "buffer_size" => {
                if let Some(arg) = args.first() {
                    if let Value::List(l) = self.eval_expr(arg)? {
                        return Ok(Some(Value::Int(l.borrow().len() as i128)));
                    }
                }
                Ok(Some(Value::Int(0)))
            }
            "buffer_get_u8" => {
                if args.len() >= 2 {
                    let buf = self.eval_expr(&args[0])?;
                    let off = match self.eval_expr(&args[1])?.unwrap_ref() {
                        Value::Int(n) => n as usize,
                        _ => 0,
                    };
                    if let Value::List(l) = buf {
                        let list = l.borrow();
                        if off < list.len() {
                            return Ok(Some(list[off].clone()));
                        }
                    }
                }
                Ok(Some(Value::Int(0)))
            }
            "buffer_set_u8" => {
                if args.len() >= 3 {
                    let buf = self.eval_expr(&args[0])?;
                    let off = match self.eval_expr(&args[1])?.unwrap_ref() {
                        Value::Int(n) => n as usize,
                        _ => 0,
                    };
                    let val = match self.eval_expr(&args[2])?.unwrap_ref() {
                        Value::Int(n) => (n & 0xFF) as u8,
                        _ => 0,
                    };
                    if let Value::List(l) = buf {
                        let mut list = l.borrow_mut();
                        if off < list.len() {
                            list[off] = Value::Int(val as i128);
                        }
                    }
                }
                Ok(Some(Value::Void))
            }
            "buffer_get_u32_le" | "buffer_get_i32_le" => {
                if args.len() >= 2 {
                    let buf = self.eval_expr(&args[0])?;
                    let off = match self.eval_expr(&args[1])?.unwrap_ref() {
                        Value::Int(n) => n as usize,
                        _ => 0,
                    };
                    if let Value::List(l) = buf {
                        let list = l.borrow();
                        if off + 4 <= list.len() {
                            let b0 = match list[off] {
                                Value::Int(n) => n as u32,
                                _ => 0,
                            };
                            let b1 = match list[off + 1] {
                                Value::Int(n) => n as u32,
                                _ => 0,
                            };
                            let b2 = match list[off + 2] {
                                Value::Int(n) => n as u32,
                                _ => 0,
                            };
                            let b3 = match list[off + 3] {
                                Value::Int(n) => n as u32,
                                _ => 0,
                            };
                            let val = b0 | (b1 << 8) | (b2 << 16) | (b3 << 24);
                            return Ok(Some(Value::Int(val as i128)));
                        }
                    }
                }
                Ok(Some(Value::Int(0)))
            }
            "buffer_set_u32_le" | "buffer_set_i32_le" => {
                if args.len() >= 3 {
                    let buf = self.eval_expr(&args[0])?;
                    let off = match self.eval_expr(&args[1])?.unwrap_ref() {
                        Value::Int(n) => n as usize,
                        _ => 0,
                    };
                    let val = match self.eval_expr(&args[2])?.unwrap_ref() {
                        Value::Int(n) => n as u32,
                        _ => 0,
                    };
                    if let Value::List(l) = buf {
                        let mut list = l.borrow_mut();
                        if off + 4 <= list.len() {
                            list[off] = Value::Int((val & 0xFF) as i128);
                            list[off + 1] = Value::Int(((val >> 8) & 0xFF) as i128);
                            list[off + 2] = Value::Int(((val >> 16) & 0xFF) as i128);
                            list[off + 3] = Value::Int(((val >> 24) & 0xFF) as i128);
                        }
                    }
                }
                Ok(Some(Value::Void))
            }
            _ => Ok(None),
        }
    }
}
