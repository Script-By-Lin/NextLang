use crate::oop::{
    ClassFieldSymbol, ClassHierarchy, ClassMethodSymbol, ClassSymbol, InterfaceMethodSymbol,
    InterfaceSymbol,
};
use crate::symbol::{Symbol, SymbolKind, SymbolTable};
use crate::types::SemType;
use nx_ast::*;
use nx_diagnostics::{DiagnosticBag, Span};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
struct FunctionContext {
    name: String,
    return_type: SemType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VarStatus {
    Live,
    Moved(Span),
    Freed(Span),
}

#[derive(Debug, Clone)]
pub struct VarOwnership {
    pub status: VarStatus,
    pub immutable_borrows: usize,
    pub has_mutable_borrow: bool,
    pub is_tem: bool,
    pub is_mutable: bool,
    pub scope_level: usize,
    pub decl_span: Span,
    pub borrowed_target: Option<(String, bool)>,
}

pub struct SemanticAnalyzer<'a> {
    symbol_table: SymbolTable,
    diagnostics: &'a mut DiagnosticBag,
    current_function: Option<FunctionContext>,
    class_hierarchy: ClassHierarchy,
    current_class: Option<String>,
    ownership_vars: HashMap<String, Vec<VarOwnership>>,
    ownership_scope_vars: Vec<Vec<String>>,
    pub in_unsafe: bool,
    pub struct_defs: HashMap<String, Vec<(String, SemType)>>,
    pub current_is_async: bool,
}

impl<'a> SemanticAnalyzer<'a> {
    pub fn new(diagnostics: &'a mut DiagnosticBag) -> Self {
        Self {
            symbol_table: SymbolTable::new(),
            diagnostics,
            current_function: None,
            class_hierarchy: ClassHierarchy::new(),
            current_class: None,
            ownership_vars: HashMap::new(),
            ownership_scope_vars: vec![Vec::new()],
            in_unsafe: false,
            struct_defs: HashMap::new(),
            current_is_async: false,
        }
    }

    pub fn has_active_mutable_borrows(&self) -> bool {
        for stack in self.ownership_vars.values() {
            if let Some(var) = stack.last() {
                if var.has_mutable_borrow {
                    return true;
                }
            }
        }
        false
    }

    fn is_assignable(&self, src: &SemType, target: &SemType) -> bool {
        src.is_assignable_to_with_hierarchy(target, &self.class_hierarchy)
    }

    pub fn push_scope(&mut self) {
        self.symbol_table.push_scope();
        self.ownership_scope_vars.push(Vec::new());
    }

    pub fn pop_scope(&mut self) {
        self.symbol_table.pop_scope();
        if let Some(vars) = self.ownership_scope_vars.pop() {
            for name in vars {
                if let Some(stack) = self.ownership_vars.get_mut(&name) {
                    if let Some(var) = stack.pop() {
                        if let Some((target, is_mut)) = var.borrowed_target {
                            self.release_borrow(&target, is_mut);
                        }
                    }
                }
            }
        }
    }

    fn release_borrow(&mut self, target: &str, is_mut: bool) {
        if let Some(stack) = self.ownership_vars.get_mut(target) {
            if let Some(var) = stack.last_mut() {
                if is_mut {
                    var.has_mutable_borrow = false;
                } else {
                    var.immutable_borrows = var.immutable_borrows.saturating_sub(1);
                }
            }
        }
    }

    fn mark_borrowed(&mut self, target: &str, is_mut: bool) {
        if let Some(stack) = self.ownership_vars.get_mut(target) {
            if let Some(var) = stack.last_mut() {
                if is_mut {
                    var.has_mutable_borrow = true;
                } else {
                    var.immutable_borrows += 1;
                }
            }
        }
    }

    fn register_var_ownership(
        &mut self,
        name: &str,
        is_mutable: bool,
        is_tem: bool,
        decl_span: Span,
    ) {
        let var_state = VarOwnership {
            status: VarStatus::Live,
            immutable_borrows: 0,
            has_mutable_borrow: false,
            is_tem,
            is_mutable,
            scope_level: self.symbol_table.current_level(),
            decl_span,
            borrowed_target: None,
        };
        self.ownership_vars
            .entry(name.to_string())
            .or_default()
            .push(var_state);
        if let Some(scope) = self.ownership_scope_vars.last_mut() {
            scope.push(name.to_string());
        }
    }

    fn get_var_ownership(&self, name: &str) -> Option<&VarOwnership> {
        self.ownership_vars.get(name).and_then(|s| s.last())
    }

    fn get_var_status(&self, name: &str) -> Option<VarStatus> {
        self.get_var_ownership(name).map(|v| v.status.clone())
    }

    fn mark_moved(&mut self, name: &str, span: Span) {
        if let Some(stack) = self.ownership_vars.get_mut(name) {
            if let Some(var) = stack.last_mut() {
                var.status = VarStatus::Moved(span);
            }
        }
    }

    fn mark_freed(&mut self, name: &str, span: Span) {
        if let Some(stack) = self.ownership_vars.get_mut(name) {
            if let Some(var) = stack.last_mut() {
                var.status = VarStatus::Freed(span);
            }
        }
    }

    fn reset_var_live(&mut self, name: &str) {
        if let Some(stack) = self.ownership_vars.get_mut(name) {
            if let Some(var) = stack.last_mut() {
                var.status = VarStatus::Live;
            }
        }
    }

    fn try_move_expr(&mut self, expr: &Expr) {
        if let Expr::Identifier(ident) = expr {
            if let Some(sym) = self.symbol_table.lookup(&ident.name) {
                if !sym.sem_type.is_copy() && !matches!(sym.sem_type, SemType::Reference { .. }) {
                    if let Some(var) = self.get_var_ownership(&ident.name) {
                        if var.immutable_borrows > 0 || var.has_mutable_borrow {
                            self.diagnostics.error_code(
                                "NX-BORROW-002",
                                ident.span,
                                format!("cannot move '{}' because it is borrowed", ident.name),
                            );
                            return;
                        }
                    }
                    self.mark_moved(&ident.name, ident.span);
                }
            }
        }
    }

    pub fn analyze_program(&mut self, program: &Program) {
        // Pass 1: Collect top-level functions, interfaces, classes, structs, and extern blocks
        for stmt in &program.statements {
            match stmt {
                Stmt::FunctionDecl(ref f) => self.register_function_signature(f),
                Stmt::InterfaceDecl(ref i) => self.register_interface_declaration(i),
                Stmt::ClassDecl(ref c) => self.register_class_declaration(c),
                Stmt::StructDecl(ref s) => self.register_struct_declaration(s),
                Stmt::ExternBlock(ref e) => self.register_extern_block(e),
                _ => {}
            }
        }

        // Pass 1.5: Validate class hierarchy relationships, inheritance cycles, and conformance
        self.class_hierarchy.validate_hierarchy(self.diagnostics);

        // Pass 2: Analyze top-level statements and bodies
        for stmt in &program.statements {
            self.analyze_stmt(stmt);
        }
    }

    fn register_interface_declaration(&mut self, iface: &InterfaceDecl) {
        if self.class_hierarchy.is_type_declared(&iface.name.name) {
            self.diagnostics.error_code_with_help(
                "NX-OOP-015",
                iface.name.span,
                format!("duplicate declaration of type '{}'", iface.name.name),
                "type names must be unique across classes and interfaces",
            );
            return;
        }

        let symbol = Symbol {
            name: iface.name.name.clone(),
            sem_type: SemType::Custom(iface.name.name.clone()),
            kind: SymbolKind::Interface,
            span: iface.name.span,
            scope_level: 0,
        };
        let _ = self.symbol_table.insert(symbol);

        let mut methods = HashMap::new();
        for m in &iface.methods {
            if methods.contains_key(&m.name.name) {
                self.diagnostics.error_code_with_help(
                    "NX-OOP-016",
                    m.name.span,
                    format!(
                        "duplicate method '{}' in interface '{}'",
                        m.name.name, iface.name.name
                    ),
                    "each method name must be unique within an interface",
                );
            }

            let mut param_names = Vec::new();
            let mut param_types = Vec::new();
            let mut seen_params = HashSet::new();
            for p in &m.params {
                if !seen_params.insert(&p.name.name) {
                    self.diagnostics.error_code_with_help(
                        "NX-FUNC-006",
                        p.name.span,
                        format!(
                            "duplicate parameter name '{}' in method '{}'",
                            p.name.name, m.name.name
                        ),
                        "parameter names must be unique within a method signature",
                    );
                }
                param_names.push(p.name.name.clone());
                param_types.push(SemType::from_ast(&p.type_annotation));
            }

            let return_type = match &m.return_type {
                Some(t) => SemType::from_ast(t),
                None => SemType::Void,
            };

            methods.insert(
                m.name.name.clone(),
                InterfaceMethodSymbol {
                    name: m.name.name.clone(),
                    param_names,
                    param_types,
                    return_type,
                    span: m.span,
                },
            );
        }

        self.class_hierarchy.interfaces.insert(
            iface.name.name.clone(),
            InterfaceSymbol {
                name: iface.name.name.clone(),
                methods,
                span: iface.span,
            },
        );
    }

    fn register_class_declaration(&mut self, cls: &ClassDecl) {
        if self.class_hierarchy.is_type_declared(&cls.name.name) {
            self.diagnostics.error_code_with_help(
                "NX-OOP-015",
                cls.name.span,
                format!("duplicate declaration of type '{}'", cls.name.name),
                "type names must be unique across classes and interfaces",
            );
            return;
        }

        let symbol = Symbol {
            name: cls.name.name.clone(),
            sem_type: SemType::Custom(cls.name.name.clone()),
            kind: SymbolKind::Class,
            span: cls.name.span,
            scope_level: 0,
        };
        let _ = self.symbol_table.insert(symbol);

        let parent = cls.parent.as_ref().map(|p| p.name.clone());
        let parent_span = cls.parent.as_ref().map(|p| p.span);
        let implements = cls
            .implements
            .iter()
            .map(|i| (i.name.clone(), i.span))
            .collect();

        let mut fields = HashMap::new();
        let mut methods = HashMap::new();

        for member in &cls.members {
            match member {
                ClassMember::Field(f) => {
                    let field_name = &f.var_decl.name.name;
                    if fields.contains_key(field_name) || methods.contains_key(field_name) {
                        self.diagnostics.error_code_with_help(
                            "NX-OOP-016",
                            f.var_decl.name.span,
                            format!(
                                "duplicate member '{}' in class '{}'",
                                field_name, cls.name.name
                            ),
                            "each member name must be unique within a class",
                        );
                    }
                    let ty = if let Some(ref ann) = f.var_decl.type_annotation {
                        SemType::from_ast(ann)
                    } else if let Some(ref init) = f.var_decl.initializer {
                        self.check_expr(init, None)
                    } else {
                        self.diagnostics.error_code_with_help(
                            "NX-OOP-017",
                            f.var_decl.name.span,
                            format!(
                                "field '{}' in class '{}' requires a type annotation",
                                field_name, cls.name.name
                            ),
                            "provide a type annotation: e.g. 'var x: Int32;'",
                        );
                        SemType::Unknown
                    };
                    fields.insert(
                        field_name.clone(),
                        ClassFieldSymbol {
                            name: field_name.clone(),
                            ty,
                            access: f.access,
                            is_static: f.is_static,
                            is_const: f.var_decl.kind == VarKind::Const,
                            span: f.span,
                        },
                    );
                }
                ClassMember::Method(m) => {
                    let method_name = &m.function.name.name;
                    if fields.contains_key(method_name) || methods.contains_key(method_name) {
                        self.diagnostics.error_code_with_help(
                            "NX-OOP-016",
                            m.function.name.span,
                            format!(
                                "duplicate member '{}' in class '{}'",
                                method_name, cls.name.name
                            ),
                            "each member name must be unique within a class",
                        );
                    }

                    let mut param_names = Vec::new();
                    let mut param_types = Vec::new();
                    let mut seen_params = HashSet::new();

                    for p in &m.function.params {
                        if !seen_params.insert(&p.name.name) {
                            self.diagnostics.error_code_with_help(
                                "NX-FUNC-006",
                                p.name.span,
                                format!(
                                    "duplicate parameter name '{}' in method '{}'",
                                    p.name.name, method_name
                                ),
                                "parameter names must be unique within a method signature",
                            );
                        }
                        param_names.push(p.name.name.clone());
                        param_types.push(SemType::from_ast(&p.type_annotation));
                    }

                    let return_type = match &m.function.return_type {
                        Some(t) => SemType::from_ast(t),
                        None => SemType::Void,
                    };

                    methods.insert(
                        method_name.clone(),
                        ClassMethodSymbol {
                            name: method_name.clone(),
                            param_names,
                            param_types,
                            return_type,
                            access: m.access,
                            is_static: m.is_static,
                            is_abstract: m.is_abstract,
                            is_final: m.is_final,
                            is_async: m.is_async || m.function.is_async,
                            span: m.span,
                        },
                    );
                }
            }
        }

        self.class_hierarchy.classes.insert(
            cls.name.name.clone(),
            ClassSymbol {
                name: cls.name.name.clone(),
                is_abstract: cls.is_abstract,
                is_final: cls.is_final,
                parent,
                parent_span,
                implements,
                fields,
                methods,
                span: cls.span,
            },
        );
    }

