use crate::ir::*;
use nx_ast::*;
use std::collections::HashMap;

pub struct AstLowering {
    module: NirModule,
    current_fn: Option<NirFunction>,
    current_block: Option<NirBlock>,
    temp_counter: usize,
    block_counter: usize,
    // Variable name -> (stack_ptr_name, NirType)
    var_env: Vec<HashMap<String, (String, NirType)>>,
    // Loop context: (continue_block, break_block)
    loop_stack: Vec<(String, String)>,
    // Class layouts: class_name -> layout
    classes: HashMap<String, NirClassLayout>,
    // Map temporary SSA names to their evaluated NirType
    temp_types: HashMap<String, NirType>,
    // Variable name -> class name
    var_classes: Vec<HashMap<String, String>>,
    // Current class being compiled (if any)
    current_class: Option<String>,
    // Extern C and systems built-in functions -> return NirType
    extern_functions: HashMap<String, NirType>,
}

impl AstLowering {
    pub fn new(module_name: impl Into<String>) -> Self {
        let mut extern_functions = HashMap::new();
        extern_functions.insert("get_pid".to_string(), NirType::I32);
        extern_functions.insert("get_cwd".to_string(), NirType::Ptr);
        extern_functions.insert("env_get".to_string(), NirType::Ptr);
        extern_functions.insert("env_set".to_string(), NirType::Bool);
        extern_functions.insert("file_open".to_string(), NirType::I64);
        extern_functions.insert("file_close".to_string(), NirType::Bool);
        extern_functions.insert("file_read_all".to_string(), NirType::Ptr);
        extern_functions.insert("file_write".to_string(), NirType::Bool);
        extern_functions.insert("file_exists".to_string(), NirType::Bool);
        extern_functions.insert("file_remove".to_string(), NirType::Bool);
        extern_functions.insert("buffer_new".to_string(), NirType::Ptr);
        extern_functions.insert("buffer_free".to_string(), NirType::Void);
        extern_functions.insert("buffer_size".to_string(), NirType::U64);
        extern_functions.insert("buffer_get_u8".to_string(), NirType::U8);
        extern_functions.insert("buffer_set_u8".to_string(), NirType::Void);
        extern_functions.insert("buffer_get_u32_le".to_string(), NirType::U32);
        extern_functions.insert("buffer_set_u32_le".to_string(), NirType::Void);
        extern_functions.insert("buffer_get_i32_le".to_string(), NirType::I32);
        extern_functions.insert("buffer_set_i32_le".to_string(), NirType::Void);
        extern_functions.insert("ptr_offset".to_string(), NirType::Ptr);
        extern_functions.insert("ptr_read_u8".to_string(), NirType::U8);
        extern_functions.insert("ptr_write_u8".to_string(), NirType::Void);
        // Phase 9 Standard Library Primitives
        extern_functions.insert("math_abs".to_string(), NirType::F64);
        extern_functions.insert("int_abs".to_string(), NirType::I64);
        extern_functions.insert("math_sqrt".to_string(), NirType::F64);
        extern_functions.insert("math_pow".to_string(), NirType::F64);
        extern_functions.insert("math_floor".to_string(), NirType::F64);
        extern_functions.insert("math_ceil".to_string(), NirType::F64);
        extern_functions.insert("math_min".to_string(), NirType::F64);
        extern_functions.insert("math_max".to_string(), NirType::F64);
        extern_functions.insert("math_sin".to_string(), NirType::F64);
        extern_functions.insert("math_cos".to_string(), NirType::F64);
        extern_functions.insert("math_random".to_string(), NirType::F64);
        extern_functions.insert("str_len".to_string(), NirType::I64);
        extern_functions.insert("str_upper".to_string(), NirType::Ptr);
        extern_functions.insert("str_lower".to_string(), NirType::Ptr);
        extern_functions.insert("str_trim".to_string(), NirType::Ptr);
        extern_functions.insert("str_sub".to_string(), NirType::Ptr);
        extern_functions.insert("str_contains".to_string(), NirType::Bool);
        extern_functions.insert("str_starts_with".to_string(), NirType::Bool);
        extern_functions.insert("str_ends_with".to_string(), NirType::Bool);
        extern_functions.insert("time_now_ms".to_string(), NirType::I64);
        extern_functions.insert("time_sleep_ms".to_string(), NirType::Void);
        extern_functions.insert("hex_encode".to_string(), NirType::Ptr);
        extern_functions.insert("hex_decode".to_string(), NirType::Ptr);
        extern_functions.insert("base64_encode".to_string(), NirType::Ptr);
        extern_functions.insert("base64_decode".to_string(), NirType::Ptr);
        extern_functions.insert("json_parse".to_string(), NirType::Ptr);
        extern_functions.insert("json_stringify".to_string(), NirType::Ptr);
        extern_functions.insert("nx_rt_assert".to_string(), NirType::Void);
        extern_functions.insert("assert".to_string(), NirType::Void);
        extern_functions.insert("nx_rt_assert_eq".to_string(), NirType::Void);
        extern_functions.insert("assert_eq".to_string(), NirType::Void);

        Self {
            module: NirModule::new(module_name),
            current_fn: None,
            current_block: None,
            temp_counter: 0,
            block_counter: 0,
            var_env: vec![HashMap::new()],
            loop_stack: Vec::new(),
            classes: HashMap::new(),
            temp_types: HashMap::new(),
            var_classes: vec![HashMap::new()],
            current_class: None,
            extern_functions,
        }
    }

    fn new_temp(&mut self) -> String {
        let name = format!("t{}", self.temp_counter);
        self.temp_counter += 1;
        name
    }

    fn new_label(&mut self, prefix: &str) -> String {
        let name = format!("{}_{}", prefix, self.block_counter);
        self.block_counter += 1;
        name
    }

    fn push_scope(&mut self) {
        self.var_env.push(HashMap::new());
        self.var_classes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.var_env.pop();
        self.var_classes.pop();
    }

    fn insert_var_class(&mut self, name: String, class_name: String) {
        if let Some(scope) = self.var_classes.last_mut() {
            scope.insert(name, class_name);
        }
    }

    fn lookup_var_class(&self, name: &str) -> Option<String> {
        for scope in self.var_classes.iter().rev() {
            if let Some(c) = scope.get(name) {
                return Some(c.clone());
            }
        }
        None
    }

    fn lookup_var(&self, name: &str) -> Option<(String, NirType)> {
        for scope in self.var_env.iter().rev() {
            if let Some(entry) = scope.get(name) {
                return Some(entry.clone());
            }
        }
        None
    }

    fn insert_var(&mut self, name: String, ptr: String, ty: NirType) {
        if let Some(scope) = self.var_env.last_mut() {
            scope.insert(name, (ptr, ty));
        }
    }

