use std::collections::HashMap;

/// Fundamental types in the NextLang Intermediate Representation (NIR).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NirType {
    I8,
    I16,
    I32,
    I64,
    I128,
    U8,
    U16,
    U32,
    U64,
    U128,
    F32,
    F64,
    Bool,
    Char,
    Void,
    Ptr,
    Struct {
        name: String,
        fields: Vec<(String, NirType)>,
    },
}

impl NirType {
    pub fn is_float(&self) -> bool {
        matches!(self, NirType::F32 | NirType::F64)
    }

    pub fn is_signed_int(&self) -> bool {
        matches!(
            self,
            NirType::I8 | NirType::I16 | NirType::I32 | NirType::I64 | NirType::I128
        )
    }

    pub fn is_unsigned_int(&self) -> bool {
        matches!(
            self,
            NirType::U8 | NirType::U16 | NirType::U32 | NirType::U64 | NirType::U128
        )
    }

    pub fn is_int(&self) -> bool {
        self.is_signed_int() || self.is_unsigned_int()
    }

    pub fn is_bool(&self) -> bool {
        matches!(self, NirType::Bool)
    }

    pub fn size_bytes(&self) -> usize {
        match self {
            NirType::I8 | NirType::U8 | NirType::Bool => 1,
            NirType::I16 | NirType::U16 => 2,
            NirType::I32 | NirType::U32 | NirType::F32 | NirType::Char => 4,
            NirType::I64 | NirType::U64 | NirType::F64 | NirType::Ptr => 8,
            NirType::I128 | NirType::U128 => 16,
            NirType::Void => 0,
            NirType::Struct { fields, .. } => fields.iter().map(|(_, ty)| ty.size_bytes()).sum(),
        }
    }
}

/// Constant values known at compile time.
#[derive(Debug, Clone, PartialEq)]
pub enum NirConst {
    Int(i64, NirType),
    Float(f64, NirType),
    Bool(bool),
    Char(char),
    String(String),
    Null,
}

impl NirConst {
    pub fn ty(&self) -> NirType {
        match self {
            NirConst::Int(_, ty) => ty.clone(),
            NirConst::Float(_, ty) => ty.clone(),
            NirConst::Bool(_) => NirType::Bool,
            NirConst::Char(_) => NirType::Char,
            NirConst::String(_) => NirType::Ptr,
            NirConst::Null => NirType::Ptr,
        }
    }
}