    fn register_struct_declaration(&mut self, s: &StructDecl) {
        if self.struct_defs.contains_key(&s.name.name)
            || self.class_hierarchy.is_type_declared(&s.name.name)
        {
            self.diagnostics.error_code_with_help(
                "NX-OOP-015",
                s.name.span,
                format!("duplicate declaration of type '{}'", s.name.name),
                "type names must be unique across classes, interfaces, and structs",
            );
            return;
        }

        let mut fields = Vec::new();
        let mut seen = HashSet::new();
        for f in &s.fields {
            if !seen.insert(&f.name.name) {
                self.diagnostics.error_code_with_help(
                    "NX-SYS-003",
                    f.name.span,
                    format!(
                        "duplicate field '{}' in struct '{}'",
                        f.name.name, s.name.name
                    ),
                    "field names in a struct must be unique",
                );
            }
            let ty = SemType::from_ast(&f.field_type);
            fields.push((f.name.name.clone(), ty));
        }

        let symbol = Symbol {
            name: s.name.name.clone(),
            sem_type: SemType::Custom(s.name.name.clone()),
            kind: SymbolKind::Struct {
                fields: fields.clone(),
            },
            span: s.name.span,
            scope_level: 0,
        };
        let _ = self.symbol_table.insert(symbol);
        self.struct_defs.insert(s.name.name.clone(), fields);
    }

    fn register_extern_block(&mut self, e: &ExternBlock) {
        for f in &e.functions {
            let mut param_types = Vec::new();
            let mut seen_params = HashSet::new();

            for p in &f.params {
                if !seen_params.insert(&p.name.name) {
                    self.diagnostics.error_code_with_help(
                        "NX-FUNC-006",
                        p.name.span,
                        format!(
                            "duplicate parameter name '{}' in extern function '{}'",
                            p.name.name, f.name.name
                        ),
                        "parameter names must be unique within an extern function signature",
                    );
                }
                param_types.push(SemType::from_ast(&p.type_annotation));
            }

            let return_type = match &f.return_type {
                Some(t) => SemType::from_ast(t),
                None => SemType::Void,
            };

            let symbol = Symbol {
                name: f.name.name.clone(),
                sem_type: return_type.clone(),
                kind: SymbolKind::ExternFunction {
                    param_types,
                    return_type,
                    abi: e.abi.clone(),
                },
                span: f.name.span,
                scope_level: 0,
            };

            if self.symbol_table.insert(symbol).is_err() {
                self.diagnostics.error_code(
                    "NX-FUNC-005",
                    f.name.span,
                    format!("function '{}' is already declared", f.name.name),
                );
            }
        }
    }