    fn emit_inst(&mut self, inst: NirInst) {
        match &inst {
            NirInst::Load { dest, ty, .. } => {
                self.temp_types.insert(dest.clone(), ty.clone());
            }
            NirInst::BinOp { dest, ty, .. } => {
                self.temp_types.insert(dest.clone(), ty.clone());
            }
            NirInst::Cmp { dest, .. } => {
                self.temp_types.insert(dest.clone(), NirType::Bool);
            }
            NirInst::UnaryOp { dest, op, ty, .. } => {
                let inferred_ty = match op {
                    NirUnaryOp::Not => NirType::Bool,
                    NirUnaryOp::Neg | NirUnaryOp::BitNot => ty.clone(),
                };
                self.temp_types.insert(dest.clone(), inferred_ty);
            }
            NirInst::Call {
                dest: Some(d),
                ret_ty,
                ..
            } => {
                self.temp_types.insert(d.clone(), ret_ty.clone());
            }
            NirInst::Input { dest, .. } => {
                self.temp_types.insert(dest.clone(), NirType::Ptr);
            }
            NirInst::ToInt { dest, .. } => {
                self.temp_types.insert(dest.clone(), NirType::I64);
            }
            NirInst::ToFloat { dest, .. } => {
                self.temp_types.insert(dest.clone(), NirType::F64);
            }
            NirInst::Alloc { dest, .. } => {
                self.temp_types.insert(dest.clone(), NirType::Ptr);
            }
            NirInst::Len { dest, .. } => {
                self.temp_types.insert(dest.clone(), NirType::I64);
            }
            _ => {}
        }
        if let Some(ref mut block) = self.current_block {
            block.instructions.push(inst);
        }
    }

    fn finish_block(&mut self, term: NirTerminator) {
        if let Some(mut block) = self.current_block.take() {
            block.terminator = term;
            if let Some(ref mut func) = self.current_fn {
                func.blocks.push(block);
            }
        }
    }

    fn start_block(&mut self, label: impl Into<String>) {
        self.current_block = Some(NirBlock::new(label, NirTerminator::Return(None)));
    }

    pub fn lower_program(mut self, program: &Program) -> NirModule {
        // Pass 1: Collect class & struct declarations and extern blocks
        for stmt in &program.statements {
            match stmt {
                Stmt::ClassDecl(c) => self.build_class_layout(c),
                Stmt::StructDecl(s) => self.build_struct_layout(s),
                Stmt::ExternBlock(e) => {
                    for f in &e.functions {
                        let ret = match &f.return_type {
                            Some(t) => self.map_type(t),
                            None => NirType::Void,
                        };
                        self.extern_functions.insert(f.name.name.clone(), ret);
                    }
                }
                _ => {}
            }
        }
        self.module.classes = self.classes.clone();

        // Pass 2: Lower explicit functions and class methods
        for stmt in &program.statements {
            match stmt {
                Stmt::FunctionDecl(f) => {
                    self.lower_function(f, None);
                }
                Stmt::ClassDecl(c) => {
                    self.lower_class_methods(c);
                }
                _ => {}
            }
        }

        // Pass 3: Lower top-level statements into main() if no explicit main exists
        let has_explicit_main = program
            .statements
            .iter()
            .any(|s| matches!(s, Stmt::FunctionDecl(f) if f.name.name == "main"));

        if !has_explicit_main {
            let top_level_stmts: Vec<&Stmt> = program
                .statements
                .iter()
                .filter(|s| {
                    !matches!(
                        s,
                        Stmt::FunctionDecl(_)
                            | Stmt::ClassDecl(_)
                            | Stmt::InterfaceDecl(_)
                            | Stmt::StructDecl(_)
                            | Stmt::ExternBlock(_)
                            | Stmt::Import(_)
                            | Stmt::TestDecl(_)
                    )
                })
                .collect();
            self.lower_top_level_main(&top_level_stmts);
        }

        self.module
    }

    fn build_struct_layout(&mut self, s: &StructDecl) {
        let mut layout = NirClassLayout::new(&s.name.name, None);
        for f in &s.fields {
            let ty = self.map_type(&f.field_type);
            layout.add_field(&f.name.name, ty);
        }
        self.classes.insert(s.name.name.clone(), layout.clone());
        self.module.classes.insert(s.name.name.clone(), layout);
    }

    fn build_class_layout(&mut self, c: &ClassDecl) {
        let parent_name = c.parent.as_ref().map(|p| p.name.clone());
        let mut layout = NirClassLayout::new(&c.name.name, parent_name.clone());

        // Inherit parent fields
        if let Some(ref parent) = parent_name {
            if let Some(parent_layout) = self.classes.get(parent) {
                for (fname, fty) in &parent_layout.fields {
                    layout.add_field(fname.clone(), fty.clone());
                }
            }
        }

        // Add instance fields
        for member in &c.members {
            if let ClassMember::Field(f) = member {
                let ty = self.map_type(&f.var_decl.type_annotation);
                layout.add_field(&f.var_decl.name.name, ty);
            } else if let ClassMember::Method(m) = member {
                layout.methods.push(m.function.name.name.clone());
            }
        }

        self.classes.insert(c.name.name.clone(), layout);
    }

    fn lower_class_methods(&mut self, c: &ClassDecl) {
        self.current_class = Some(c.name.name.clone());

        // Constructor: ClassName_init
        let init_method = c.members.iter().find_map(|m| {
            if let ClassMember::Method(m) = m {
                if m.function.name.name == "init" {
                    return Some(&m.function);
                }
            }
            None
        });

        let func_name = format!("{}_init", c.name.name);
        let mut params = Vec::new();
        if let Some(init) = init_method {
            for p in &init.params {
                params.push((p.name.name.clone(), self.map_type(&p.type_annotation)));
            }
        }

        let func = NirFunction::new(func_name, params.clone(), NirType::Ptr);
        self.current_fn = Some(func);
        self.start_block("entry");
        self.push_scope();

        // Allocate instance
        let layout = self.classes.get(&c.name.name).cloned().unwrap();
        let size = layout.total_size_bytes.max(8);
        let self_var = self.new_temp();
        self.emit_inst(NirInst::Alloc {
            dest: self_var.clone(),
            size_bytes: NirOperand::Const(NirConst::Int(size as i64, NirType::I64)),
        });

        // Store self in scope
        let self_ptr = self.new_temp();
        self.emit_inst(NirInst::Alloca {
            dest: self_ptr.clone(),
            ty: NirType::Ptr,
        });
        self.emit_inst(NirInst::Store {
            dest_ptr: self_ptr.clone(),
            src: NirOperand::Var(self_var.clone()),
        });
        self.insert_var("self".to_string(), self_ptr, NirType::Ptr);
        self.insert_var_class("self".to_string(), c.name.name.clone());

        // Store constructor parameters
        for (pname, pty) in params {
            let p_ptr = self.new_temp();
            self.emit_inst(NirInst::Alloca {
                dest: p_ptr.clone(),
                ty: pty.clone(),
            });
            self.emit_inst(NirInst::Store {
                dest_ptr: p_ptr.clone(),
                src: NirOperand::Var(pname.clone()),
            });
            self.insert_var(pname, p_ptr, pty);
        }

        if let Some(init) = init_method {
            for stmt in &init.body.statements {
                self.lower_statement(stmt);
            }
        }

        self.finish_block(NirTerminator::Return(Some(NirOperand::Var(self_var))));
        self.pop_scope();

        if let Some(f) = self.current_fn.take() {
            self.module.functions.push(f);
        }

        // Other methods
        for member in &c.members {
            if let ClassMember::Method(m) = member {
                if m.function.name.name == "init" {
                    continue;
                }
                let method_name = format!("{}_{}", c.name.name, m.function.name.name);
                let mut m_params = Vec::new();
                if !m.is_static {
                    m_params.push(("self".to_string(), NirType::Ptr));
                }
                for p in &m.function.params {
                    m_params.push((p.name.name.clone(), self.map_type(&p.type_annotation)));
                }

                let ret_ty = m
                    .function
                    .return_type
                    .as_ref()
                    .map(|t| self.map_type(t))
                    .unwrap_or(NirType::Void);

                let m_func = NirFunction::new(method_name, m_params.clone(), ret_ty);
                self.current_fn = Some(m_func);
                self.start_block("entry");
                self.push_scope();

                for (pname, pty) in m_params {
                    let p_ptr = self.new_temp();
                    self.emit_inst(NirInst::Alloca {
                        dest: p_ptr.clone(),
                        ty: pty.clone(),
                    });
                    self.emit_inst(NirInst::Store {
                        dest_ptr: p_ptr.clone(),
                        src: NirOperand::Var(pname.clone()),
                    });
                    if pname == "self" {
                        self.insert_var_class("self".to_string(), c.name.name.clone());
                    }
                    self.insert_var(pname, p_ptr, pty);
                }

                for stmt in &m.function.body.statements {
                    self.lower_statement(stmt);
                }

                if self.current_block.is_some() {
                    self.finish_block(NirTerminator::Return(None));
                }
                self.pop_scope();

                if let Some(f) = self.current_fn.take() {
                    self.module.functions.push(f);
                }
            }
        }
        self.current_class = None;
    }