/// An operand representing either a literal constant or a named variable/register.
#[derive(Debug, Clone, PartialEq)]
pub enum NirOperand {
    Const(NirConst),
    Var(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NirBinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NirCmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NirUnaryOp {
    Neg,
    Not,
    BitNot,
}

/// Individual instructions executed within a basic block.
#[derive(Debug, Clone, PartialEq)]
pub enum NirInst {
    /// Allocate space on the stack for a local variable.
    Alloca { dest: String, ty: NirType },
    /// Load value from a pointer to a destination register.
    Load {
        dest: String,
        src_ptr: String,
        ty: NirType,
    },
    /// Store an operand to a pointer.
    Store { dest_ptr: String, src: NirOperand },
    /// Compute pointer to a struct field with given byte offset.
    GetFieldPtr {
        dest: String,
        base_ptr: String,
        byte_offset: usize,
        ty: NirType,
    },
    /// Binary arithmetic or bitwise operation.
    BinOp {
        dest: String,
        op: NirBinaryOp,
        lhs: NirOperand,
        rhs: NirOperand,
        ty: NirType,
    },
    /// Comparison operation returning boolean.
    Cmp {
        dest: String,
        op: NirCmpOp,
        lhs: NirOperand,
        rhs: NirOperand,
    },
    /// Unary operation.
    UnaryOp {
        dest: String,
        op: NirUnaryOp,
        operand: NirOperand,
        ty: NirType,
    },
    /// Function or runtime helper call.
    Call {
        dest: Option<String>,
        callee: String,
        args: Vec<NirOperand>,
        ret_ty: NirType,
    },
    /// Explicit heap memory allocation.
    Alloc {
        dest: String,
        size_bytes: NirOperand,
    },
    /// Explicit heap memory deallocation.
    Free { ptr: NirOperand },
    /// NextLang standard output builtin.
    Vout { args: Vec<(NirOperand, NirType)> },
    /// Interactive console input.
    Input {
        dest: String,
        prompt: Option<String>,
    },
    /// Type conversion to integer.
    ToInt {
        dest: String,
        src: NirOperand,
        src_ty: NirType,
    },
    /// Type conversion to float.
    ToFloat {
        dest: String,
        src: NirOperand,
        src_ty: NirType,
    },
    /// Collection / string length query.
    Len {
        dest: String,
        src: NirOperand,
        src_ty: NirType,
    },
}

/// Terminator instruction ending every basic block.
#[derive(Debug, Clone, PartialEq)]
pub enum NirTerminator {
    Return(Option<NirOperand>),
    Branch(String),
    CondBranch {
        cond: NirOperand,
        then_block: String,
        else_block: String,
    },
    Panic(String),
}

/// A basic block of straight-line instructions ending with a terminator.
#[derive(Debug, Clone, PartialEq)]
pub struct NirBlock {
    pub label: String,
    pub instructions: Vec<NirInst>,
    pub terminator: NirTerminator,
}

impl NirBlock {
    pub fn new(label: impl Into<String>, terminator: NirTerminator) -> Self {
        Self {
            label: label.into(),
            instructions: Vec::new(),
            terminator,
        }
    }
}

/// A function in NIR consisting of a sequence of basic blocks.
#[derive(Debug, Clone, PartialEq)]
pub struct NirFunction {
    pub name: String,
    pub params: Vec<(String, NirType)>,
    pub ret_ty: NirType,
    pub blocks: Vec<NirBlock>,
    pub is_entry: bool,
}

impl NirFunction {
    pub fn new(name: impl Into<String>, params: Vec<(String, NirType)>, ret_ty: NirType) -> Self {
        Self {
            name: name.into(),
            params,
            ret_ty,
            blocks: Vec::new(),
            is_entry: false,
        }
    }
}

/// OOP Class layout tracking fields, offsets, and methods.
#[derive(Debug, Clone, PartialEq)]
pub struct NirClassLayout {
    pub name: String,
    pub parent: Option<String>,
    pub fields: Vec<(String, NirType)>,
    pub field_offsets: HashMap<String, usize>,
    pub total_size_bytes: usize,
    pub methods: Vec<String>,
}

impl NirClassLayout {
    pub fn new(name: impl Into<String>, parent: Option<String>) -> Self {
        Self {
            name: name.into(),
            parent,
            fields: Vec::new(),
            field_offsets: HashMap::new(),
            total_size_bytes: 0,
            methods: Vec::new(),
        }
    }

    pub fn add_field(&mut self, name: impl Into<String>, ty: NirType) {
        let field_name = name.into();
        let size = ty.size_bytes();
        let offset = self.total_size_bytes;
        self.field_offsets.insert(field_name.clone(), offset);
        self.fields.push((field_name, ty));
        self.total_size_bytes += size;
    }
}

/// A compilation unit containing functions, class layouts, and string constants.
#[derive(Debug, Clone, PartialEq)]
pub struct NirModule {
    pub name: String,
    pub string_literals: Vec<(String, String)>,
    pub classes: HashMap<String, NirClassLayout>,
    pub functions: Vec<NirFunction>,
}

impl NirModule {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            string_literals: Vec::new(),
            classes: HashMap::new(),
            functions: Vec::new(),
        }
    }

    /// Add a string literal and return its unique label name.
    pub fn add_string_literal(&mut self, content: &str) -> String {
        for (label, s) in &self.string_literals {
            if s == content {
                return label.clone();
            }
        }
        let label = format!("__str_{}", self.string_literals.len());
        self.string_literals
            .push((label.clone(), content.to_string()));
        label
    }
}