    fn analyze_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::VarDecl(decl) => self.analyze_var_decl(decl),
            Stmt::Assignment(assign) => self.analyze_assignment(assign),
            Stmt::Update(up) => self.analyze_update_stmt(up),
            Stmt::FunctionDecl(f) => self.analyze_function_body(f),
            Stmt::Return(ret) => self.analyze_return_stmt(ret),
            Stmt::If(if_stmt) => self.analyze_if_stmt(if_stmt),
            Stmt::AsLoop(as_loop) => self.analyze_as_loop(as_loop),
            Stmt::ForLoop(for_loop) => self.analyze_for_loop(for_loop),
            Stmt::DoAsLoop(do_as) => self.analyze_do_as_loop(do_as),
            Stmt::ClassDecl(c) => self.analyze_class_body(c),
            Stmt::InterfaceDecl(_) => {}
            Stmt::StructDecl(_) => {}
            Stmt::ExternBlock(_) => {}
            Stmt::Unsafe(u) => {
                let prev = self.in_unsafe;
                self.in_unsafe = true;
                self.push_scope();
                for s in &u.body.statements {
                    self.analyze_stmt(s);
                }
                self.pop_scope();
                self.in_unsafe = prev;
            }
            Stmt::Match(match_stmt) => self.analyze_match_stmt(match_stmt),
            Stmt::Block(block) => {
                self.push_scope();
                for s in &block.statements {
                    self.analyze_stmt(s);
                }
                self.pop_scope();
            }
            Stmt::Expr(expr) => {
                self.check_expr(expr, None);
            }
            Stmt::Import(_) => {}
            Stmt::TestDecl(t) => {
                self.push_scope();
                for s in &t.body.statements {
                    self.analyze_stmt(s);
                }
                self.pop_scope();
            }
        }
    }

    fn analyze_match_stmt(&mut self, match_stmt: &MatchStmt) {
        let target_type = self.check_expr(&match_stmt.target, None);
        let mut seen_literals = HashSet::new();
        let mut has_wildcard = false;

        for case in &match_stmt.cases {
            if has_wildcard {
                self.diagnostics.error_code_with_help(
                    "NX-MATCH-004",
                    case.span,
                    "wildcard pattern '_' must be the last case in match statement",
                    "move the wildcard case to the end of the match block",
                );
            }

            self.check_pattern(&case.pattern, &target_type, &mut seen_literals);
            if matches!(case.pattern, Pattern::Wildcard(_)) {
                has_wildcard = true;
            }

            self.push_scope();

            let mut bindings = Vec::new();
            Self::collect_pattern_bindings(&case.pattern, &mut bindings);
            for (name, span) in bindings {
                let symbol = Symbol {
                    name: name.clone(),
                    sem_type: target_type.clone(),
                    kind: SymbolKind::Var,
                    span,
                    scope_level: self.symbol_table.current_level(),
                };
                let _ = self.symbol_table.insert(symbol);
                self.register_var_ownership(&name, false, false, span);
            }

            if let Some(ref guard) = case.guard {
                let guard_type = self.check_expr(guard, Some(&SemType::Bool));
                if guard_type != SemType::Bool && guard_type != SemType::Unknown {
                    self.diagnostics.error_code(
                        "NX-MATCH-002",
                        guard.span(),
                        format!(
                            "match guard must evaluate to a boolean expression, found {}",
                            guard_type
                        ),
                    );
                }
            }

            for stmt in &case.body.statements {
                self.analyze_stmt(stmt);
            }

            self.pop_scope();
        }
    }

    fn check_pattern(
        &mut self,
        pattern: &Pattern,
        target_type: &SemType,
        seen_literals: &mut HashSet<String>,
    ) {
        match pattern {
            Pattern::Literal(lit) => match lit {
                Literal::Integer { value, raw, span } => {
                    if !target_type.is_integer()
                        && *target_type != SemType::Any
                        && *target_type != SemType::Unknown
                    {
                        self.diagnostics.error_code(
                            "NX-MATCH-001",
                            *span,
                            format!(
                                "pattern type mismatch: expected {}, found integer",
                                target_type
                            ),
                        );
                    }
                    let key = format!("int:{}", value);
                    if !seen_literals.insert(key) {
                        self.diagnostics.error_code(
                            "NX-MATCH-003",
                            *span,
                            format!("duplicate pattern case '{}' in match statement", raw),
                        );
                    }
                }
                Literal::Float { raw, span, .. } => {
                    if !target_type.is_float()
                        && *target_type != SemType::Any
                        && *target_type != SemType::Unknown
                    {
                        self.diagnostics.error_code(
                            "NX-MATCH-001",
                            *span,
                            format!(
                                "pattern type mismatch: expected {}, found float",
                                target_type
                            ),
                        );
                    }
                    let key = format!("float:{}", raw);
                    if !seen_literals.insert(key) {
                        self.diagnostics.error_code(
                            "NX-MATCH-003",
                            *span,
                            format!("duplicate pattern case '{}' in match statement", raw),
                        );
                    }
                }
                Literal::String { value, span, .. } => {
                    if *target_type != SemType::String
                        && *target_type != SemType::Any
                        && *target_type != SemType::Unknown
                    {
                        self.diagnostics.error_code(
                            "NX-MATCH-001",
                            *span,
                            format!(
                                "pattern type mismatch: expected {}, found STRING",
                                target_type
                            ),
                        );
                    }
                    let key = format!("str:{}", value);
                    if !seen_literals.insert(key) {
                        self.diagnostics.error_code(
                            "NX-MATCH-003",
                            *span,
                            format!("duplicate pattern case '\"{}\"' in match statement", value),
                        );
                    }
                }
                Literal::Bool { value, span, .. } => {
                    if *target_type != SemType::Bool
                        && *target_type != SemType::Any
                        && *target_type != SemType::Unknown
                    {
                        self.diagnostics.error_code(
                            "NX-MATCH-001",
                            *span,
                            format!(
                                "pattern type mismatch: expected {}, found BOOL",
                                target_type
                            ),
                        );
                    }
                    let key = format!("bool:{}", value);
                    if !seen_literals.insert(key) {
                        self.diagnostics.error_code(
                            "NX-MATCH-003",
                            *span,
                            format!("duplicate pattern case '{}' in match statement", value),
                        );
                    }
                }
                Literal::Char { value, span, .. } => {
                    if *target_type != SemType::Char
                        && *target_type != SemType::Any
                        && *target_type != SemType::Unknown
                    {
                        self.diagnostics.error_code(
                            "NX-MATCH-001",
                            *span,
                            format!(
                                "pattern type mismatch: expected {}, found CHAR",
                                target_type
                            ),
                        );
                    }
                    let key = format!("char:{}", value);
                    if !seen_literals.insert(key) {
                        self.diagnostics.error_code(
                            "NX-MATCH-003",
                            *span,
                            format!("duplicate pattern case ''{}'' in match statement", value),
                        );
                    }
                }
                Literal::Null { span } => {
                    let key = "null".to_string();
                    if !seen_literals.insert(key) {
                        self.diagnostics.error_code(
                            "NX-MATCH-003",
                            *span,
                            "duplicate pattern case 'null' in match statement",
                        );
                    }
                }
            },
            Pattern::Wildcard(span) => {
                let key = "_".to_string();
                if !seen_literals.insert(key) {
                    self.diagnostics.error_code(
                        "NX-MATCH-004",
                        *span,
                        "duplicate wildcard pattern '_' in match statement",
                    );
                }
            }
            Pattern::Binding(_) => {}
            Pattern::Alternative(alts, _) => {
                for alt in alts {
                    self.check_pattern(alt, target_type, seen_literals);
                }
            }
        }
    }

    fn collect_pattern_bindings(pattern: &Pattern, bindings: &mut Vec<(String, Span)>) {
        match pattern {
            Pattern::Binding(ident) => bindings.push((ident.name.clone(), ident.span)),
            Pattern::Alternative(alts, _) => {
                if let Some(first) = alts.first() {
                    Self::collect_pattern_bindings(first, bindings);
                }
            }
            _ => {}
        }
    }

    fn analyze_class_body(&mut self, cls: &ClassDecl) {
        let prev_class = self.current_class.replace(cls.name.name.clone());

        for member in &cls.members {
            match member {
                ClassMember::Field(f) => {
                    if let Some(ref init) = f.var_decl.initializer {
                        let declared_t = if let Some(ref ann) = f.var_decl.type_annotation {
                            SemType::from_ast(ann)
                        } else {
                            self.check_expr(init, None)
                        };
                        let init_t = self.check_expr(init, Some(&declared_t));
                        if !self.is_assignable(&init_t, &declared_t) {
                            self.diagnostics.error_code_with_help(
                                "NX-TYPE-001",
                                init.span(),
                                format!(
                                    "type mismatch in field '{}': expected {}, found {}",
                                    f.var_decl.name.name, declared_t, init_t
                                ),
                                format!("field is declared with type {}", declared_t),
                            );
                        }
                    }
                }
                ClassMember::Method(m) => {
                    if m.is_abstract {
                        continue;
                    }

                    let return_type = match &m.function.return_type {
                        Some(t) => SemType::from_ast(t),
                        None => SemType::Void,
                    };

                    self.current_function = Some(FunctionContext {
                        name: format!("{}::{}", cls.name.name, m.function.name.name),
                        return_type: return_type.clone(),
                    });

                    self.push_scope();

                    // If instance method, inject `self` (and `super` if subclass)
                    if !m.is_static {
                        let self_sym = Symbol {
                            name: "self".to_string(),
                            sem_type: SemType::Custom(cls.name.name.clone()),
                            kind: SymbolKind::Param,
                            span: m.span,
                            scope_level: self.symbol_table.current_level(),
                        };
                        let _ = self.symbol_table.insert(self_sym);
                        self.register_var_ownership("self", true, false, m.span);

                        if let Some(ref parent_name) = cls.parent {
                            let super_sym = Symbol {
                                name: "super".to_string(),
                                sem_type: SemType::Custom(parent_name.name.clone()),
                                kind: SymbolKind::Param,
                                span: m.span,
                                scope_level: self.symbol_table.current_level(),
                            };
                            let _ = self.symbol_table.insert(super_sym);
                            self.register_var_ownership("super", false, false, m.span);
                        }
                    }

                    // Register parameters
                    for param in &m.function.params {
                        let p_type = SemType::from_ast(&param.type_annotation);
                        let sym = Symbol {
                            name: param.name.name.clone(),
                            sem_type: p_type,
                            kind: SymbolKind::Param,
                            span: param.name.span,
                            scope_level: self.symbol_table.current_level(),
                        };
                        let _ = self.symbol_table.insert(sym);
                        self.register_var_ownership(&param.name.name, true, false, param.name.span);
                    }

                    // Check statements
                    let prev_async = self.current_is_async;
                    self.current_is_async = m.is_async || m.function.is_async;
                    for s in &m.function.body.statements {
                        self.analyze_stmt(s);
                    }
                    self.current_is_async = prev_async;

                    // Non-void methods must definitely return
                    if return_type != SemType::Void && !block_definitely_returns(&m.function.body) {
                        self.diagnostics.error_code_with_help(
                            "NX-FUNC-005",
                            m.function.span,
                            format!(
                                "method '{}' with return type {} is missing a return statement on one or more control paths",
                                m.function.name.name, return_type
                            ),
                            "ensure all code branches explicitly return a value",
                        );
                    }

                    self.pop_scope();
                    self.current_function = None;
                }
            }
        }

        self.current_class = prev_class;
    }

    fn register_function_signature(&mut self, f: &FunctionDecl) {
        let return_type = match &f.return_type {
            Some(t) => SemType::from_ast(t),
            None => SemType::Void,
        };

        let mut param_types = Vec::new();
        let mut seen_params = std::collections::HashSet::new();

        for param in &f.params {
            if !seen_params.insert(&param.name.name) {
                self.diagnostics.error_code_with_help(
                    "NX-FUNC-006",
                    param.name.span,
                    format!(
                        "duplicate parameter name '{}' in function '{}'",
                        param.name.name, f.name.name
                    ),
                    "each parameter name must be unique within the function signature",
                );
            }
            param_types.push(SemType::from_ast(&param.type_annotation));
        }

        let symbol = Symbol {
            name: f.name.name.clone(),
            sem_type: return_type.clone(),
            kind: SymbolKind::Function {
                param_types,
                return_type,
                is_async: f.is_async,
            },
            span: f.name.span,
            scope_level: 0,
        };

        if let Err(existing) = self.symbol_table.insert(symbol) {
            self.diagnostics.error_code_with_help(
                "NX-SCOPE-001",
                f.name.span,
                format!("duplicate declaration of function '{}'", f.name.name),
                format!("previously declared here at {}", existing.span),
            );
        }
    }

    fn analyze_var_decl(&mut self, decl: &VarDecl) {
        let declared_type = if let Some(ref type_ann) = decl.type_annotation {
            let declared_type = SemType::from_ast(type_ann);
            if let Some(ref init_expr) = decl.initializer {
                let init_type = self.check_expr(init_expr, Some(&declared_type));

                if !self.is_assignable(&init_type, &declared_type) {
                    self.diagnostics.error_code_with_help(
                        "NX-TYPE-001",
                        init_expr.span(),
                        format!(
                            "incompatible types in variable initialization: expected {}, found {}",
                            declared_type, init_type
                        ),
                        format!(
                            "variable '{}' has declared type <{}>",
                            decl.name.name, declared_type
                        ),
                    );
                }
            }
            declared_type
        } else if let Some(ref init_expr) = decl.initializer {
            let init_type = self.check_expr(init_expr, None);
            if init_type == SemType::Unknown || init_type == SemType::Void {
                self.diagnostics.error_code_with_help(
                    "NX-TYPE-001",
                    decl.name.span,
                    format!("cannot infer type for variable '{}'", decl.name.name),
                    "provide an explicit type annotation: e.g. 'var x: Int32 = ...;'",
                );
                SemType::Unknown
            } else {
                init_type
            }
        } else {
            self.diagnostics.error_code_with_help(
                "NX-TYPE-001",
                decl.name.span,
                format!(
                    "variable '{}' requires a type annotation or an initializer",
                    decl.name.name
                ),
                "provide a type annotation or initializer: e.g. 'var x: Int32;' or 'var x = 10;'",
            );
            SemType::Unknown
        };

        let is_mut = decl.kind != VarKind::Const;
        let is_tem = decl.kind == VarKind::Tem;
        self.register_var_ownership(&decl.name.name, is_mut, is_tem, decl.name.span);

        if let Some(ref init_expr) = decl.initializer {
            if let Expr::Borrow {
                expr,
                is_mut: b_mut,
                ..
            } = init_expr
            {
                if let Expr::Identifier(ref target) = expr.as_ref() {
                    if let Some(stack) = self.ownership_vars.get_mut(&decl.name.name) {
                        if let Some(r_var) = stack.last_mut() {
                            r_var.borrowed_target = Some((target.name.clone(), *b_mut));
                        }
                    }
                }
            } else {
                self.try_move_expr(init_expr);
            }
        }

        let kind = match decl.kind {
            VarKind::Var => SymbolKind::Var,
            VarKind::Const => SymbolKind::Const,
            VarKind::Tem => SymbolKind::Tem,
        };

        let symbol = Symbol {
            name: decl.name.name.clone(),
            sem_type: declared_type,
            kind,
            span: decl.name.span,
            scope_level: self.symbol_table.current_level(),
        };

        if let Err(existing) = self.symbol_table.insert(symbol) {
            self.diagnostics.error_code_with_help(
                "NX-SCOPE-001",
                decl.name.span,
                format!(
                    "identifier '{}' is already declared in this scope",
                    decl.name.name
                ),
                format!("previously declared here at {}", existing.span),
            );
        }
    }

    fn analyze_assignment(&mut self, assign: &Assignment) {
        let target_type = self.check_assignment_target(&assign.target);
        let value_type = self.check_expr(&assign.value, Some(&target_type));

        if !self.is_assignable(&value_type, &target_type) {
            self.diagnostics.error_code_with_help(
                "NX-TYPE-001",
                assign.value.span(),
                format!(
                    "incompatible assignment: expected {}, found {}",
                    target_type, value_type
                ),
                format!("assignment target has type <{}>", target_type),
            );
        }

        if let Expr::Identifier(ident) = &assign.target {
            if let Some(var) = self.get_var_ownership(&ident.name) {
                if var.immutable_borrows > 0 || var.has_mutable_borrow {
                    self.diagnostics.error_code_with_help(
                        "NX-BORROW-001",
                        assign.target.span(),
                        format!("cannot mutate '{}' while it is borrowed", ident.name),
                        "cannot reassign a borrowed variable",
                    );
                }
            }
            self.reset_var_live(&ident.name);
        }

        if !matches!(assign.value, Expr::Borrow { .. }) {
            self.try_move_expr(&assign.value);
        }
    }

    fn check_assignment_target(&mut self, target: &Expr) -> SemType {
        match target {
            Expr::Identifier(ident) => match self.symbol_table.lookup(&ident.name) {
                Some(sym) => {
                    if sym.kind.is_constant() {
                        self.diagnostics.error_code_with_help(
                            "NX-CONST-001",
                            ident.span,
                            format!("cannot reassign constant '{}'", ident.name),
                            format!(
                                "'{}' was declared as a constant and cannot be reassigned",
                                ident.name
                            ),
                        );
                    }
                    sym.sem_type.clone()
                }
                None => {
                    self.diagnostics.error_code_with_help(
                        "NX-RESOLVE-001",
                        ident.span,
                        format!("undeclared variable '{}'", ident.name),
                        format!(
                            "variable '{}' must be declared with 'var', 'const', or 'tem' before assignment",
                            ident.name
                        ),
                    );
                    SemType::Unknown
                }
            },
            Expr::Index {
                target,
                index,
                span,
            } => {
                let target_t = self.check_expr(target, None);
                let idx_t = self.check_expr(index, Some(&SemType::Int32));
                match target_t {
                    SemType::List(elem) => {
                        if !idx_t.is_integer() && idx_t != SemType::Unknown {
                            self.diagnostics.error_code(
                                "NX-TYPE-004",
                                index.span(),
                                format!("list index must be an integer, found {}", idx_t),
                            );
                        }
                        *elem
                    }
                    SemType::Map(k, v) => {
                        if !self.is_assignable(&idx_t, &k) && idx_t != SemType::Unknown {
                            self.diagnostics.error_code(
                                "NX-TYPE-004",
                                index.span(),
                                format!("map key mismatch: expected {}, found {}", k, idx_t),
                            );
                        }
                        *v
                    }
                    SemType::Unknown => SemType::Unknown,
                    other => {
                        self.diagnostics.error_code(
                            "NX-TYPE-004",
                            *span,
                            format!("type {} cannot be indexed as an assignment target", other),
                        );
                        SemType::Unknown
                    }
                }
            }
            Expr::PropertyAccess {
                target,
                property,
                span,
            } => {
                // Static property assignment
                if let Expr::Identifier(target_ident) = &**target {
                    if let Some(cls) = self
                        .class_hierarchy
                        .lookup_class(&target_ident.name)
                        .cloned()
                    {
                        if let Some((decl_cls, field)) = self
                            .class_hierarchy
                            .lookup_field(&cls.name, &property.name)
                            .map(|(c, f)| (c.clone(), f.clone()))
                        {
                            if !field.is_static {
                                self.diagnostics.error_code_with_help(
                                    "NX-OOP-013",
                                    *span,
                                    format!(
                                        "cannot access instance field '{}' through class name without an instance",
                                        property.name
                                    ),
                                    "access instance fields on an object instance or mark the field as 'static'",
                                );
                            }
                            if field.is_const {
                                self.diagnostics.error_code_with_help(
                                    "NX-CONST-001",
                                    property.span,
                                    format!("cannot reassign constant field '{}'", property.name),
                                    "field was declared as constant and cannot be reassigned",
                                );
                            }
                            if !self.class_hierarchy.can_access(
                                &decl_cls.name,
                                field.access,
                                self.current_class.as_deref(),
                            ) {
                                match field.access {
                                    AccessModifier::Private => {
                                        self.diagnostics.error_code_with_help(
                                            "NX-OOP-005",
                                            *span,
                                            format!(
                                                "cannot access private field '{}' of class '{}' from unauthorized context",
                                                property.name, decl_cls.name
                                            ),
                                            "private members can only be accessed within their declaring class",
                                        );
                                    }
                                    AccessModifier::Protected => {
                                        self.diagnostics.error_code_with_help(
                                            "NX-OOP-006",
                                            *span,
                                            format!(
                                                "cannot access protected field '{}' of class '{}' from unauthorized context",
                                                property.name, decl_cls.name
                                            ),
                                            "protected members can only be accessed within the declaring class and its subclasses",
                                        );
                                    }
                                    AccessModifier::Public => {}
                                }
                            }
                            return field.ty;
                        } else {
                            self.diagnostics.error_code_with_help(
                                "NX-OOP-012",
                                property.span,
                                format!(
                                    "unknown field '{}' on class '{}'",
                                    property.name, cls.name
                                ),
                                "ensure field is declared on the class",
                            );
                            return SemType::Unknown;
                        }
                    }
                }

                // Instance property assignment
                let target_t = self.check_expr(target, None);
                if let SemType::Custom(ref class_name) = target_t {
                    if let Some(struct_fields) = self.struct_defs.get(class_name) {
                        if let Some((_, field_ty)) =
                            struct_fields.iter().find(|(n, _)| n == &property.name)
                        {
                            return field_ty.clone();
                        } else {
                            self.diagnostics.error_code(
                                "NX-SYS-005",
                                property.span,
                                format!("struct '{}' has no field '{}'", class_name, property.name),
                            );
                            return SemType::Unknown;
                        }
                    }
                    if let Some((decl_cls, field)) = self
                        .class_hierarchy
                        .lookup_field(class_name, &property.name)
                        .map(|(c, f)| (c.clone(), f.clone()))
                    {
                        if field.is_static {
                            self.diagnostics.error_code_with_help(
                                "NX-OOP-013",
                                *span,
                                format!(
                                    "static field '{}' must be accessed through class name '{}', not through an instance",
                                    property.name, decl_cls.name
                                ),
                                format!("access this field as '{}.{}'", decl_cls.name, property.name),
                            );
                        }
                        if field.is_const {
                            self.diagnostics.error_code_with_help(
                                "NX-CONST-001",
                                property.span,
                                format!("cannot reassign constant field '{}'", property.name),
                                "field was declared as constant and cannot be reassigned",
                            );
                        }
                        if !self.class_hierarchy.can_access(
                            &decl_cls.name,
                            field.access,
                            self.current_class.as_deref(),
                        ) {
                            match field.access {
                                AccessModifier::Private => {
                                    self.diagnostics.error_code_with_help(
                                        "NX-OOP-005",
                                        *span,
                                        format!(
                                            "cannot access private field '{}' of class '{}' from unauthorized context",
                                            property.name, decl_cls.name
                                        ),
                                        "private members can only be accessed within their declaring class",
                                    );
                                }
                                AccessModifier::Protected => {
                                    self.diagnostics.error_code_with_help(
                                        "NX-OOP-006",
                                        *span,
                                        format!(
                                            "cannot access protected field '{}' of class '{}' from unauthorized context",
                                            property.name, decl_cls.name
                                        ),
                                        "protected members can only be accessed within the declaring class and its subclasses",
                                    );
                                }
                                AccessModifier::Public => {}
                            }
                        }
                        field.ty
                    } else {
                        self.diagnostics.error_code_with_help(
                            "NX-OOP-012",
                            property.span,
                            format!(
                                "unknown property or field '{}' on type '{}'",
                                property.name, class_name
                            ),
                            "ensure field is declared on the type",
                        );
                        SemType::Unknown
                    }
                } else if target_t == SemType::Any {
                    SemType::Any
                } else if target_t != SemType::Unknown {
                    self.diagnostics.error_code(
                        "NX-TYPE-004",
                        *span,
                        format!("cannot assign property on non-object type {}", target_t),
                    );
                    SemType::Unknown
                } else {
                    SemType::Unknown
                }
            }
            other => {
                self.diagnostics.error_code(
                    "NX-TYPE-004",
                    other.span(),
                    "invalid assignment target; expected variable or property expression",
                );
                SemType::Unknown
            }
        }
    }

    fn analyze_update_stmt(&mut self, up: &UpdateStmt) {
        self.check_update_target(&up.target, up.op, up.span);
    }

    fn check_update_target(&mut self, target: &Expr, op: UpdateOp, span: Span) -> SemType {
        match target {
            Expr::Identifier(ident) => match self.symbol_table.lookup(&ident.name) {
                Some(sym) => {
                    if sym.kind.is_constant() {
                        self.diagnostics.error_code_with_help(
                            "NX-CONST-001",
                            ident.span,
                            format!("cannot mutate constant '{}'", ident.name),
                            format!(
                                "'{}' was declared as a constant and cannot be updated",
                                ident.name
                            ),
                        );
                    }
                    if let Some(var) = self.get_var_ownership(&ident.name) {
                        if var.immutable_borrows > 0 || var.has_mutable_borrow {
                            self.diagnostics.error_code_with_help(
                                "NX-BORROW-001",
                                ident.span,
                                format!("cannot mutate '{}' while it is borrowed", ident.name),
                                "cannot update a borrowed variable",
                            );
                        }
                    }
                    if !sym.sem_type.is_numeric() && sym.sem_type != SemType::Unknown {
                        self.diagnostics.error_code(
                            "NX-TYPE-004",
                            span,
                            format!(
                                "update operator '{}' requires numeric type, found {}",
                                op, sym.sem_type
                            ),
                        );
                    }
                    sym.sem_type.clone()
                }
                None => {
                    self.diagnostics.error_code_with_help(
                            "NX-RESOLVE-001",
                            ident.span,
                            format!("undeclared variable '{}'", ident.name),
                            format!(
                                "variable '{}' must be declared with 'var', 'const', or 'tem' before update",
                                ident.name
                            ),
                        );
                    SemType::Unknown
                }
            },
            Expr::Index {
                target: idx_target,
                index,
                ..
            } => {
                let target_t = self.check_expr(idx_target, None);
                let _idx_t = self.check_expr(index, Some(&SemType::Int32));
                match target_t {
                    SemType::List(elem) => {
                        if !elem.is_numeric() && *elem != SemType::Unknown {
                            self.diagnostics.error_code(
                                "NX-TYPE-004",
                                span,
                                format!(
                                    "update operator '{}' requires numeric element type, found {}",
                                    op, elem
                                ),
                            );
                        }
                        *elem
                    }
                    _ => {
                        self.diagnostics.error_code(
                            "NX-TYPE-004",
                            span,
                            "invalid target for update operator",
                        );
                        SemType::Unknown
                    }
                }
            }
            other => {
                self.diagnostics.error_code(
                    "NX-TYPE-004",
                    other.span(),
                    "invalid target for update operator",
                );
                SemType::Unknown
            }
        }
    }

    fn analyze_function_body(&mut self, f: &FunctionDecl) {
        let return_type = match &f.return_type {
            Some(t) => SemType::from_ast(t),
            None => SemType::Void,
        };

        self.push_scope();

        for param in &f.params {
            let param_t = SemType::from_ast(&param.type_annotation);
            let symbol = Symbol {
                name: param.name.name.clone(),
                sem_type: param_t,
                kind: SymbolKind::Param,
                span: param.name.span,
                scope_level: self.symbol_table.current_level(),
            };
            let _ = self.symbol_table.insert(symbol);
            self.register_var_ownership(&param.name.name, true, false, param.name.span);
        }

        let prev_fn = self.current_function.replace(FunctionContext {
            name: f.name.name.clone(),
            return_type: return_type.clone(),
        });

        let prev_async = self.current_is_async;
        self.current_is_async = f.is_async;
        for s in &f.body.statements {
            self.analyze_stmt(s);
        }
        self.current_is_async = prev_async;

        // Return path validation
        if return_type != SemType::Void && !block_definitely_returns(&f.body) {
            self.diagnostics.error_code_with_help(
                "NX-FUNC-005",
                f.span,
                format!(
                    "missing return statement in function '{}' with return type <{}>",
                    f.name.name, return_type
                ),
                format!(
                    "all execution paths in function '{}' must return a value of type <{}>",
                    f.name.name, return_type
                ),
            );
        }

        self.current_function = prev_fn;
        self.pop_scope();
    }

    fn analyze_return_stmt(&mut self, ret: &ReturnStmt) {
        let fn_ctx = match &self.current_function {
            Some(ctx) => ctx.clone(),
            None => {
                self.diagnostics.error_code_with_help(
                    "NX-FUNC-004",
                    ret.span,
                    "return statement outside of function",
                    "return statements can only be used inside a function body",
                );
                return;
            }
        };

        if let Some(ref val_expr) = ret.value {
            if let Expr::Identifier(ident) = val_expr {
                if let Some(var) = self.get_var_ownership(&ident.name) {
                    if var.is_tem {
                        self.diagnostics.error_code_with_help(
                            "NX-OWN-003",
                            val_expr.span(),
                            format!(
                                "temporary variable '{}' cannot escape enclosing scope",
                                ident.name
                            ),
                            "variables declared with 'tem' are dropped at block exit",
                        );
                    }
                }
            }
            if fn_ctx.return_type == SemType::Void {
                self.diagnostics.error_code_with_help(
                    "NX-FUNC-003",
                    val_expr.span(),
                    format!(
                        "function '{}' has return type VOID and cannot return a value",
                        fn_ctx.name
                    ),
                    "remove the return value or specify a non-VOID return type in the function header",
                );
                self.check_expr(val_expr, None);
            } else {
                let actual_t = self.check_expr(val_expr, Some(&fn_ctx.return_type));
                if !self.is_assignable(&actual_t, &fn_ctx.return_type) {
                    self.diagnostics.error_code_with_help(
                        "NX-FUNC-003",
                        val_expr.span(),
                        format!(
                            "return type mismatch in function '{}': expected {}, found {}",
                            fn_ctx.name, fn_ctx.return_type, actual_t
                        ),
                        format!(
                            "function header declares return type <{}>",
                            fn_ctx.return_type
                        ),
                    );
                }
            }
            self.try_move_expr(val_expr);
        } else if fn_ctx.return_type != SemType::Void {
            self.diagnostics.error_code_with_help(
                "NX-FUNC-003",
                ret.span,
                format!(
                    "function '{}' expects a return value of type <{}>",
                    fn_ctx.name, fn_ctx.return_type
                ),
                format!("add a return expression matching <{}>", fn_ctx.return_type),
            );
        }
    }

    fn analyze_if_stmt(&mut self, if_stmt: &IfStmt) {
        let cond_t = self.check_expr(&if_stmt.condition, Some(&SemType::Bool));
        if cond_t != SemType::Bool && cond_t != SemType::Unknown {
            self.diagnostics.error_code_with_help(
                "NX-TYPE-002",
                if_stmt.condition.span(),
                format!("if condition must be of type BOOL, found {}", cond_t),
                "ensure condition evaluates to a boolean expression (e.g. using ==, !=, <, >, &, |)",
            );
        }

        self.push_scope();
        for s in &if_stmt.then_branch.statements {
            self.analyze_stmt(s);
        }
        self.pop_scope();

        for ow in &if_stmt.otherwise_branches {
            let ow_cond_t = self.check_expr(&ow.condition, Some(&SemType::Bool));
            if ow_cond_t != SemType::Bool && ow_cond_t != SemType::Unknown {
                self.diagnostics.error_code_with_help(
                    "NX-TYPE-002",
                    ow.condition.span(),
                    format!(
                        "otherwise condition must be of type BOOL, found {}",
                        ow_cond_t
                    ),
                    "ensure condition evaluates to a boolean expression",
                );
            }

            self.push_scope();
            for s in &ow.body.statements {
                self.analyze_stmt(s);
            }
            self.pop_scope();
        }

        if let Some(ref else_block) = if_stmt.else_branch {
            self.push_scope();
            for s in &else_block.statements {
                self.analyze_stmt(s);
            }
            self.pop_scope();
        }
    }

    fn analyze_as_loop(&mut self, as_loop: &AsLoopStmt) {
        let cond_t = self.check_expr(&as_loop.condition, Some(&SemType::Bool));
        if cond_t != SemType::Bool && cond_t != SemType::Unknown {
            self.diagnostics.error_code_with_help(
                "NX-TYPE-002",
                as_loop.condition.span(),
                format!("As loop condition must be of type BOOL, found {}", cond_t),
                "ensure condition evaluates to a boolean expression",
            );
        }

        self.push_scope();
        for s in &as_loop.body.statements {
            self.analyze_stmt(s);
        }
        self.pop_scope();
    }

    fn analyze_for_loop(&mut self, for_loop: &ForLoopStmt) {
        // Loop scope encapsulates init, cond, update, and body!
        self.push_scope();

        if let Some(ref init_stmt) = for_loop.init {
            self.analyze_stmt(init_stmt);
        }

        if let Some(ref cond_expr) = for_loop.condition {
            let cond_t = self.check_expr(cond_expr, Some(&SemType::Bool));
            if cond_t != SemType::Bool && cond_t != SemType::Unknown {
                self.diagnostics.error_code_with_help(
                    "NX-TYPE-002",
                    cond_expr.span(),
                    format!("for-loop condition must be of type BOOL, found {}", cond_t),
                    "ensure loop condition evaluates to a boolean expression",
                );
            }
        }

        if let Some(ref update_stmt) = for_loop.update {
            self.analyze_stmt(update_stmt);
        }

        for s in &for_loop.body.statements {
            self.analyze_stmt(s);
        }

        self.pop_scope();
    }

    fn analyze_do_as_loop(&mut self, do_as: &DoAsLoopStmt) {
        self.push_scope();
        for s in &do_as.body.statements {
            self.analyze_stmt(s);
        }
        self.pop_scope();

        let cond_t = self.check_expr(&do_as.condition, Some(&SemType::Bool));
        if cond_t != SemType::Bool && cond_t != SemType::Unknown {
            self.diagnostics.error_code_with_help(
                "NX-TYPE-002",
                do_as.condition.span(),
                format!(
                    "Do ... As loop condition must be of type BOOL, found {}",
                    cond_t
                ),
                "ensure condition evaluates to a boolean expression",
            );
        }
    }

    // --- Expression Type Checking ---

    pub fn check_expr(&mut self, expr: &Expr, expected: Option<&SemType>) -> SemType {
        match expr {
            Expr::Literal(lit) => self.check_literal(lit, expected),
            Expr::Identifier(ident) => match self.symbol_table.lookup(&ident.name) {
                Some(sym) => {
                    if let Some(status) = self.get_var_status(&ident.name) {
                        match status {
                            VarStatus::Moved(moved_at) => {
                                self.diagnostics.error_code_with_help(
                                    "NX-OWN-001",
                                    ident.span,
                                    format!("use of moved value '{}'", ident.name),
                                    format!("value was moved at span {}", moved_at),
                                );
                            }
                            VarStatus::Freed(freed_at) => {
                                self.diagnostics.error_code_with_help(
                                    "NX-OWN-002",
                                    ident.span,
                                    format!("use of deallocated value '{}'", ident.name),
                                    format!("value was deallocated at span {}", freed_at),
                                );
                            }
                            VarStatus::Live => {}
                        }
                    }
                    sym.sem_type.clone()
                }
                None => {
                    self.diagnostics.error_code_with_help(
                        "NX-RESOLVE-001",
                        ident.span,
                        format!("undeclared identifier '{}'", ident.name),
                        format!("'{}' has not been declared in this scope", ident.name),
                    );
                    SemType::Unknown
                }
            },
            Expr::Unary { op, operand, span } => {
                let op_type = self.check_expr(operand, expected);
                match op {
                    UnaryOp::Not => {
                        if op_type != SemType::Bool && op_type != SemType::Unknown {
                            self.diagnostics.error_code_with_help(
                                "NX-TYPE-002",
                                *span,
                                format!(
                                    "logical operator 'not' expects BOOL operand, found {}",
                                    op_type
                                ),
                                "apply 'not' only to boolean values",
                            );
                        }
                        SemType::Bool
                    }
                    UnaryOp::BitNot => {
                        if !op_type.is_integer() && op_type != SemType::Unknown {
                            self.diagnostics.error_code(
                                "NX-SYS-003",
                                *span,
                                format!(
                                    "bitwise operator '~' requires integer operand, found {}",
                                    op_type
                                ),
                            );
                        }
                        op_type
                    }
                    UnaryOp::Neg => {
                        if !op_type.is_numeric() && op_type != SemType::Unknown {
                            self.diagnostics.error_code(
                                "NX-TYPE-004",
                                *span,
                                format!("cannot negate non-numeric type {}", op_type),
                            );
                        } else if op_type.is_unsigned_integer() {
                            self.diagnostics.error_code_with_help(
                                "NX-TYPE-004",
                                *span,
                                format!(
                                    "cannot apply unary '-' to unsigned integer type {}",
                                    op_type
                                ),
                                "use signed integer types (e.g. INT32, INT64) for negative values",
                            );
                        }
                        op_type
                    }
                    UnaryOp::Pos => {
                        if !op_type.is_numeric() && op_type != SemType::Unknown {
                            self.diagnostics.error_code(
                                "NX-TYPE-004",
                                *span,
                                format!("cannot apply unary '+' to non-numeric type {}", op_type),
                            );
                        }
                        op_type
                    }
                }
            }
            Expr::Binary {
                op,
                left,
                right,
                span,
            } => self.check_binary_expr(*op, left, right, *span, expected),
            Expr::Update {
                op, target, span, ..
            } => self.check_update_target(target, *op, *span),
            Expr::Call { callee, args, span } => self.check_call_expr(callee, args, *span),
            Expr::Index {
                target,
                index,
                span,
            } => {
                let target_t = self.check_expr(target, None);
                let idx_t = self.check_expr(index, Some(&SemType::Int32));
                match target_t {
                    SemType::List(elem) => {
                        if !idx_t.is_integer() && idx_t != SemType::Unknown {
                            self.diagnostics.error_code(
                                "NX-TYPE-004",
                                index.span(),
                                format!("list index must be an integer, found {}", idx_t),
                            );
                        }
                        // Statically provable out-of-bounds on list literal
                        if let Expr::ListLiteral { elements, .. } = &**target {
                            if let Expr::Literal(Literal::Integer { value, .. }) = &**index {
                                if *value < 0 || (*value as usize) >= elements.len() {
                                    self.diagnostics.error_code(
                                        "NX-SAFETY-002",
                                        *span,
                                        format!(
                                            "index {} is out of bounds for list literal of length {}",
                                            value,
                                            elements.len()
                                        ),
                                    );
                                }
                            }
                        }
                        *elem
                    }
                    SemType::String => {
                        if !idx_t.is_integer() && idx_t != SemType::Unknown {
                            self.diagnostics.error_code(
                                "NX-TYPE-004",
                                index.span(),
                                format!("string index must be an integer, found {}", idx_t),
                            );
                        }
                        SemType::Char
                    }
                    SemType::Map(k, v) => {
                        if !idx_t.is_assignable_to(&k) && idx_t != SemType::Unknown {
                            self.diagnostics.error_code(
                                "NX-TYPE-004",
                                index.span(),
                                format!("map key mismatch: expected {}, found {}", k, idx_t),
                            );
                        }
                        *v
                    }
                    SemType::Unknown => SemType::Unknown,
                    other => {
                        self.diagnostics.error_code(
                            "NX-TYPE-004",
                            *span,
                            format!("type {} cannot be indexed", other),
                        );
                        SemType::Unknown
                    }
                }
            }
            Expr::PropertyAccess {
                target,
                property,
                span,
            } => {
                // Static property access: ClassName.prop
                if let Expr::Identifier(target_ident) = &**target {
                    if let Some(cls) = self
                        .class_hierarchy
                        .lookup_class(&target_ident.name)
                        .cloned()
                    {
                        if let Some((decl_cls, field)) = self
                            .class_hierarchy
                            .lookup_field(&cls.name, &property.name)
                            .map(|(c, f)| (c.clone(), f.clone()))
                        {
                            if !field.is_static {
                                self.diagnostics.error_code_with_help(
                                    "NX-OOP-013",
                                    *span,
                                    format!(
                                        "cannot access instance field '{}' through class name without an instance",
                                        property.name
                                    ),
                                    "access instance fields on an object instance or mark the field as 'static'",
                                );
                            }

                            if !self.class_hierarchy.can_access(
                                &decl_cls.name,
                                field.access,
                                self.current_class.as_deref(),
                            ) {
                                match field.access {
                                    AccessModifier::Private => {
                                        self.diagnostics.error_code_with_help(
                                            "NX-OOP-005",
                                            *span,
                                            format!(
                                                "cannot access private field '{}' of class '{}' from unauthorized context",
                                                property.name, decl_cls.name
                                            ),
                                            "private members can only be accessed within their declaring class",
                                        );
                                    }
                                    AccessModifier::Protected => {
                                        self.diagnostics.error_code_with_help(
                                            "NX-OOP-006",
                                            *span,
                                            format!(
                                                "cannot access protected field '{}' of class '{}' from unauthorized context",
                                                property.name, decl_cls.name
                                            ),
                                            "protected members can only be accessed within the declaring class and its subclasses",
                                        );
                                    }
                                    AccessModifier::Public => {}
                                }
                            }
                            return field.ty;
                        } else if let Some((_, method)) = self
                            .class_hierarchy
                            .lookup_method(&cls.name, &property.name)
                        {
                            return method.return_type.clone();
                        } else {
                            self.diagnostics.error_code_with_help(
                                "NX-OOP-012",
                                property.span,
                                format!(
                                    "unknown member '{}' on class '{}'",
                                    property.name, cls.name
                                ),
                                "ensure member name is spelled correctly and declared on the class",
                            );
                            return SemType::Unknown;
                        }
                    }
                }

                // Instance property access
                let target_t = self.check_expr(target, None);
                if let SemType::Custom(ref class_name) = target_t {
                    if let Some(struct_fields) = self.struct_defs.get(class_name) {
                        if let Some((_, field_ty)) =
                            struct_fields.iter().find(|(n, _)| n == &property.name)
                        {
                            return field_ty.clone();
                        } else {
                            self.diagnostics.error_code(
                                "NX-SYS-005",
                                property.span,
                                format!("struct '{}' has no field '{}'", class_name, property.name),
                            );
                            return SemType::Unknown;
                        }
                    }
                    if let Some((decl_cls, field)) = self
                        .class_hierarchy
                        .lookup_field(class_name, &property.name)
                        .map(|(c, f)| (c.clone(), f.clone()))
                    {
                        if field.is_static {
                            self.diagnostics.error_code_with_help(
                                "NX-OOP-013",
                                *span,
                                format!(
                                    "static field '{}' must be accessed through class name '{}', not through an instance",
                                    property.name, decl_cls.name
                                ),
                                format!("access this field as '{}.{}'", decl_cls.name, property.name),
                            );
                        }

                        if !self.class_hierarchy.can_access(
                            &decl_cls.name,
                            field.access,
                            self.current_class.as_deref(),
                        ) {
                            match field.access {
                                AccessModifier::Private => {
                                    self.diagnostics.error_code_with_help(
                                        "NX-OOP-005",
                                        *span,
                                        format!(
                                            "cannot access private field '{}' of class '{}' from unauthorized context",
                                            property.name, decl_cls.name
                                        ),
                                        "private members can only be accessed within their declaring class",
                                    );
                                }
                                AccessModifier::Protected => {
                                    self.diagnostics.error_code_with_help(
                                        "NX-OOP-006",
                                        *span,
                                        format!(
                                            "cannot access protected field '{}' of class '{}' from unauthorized context",
                                            property.name, decl_cls.name
                                        ),
                                        "protected members can only be accessed within the declaring class and its subclasses",
                                    );
                                }
                                AccessModifier::Public => {}
                            }
                        }
                        field.ty
                    } else if let Some((_, method)) = self
                        .class_hierarchy
                        .lookup_method(class_name, &property.name)
                    {
                        method.return_type.clone()
                    } else {
                        self.diagnostics.error_code_with_help(
                            "NX-OOP-012",
                            property.span,
                            format!(
                                "unknown property or field '{}' on type '{}'",
                                property.name, class_name
                            ),
                            "ensure field is declared on the type",
                        );
                        SemType::Unknown
                    }
                } else if target_t == SemType::Any {
                    SemType::Any
                } else if target_t != SemType::Unknown {
                    self.diagnostics.error_code(
                        "NX-TYPE-004",
                        *span,
                        format!(
                            "cannot access property '.{}' on type {}",
                            property.name, target_t
                        ),
                    );
                    SemType::Unknown
                } else {
                    SemType::Unknown
                }
            }
            Expr::ListLiteral { elements, span } => {
                let expected_elem = match expected {
                    Some(SemType::List(elem)) => Some(&**elem),
                    Some(SemType::Set(elem)) => Some(&**elem),
                    _ => None,
                };

                if elements.is_empty() {
                    if let Some(elem_t) = expected_elem {
                        return match expected {
                            Some(SemType::Set(_)) => SemType::Set(Box::new(elem_t.clone())),
                            _ => SemType::List(Box::new(elem_t.clone())),
                        };
                    } else {
                        self.diagnostics.error_code_with_help(
                            "NX-TYPE-006",
                            *span,
                            "cannot infer element type for empty collection literal",
                            "provide an explicit type annotation, e.g. 'var x:<LIST<INT32>> = [];'",
                        );
                        return SemType::List(Box::new(SemType::Unknown));
                    }
                }

                let mut inferred_elem_type = expected_elem.cloned();

                for (idx, elem) in elements.iter().enumerate() {
                    let elem_t = self.check_expr(elem, inferred_elem_type.as_ref());
                    if let Some(ref exp) = inferred_elem_type {
                        if !elem_t.is_assignable_to(exp) {
                            self.diagnostics.error_code_with_help(
                                "NX-TYPE-005",
                                elem.span(),
                                format!(
                                    "element type mismatch in collection literal at index {}: expected {}, found {}",
                                    idx, exp, elem_t
                                ),
                                format!("all elements must be compatible with <{}>", exp),
                            );
                        }
                    } else {
                        inferred_elem_type = Some(elem_t);
                    }
                }

                let final_elem = inferred_elem_type.unwrap_or(SemType::Unknown);
                match expected {
                    Some(SemType::Set(_)) => SemType::Set(Box::new(final_elem)),
                    _ => SemType::List(Box::new(final_elem)),
                }
            }
            Expr::MapLiteral { entries, span } => {
                let (expected_k, expected_v) = match expected {
                    Some(SemType::Map(k, v)) => (Some(&**k), Some(&**v)),
                    _ => (None, None),
                };

                if entries.is_empty() {
                    if let (Some(k), Some(v)) = (expected_k, expected_v) {
                        return SemType::Map(Box::new(k.clone()), Box::new(v.clone()));
                    } else {
                        self.diagnostics.error_code_with_help(
                            "NX-TYPE-006",
                            *span,
                            "cannot infer key and value types for empty map literal",
                            "provide an explicit type annotation, e.g. 'var m:<MAP<STRING, INT32>> = {};'",
                        );
                        return SemType::Map(
                            Box::new(SemType::Unknown),
                            Box::new(SemType::Unknown),
                        );
                    }
                }

                let mut inferred_k = expected_k.cloned();
                let mut inferred_v = expected_v.cloned();

                for (key, val) in entries {
                    let kt = self.check_expr(key, inferred_k.as_ref());
                    if let Some(ref exp_k) = inferred_k {
                        if !kt.is_assignable_to(exp_k) {
                            self.diagnostics.error_code(
                                "NX-TYPE-005",
                                key.span(),
                                format!("map key type mismatch: expected {}, found {}", exp_k, kt),
                            );
                        }
                    } else {
                        inferred_k = Some(kt);
                    }

                    let vt = self.check_expr(val, inferred_v.as_ref());
                    if let Some(ref exp_v) = inferred_v {
                        if !vt.is_assignable_to(exp_v) {
                            self.diagnostics.error_code(
                                "NX-TYPE-005",
                                val.span(),
                                format!(
                                    "map value type mismatch: expected {}, found {}",
                                    exp_v, vt
                                ),
                            );
                        }
                    } else {
                        inferred_v = Some(vt);
                    }
                }

                let final_k = inferred_k.unwrap_or(SemType::Unknown);
                let final_v = inferred_v.unwrap_or(SemType::Unknown);
                SemType::Map(Box::new(final_k), Box::new(final_v))
            }
            Expr::Group { expr, .. } => self.check_expr(expr, expected),
            Expr::Borrow { expr, is_mut, span } => {
                let inner_type = self.check_expr(expr, None);
                if let Expr::Identifier(ident) = expr.as_ref() {
                    if let Some(var) = self.get_var_ownership(&ident.name) {
                        match var.status {
                            VarStatus::Moved(_) | VarStatus::Freed(_) => {}
                            VarStatus::Live => {
                                if *is_mut {
                                    if !var.is_mutable {
                                        self.diagnostics.error_code_with_help(
                                            "NX-BORROW-003",
                                            *span,
                                            format!(
                                                "cannot borrow immutable variable '{}' as mutable",
                                                ident.name
                                            ),
                                            "declare the variable with 'var' to allow mutable borrows",
                                        );
                                    } else if var.immutable_borrows > 0 {
                                        self.diagnostics.error_code_with_help(
                                            "NX-BORROW-003",
                                            *span,
                                            format!(
                                                "cannot borrow '{}' as mutable because it is already borrowed as immutable",
                                                ident.name
                                            ),
                                            "mutable borrows require exclusive access",
                                        );
                                    } else if var.has_mutable_borrow {
                                        self.diagnostics.error_code_with_help(
                                            "NX-BORROW-003",
                                            *span,
                                            format!(
                                                "cannot borrow '{}' as mutable more than once at a time",
                                                ident.name
                                            ),
                                            "only one mutable borrow may exist at a time",
                                        );
                                    } else {
                                        self.mark_borrowed(&ident.name, true);
                                    }
                                } else if var.has_mutable_borrow {
                                    self.diagnostics.error_code_with_help(
                                        "NX-BORROW-003",
                                        *span,
                                        format!(
                                            "cannot borrow '{}' as immutable because it is already borrowed as mutable",
                                            ident.name
                                        ),
                                        "cannot borrow while a mutable borrow is active",
                                    );
                                } else {
                                    self.mark_borrowed(&ident.name, false);
                                }
                            }
                        }
                    }
                }
                SemType::Reference {
                    inner: Box::new(inner_type),
                    is_mut: *is_mut,
                }
            }
            Expr::Alloc { expr, span: _ } => self.check_expr(expr, expected),
            Expr::Free { target, span } => {
                let _ = self.check_expr(target, None);
                if let Expr::Identifier(ident) = target.as_ref() {
                    if let Some(var) = self.get_var_ownership(&ident.name) {
                        match var.status {
                            VarStatus::Moved(_) => {
                                self.diagnostics.error_code(
                                    "NX-OWN-001",
                                    *span,
                                    format!("cannot deallocate moved value '{}'", ident.name),
                                );
                            }
                            VarStatus::Freed(_) => {
                                self.diagnostics.error_code_with_help(
                                    "NX-OWN-002",
                                    *span,
                                    format!(
                                        "cannot deallocate already freed value '{}'",
                                        ident.name
                                    ),
                                    "value has already been released",
                                );
                            }
                            VarStatus::Live => {
                                if var.immutable_borrows > 0 || var.has_mutable_borrow {
                                    self.diagnostics.error_code(
                                        "NX-BORROW-002",
                                        *span,
                                        format!(
                                            "cannot free '{}' while it is borrowed",
                                            ident.name
                                        ),
                                    );
                                } else {
                                    self.mark_freed(&ident.name, *span);
                                }
                            }
                        }
                    }
                }
                SemType::Void
            }
            Expr::StructLiteral {
                name,
                fields,
                span: _,
            } => {
                if let Some(struct_fields) = self.struct_defs.get(&name.name).cloned() {
                    for (fname, fval) in fields {
                        let expected_t = struct_fields
                            .iter()
                            .find(|(n, _)| n == &fname.name)
                            .map(|(_, t)| t);
                        let val_t = self.check_expr(fval, expected_t);
                        if let Some(exp) = expected_t {
                            if !val_t.is_assignable_to(exp) && val_t != SemType::Unknown {
                                self.diagnostics.error_code(
                                    "NX-TYPE-001",
                                    fval.span(),
                                    format!("type mismatch for struct field '{}': expected {}, found {}", fname.name, exp, val_t),
                                );
                            }
                        } else {
                            self.diagnostics.error_code(
                                "NX-SYS-005",
                                fname.span,
                                format!("struct '{}' has no field '{}'", name.name, fname.name),
                            );
                        }
                    }
                    SemType::Custom(name.name.clone())
                } else {
                    self.diagnostics.error_code(
                        "NX-SYS-006",
                        name.span,
                        format!("'{}' is not a declared struct", name.name),
                    );
                    SemType::Unknown
                }
            }
            Expr::Await { expr, span } => {
                if !self.current_is_async {
                    self.diagnostics.error_code_with_help(
                        "NX-ASYNC-001",
                        *span,
                        "await is only permitted inside async functions",
                        "mark the enclosing function with 'async fn'",
                    );
                }

                if self.has_active_mutable_borrows() {
                    self.diagnostics.error_code_with_help(
                        "NX-OWN-004",
                        *span,
                        "cannot yield or await across an active mutable reference",
                        "release the mutable borrow before calling await",
                    );
                }

                let expr_t = self.check_expr(expr, None);
                match expr_t {
                    SemType::Future(inner) => *inner,
                    SemType::Any => SemType::Any,
                    SemType::Unknown => SemType::Unknown,
                    other => {
                        self.diagnostics.error_code_with_help(
                            "NX-TYPE-001",
                            *span,
                            format!("cannot await non-future type '{}'", other),
                            "await expression requires a FUTURE<T> or async function return",
                        );
                        SemType::Unknown
                    }
                }
            }
        }
    }

    fn check_literal(&mut self, lit: &Literal, expected: Option<&SemType>) -> SemType {
        match lit {
            Literal::Integer { value, raw, span } => {
                if let Some(exp) = expected {
                    if exp.is_integer() {
                        if !exp.fits_integer_literal(*value) {
                            self.diagnostics.error_code_with_help(
                                "NX-TYPE-003",
                                *span,
                                format!(
                                    "integer literal '{}' is out of range for {} ({})",
                                    raw,
                                    exp,
                                    exp.integer_range_str()
                                ),
                                "ensure literal value fits within the target type boundaries",
                            );
                        }
                        return exp.clone();
                    }
                    if exp.is_float() || *exp == SemType::Any {
                        return exp.clone();
                    }
                    if let SemType::Union(members) = exp {
                        if let Some(m) = members
                            .iter()
                            .find(|m| m.is_integer() && m.fits_integer_literal(*value))
                        {
                            return m.clone();
                        }
                    }
                }
                // Default inference
                if *value >= i32::MIN as i128 && *value <= i32::MAX as i128 {
                    SemType::Int32
                } else {
                    SemType::Int64
                }
            }
            Literal::Float { raw: _, span: _ } => {
                if let Some(exp) = expected {
                    if exp.is_float() || *exp == SemType::Any {
                        return exp.clone();
                    }
                }
                SemType::Fl64
            }
            Literal::String { .. } => SemType::String,
            Literal::Char { .. } => SemType::Char,
            Literal::Bool { .. } => SemType::Bool,
            Literal::Null { .. } => SemType::Any,
        }
    }

    fn check_binary_expr(
        &mut self,
        op: BinaryOp,
        left: &Expr,
        right: &Expr,
        span: Span,
        expected: Option<&SemType>,
    ) -> SemType {
        match op {
            BinaryOp::And | BinaryOp::Or => {
                if expected == Some(&SemType::Bool) {
                    let lt = self.check_expr(left, Some(&SemType::Bool));
                    let rt = self.check_expr(right, Some(&SemType::Bool));
                    if lt != SemType::Bool && lt != SemType::Unknown {
                        self.diagnostics.error_code_with_help(
                            "NX-TYPE-002",
                            left.span(),
                            format!(
                                "logical operator '{}' expects BOOL operand, found {}",
                                op, lt
                            ),
                            "operands of '&' and '|' in boolean context must be boolean expressions",
                        );
                    }

                    if rt != SemType::Bool && rt != SemType::Unknown {
                        self.diagnostics.error_code_with_help(
                            "NX-TYPE-002",
                            right.span(),
                            format!(
                                "logical operator '{}' expects BOOL operand, found {}",
                                op, rt
                            ),
                            "operands of '&' and '|' in boolean context must be boolean expressions",
                        );
                    }

                    return SemType::Bool;
                }

                let lt = self.check_expr(left, expected);
                let rt = self.check_expr(right, Some(&lt));
                if lt.is_integer() && rt.is_integer() {
                    return lt;
                }

                if lt != SemType::Bool && lt != SemType::Unknown {
                    self.diagnostics.error_code_with_help(
                        "NX-TYPE-002",
                        left.span(),
                        format!(
                            "logical operator '{}' expects BOOL operand, found {}",
                            op, lt
                        ),
                        "operands of '&' and '|' must be boolean or integer expressions",
                    );
                }

                if rt != SemType::Bool && rt != SemType::Unknown {
                    self.diagnostics.error_code_with_help(
                        "NX-TYPE-002",
                        right.span(),
                        format!(
                            "logical operator '{}' expects BOOL operand, found {}",
                            op, rt
                        ),
                        "operands of '&' and '|' must be boolean or integer expressions",
                    );
                }

                SemType::Bool
            }
            BinaryOp::BitXor | BinaryOp::Shl | BinaryOp::Shr => {
                let lt = self.check_expr(left, expected);
                let rt = self.check_expr(right, Some(&lt));
                if !lt.is_integer() && lt != SemType::Unknown {
                    self.diagnostics.error_code(
                        "NX-SYS-003",
                        left.span(),
                        format!(
                            "bitwise operator '{}' requires integer left operand, found {}",
                            op, lt
                        ),
                    );
                }
                if !rt.is_integer() && rt != SemType::Unknown {
                    self.diagnostics.error_code(
                        "NX-SYS-003",
                        right.span(),
                        format!(
                            "bitwise operator '{}' requires integer right operand, found {}",
                            op, rt
                        ),
                    );
                }
                lt
            }
            BinaryOp::Eq | BinaryOp::NotEq => {
                let lt = self.check_expr(left, None);
                let rt = self.check_expr(right, Some(&lt));

                if !lt.is_assignable_to(&rt)
                    && !rt.is_assignable_to(&lt)
                    && lt != SemType::Unknown
                    && rt != SemType::Unknown
                {
                    self.diagnostics.error_code(
                        "NX-TYPE-004",
                        span,
                        format!(
                            "cannot compare incompatible types {} and {} with '{}'",
                            lt, rt, op
                        ),
                    );
                }
                SemType::Bool
            }
            BinaryOp::Lt | BinaryOp::LtEq | BinaryOp::Gt | BinaryOp::GtEq => {
                let lt = self.check_expr(left, None);
                let rt = self.check_expr(right, Some(&lt));

                if (!lt.is_numeric() || !rt.is_numeric())
                    && lt != rt
                    && lt != SemType::Unknown
                    && rt != SemType::Unknown
                {
                    self.diagnostics.error_code(
                        "NX-TYPE-004",
                        span,
                        format!(
                            "relational operator '{}' cannot compare {} and {}",
                            op, lt, rt
                        ),
                    );
                }
                SemType::Bool
            }
            BinaryOp::Range => {
                let lt = self.check_expr(left, Some(&SemType::Int32));
                let rt = self.check_expr(right, Some(&SemType::Int32));
                if (!lt.is_integer() || !rt.is_integer())
                    && lt != SemType::Unknown
                    && rt != SemType::Unknown
                {
                    self.diagnostics.error_code(
                        "NX-TYPE-004",
                        span,
                        format!(
                            "range operator '..' requires integer bounds, found {} and {}",
                            lt, rt
                        ),
                    );
                }
                SemType::List(Box::new(SemType::Int32))
            }
            BinaryOp::Add => {
                let lt = self.check_expr(left, None);
                let rt = self.check_expr(right, Some(&lt));

                // String concatenation
                if lt == SemType::String && rt == SemType::String {
                    return SemType::String;
                }

                if lt.is_numeric() && rt.is_numeric() {
                    if lt == rt {
                        return lt;
                    }
                    if lt.is_assignable_to(&rt) {
                        return rt;
                    }
                    if rt.is_assignable_to(&lt) {
                        return lt;
                    }
                }

                if lt != SemType::Unknown && rt != SemType::Unknown {
                    self.diagnostics.error_code(
                        "NX-TYPE-004",
                        span,
                        format!("cannot apply operator '+' to {} and {}", lt, rt),
                    );
                }
                SemType::Unknown
            }
            BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => {
                // Division by zero check
                if matches!(op, BinaryOp::Div | BinaryOp::Mod) {
                    if let Expr::Literal(Literal::Integer {
                        value: 0,
                        span: zero_span,
                        ..
                    }) = right
                    {
                        self.diagnostics.error_code_with_help(
                            "NX-SAFETY-001",
                            *zero_span,
                            format!(
                                "attempt to divide by zero detected statically using '{}'",
                                op
                            ),
                            "divisor must not be constant zero",
                        );
                    }
                }

                let lt = self.check_expr(left, None);
                let rt = self.check_expr(right, Some(&lt));

                if lt.is_numeric() && rt.is_numeric() {
                    if lt == rt {
                        return lt;
                    }
                    if lt.is_assignable_to(&rt) {
                        return rt;
                    }
                    if rt.is_assignable_to(&lt) {
                        return lt;
                    }
                }

                if lt != SemType::Unknown && rt != SemType::Unknown {
                    self.diagnostics.error_code(
                        "NX-TYPE-004",
                        span,
                        format!("cannot apply operator '{}' to {} and {}", op, lt, rt),
                    );
                }
                SemType::Unknown
            }
        }
    }

    fn check_call_expr(&mut self, callee: &Expr, args: &[Expr], span: Span) -> SemType {
        // Builtin `vout`
        if let Expr::Identifier(ident) = callee {
            if ident.name == "vout" {
                for arg in args {
                    self.check_expr(arg, None);
                }
                return SemType::Void;
            }

            // 1. Check if identifier is a Class (Constructor invocation)
            if let Some(cls) = self.class_hierarchy.lookup_class(&ident.name).cloned() {
                if cls.is_abstract {
                    self.diagnostics.error_code_with_help(
                        "NX-OOP-003",
                        ident.span,
                        format!("cannot instantiate abstract class '{}'", ident.name),
                        "abstract classes cannot be directly instantiated; instantiate a concrete subclass instead",
                    );
                    for arg in args {
                        self.check_expr(arg, None);
                    }
                    return SemType::Custom(ident.name.clone());
                }

                if let Some(constructor) = cls.constructor().cloned() {
                    let expected_params = constructor.param_types;
                    if args.len() != expected_params.len() {
                        self.diagnostics.error_code_with_help(
                            "NX-OOP-014",
                            span,
                            format!(
                                "constructor for class '{}' expects {} arguments, but {} were provided",
                                ident.name,
                                expected_params.len(),
                                args.len()
                            ),
                            format!(
                                "provide exactly {} arguments matching constructor 'init'",
                                expected_params.len()
                            ),
                        );
                    }

                    for (i, arg) in args.iter().enumerate() {
                        let expected_p = expected_params.get(i);
                        let actual_arg_t = self.check_expr(arg, expected_p);
                        if let Some(expected_p) = expected_p {
                            if !self.is_assignable(&actual_arg_t, expected_p) {
                                self.diagnostics.error_code_with_help(
                                    "NX-OOP-014",
                                    arg.span(),
                                    format!(
                                        "argument {} type mismatch in constructor for '{}': expected {}, found {}",
                                        i + 1,
                                        ident.name,
                                        expected_p,
                                        actual_arg_t
                                    ),
                                    format!("parameter {} is declared as <{}>", i + 1, expected_p),
                                );
                            }
                        }
                    }
                } else {
                    if !args.is_empty() {
                        self.diagnostics.error_code_with_help(
                            "NX-OOP-014",
                            span,
                            format!(
                                "class '{}' has no explicit constructor and expects 0 arguments, but {} were provided",
                                ident.name,
                                args.len()
                            ),
                            "declare an 'init' method to accept constructor arguments, or pass 0 arguments",
                        );
                    }
                    for arg in args {
                        self.check_expr(arg, None);
                    }
                }

                return SemType::Custom(ident.name.clone());
            }

            // 2. Check if identifier is an Interface (Cannot be instantiated)
            if self.class_hierarchy.lookup_interface(&ident.name).is_some() {
                self.diagnostics.error_code_with_help(
                    "NX-OOP-004",
                    ident.span,
                    format!("cannot instantiate interface '{}'", ident.name),
                    "interfaces cannot be instantiated directly",
                );
                for arg in args {
                    self.check_expr(arg, None);
                }
                return SemType::Custom(ident.name.clone());
            }
        }

        // 3. Method invocation: target.property(args...)
        if let Expr::PropertyAccess {
            target,
            property,
            span: prop_span,
        } = callee
        {
            // Case 3a: Static method call ClassName.method(...)
            if let Expr::Identifier(target_ident) = &**target {
                if let Some(cls) = self
                    .class_hierarchy
                    .lookup_class(&target_ident.name)
                    .cloned()
                {
                    if let Some((decl_cls, method)) = self
                        .class_hierarchy
                        .lookup_method(&cls.name, &property.name)
                        .map(|(c, m)| (c.clone(), m.clone()))
                    {
                        if !method.is_static {
                            self.diagnostics.error_code_with_help(
                                "NX-OOP-013",
                                *prop_span,
                                format!(
                                    "cannot call instance method '{}' through class name without an instance",
                                    property.name
                                ),
                                "call instance methods on an object instance or mark the method as 'static'",
                            );
                        }

                        if !self.class_hierarchy.can_access(
                            &decl_cls.name,
                            method.access,
                            self.current_class.as_deref(),
                        ) {
                            match method.access {
                                AccessModifier::Private => {
                                    self.diagnostics.error_code_with_help(
                                        "NX-OOP-005",
                                        *prop_span,
                                        format!(
                                            "cannot call private method '{}' of class '{}' from unauthorized context",
                                            property.name, decl_cls.name
                                        ),
                                        "private members can only be accessed within their declaring class",
                                    );
                                }
                                AccessModifier::Protected => {
                                    self.diagnostics.error_code_with_help(
                                        "NX-OOP-006",
                                        *prop_span,
                                        format!(
                                            "cannot call protected method '{}' of class '{}' from unauthorized context",
                                            property.name, decl_cls.name
                                        ),
                                        "protected members can only be accessed within the declaring class and its subclasses",
                                    );
                                }
                                AccessModifier::Public => {}
                            }
                        }

                        let expected_params = method.param_types;
                        if args.len() != expected_params.len() {
                            self.diagnostics.error_code_with_help(
                                "NX-FUNC-001",
                                span,
                                format!(
                                    "static method '{}' expects {} arguments, but {} were provided",
                                    property.name,
                                    expected_params.len(),
                                    args.len()
                                ),
                                format!("provide exactly {} arguments", expected_params.len()),
                            );
                        }

                        for (i, arg) in args.iter().enumerate() {
                            let expected_p = expected_params.get(i);
                            let actual_arg_t = self.check_expr(arg, expected_p);
                            if let Some(expected_p) = expected_p {
                                if !self.is_assignable(&actual_arg_t, expected_p) {
                                    self.diagnostics.error_code_with_help(
                                        "NX-FUNC-002",
                                        arg.span(),
                                        format!(
                                            "argument {} type mismatch in call to '{}': expected {}, found {}",
                                            i + 1,
                                            property.name,
                                            expected_p,
                                            actual_arg_t
                                        ),
                                        format!("parameter {} is declared as <{}>", i + 1, expected_p),
                                    );
                                }
                            }
                        }

                        return if method.is_async {
                            SemType::Future(Box::new(method.return_type))
                        } else {
                            method.return_type
                        };
                    } else {
                        self.diagnostics.error_code_with_help(
                            "NX-OOP-012",
                            property.span,
                            format!("unknown method '{}' on class '{}'", property.name, cls.name),
                            "ensure method name is spelled correctly and declared on the class",
                        );
                        for arg in args {
                            self.check_expr(arg, None);
                        }
                        return SemType::Unknown;
                    }
                }
            }

            // Case 3b: Instance method call obj.method(...)
            let target_t = self.check_expr(target, None);
            if let SemType::Custom(ref class_name) = target_t {
                if let Some((decl_cls, method)) = self
                    .class_hierarchy
                    .lookup_method(class_name, &property.name)
                    .map(|(c, m)| (c.clone(), m.clone()))
                {
                    if method.is_static {
                        self.diagnostics.error_code_with_help(
                            "NX-OOP-013",
                            *prop_span,
                            format!(
                                "static method '{}' must be invoked through class name '{}', not through an instance",
                                property.name, decl_cls.name
                            ),
                            format!("call this method as '{}.{}()'", decl_cls.name, property.name),
                        );
                    }

                    if !self.class_hierarchy.can_access(
                        &decl_cls.name,
                        method.access,
                        self.current_class.as_deref(),
                    ) {
                        match method.access {
                            AccessModifier::Private => {
                                self.diagnostics.error_code_with_help(
                                    "NX-OOP-005",
                                    *prop_span,
                                    format!(
                                        "cannot call private method '{}' of class '{}' from unauthorized context",
                                        property.name, decl_cls.name
                                    ),
                                    "private members can only be accessed within their declaring class",
                                );
                            }
                            AccessModifier::Protected => {
                                self.diagnostics.error_code_with_help(
                                    "NX-OOP-006",
                                    *prop_span,
                                    format!(
                                        "cannot call protected method '{}' of class '{}' from unauthorized context",
                                        property.name, decl_cls.name
                                    ),
                                    "protected members can only be accessed within the declaring class and its subclasses",
                                );
                            }
                            AccessModifier::Public => {}
                        }
                    }

                    let expected_params = method.param_types;
                    if args.len() != expected_params.len() {
                        self.diagnostics.error_code_with_help(
                            "NX-FUNC-001",
                            span,
                            format!(
                                "method '{}' expects {} arguments, but {} were provided",
                                property.name,
                                expected_params.len(),
                                args.len()
                            ),
                            format!("provide exactly {} arguments", expected_params.len()),
                        );
                    }

                    for (i, arg) in args.iter().enumerate() {
                        let expected_p = expected_params.get(i);
                        let actual_arg_t = self.check_expr(arg, expected_p);
                        if let Some(expected_p) = expected_p {
                            if !self.is_assignable(&actual_arg_t, expected_p) {
                                self.diagnostics.error_code_with_help(
                                    "NX-FUNC-002",
                                    arg.span(),
                                    format!(
                                        "argument {} type mismatch in call to '{}': expected {}, found {}",
                                        i + 1,
                                        property.name,
                                        expected_p,
                                        actual_arg_t
                                    ),
                                    format!("parameter {} is declared as <{}>", i + 1, expected_p),
                                );
                            }
                        }
                    }

                    return if method.is_async {
                        SemType::Future(Box::new(method.return_type))
                    } else {
                        method.return_type
                    };
                } else if let Some(iface) =
                    self.class_hierarchy.lookup_interface(class_name).cloned()
                {
                    if let Some(iface_method) = iface.methods.get(&property.name).cloned() {
                        let expected_params = iface_method.param_types;
                        if args.len() != expected_params.len() {
                            self.diagnostics.error_code_with_help(
                                "NX-FUNC-001",
                                span,
                                format!(
                                    "interface method '{}' expects {} arguments, but {} were provided",
                                    property.name,
                                    expected_params.len(),
                                    args.len()
                                ),
                                format!("provide exactly {} arguments", expected_params.len()),
                            );
                        }
                        for (i, arg) in args.iter().enumerate() {
                            let expected_p = expected_params.get(i);
                            let actual_arg_t = self.check_expr(arg, expected_p);
                            if let Some(expected_p) = expected_p {
                                if !self.is_assignable(&actual_arg_t, expected_p) {
                                    self.diagnostics.error_code_with_help(
                                        "NX-FUNC-002",
                                        arg.span(),
                                        format!(
                                            "argument {} type mismatch in call to '{}': expected {}, found {}",
                                            i + 1,
                                            property.name,
                                            expected_p,
                                            actual_arg_t
                                        ),
                                        format!("parameter {} is declared as <{}>", i + 1, expected_p),
                                    );
                                }
                            }
                        }
                        return iface_method.return_type;
                    } else {
                        self.diagnostics.error_code_with_help(
                            "NX-OOP-012",
                            property.span,
                            format!(
                                "unknown method '{}' on interface '{}'",
                                property.name, class_name
                            ),
                            "ensure method is declared in the interface",
                        );
                        for arg in args {
                            self.check_expr(arg, None);
                        }
                        return SemType::Unknown;
                    }
                } else {
                    self.diagnostics.error_code_with_help(
                        "NX-OOP-012",
                        property.span,
                        format!(
                            "unknown method '{}' on type '{}'",
                            property.name, class_name
                        ),
                        "ensure method name is spelled correctly and declared on the type",
                    );
                    for arg in args {
                        self.check_expr(arg, None);
                    }
                    return SemType::Unknown;
                }
            } else if target_t == SemType::Any {
                for arg in args {
                    self.check_expr(arg, None);
                }
                return SemType::Any;
            } else if target_t != SemType::Unknown {
                self.diagnostics.error_code(
                    "NX-TYPE-004",
                    *prop_span,
                    format!(
                        "cannot invoke method '{}' on non-object type {}",
                        property.name, target_t
                    ),
                );
                for arg in args {
                    self.check_expr(arg, None);
                }
                return SemType::Unknown;
            }
        }

        // 4. Regular function call
        let callee_name = match callee {
            Expr::Identifier(ident) => Some(ident.name.clone()),
            _ => None,
        };

        if let Some(ref name) = callee_name {
            let symbol_opt = self.symbol_table.lookup(name).cloned();
            match symbol_opt {
                Some(Symbol {
                    kind:
                        SymbolKind::Function {
                            param_types,
                            return_type,
                            is_async,
                        },
                    ..
                }) => {
                    let expected_params = param_types;
                    let ret_t = return_type;

                    if args.len() != expected_params.len() {
                        self.diagnostics.error_code_with_help(
                            "NX-FUNC-001",
                            span,
                            format!(
                                "function '{}' expects {} arguments, but {} were provided",
                                name,
                                expected_params.len(),
                                args.len()
                            ),
                            format!(
                                "provide exactly {} arguments matching the function signature",
                                expected_params.len()
                            ),
                        );
                    }

                    for (i, arg) in args.iter().enumerate() {
                        let expected_p = expected_params.get(i);
                        let actual_arg_t = self.check_expr(arg, expected_p);
                        if let Some(expected_p) = expected_p {
                            if !self.is_assignable(&actual_arg_t, expected_p) {
                                self.diagnostics.error_code_with_help(
                                    "NX-FUNC-002",
                                    arg.span(),
                                    format!(
                                        "argument {} type mismatch in call to '{}': expected {}, found {}",
                                        i + 1,
                                        name,
                                        expected_p,
                                        actual_arg_t
                                    ),
                                    format!("parameter {} is declared as <{}>", i + 1, expected_p),
                                );
                            }
                        }
                    }

                    return if is_async {
                        SemType::Future(Box::new(ret_t))
                    } else {
                        ret_t
                    };
                }
                Some(Symbol {
                    kind:
                        SymbolKind::ExternFunction {
                            param_types,
                            return_type,
                            abi: _,
                        },
                    ..
                }) => {
                    if !self.in_unsafe {
                        self.diagnostics.error_code_with_help(
                            "NX-SYS-001",
                            span,
                            format!(
                                "call to extern function '{}' requires an unsafe block",
                                name
                            ),
                            "wrap calls to external C ABI functions in an 'unsafe { ... }' block",
                        );
                    }
                    let expected_params = param_types;
                    let ret_t = return_type;
                    for (i, arg) in args.iter().enumerate() {
                        let expected_p = expected_params.get(i);
                        let actual_arg_t = self.check_expr(arg, expected_p);
                        if let Some(expected_p) = expected_p {
                            if !self.is_assignable(&actual_arg_t, expected_p) {
                                self.diagnostics.error_code(
                                    "NX-FUNC-002",
                                    arg.span(),
                                    format!("argument {} type mismatch in call to '{}': expected {}, found {}", i + 1, name, expected_p, actual_arg_t),
                                );
                            }
                        }
                    }
                    return ret_t;
                }
                Some(Symbol {
                    kind: SymbolKind::Builtin,
                    sem_type,
                    ..
                }) => {
                    for arg in args {
                        self.check_expr(arg, None);
                    }
                    return sem_type;
                }
                Some(other) => {
                    self.diagnostics.error_code(
                        "NX-TYPE-004",
                        callee.span(),
                        format!(
                            "'{}' is a {} and cannot be called as a function",
                            name,
                            other.kind.as_str()
                        ),
                    );
                    return SemType::Unknown;
                }
                None => {
                    self.diagnostics.error_code_with_help(
                        "NX-RESOLVE-001",
                        callee.span(),
                        format!("call to undeclared function '{}'", name),
                        format!("function '{}' has not been declared", name),
                    );
                    for arg in args {
                        self.check_expr(arg, None);
                    }
                    return SemType::Unknown;
                }
            }
        }

        let callee_t = self.check_expr(callee, None);
        for arg in args {
            self.check_expr(arg, None);
        }
        if callee_t != SemType::Unknown {
            self.diagnostics.error_code(
                "NX-TYPE-004",
                callee.span(),
                format!("expression of type {} is not callable", callee_t),
            );
        }
        SemType::Unknown
    }
}

fn stmt_definitely_returns(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Return(_) => true,
        Stmt::Block(block) => block_definitely_returns(block),
        Stmt::Unsafe(u) => block_definitely_returns(&u.body),
        Stmt::If(if_stmt) => {
            if let Some(ref else_block) = if_stmt.else_branch {
                if !block_definitely_returns(&if_stmt.then_branch) {
                    return false;
                }
                for ow in &if_stmt.otherwise_branches {
                    if !block_definitely_returns(&ow.body) {
                        return false;
                    }
                }
                block_definitely_returns(else_block)
            } else {
                false
            }
        }
        Stmt::Match(match_stmt) => {
            let has_wildcard = match_stmt
                .cases
                .iter()
                .any(|c| matches!(c.pattern, Pattern::Wildcard(_)) && c.guard.is_none());
            if has_wildcard && !match_stmt.cases.is_empty() {
                match_stmt
                    .cases
                    .iter()
                    .all(|c| c.body.statements.iter().any(stmt_definitely_returns))
            } else {
                false
            }
        }
        _ => false,
    }
}

fn block_definitely_returns(block: &Block) -> bool {
    for stmt in &block.statements {
        if stmt_definitely_returns(stmt) {
            return true;
        }
    }
    false
}
