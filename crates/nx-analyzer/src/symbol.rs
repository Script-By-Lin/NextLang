use crate::types::SemType;
use nx_diagnostics::Span;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolKind {
    Var,
    Const,
    Tem,
    Param,
    Function {
        param_types: Vec<SemType>,
        return_type: SemType,
        is_async: bool,
    },
    ExternFunction {
        param_types: Vec<SemType>,
        return_type: SemType,
        abi: String,
    },
    Builtin,
    Class,
    Interface,
    Struct {
        fields: Vec<(String, SemType)>,
    },
}

impl SymbolKind {
    pub fn is_constant(&self) -> bool {
        matches!(
            self,
            SymbolKind::Const
                | SymbolKind::Function { .. }
                | SymbolKind::ExternFunction { .. }
                | SymbolKind::Builtin
                | SymbolKind::Class
                | SymbolKind::Interface
                | SymbolKind::Struct { .. }
        )
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SymbolKind::Var => "variable",
            SymbolKind::Const => "constant",
            SymbolKind::Tem => "temporary variable",
            SymbolKind::Param => "parameter",
            SymbolKind::Function { .. } => "function",
            SymbolKind::ExternFunction { .. } => "extern function",
            SymbolKind::Builtin => "builtin function",
            SymbolKind::Class => "class",
            SymbolKind::Interface => "interface",
            SymbolKind::Struct { .. } => "struct",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub sem_type: SemType,
    pub kind: SymbolKind,
    pub span: Span,
    pub scope_level: usize,
}

#[derive(Debug, Clone)]
pub struct Scope {
    pub level: usize,
    pub symbols: HashMap<String, Symbol>,
}

impl Scope {
    pub fn new(level: usize) -> Self {
        Self {
            level,
            symbols: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SymbolTable {
    scopes: Vec<Scope>,
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

impl SymbolTable {
    pub fn new() -> Self {
        let mut root = Scope::new(0);
        // Builtin `vout` function accepting any number of arguments
        root.symbols.insert(
            "vout".to_string(),
            Symbol {
                name: "vout".to_string(),
                sem_type: SemType::Void,
                kind: SymbolKind::Builtin,
                span: Span::empty(0),
                scope_level: 0,
            },
        );
        root.symbols.insert(
            "len".to_string(),
            Symbol {
                name: "len".to_string(),
                sem_type: SemType::Int64,
                kind: SymbolKind::Builtin,
                span: Span::empty(0),
                scope_level: 0,
            },
        );
        root.symbols.insert(
            "input".to_string(),
            Symbol {
                name: "input".to_string(),
                sem_type: SemType::String,
                kind: SymbolKind::Builtin,
                span: Span::empty(0),
                scope_level: 0,
            },
        );
        root.symbols.insert(
            "to_int".to_string(),
            Symbol {
                name: "to_int".to_string(),
                sem_type: SemType::Int64,
                kind: SymbolKind::Builtin,
                span: Span::empty(0),
                scope_level: 0,
            },
        );
        root.symbols.insert(
            "to_float".to_string(),
            Symbol {
                name: "to_float".to_string(),
                sem_type: SemType::Fl64,
                kind: SymbolKind::Builtin,
                span: Span::empty(0),
                scope_level: 0,
            },
        );

        // System, File, and Buffer builtins for systems programming
        let sys_builtins: Vec<(&str, SemType)> = vec![
            ("file_open", SemType::Int64),
            ("file_read_all", SemType::String),
            ("file_read_str", SemType::String),
            ("file_write", SemType::Bool),
            ("file_close", SemType::Bool),
            ("file_exists", SemType::Bool),
            ("file_remove", SemType::Bool),
            ("env_get", SemType::String),
            ("env_set", SemType::Bool),
            ("get_cwd", SemType::String),
            ("get_pid", SemType::Int32),
            ("buffer_new", SemType::Ptr),
            ("buffer_free", SemType::Void),
            ("buffer_size", SemType::Uint64),
            ("buffer_get_u8", SemType::Uint8),
            ("buffer_set_u8", SemType::Void),
            ("buffer_get_u32_le", SemType::Uint32),
            ("buffer_set_u32_le", SemType::Void),
            ("buffer_get_i32_le", SemType::Int32),
            ("buffer_set_i32_le", SemType::Void),
            ("ptr_offset", SemType::Ptr),
            ("ptr_read_u8", SemType::Uint8),
            ("ptr_write_u8", SemType::Void),
            // Phase 9 Standard Library builtins
            ("math_abs", SemType::Fl64),
            ("int_abs", SemType::Int64),
            ("math_sqrt", SemType::Fl64),
            ("math_pow", SemType::Fl64),
            ("math_floor", SemType::Fl64),
            ("math_ceil", SemType::Fl64),
            ("math_min", SemType::Fl64),
            ("math_max", SemType::Fl64),
            ("math_sin", SemType::Fl64),
            ("math_cos", SemType::Fl64),
            ("math_random", SemType::Fl64),
            ("str_len", SemType::Int64),
            ("str_upper", SemType::String),
            ("str_lower", SemType::String),
            ("str_trim", SemType::String),
            ("str_sub", SemType::String),
            ("str_contains", SemType::Bool),
            ("str_starts_with", SemType::Bool),
            ("str_ends_with", SemType::Bool),
            ("time_now_ms", SemType::Int64),
            ("time_sleep_ms", SemType::Void),
            ("hex_encode", SemType::String),
            ("hex_decode", SemType::String),
            ("base64_encode", SemType::String),
            ("base64_decode", SemType::String),
            ("json_parse", SemType::String),
            ("json_stringify", SemType::String),
            ("assert", SemType::Void),
            ("assert_eq", SemType::Void),
            // Phase 10: Asynchronous Runtime Builtins
            ("async_sleep", SemType::Future(Box::new(SemType::Void))),
            ("sleep", SemType::Future(Box::new(SemType::Void))),
            ("tcp_listen", SemType::Future(Box::new(SemType::Int64))),
            ("tcp_accept", SemType::Future(Box::new(SemType::Int64))),
            ("tcp_connect", SemType::Future(Box::new(SemType::Int64))),
            ("tcp_read", SemType::Future(Box::new(SemType::String))),
            ("tcp_write", SemType::Future(Box::new(SemType::Int64))),
            ("tcp_close", SemType::Void),
            ("task_spawn", SemType::Any),
            ("task_join", SemType::Future(Box::new(SemType::Any))),
            ("task_cancel", SemType::Bool),
            ("timeout", SemType::Future(Box::new(SemType::Any))),
            // Phase 11: HTTP & NextAPI runtime builtins
            ("http_parse", SemType::String),
            ("url_decode", SemType::String),
            ("url_encode", SemType::String),
            ("http_status_message", SemType::String),
            ("int_to_str", SemType::String),
        ];

        for (bname, bty) in sys_builtins {
            root.symbols.insert(
                bname.to_string(),
                Symbol {
                    name: bname.to_string(),
                    sem_type: bty,
                    kind: SymbolKind::Builtin,
                    span: Span::empty(0),
                    scope_level: 0,
                },
            );
        }

        Self { scopes: vec![root] }
    }

    pub fn push_scope(&mut self) {
        let next_level = self.scopes.len();
        self.scopes.push(Scope::new(next_level));
    }

    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    pub fn current_level(&self) -> usize {
        self.scopes.len().saturating_sub(1)
    }

    pub fn insert(&mut self, symbol: Symbol) -> Result<(), Box<Symbol>> {
        let current = self.scopes.last_mut().expect("at least root scope exists");
        if let Some(existing) = current.symbols.get(&symbol.name) {
            Err(Box::new(existing.clone()))
        } else {
            current.symbols.insert(symbol.name.clone(), symbol);
            Ok(())
        }
    }

    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        for scope in self.scopes.iter().rev() {
            if let Some(sym) = scope.symbols.get(name) {
                return Some(sym);
            }
        }
        None
    }

    pub fn lookup_current(&self, name: &str) -> Option<&Symbol> {
        self.scopes.last().and_then(|s| s.symbols.get(name))
    }
}