    fn lower_function(&mut self, f: &FunctionDecl, prefix: Option<&str>) {
        let func_name = if let Some(p) = prefix {
            format!("{}_{}", p, f.name.name)
        } else {
            f.name.name.clone()
        };

        let params: Vec<(String, NirType)> = f
            .params
            .iter()
            .map(|p| (p.name.name.clone(), self.map_type(&p.type_annotation)))
            .collect();

        let ret_ty = if f.name.name == "main" {
            NirType::I32
        } else {
            f.return_type
                .as_ref()
                .map(|t| self.map_type(t))
                .unwrap_or(NirType::Void)
        };

        let mut func = NirFunction::new(func_name, params.clone(), ret_ty.clone());
        if f.name.name == "main" {
            func.is_entry = true;
        }

        self.current_fn = Some(func);
        self.start_block("entry");
        self.push_scope();

        for (pname, pty) in params {
            let p_ptr = self.new_temp();
            self.emit_inst(NirInst::Alloca {
                dest: p_ptr.clone(),
                ty: pty.clone(),
            });
            self.emit_inst(NirInst::Store {
                dest_ptr: p_ptr.clone(),
                src: NirOperand::Var(pname.clone()),
            });
            self.insert_var(pname, p_ptr, pty);
        }

        for stmt in &f.body.statements {
            self.lower_statement(stmt);
        }

        if self.current_block.is_some() {
            let ret_val = if ret_ty == NirType::I32 && f.name.name == "main" {
                Some(NirOperand::Const(NirConst::Int(0, NirType::I32)))
            } else {
                None
            };
            self.finish_block(NirTerminator::Return(ret_val));
        }

        self.pop_scope();
        if let Some(f) = self.current_fn.take() {
            self.module.functions.push(f);
        }
    }

    fn lower_top_level_main(&mut self, stmts: &[&Stmt]) {
        let mut func = NirFunction::new("main", Vec::new(), NirType::I32);
        func.is_entry = true;
        self.current_fn = Some(func);
        self.start_block("entry");
        self.push_scope();

        for stmt in stmts {
            self.lower_statement(stmt);
        }

        if self.current_block.is_some() {
            self.finish_block(NirTerminator::Return(Some(NirOperand::Const(
                NirConst::Int(0, NirType::I32),
            ))));
        }

        self.pop_scope();
        if let Some(f) = self.current_fn.take() {
            self.module.functions.push(f);
        }
    }

    fn lower_statement(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::VarDecl(decl) => {
                let ty = self.map_type(&decl.type_annotation);
                let ptr = self.new_temp();
                self.emit_inst(NirInst::Alloca {
                    dest: ptr.clone(),
                    ty: ty.clone(),
                });

                if let Some(ref init) = decl.initializer {
                    let val = self.lower_expr(init);
                    self.emit_inst(NirInst::Store {
                        dest_ptr: ptr.clone(),
                        src: val,
                    });
                }
                if let Type::Named { name, .. } = &decl.type_annotation {
                    if self.classes.contains_key(name) {
                        self.insert_var_class(decl.name.name.clone(), name.clone());
                    }
                }
                if let Some(Expr::Call { callee, .. }) = &decl.initializer {
                    if let Expr::Identifier(id) = &**callee {
                        if self.classes.contains_key(&id.name) {
                            self.insert_var_class(decl.name.name.clone(), id.name.clone());
                        }
                    }
                }
                self.insert_var(decl.name.name.clone(), ptr, ty);
            }
            Stmt::Assignment(assign) => {
                let rhs_val = self.lower_expr(&assign.value);
                match &assign.target {
                    Expr::Identifier(ident) => {
                        if let Some((ptr, ty)) = self.lookup_var(&ident.name) {
                            let final_val = match assign.op {
                                AssignOp::Assign => rhs_val,
                                AssignOp::AddAssign => {
                                    let cur = self.new_temp();
                                    self.emit_inst(NirInst::Load {
                                        dest: cur.clone(),
                                        src_ptr: ptr.clone(),
                                        ty: ty.clone(),
                                    });
                                    let res = self.new_temp();
                                    self.emit_inst(NirInst::BinOp {
                                        dest: res.clone(),
                                        op: NirBinaryOp::Add,
                                        lhs: NirOperand::Var(cur),
                                        rhs: rhs_val,
                                        ty,
                                    });
                                    NirOperand::Var(res)
                                }
                                AssignOp::SubAssign => {
                                    let cur = self.new_temp();
                                    self.emit_inst(NirInst::Load {
                                        dest: cur.clone(),
                                        src_ptr: ptr.clone(),
                                        ty: ty.clone(),
                                    });
                                    let res = self.new_temp();
                                    self.emit_inst(NirInst::BinOp {
                                        dest: res.clone(),
                                        op: NirBinaryOp::Sub,
                                        lhs: NirOperand::Var(cur),
                                        rhs: rhs_val,
                                        ty,
                                    });
                                    NirOperand::Var(res)
                                }
                                _ => rhs_val,
                            };
                            self.emit_inst(NirInst::Store {
                                dest_ptr: ptr,
                                src: final_val,
                            });
                        }
                    }
                    Expr::PropertyAccess {
                        target, property, ..
                    } => {
                        let obj_ptr = self.lower_expr(target);
                        let obj_var = match obj_ptr {
                            NirOperand::Var(v) => v,
                            _ => self.new_temp(),
                        };
                        let (offset, field_ty) = self.lookup_field_info(&property.name);
                        let field_ptr = self.new_temp();
                        self.emit_inst(NirInst::GetFieldPtr {
                            dest: field_ptr.clone(),
                            base_ptr: obj_var,
                            byte_offset: offset,
                            ty: field_ty,
                        });
                        self.emit_inst(NirInst::Store {
                            dest_ptr: field_ptr,
                            src: rhs_val,
                        });
                    }
                    _ => {}
                }
            }
            Stmt::Update(up) => {
                let val = self.lower_expr(&up.target);
                if let Expr::Identifier(id) = &up.target {
                    if let Some((ptr, ty)) = self.lookup_var(&id.name) {
                        let one = NirOperand::Const(NirConst::Int(1, ty.clone()));
                        let op = match up.op {
                            UpdateOp::Increment => NirBinaryOp::Add,
                            UpdateOp::Decrement => NirBinaryOp::Sub,
                        };
                        let updated = self.new_temp();
                        self.emit_inst(NirInst::BinOp {
                            dest: updated.clone(),
                            op,
                            lhs: val,
                            rhs: one,
                            ty,
                        });
                        self.emit_inst(NirInst::Store {
                            dest_ptr: ptr,
                            src: NirOperand::Var(updated),
                        });
                    }
                }
            }
            Stmt::Expr(expr) => {
                self.lower_expr(expr);
            }
            Stmt::Return(ret) => {
                let val = if let Some(v) = &ret.value {
                    Some(self.lower_expr(v))
                } else if let Some(f) = &self.current_fn {
                    if f.ret_ty == NirType::Ptr && f.name.ends_with("_init") {
                        if let Some((ptr, ty)) = self.lookup_var("self") {
                            let dest = self.new_temp();
                            self.emit_inst(NirInst::Load {
                                dest: dest.clone(),
                                src_ptr: ptr,
                                ty,
                            });
                            Some(NirOperand::Var(dest))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };
                self.finish_block(NirTerminator::Return(val));
                let unreach_lbl = self.new_label("unreachable");
                self.start_block(unreach_lbl);
            }
            Stmt::If(if_stmt) => {
                let then_lbl = self.new_label("if_then");
                let merge_lbl = self.new_label("if_merge");

                // Evaluate condition
                let cond = self.lower_expr(&if_stmt.condition);

                let next_lbl =
                    if !if_stmt.otherwise_branches.is_empty() || if_stmt.else_branch.is_some() {
                        self.new_label("if_otherwise_or_else")
                    } else {
                        merge_lbl.clone()
                    };

                self.finish_block(NirTerminator::CondBranch {
                    cond,
                    then_block: then_lbl.clone(),
                    else_block: next_lbl.clone(),
                });

                // Then block
                self.start_block(then_lbl);
                self.push_scope();
                for s in &if_stmt.then_branch.statements {
                    self.lower_statement(s);
                }
                self.pop_scope();
                if self.current_block.is_some() {
                    self.finish_block(NirTerminator::Branch(merge_lbl.clone()));
                }

                // Otherwise branches
                let mut current_next = next_lbl;
                for (i, ow) in if_stmt.otherwise_branches.iter().enumerate() {
                    self.start_block(current_next);
                    let ow_cond = self.lower_expr(&ow.condition);
                    let ow_then = self.new_label("ow_then");
                    current_next = if i + 1 < if_stmt.otherwise_branches.len()
                        || if_stmt.else_branch.is_some()
                    {
                        self.new_label("ow_next")
                    } else {
                        merge_lbl.clone()
                    };

                    self.finish_block(NirTerminator::CondBranch {
                        cond: ow_cond,
                        then_block: ow_then.clone(),
                        else_block: current_next.clone(),
                    });

                    self.start_block(ow_then);
                    self.push_scope();
                    for s in &ow.body.statements {
                        self.lower_statement(s);
                    }
                    self.pop_scope();
                    if self.current_block.is_some() {
                        self.finish_block(NirTerminator::Branch(merge_lbl.clone()));
                    }
                }

                // Else block
                if let Some(ref else_block) = if_stmt.else_branch {
                    self.start_block(current_next);
                    self.push_scope();
                    for s in &else_block.statements {
                        self.lower_statement(s);
                    }
                    self.pop_scope();
                    if self.current_block.is_some() {
                        self.finish_block(NirTerminator::Branch(merge_lbl.clone()));
                    }
                }

                // Merge block
                self.start_block(merge_lbl);
            }
            Stmt::AsLoop(as_loop) => {
                let header_lbl = self.new_label("as_header");
                let body_lbl = self.new_label("as_body");
                let exit_lbl = self.new_label("as_exit");

                self.finish_block(NirTerminator::Branch(header_lbl.clone()));

                // Header
                self.start_block(header_lbl.clone());
                let cond = self.lower_expr(&as_loop.condition);
                self.finish_block(NirTerminator::CondBranch {
                    cond,
                    then_block: body_lbl.clone(),
                    else_block: exit_lbl.clone(),
                });

                // Body
                self.start_block(body_lbl);
                self.loop_stack.push((header_lbl.clone(), exit_lbl.clone()));
                self.push_scope();
                for s in &as_loop.body.statements {
                    self.lower_statement(s);
                }
                self.pop_scope();
                self.loop_stack.pop();
                if self.current_block.is_some() {
                    self.finish_block(NirTerminator::Branch(header_lbl));
                }

                // Exit
                self.start_block(exit_lbl);
            }
            Stmt::ForLoop(fr_loop) => {
                let header_lbl = self.new_label("for_header");
                let body_lbl = self.new_label("for_body");
                let update_lbl = self.new_label("for_update");
                let exit_lbl = self.new_label("for_exit");

                self.push_scope();
                if let Some(ref init) = fr_loop.init {
                    self.lower_statement(init);
                }
                self.finish_block(NirTerminator::Branch(header_lbl.clone()));

                // Header
                self.start_block(header_lbl.clone());
                let cond = if let Some(ref c) = fr_loop.condition {
                    self.lower_expr(c)
                } else {
                    NirOperand::Const(NirConst::Bool(true))
                };
                self.finish_block(NirTerminator::CondBranch {
                    cond,
                    then_block: body_lbl.clone(),
                    else_block: exit_lbl.clone(),
                });

                // Body
                self.start_block(body_lbl);
                self.loop_stack.push((update_lbl.clone(), exit_lbl.clone()));
                for s in &fr_loop.body.statements {
                    self.lower_statement(s);
                }
                self.loop_stack.pop();
                if self.current_block.is_some() {
                    self.finish_block(NirTerminator::Branch(update_lbl.clone()));
                }

                // Update
                self.start_block(update_lbl);
                if let Some(ref up) = fr_loop.update {
                    self.lower_statement(up);
                }
                self.finish_block(NirTerminator::Branch(header_lbl));

                // Exit
                self.start_block(exit_lbl);
                self.pop_scope();
            }
            Stmt::DoAsLoop(do_loop) => {
                let body_lbl = self.new_label("do_body");
                let cond_lbl = self.new_label("do_cond");
                let exit_lbl = self.new_label("do_exit");

                self.finish_block(NirTerminator::Branch(body_lbl.clone()));

                // Body
                self.start_block(body_lbl.clone());
                self.loop_stack.push((cond_lbl.clone(), exit_lbl.clone()));
                self.push_scope();
                for s in &do_loop.body.statements {
                    self.lower_statement(s);
                }
                self.pop_scope();
                self.loop_stack.pop();
                if self.current_block.is_some() {
                    self.finish_block(NirTerminator::Branch(cond_lbl.clone()));
                }

                // Cond
                self.start_block(cond_lbl);
                let cond = self.lower_expr(&do_loop.condition);
                self.finish_block(NirTerminator::CondBranch {
                    cond,
                    then_block: body_lbl,
                    else_block: exit_lbl.clone(),
                });

                // Exit
                self.start_block(exit_lbl);
            }
            Stmt::Block(b) => {
                self.push_scope();
                for s in &b.statements {
                    self.lower_statement(s);
                }
                self.pop_scope();
            }
            Stmt::Unsafe(u) => {
                self.push_scope();
                for s in &u.body.statements {
                    self.lower_statement(s);
                }
                self.pop_scope();
            }
            Stmt::Match(m) => {
                self.lower_match(m);
            }
            _ => {}
        }
    }

    fn lower_match(&mut self, m: &MatchStmt) {
        let target_val = self.lower_expr(&m.target);
        let target_ty = NirType::I64;
        let merge_lbl = self.new_label("match_merge");

        let mut next_case_lbl = self.new_label("case");
        self.finish_block(NirTerminator::Branch(next_case_lbl.clone()));

        for (idx, c) in m.cases.iter().enumerate() {
            let current_case_lbl = next_case_lbl;
            next_case_lbl = if idx + 1 < m.cases.len() {
                self.new_label("case")
            } else {
                merge_lbl.clone()
            };

            let body_lbl = self.new_label("case_body");
            self.start_block(current_case_lbl);

            match &c.pattern {
                Pattern::Literal(lit) => {
                    let lit_op = self.lower_literal(lit);
                    let cmp_res = self.new_temp();
                    self.emit_inst(NirInst::Cmp {
                        dest: cmp_res.clone(),
                        op: NirCmpOp::Eq,
                        lhs: target_val.clone(),
                        rhs: lit_op,
                    });
                    self.finish_block(NirTerminator::CondBranch {
                        cond: NirOperand::Var(cmp_res),
                        then_block: body_lbl.clone(),
                        else_block: next_case_lbl.clone(),
                    });
                }
                Pattern::Wildcard(_) => {
                    self.finish_block(NirTerminator::Branch(body_lbl.clone()));
                }
                Pattern::Binding(id) => {
                    let mut cond_op = NirOperand::Const(NirConst::Bool(true));
                    if let Some(ref guard) = c.guard {
                        self.push_scope();
                        let id_ptr = self.new_temp();
                        self.emit_inst(NirInst::Alloca {
                            dest: id_ptr.clone(),
                            ty: target_ty.clone(),
                        });
                        self.emit_inst(NirInst::Store {
                            dest_ptr: id_ptr.clone(),
                            src: target_val.clone(),
                        });
                        self.insert_var(id.name.clone(), id_ptr, target_ty.clone());
                        cond_op = self.lower_expr(guard);
                        self.pop_scope();
                    }
                    self.finish_block(NirTerminator::CondBranch {
                        cond: cond_op,
                        then_block: body_lbl.clone(),
                        else_block: next_case_lbl.clone(),
                    });
                }
                Pattern::Alternative(patterns, _) => {
                    let mut or_cond: Option<NirOperand> = None;
                    for p in patterns {
                        if let Pattern::Literal(lit) = p {
                            let lit_op = self.lower_literal(lit);
                            let eq_tmp = self.new_temp();
                            self.emit_inst(NirInst::Cmp {
                                dest: eq_tmp.clone(),
                                op: NirCmpOp::Eq,
                                lhs: target_val.clone(),
                                rhs: lit_op,
                            });
                            if let Some(prev) = or_cond {
                                let merged = self.new_temp();
                                self.emit_inst(NirInst::BinOp {
                                    dest: merged.clone(),
                                    op: NirBinaryOp::BitOr,
                                    lhs: prev,
                                    rhs: NirOperand::Var(eq_tmp),
                                    ty: NirType::Bool,
                                });
                                or_cond = Some(NirOperand::Var(merged));
                            } else {
                                or_cond = Some(NirOperand::Var(eq_tmp));
                            }
                        }
                    }
                    let cond = or_cond.unwrap_or(NirOperand::Const(NirConst::Bool(false)));
                    self.finish_block(NirTerminator::CondBranch {
                        cond,
                        then_block: body_lbl.clone(),
                        else_block: next_case_lbl.clone(),
                    });
                }
            }

            // Case body
            self.start_block(body_lbl);
            self.push_scope();
            if let Pattern::Binding(id) = &c.pattern {
                let id_ptr = self.new_temp();
                self.emit_inst(NirInst::Alloca {
                    dest: id_ptr.clone(),
                    ty: target_ty.clone(),
                });
                self.emit_inst(NirInst::Store {
                    dest_ptr: id_ptr.clone(),
                    src: target_val.clone(),
                });
                self.insert_var(id.name.clone(), id_ptr, target_ty.clone());
            }
            for s in &c.body.statements {
                self.lower_statement(s);
            }
            self.pop_scope();
            if self.current_block.is_some() {
                self.finish_block(NirTerminator::Branch(merge_lbl.clone()));
            }
        }

        self.start_block(merge_lbl);
    }

    fn lower_expr(&mut self, expr: &Expr) -> NirOperand {
        match expr {
            Expr::Literal(lit) => self.lower_literal(lit),
            Expr::Identifier(ident) => {
                if let Some((ptr, ty)) = self.lookup_var(&ident.name) {
                    let dest = self.new_temp();
                    self.emit_inst(NirInst::Load {
                        dest: dest.clone(),
                        src_ptr: ptr,
                        ty,
                    });
                    NirOperand::Var(dest)
                } else {
                    NirOperand::Const(NirConst::Null)
                }
            }
            Expr::Binary {
                op, left, right, ..
            } => {
                let left_val = self.lower_expr(left);
                let left_ty = self.infer_operand_type(&left_val, left);

                if *op == BinaryOp::And && left_ty == NirType::Bool {
                    let result_ptr = self.new_temp();
                    self.emit_inst(NirInst::Alloca {
                        dest: result_ptr.clone(),
                        ty: NirType::Bool,
                    });
                    self.emit_inst(NirInst::Store {
                        dest_ptr: result_ptr.clone(),
                        src: NirOperand::Const(NirConst::Bool(false)),
                    });

                    let eval_right_lbl = self.new_label("and_rhs");
                    let merge_lbl = self.new_label("and_merge");

                    self.finish_block(NirTerminator::CondBranch {
                        cond: left_val,
                        then_block: eval_right_lbl.clone(),
                        else_block: merge_lbl.clone(),
                    });

                    self.start_block(eval_right_lbl);
                    let right_val = self.lower_expr(right);
                    self.emit_inst(NirInst::Store {
                        dest_ptr: result_ptr.clone(),
                        src: right_val,
                    });
                    self.finish_block(NirTerminator::Branch(merge_lbl.clone()));

                    self.start_block(merge_lbl);
                    let dest = self.new_temp();
                    self.emit_inst(NirInst::Load {
                        dest: dest.clone(),
                        src_ptr: result_ptr,
                        ty: NirType::Bool,
                    });
                    return NirOperand::Var(dest);
                }

                if *op == BinaryOp::Or && left_ty == NirType::Bool {
                    let result_ptr = self.new_temp();
                    self.emit_inst(NirInst::Alloca {
                        dest: result_ptr.clone(),
                        ty: NirType::Bool,
                    });
                    self.emit_inst(NirInst::Store {
                        dest_ptr: result_ptr.clone(),
                        src: NirOperand::Const(NirConst::Bool(true)),
                    });

                    let eval_right_lbl = self.new_label("or_rhs");
                    let merge_lbl = self.new_label("or_merge");

                    self.finish_block(NirTerminator::CondBranch {
                        cond: left_val,
                        then_block: merge_lbl.clone(),
                        else_block: eval_right_lbl.clone(),
                    });

                    self.start_block(eval_right_lbl);
                    let right_val = self.lower_expr(right);
                    self.emit_inst(NirInst::Store {
                        dest_ptr: result_ptr.clone(),
                        src: right_val,
                    });
                    self.finish_block(NirTerminator::Branch(merge_lbl.clone()));

                    self.start_block(merge_lbl);
                    let dest = self.new_temp();
                    self.emit_inst(NirInst::Load {
                        dest: dest.clone(),
                        src_ptr: result_ptr,
                        ty: NirType::Bool,
                    });
                    return NirOperand::Var(dest);
                }

                let lhs = left_val;
                let rhs = self.lower_expr(right);
                let lhs_ty = left_ty;
                let rhs_ty = self.infer_operand_type(&rhs, right);
                let calc_ty =
                    if *op == BinaryOp::Add && (lhs_ty == NirType::Ptr || rhs_ty == NirType::Ptr) {
                        NirType::Ptr
                    } else if lhs_ty.is_float() || rhs_ty.is_float() {
                        NirType::F64
                    } else {
                        lhs_ty
                    };

                match op {
                    BinaryOp::Add
                    | BinaryOp::Sub
                    | BinaryOp::Mul
                    | BinaryOp::Div
                    | BinaryOp::Mod
                    | BinaryOp::And
                    | BinaryOp::Or
                    | BinaryOp::BitXor
                    | BinaryOp::Shl
                    | BinaryOp::Shr => {
                        let nir_op = match op {
                            BinaryOp::Add => NirBinaryOp::Add,
                            BinaryOp::Sub => NirBinaryOp::Sub,
                            BinaryOp::Mul => NirBinaryOp::Mul,
                            BinaryOp::Div => NirBinaryOp::Div,
                            BinaryOp::Mod => NirBinaryOp::Mod,
                            BinaryOp::And => NirBinaryOp::BitAnd,
                            BinaryOp::Or => NirBinaryOp::BitOr,
                            BinaryOp::BitXor => NirBinaryOp::BitXor,
                            BinaryOp::Shl => NirBinaryOp::Shl,
                            BinaryOp::Shr => NirBinaryOp::Shr,
                            _ => unreachable!(),
                        };
                        let dest = self.new_temp();
                        self.emit_inst(NirInst::BinOp {
                            dest: dest.clone(),
                            op: nir_op,
                            lhs,
                            rhs,
                            ty: calc_ty,
                        });
                        NirOperand::Var(dest)
                    }
                    BinaryOp::Eq
                    | BinaryOp::NotEq
                    | BinaryOp::Lt
                    | BinaryOp::LtEq
                    | BinaryOp::Gt
                    | BinaryOp::GtEq => {
                        let cmp_op = match op {
                            BinaryOp::Eq => NirCmpOp::Eq,
                            BinaryOp::NotEq => NirCmpOp::Ne,
                            BinaryOp::Lt => NirCmpOp::Lt,
                            BinaryOp::LtEq => NirCmpOp::Le,
                            BinaryOp::Gt => NirCmpOp::Gt,
                            BinaryOp::GtEq => NirCmpOp::Ge,
                            _ => unreachable!(),
                        };
                        let dest = self.new_temp();
                        self.emit_inst(NirInst::Cmp {
                            dest: dest.clone(),
                            op: cmp_op,
                            lhs,
                            rhs,
                        });
                        NirOperand::Var(dest)
                    }
                    _ => lhs,
                }
            }
            Expr::Unary { op, operand, .. } => {
                let val = self.lower_expr(operand);
                let op_ty = self.infer_operand_type(&val, operand);
                let dest = self.new_temp();
                match op {
                    UnaryOp::Neg => {
                        self.emit_inst(NirInst::UnaryOp {
                            dest: dest.clone(),
                            op: NirUnaryOp::Neg,
                            operand: val,
                            ty: op_ty,
                        });
                        NirOperand::Var(dest)
                    }
                    UnaryOp::Not => {
                        self.emit_inst(NirInst::UnaryOp {
                            dest: dest.clone(),
                            op: NirUnaryOp::Not,
                            operand: val,
                            ty: NirType::Bool,
                        });
                        NirOperand::Var(dest)
                    }
                    UnaryOp::BitNot => {
                        self.emit_inst(NirInst::UnaryOp {
                            dest: dest.clone(),
                            op: NirUnaryOp::BitNot,
                            operand: val,
                            ty: op_ty,
                        });
                        NirOperand::Var(dest)
                    }
                    UnaryOp::Pos => val,
                }
            }
            Expr::Call { callee, args, .. } => {
                if let Expr::Identifier(ref id) = **callee {
                    // Builtin: vout
                    if id.name == "vout" {
                        let mut arg_ops = Vec::new();
                        for a in args {
                            let op = self.lower_expr(a);
                            let ty = self.infer_operand_type(&op, a);
                            arg_ops.push((op, ty));
                        }
                        self.emit_inst(NirInst::Vout { args: arg_ops });
                        return NirOperand::Const(NirConst::Null);
                    }
                    // Builtin: input
                    if id.name == "input" {
                        let prompt = if let Some(Expr::Literal(Literal::String { value, .. })) =
                            args.first()
                        {
                            Some(value.clone())
                        } else {
                            None
                        };
                        let dest = self.new_temp();
                        self.emit_inst(NirInst::Input {
                            dest: dest.clone(),
                            prompt,
                        });
                        return NirOperand::Var(dest);
                    }
                    // Builtin: to_int
                    if id.name == "to_int" {
                        if let Some(arg) = args.first() {
                            let src = self.lower_expr(arg);
                            let dest = self.new_temp();
                            self.emit_inst(NirInst::ToInt {
                                dest: dest.clone(),
                                src,
                                src_ty: NirType::Ptr,
                            });
                            return NirOperand::Var(dest);
                        }
                    }
                    // Builtin: to_float
                    if id.name == "to_float" {
                        if let Some(arg) = args.first() {
                            let src = self.lower_expr(arg);
                            let dest = self.new_temp();
                            self.emit_inst(NirInst::ToFloat {
                                dest: dest.clone(),
                                src,
                                src_ty: NirType::Ptr,
                            });
                            return NirOperand::Var(dest);
                        }
                    }
                    // Builtin: len
                    if id.name == "len" {
                        if let Some(arg) = args.first() {
                            let src = self.lower_expr(arg);
                            let dest = self.new_temp();
                            self.emit_inst(NirInst::Len {
                                dest: dest.clone(),
                                src,
                                src_ty: NirType::Ptr,
                            });
                            return NirOperand::Var(dest);
                        }
                    }
                    // Builtin: alloc
                    if id.name == "alloc" {
                        if let Some(arg) = args.first() {
                            let lowered_arg = self.lower_expr(arg);
                            let arg_ty = self.infer_operand_type(&lowered_arg, arg);
                            if arg_ty == NirType::Ptr {
                                let empty_lbl = self.module.add_string_literal("");
                                let empty_op = NirOperand::Const(NirConst::String(empty_lbl));
                                let dest = self.new_temp();
                                self.emit_inst(NirInst::BinOp {
                                    dest: dest.clone(),
                                    op: NirBinaryOp::Add,
                                    lhs: lowered_arg,
                                    rhs: empty_op,
                                    ty: NirType::Ptr,
                                });
                                return NirOperand::Var(dest);
                            } else {
                                let dest = self.new_temp();
                                self.emit_inst(NirInst::Alloc {
                                    dest: dest.clone(),
                                    size_bytes: lowered_arg,
                                });
                                return NirOperand::Var(dest);
                            }
                        }
                        let dest = self.new_temp();
                        self.emit_inst(NirInst::Alloc {
                            dest: dest.clone(),
                            size_bytes: NirOperand::Const(NirConst::Int(64, NirType::I64)),
                        });
                        return NirOperand::Var(dest);
                    }
                    // Builtin: free
                    if id.name == "free" {
                        if let Some(arg) = args.first() {
                            let ptr = self.lower_expr(arg);
                            self.emit_inst(NirInst::Free { ptr });
                        }
                        return NirOperand::Const(NirConst::Null);
                    }

                    // Class constructor: ClassName(...)
                    if self.classes.contains_key(&id.name) {
                        let init_func = format!("{}_init", id.name);
                        let lowered_args: Vec<NirOperand> =
                            args.iter().map(|a| self.lower_expr(a)).collect();
                        let dest = self.new_temp();
                        self.emit_inst(NirInst::Call {
                            dest: Some(dest.clone()),
                            callee: init_func,
                            args: lowered_args,
                            ret_ty: NirType::Ptr,
                        });
                        return NirOperand::Var(dest);
                    }

                    // Extern C or native runtime function call
                    if let Some(ret_ty) = self.extern_functions.get(&id.name).cloned() {
                        let lowered_args: Vec<NirOperand> =
                            args.iter().map(|a| self.lower_expr(a)).collect();
                        let dest = if ret_ty == NirType::Void {
                            None
                        } else {
                            Some(self.new_temp())
                        };
                        self.emit_inst(NirInst::Call {
                            dest: dest.clone(),
                            callee: id.name.clone(),
                            args: lowered_args,
                            ret_ty,
                        });
                        return dest
                            .map(NirOperand::Var)
                            .unwrap_or(NirOperand::Const(NirConst::Null));
                    }

                    // Regular function call
                    let lowered_args: Vec<NirOperand> =
                        args.iter().map(|a| self.lower_expr(a)).collect();
                    let ret_ty = self
                        .module
                        .functions
                        .iter()
                        .find(|f| f.name == id.name)
                        .map(|f| f.ret_ty.clone())
                        .unwrap_or(NirType::I64);
                    let dest = if ret_ty == NirType::Void {
                        None
                    } else {
                        Some(self.new_temp())
                    };
                    self.emit_inst(NirInst::Call {
                        dest: dest.clone(),
                        callee: id.name.clone(),
                        args: lowered_args,
                        ret_ty,
                    });
                    return dest
                        .map(NirOperand::Var)
                        .unwrap_or(NirOperand::Const(NirConst::Null));
                }

                // Method call: obj.method(args...)
                if let Expr::PropertyAccess {
                    target, property, ..
                } = &**callee
                {
                    // Check if static method call: ClassName.static_method(...)
                    if let Expr::Identifier(id) = &**target {
                        if self.classes.contains_key(&id.name) {
                            let method_fn = format!("{}_{}", id.name, property.name);
                            let call_args: Vec<NirOperand> =
                                args.iter().map(|a| self.lower_expr(a)).collect();
                            let ret_ty = self
                                .module
                                .functions
                                .iter()
                                .find(|f| f.name == method_fn)
                                .map(|f| f.ret_ty.clone())
                                .unwrap_or(NirType::I64);
                            let dest = if ret_ty == NirType::Void {
                                None
                            } else {
                                Some(self.new_temp())
                            };
                            self.emit_inst(NirInst::Call {
                                dest: dest.clone(),
                                callee: method_fn,
                                args: call_args,
                                ret_ty,
                            });
                            return dest
                                .map(NirOperand::Var)
                                .unwrap_or(NirOperand::Const(NirConst::Null));
                        }
                    }

                    // Instance method call or super call
                    let (method_fn, ret_ty) = self.resolve_method(target, &property.name);
                    let obj_op = if let Expr::Identifier(id) = &**target {
                        if id.name == "super" {
                            if let Some((ptr, ty)) = self.lookup_var("self") {
                                let dest = self.new_temp();
                                self.emit_inst(NirInst::Load {
                                    dest: dest.clone(),
                                    src_ptr: ptr,
                                    ty,
                                });
                                NirOperand::Var(dest)
                            } else {
                                self.lower_expr(target)
                            }
                        } else {
                            self.lower_expr(target)
                        }
                    } else {
                        self.lower_expr(target)
                    };

                    let mut call_args = vec![obj_op];
                    for a in args {
                        call_args.push(self.lower_expr(a));
                    }
                    let dest = if ret_ty == NirType::Void {
                        None
                    } else {
                        Some(self.new_temp())
                    };
                    self.emit_inst(NirInst::Call {
                        dest: dest.clone(),
                        callee: method_fn,
                        args: call_args,
                        ret_ty,
                    });
                    return dest
                        .map(NirOperand::Var)
                        .unwrap_or(NirOperand::Const(NirConst::Null));
                }

                NirOperand::Const(NirConst::Null)
            }
            Expr::PropertyAccess {
                target, property, ..
            } => {
                let obj_op = self.lower_expr(target);
                let obj_var = match obj_op {
                    NirOperand::Var(v) => v,
                    _ => self.new_temp(),
                };
                let (offset, field_ty) = self.lookup_field_info(&property.name);
                let dest = self.new_temp();
                self.emit_inst(NirInst::GetFieldPtr {
                    dest: dest.clone(),
                    base_ptr: obj_var,
                    byte_offset: offset,
                    ty: field_ty.clone(),
                });
                let loaded = self.new_temp();
                self.emit_inst(NirInst::Load {
                    dest: loaded.clone(),
                    src_ptr: dest,
                    ty: field_ty,
                });
                NirOperand::Var(loaded)
            }
            Expr::Group { expr, .. } => self.lower_expr(expr),
            Expr::Borrow { expr, .. } => self.lower_expr(expr),
            Expr::Alloc { expr, .. } => {
                let inner = self.lower_expr(expr);
                let inner_ty = self.infer_operand_type(&inner, expr);
                if inner_ty == NirType::Ptr {
                    let empty_lbl = self.module.add_string_literal("");
                    let empty_op = NirOperand::Const(NirConst::String(empty_lbl));
                    let dest = self.new_temp();
                    self.emit_inst(NirInst::BinOp {
                        dest: dest.clone(),
                        op: NirBinaryOp::Add,
                        lhs: inner,
                        rhs: empty_op,
                        ty: NirType::Ptr,
                    });
                    NirOperand::Var(dest)
                } else {
                    let dest = self.new_temp();
                    self.emit_inst(NirInst::Alloc {
                        dest: dest.clone(),
                        size_bytes: NirOperand::Const(NirConst::Int(64, NirType::I64)),
                    });
                    self.emit_inst(NirInst::Store {
                        dest_ptr: dest.clone(),
                        src: inner,
                    });
                    NirOperand::Var(dest)
                }
            }
            Expr::Free { target, .. } => {
                let ptr = self.lower_expr(target);
                self.emit_inst(NirInst::Free { ptr });
                NirOperand::Const(NirConst::Null)
            }
            Expr::Update {
                op,
                target,
                is_prefix,
                ..
            } => {
                let val = self.lower_expr(target);
                if let Expr::Identifier(id) = &**target {
                    if let Some((ptr, ty)) = self.lookup_var(&id.name) {
                        let one = NirOperand::Const(NirConst::Int(1, ty.clone()));
                        let bop = match op {
                            UpdateOp::Increment => NirBinaryOp::Add,
                            UpdateOp::Decrement => NirBinaryOp::Sub,
                        };
                        let updated = self.new_temp();
                        self.emit_inst(NirInst::BinOp {
                            dest: updated.clone(),
                            op: bop,
                            lhs: val.clone(),
                            rhs: one,
                            ty,
                        });
                        self.emit_inst(NirInst::Store {
                            dest_ptr: ptr,
                            src: NirOperand::Var(updated.clone()),
                        });
                        if *is_prefix {
                            return NirOperand::Var(updated);
                        } else {
                            return val;
                        }
                    }
                }
                val
            }
            Expr::ListLiteral { elements, .. } => {
                let dest = self.new_temp();
                let size = (elements.len() * 8 + 16) as i64;
                self.emit_inst(NirInst::Alloc {
                    dest: dest.clone(),
                    size_bytes: NirOperand::Const(NirConst::Int(size, NirType::I64)),
                });
                NirOperand::Var(dest)
            }
            Expr::MapLiteral { .. } => {
                let dest = self.new_temp();
                self.emit_inst(NirInst::Alloc {
                    dest: dest.clone(),
                    size_bytes: NirOperand::Const(NirConst::Int(64, NirType::I64)),
                });
                NirOperand::Var(dest)
            }
            Expr::Index { target, .. } => self.lower_expr(target),
            Expr::StructLiteral { name, fields, .. } => {
                let layout = self.classes.get(&name.name).cloned();
                let size = layout
                    .as_ref()
                    .map(|l| l.total_size_bytes.max(8))
                    .unwrap_or(64);
                let struct_ptr = self.new_temp();
                self.emit_inst(NirInst::Alloc {
                    dest: struct_ptr.clone(),
                    size_bytes: NirOperand::Const(NirConst::Int(size as i64, NirType::I64)),
                });
                for (fname, fval) in fields {
                    let val = self.lower_expr(fval);
                    let (offset, field_ty) = if let Some(ref l) = layout {
                        let off = l.field_offsets.get(&fname.name).copied().unwrap_or(0);
                        let ty = l
                            .fields
                            .iter()
                            .find(|(n, _)| n == &fname.name)
                            .map(|(_, t)| t.clone())
                            .unwrap_or(NirType::I64);
                        (off, ty)
                    } else {
                        self.lookup_field_info(&fname.name)
                    };
                    let field_ptr = self.new_temp();
                    self.emit_inst(NirInst::GetFieldPtr {
                        dest: field_ptr.clone(),
                        base_ptr: struct_ptr.clone(),
                        byte_offset: offset,
                        ty: field_ty,
                    });
                    self.emit_inst(NirInst::Store {
                        dest_ptr: field_ptr,
                        src: val,
                    });
                }
                NirOperand::Var(struct_ptr)
            }
        }
    }

    fn lower_literal(&mut self, lit: &Literal) -> NirOperand {
        match lit {
            Literal::Integer { value, .. } => {
                NirOperand::Const(NirConst::Int(*value as i64, NirType::I64))
            }
            Literal::Float { raw, .. } => {
                let val = raw.parse::<f64>().unwrap_or(0.0);
                NirOperand::Const(NirConst::Float(val, NirType::F64))
            }
            Literal::Bool { value, .. } => NirOperand::Const(NirConst::Bool(*value)),
            Literal::Char { value, .. } => NirOperand::Const(NirConst::Char(*value)),
            Literal::String { value, .. } => {
                let label = self.module.add_string_literal(value);
                NirOperand::Const(NirConst::String(label))
            }
            Literal::Null { .. } => NirOperand::Const(NirConst::Null),
        }
    }

    fn infer_operand_type(&self, op: &NirOperand, expr: &Expr) -> NirType {
        match op {
            NirOperand::Const(c) => c.ty(),
            NirOperand::Var(name) => {
                if let Some(ty) = self.temp_types.get(name) {
                    return ty.clone();
                }
                if let Expr::Identifier(ident) = expr {
                    if let Some((_, ty)) = self.lookup_var(&ident.name) {
                        return ty;
                    }
                }
                for scope in self.var_env.iter().rev() {
                    for (vname, (ptr, ty)) in scope {
                        if ptr == name || vname == name {
                            return ty.clone();
                        }
                    }
                }
                match expr {
                    Expr::Literal(Literal::Float { .. }) => NirType::F64,
                    Expr::Literal(Literal::Bool { .. }) => NirType::Bool,
                    Expr::Literal(Literal::String { .. }) => NirType::Ptr,
                    Expr::StructLiteral { .. } => NirType::Ptr,
                    Expr::Call { callee, .. } => {
                        if let Expr::Identifier(id) = &**callee {
                            if id.name == "input" {
                                return NirType::Ptr;
                            } else if id.name == "to_int" {
                                return NirType::I64;
                            } else if id.name == "to_float" {
                                return NirType::F64;
                            } else if let Some(ret_ty) = self.extern_functions.get(&id.name) {
                                return ret_ty.clone();
                            } else if let Some(func) =
                                self.module.functions.iter().find(|f| f.name == id.name)
                            {
                                return func.ret_ty.clone();
                            }
                        }
                        NirType::I64
                    }
                    _ => NirType::I64,
                }
            }
        }
    }

    fn lookup_field_info(&self, field_name: &str) -> (usize, NirType) {
        if let Some(curr) = &self.current_class {
            if let Some(layout) = self.classes.get(curr) {
                if let Some(off) = layout.field_offsets.get(field_name) {
                    if let Some((_, ty)) = layout.fields.iter().find(|(f, _)| f == field_name) {
                        return (*off, ty.clone());
                    }
                }
            }
        }
        for layout in self.classes.values() {
            if let Some(off) = layout.field_offsets.get(field_name) {
                if let Some((_, ty)) = layout.fields.iter().find(|(f, _)| f == field_name) {
                    return (*off, ty.clone());
                }
                return (*off, NirType::I64);
            }
        }
        (0, NirType::I64)
    }

    fn resolve_method(&self, target_expr: &Expr, method_name: &str) -> (String, NirType) {
        let class_name = match target_expr {
            Expr::Identifier(id) if id.name == "super" => {
                if let Some(curr) = &self.current_class {
                    self.classes
                        .get(curr)
                        .and_then(|l| l.parent.clone())
                        .unwrap_or_default()
                } else {
                    String::new()
                }
            }
            Expr::Identifier(id) if id.name == "self" => {
                self.current_class.clone().unwrap_or_default()
            }
            Expr::Identifier(id) => self
                .lookup_var_class(&id.name)
                .unwrap_or_else(|| id.name.clone()),
            _ => String::new(),
        };

        let mut curr = class_name.clone();
        while !curr.is_empty() {
            let func_name = format!("{}_{}", curr, method_name);
            if let Some(f) = self.module.functions.iter().find(|f| f.name == func_name) {
                return (func_name, f.ret_ty.clone());
            }
            if let Some(layout) = self.classes.get(&curr) {
                if let Some(parent) = &layout.parent {
                    curr = parent.clone();
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        for f in &self.module.functions {
            if f.name.ends_with(&format!("_{}", method_name)) {
                return (f.name.clone(), f.ret_ty.clone());
            }
        }

        (format!("{}_{}", class_name, method_name), NirType::I64)
    }

    fn map_type(&self, ty: &Type) -> NirType {
        match ty {
            Type::Named { name, .. } => match name.as_str() {
                "INT8" => NirType::I8,
                "INT16" => NirType::I16,
                "INT32" => NirType::I32,
                "INT64" => NirType::I64,
                "INT128" => NirType::I128,
                "ISIZE" => NirType::I64,
                "UINT8" => NirType::U8,
                "UINT16" => NirType::U16,
                "UINT32" => NirType::U32,
                "UINT64" => NirType::U64,
                "UINT128" => NirType::U128,
                "USIZE" => NirType::U64,
                "PTR" => NirType::Ptr,
                "FL32" => NirType::F32,
                "FL64" => NirType::F64,
                "BOOL" => NirType::Bool,
                "CHAR" => NirType::Char,
                "STRING" => NirType::Ptr,
                "VOID" => NirType::Void,
                _ => NirType::Ptr,
            },
            Type::Generic { name, .. } => match name.as_str() {
                "HEAP" | "LIST" | "SET" | "MAP" => NirType::Ptr,
                _ => NirType::Ptr,
            },
            Type::Reference { .. } => NirType::Ptr,
            Type::Union { .. } => NirType::Ptr,
        }
    }
}
