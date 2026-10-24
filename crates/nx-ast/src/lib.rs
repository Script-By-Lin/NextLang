use nx_diagnostics::Span;
use std::fmt;

/// An identifier with its source span.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

impl Ident {
    pub fn new(name: impl Into<String>, span: Span) -> Self {
        Self {
            name: name.into(),
            span,
        }
    }
}

impl fmt::Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// Variable declaration kind: `var`, `const`, or `tem`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VarKind {
    Var,
    Const,
    Tem,
}

impl VarKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            VarKind::Var => "var",
            VarKind::Const => "const",
            VarKind::Tem => "tem",
        }
    }
}

impl fmt::Display for VarKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Type representation in the AST.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    /// Primitive or named type, e.g. `INT32`, `STRING`, `User`.
    Named { name: String, span: Span },
    /// Generic collection type, e.g. `LIST<INT32>`, `MAP<STRING, INT32>`.
    Generic {
        name: String,
        type_args: Vec<Type>,
        span: Span,
    },
    /// Union type, e.g. `INT32 | FL64`.
    Union { types: Vec<Type>, span: Span },
    /// Reference type: `&T` or `&mut T`.
    Reference {
        inner: Box<Type>,
        is_mut: bool,
        span: Span,
    },
}

impl Type {
    pub fn span(&self) -> Span {
        match self {
            Type::Named { span, .. } => *span,
            Type::Generic { span, .. } => *span,
            Type::Union { span, .. } => *span,
            Type::Reference { span, .. } => *span,
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Named { name, .. } => write!(f, "{}", name),
            Type::Reference { inner, is_mut, .. } => {
                if *is_mut {
                    write!(f, "&mut {}", inner)
                } else {
                    write!(f, "&{}", inner)
                }
            }
            Type::Generic {
                name, type_args, ..
            } => {
                write!(f, "{}<", name)?;
                for (i, arg) in type_args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ">")
            }
            Type::Union { types, .. } => {
                for (i, t) in types.iter().enumerate() {
                    if i > 0 {
                        write!(f, " | ")?;
                    }
                    write!(f, "{}", t)?;
                }
                Ok(())
            }
        }
    }
}

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    Add,    // +
    Sub,    // -
    Mul,    // *
    Div,    // /
    Mod,    // %
    Eq,     // ==
    NotEq,  // !=
    Lt,     // <
    LtEq,   // <=
    Gt,     // >
    GtEq,   // >=
    And,    // &
    Or,     // |
    BitXor, // ^
    Shl,    // <<
    Shr,    // >>
    Range,  // ..
}

impl BinaryOp {
    pub fn as_str(&self) -> &'static str {
        match self {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Mod => "%",
            BinaryOp::Eq => "==",
            BinaryOp::NotEq => "!=",
            BinaryOp::Lt => "<",
            BinaryOp::LtEq => "<=",
            BinaryOp::Gt => ">",
            BinaryOp::GtEq => ">=",
            BinaryOp::And => "&",
            BinaryOp::Or => "|",
            BinaryOp::BitXor => "^",
            BinaryOp::Shl => "<<",
            BinaryOp::Shr => ">>",
            BinaryOp::Range => "..",
        }
    }
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    Not,    // not
    Neg,    // -
    Pos,    // +
    BitNot, // ~
}

impl UnaryOp {
    pub fn as_str(&self) -> &'static str {
        match self {
            UnaryOp::Not => "not",
            UnaryOp::Neg => "-",
            UnaryOp::Pos => "+",
            UnaryOp::BitNot => "~",
        }
    }
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Update operators: `++` and `--`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UpdateOp {
    Increment, // ++
    Decrement, // --
}

impl UpdateOp {
    pub fn as_str(&self) -> &'static str {
        match self {
            UpdateOp::Increment => "++",
            UpdateOp::Decrement => "--",
        }
    }
}

impl fmt::Display for UpdateOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Assignment operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssignOp {
    Assign,    // =
    AddAssign, // +=
    SubAssign, // -=
    MulAssign, // *=
    DivAssign, // /=
    ModAssign, // %=
}

impl AssignOp {
    pub fn as_str(&self) -> &'static str {
        match self {
            AssignOp::Assign => "=",
            AssignOp::AddAssign => "+=",
            AssignOp::SubAssign => "-=",
            AssignOp::MulAssign => "*=",
            AssignOp::DivAssign => "/=",
            AssignOp::ModAssign => "%=",
        }
    }
}

impl fmt::Display for AssignOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Literals.
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Integer {
        value: i128,
        raw: String,
        span: Span,
    },
    Float {
        raw: String,
        span: Span,
    },
    String {
        value: String,
        span: Span,
    },
    Char {
        value: char,
        span: Span,
    },
    Bool {
        value: bool,
        span: Span,
    },
    Null {
        span: Span,
    },
}

impl Literal {
    pub fn span(&self) -> Span {
        match self {
            Literal::Integer { span, .. } => *span,
            Literal::Float { span, .. } => *span,
            Literal::String { span, .. } => *span,
            Literal::Char { span, .. } => *span,
            Literal::Bool { span, .. } => *span,
            Literal::Null { span } => *span,
        }
    }
}

/// Expression nodes in the AST.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal(Literal),
    Identifier(Ident),
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
        span: Span,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    Update {
        op: UpdateOp,
        target: Box<Expr>,
        is_prefix: bool,
        span: Span,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    Index {
        target: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    PropertyAccess {
        target: Box<Expr>,
        property: Ident,
        span: Span,
    },
    ListLiteral {
        elements: Vec<Expr>,
        span: Span,
    },
    MapLiteral {
        entries: Vec<(Expr, Expr)>,
        span: Span,
    },
    Group {
        expr: Box<Expr>,
        span: Span,
    },
    Borrow {
        expr: Box<Expr>,
        is_mut: bool,
        span: Span,
    },
    Alloc {
        expr: Box<Expr>,
        span: Span,
    },
    Free {
        target: Box<Expr>,
        span: Span,
    },
    StructLiteral {
        name: Ident,
        fields: Vec<(Ident, Expr)>,
        span: Span,
    },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Literal(lit) => lit.span(),
            Expr::Identifier(id) => id.span,
            Expr::Unary { span, .. } => *span,
            Expr::Binary { span, .. } => *span,
            Expr::Update { span, .. } => *span,
            Expr::Call { span, .. } => *span,
            Expr::Index { span, .. } => *span,
            Expr::PropertyAccess { span, .. } => *span,
            Expr::ListLiteral { span, .. } => *span,
            Expr::MapLiteral { span, .. } => *span,
            Expr::Group { span, .. } => *span,
            Expr::Borrow { span, .. } => *span,
            Expr::Alloc { span, .. } => *span,
            Expr::Free { span, .. } => *span,
            Expr::StructLiteral { span, .. } => *span,
        }
    }
}

/// Function parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub name: Ident,
    pub type_annotation: Type,
    pub span: Span,
}

/// A block of statements enclosed in `{ ... }`.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub statements: Vec<Stmt>,
    pub span: Span,
}

/// `otherwise <cond> { ... }` branch.
#[derive(Debug, Clone, PartialEq)]
pub struct OtherwiseBranch {
    pub condition: Expr,
    pub body: Block,
    pub span: Span,
}

/// Variable declaration: `var/const/tem name:<TYPE> = expr;`
#[derive(Debug, Clone, PartialEq)]
pub struct VarDecl {
    pub kind: VarKind,
    pub name: Ident,
    pub type_annotation: Type,
    pub initializer: Option<Expr>,
    pub span: Span,
}

/// Assignment statement: `target = value;`, `target += value;`, etc.
#[derive(Debug, Clone, PartialEq)]
pub struct Assignment {
    pub target: Expr,
    pub op: AssignOp,
    pub value: Expr,
    pub span: Span,
}

/// Standalone update statement: `i++;`, `i--;`
#[derive(Debug, Clone, PartialEq)]
pub struct UpdateStmt {
    pub target: Expr,
    pub op: UpdateOp,
    pub is_prefix: bool,
    pub span: Span,
}

/// Function declaration: `fn name(params):<RetType> { body }`
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDecl {
    pub name: Ident,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
    pub body: Block,
    pub span: Span,
}

/// Return statement: `return;` or `return expr;`
#[derive(Debug, Clone, PartialEq)]
pub struct ReturnStmt {
    pub value: Option<Expr>,
    pub span: Span,
}

/// Conditional statement: `if <cond> { ... } otherwise <cond> { ... } else { ... }`
#[derive(Debug, Clone, PartialEq)]
pub struct IfStmt {
    pub condition: Expr,
    pub then_branch: Block,
    pub otherwise_branches: Vec<OtherwiseBranch>,
    pub else_branch: Option<Block>,
    pub span: Span,
}

/// While loop: `As <condition> { ... }`
#[derive(Debug, Clone, PartialEq)]
pub struct AsLoopStmt {
    pub condition: Expr,
    pub body: Block,
    pub span: Span,
}

/// For loop: `fr<init : cond : update> { ... }`
#[derive(Debug, Clone, PartialEq)]
pub struct ForLoopStmt {
    pub init: Option<Box<Stmt>>,
    pub condition: Option<Expr>,
    pub update: Option<Box<Stmt>>,
    pub body: Block,
    pub span: Span,
}

/// Do-while loop: `Do { ... } As <condition>;`
#[derive(Debug, Clone, PartialEq)]
pub struct DoAsLoopStmt {
    pub body: Block,
    pub condition: Expr,
    pub span: Span,
}

/// Access modifier: `public`, `private`, `protected`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccessModifier {
    Public,
    Private,
    Protected,
}

impl AccessModifier {
    pub fn as_str(&self) -> &'static str {
        match self {
            AccessModifier::Public => "public",
            AccessModifier::Private => "private",
            AccessModifier::Protected => "protected",
        }
    }
}

impl fmt::Display for AccessModifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Class field declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldDecl {
    pub access: AccessModifier,
    pub is_static: bool,
    pub var_decl: VarDecl,
    pub span: Span,
}

/// Class method declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct MethodDecl {
    pub access: AccessModifier,
    pub is_static: bool,
    pub is_abstract: bool,
    pub is_final: bool,
    pub function: FunctionDecl,
    pub span: Span,
}

/// Class member (field or method).
#[derive(Debug, Clone, PartialEq)]
pub enum ClassMember {
    Field(FieldDecl),
    Method(MethodDecl),
}

impl ClassMember {
    pub fn span(&self) -> Span {
        match self {
            ClassMember::Field(f) => f.span,
            ClassMember::Method(m) => m.span,
        }
    }
}

/// Class declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassDecl {
    pub is_abstract: bool,
    pub is_final: bool,
    pub name: Ident,
    pub parent: Option<Ident>,
    pub implements: Vec<Ident>,
    pub members: Vec<ClassMember>,
    pub span: Span,
}

/// Interface method signature declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceMethod {
    pub name: Ident,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
    pub span: Span,
}

/// Interface declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceDecl {
    pub name: Ident,
    pub methods: Vec<InterfaceMethod>,
    pub span: Span,
}

/// Pattern in a match-case arm.
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Literal(Literal),
    Wildcard(Span),
    Binding(Ident),
    Alternative(Vec<Pattern>, Span),
}

impl Pattern {
    pub fn span(&self) -> Span {
        match self {
            Pattern::Literal(lit) => lit.span(),
            Pattern::Wildcard(span) => *span,
            Pattern::Binding(ident) => ident.span,
            Pattern::Alternative(_, span) => *span,
        }
    }
}

/// A case arm in a match statement: `case pattern [if guard] { body }`
#[derive(Debug, Clone, PartialEq)]
pub struct MatchCase {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: Block,
    pub span: Span,
}

/// A match statement: `match expression { case ... }`
#[derive(Debug, Clone, PartialEq)]
pub struct MatchStmt {
    pub target: Expr,
    pub cases: Vec<MatchCase>,
    pub span: Span,
}

/// Struct field definition: `name:<TYPE>`
#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    pub name: Ident,
    pub field_type: Type,
    pub span: Span,
}

/// Native struct declaration: `struct Name { ... }`
#[derive(Debug, Clone, PartialEq)]
pub struct StructDecl {
    pub name: Ident,
    pub fields: Vec<StructField>,
    pub span: Span,
}

/// Unsafe block statement: `unsafe { ... }`
#[derive(Debug, Clone, PartialEq)]
pub struct UnsafeBlock {
    pub body: Block,
    pub span: Span,
}

/// External C function declaration: `fn name(params):<Ret>;`
#[derive(Debug, Clone, PartialEq)]
pub struct ExternFnDecl {
    pub name: Ident,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
    pub span: Span,
}

/// Extern block: `extern "C" { ... }`
#[derive(Debug, Clone, PartialEq)]
pub struct ExternBlock {
    pub abi: String,
    pub functions: Vec<ExternFnDecl>,
    pub span: Span,
}

/// Statements in NextLang.
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    VarDecl(VarDecl),
    Assignment(Assignment),
    Update(UpdateStmt),
    FunctionDecl(FunctionDecl),
    Return(ReturnStmt),
    If(IfStmt),
    AsLoop(AsLoopStmt),
    ForLoop(ForLoopStmt),
    DoAsLoop(DoAsLoopStmt),
    Block(Block),
    Expr(Expr),
    ClassDecl(ClassDecl),
    InterfaceDecl(InterfaceDecl),
    Match(MatchStmt),
    StructDecl(StructDecl),
    Unsafe(UnsafeBlock),
    ExternBlock(ExternBlock),
}

impl Stmt {
    pub fn span(&self) -> Span {
        match self {
            Stmt::VarDecl(s) => s.span,
            Stmt::Assignment(s) => s.span,
            Stmt::Update(s) => s.span,
            Stmt::FunctionDecl(s) => s.span,
            Stmt::Return(s) => s.span,
            Stmt::If(s) => s.span,
            Stmt::AsLoop(s) => s.span,
            Stmt::ForLoop(s) => s.span,
            Stmt::DoAsLoop(s) => s.span,
            Stmt::Block(s) => s.span,
            Stmt::Expr(e) => e.span(),
            Stmt::ClassDecl(c) => c.span,
            Stmt::InterfaceDecl(i) => i.span,
            Stmt::Match(m) => m.span,
            Stmt::StructDecl(s) => s.span,
            Stmt::Unsafe(u) => u.span,
            Stmt::ExternBlock(e) => e.span,
        }
    }
}

/// Program root AST node.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Stmt>,
    pub span: Span,
}

impl Program {
    pub fn new(statements: Vec<Stmt>, span: Span) -> Self {
        Self { statements, span }
    }
}

/// Pretty-printer for formatted AST visualization (`nx parse`).
pub struct AstFormatter {
    indent_level: usize,
    output: String,
}

impl Default for AstFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl AstFormatter {
    pub fn new() -> Self {
        Self {
            indent_level: 0,
            output: String::new(),
        }
    }

    pub fn format_program(mut self, program: &Program) -> String {
        self.line(&format!("Program [{}]", program.span));
        self.indent();
        if program.statements.is_empty() {
            self.line("<empty>");
        } else {
            for stmt in &program.statements {
                self.format_stmt(stmt);
            }
        }
        self.dedent();
        self.output
    }

    fn indent(&mut self) {
        self.indent_level += 1;
    }

    fn dedent(&mut self) {
        self.indent_level = self.indent_level.saturating_sub(1);
    }

    fn line(&mut self, text: &str) {
        for _ in 0..self.indent_level {
            self.output.push_str("  ");
        }
        self.output.push_str(text);
        self.output.push('\n');
    }

    fn format_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::VarDecl(var_decl) => {
                self.line(&format!(
                    "VarDecl ({}) [{}]",
                    var_decl.kind.as_str(),
                    var_decl.span
                ));
                self.indent();
                self.line(&format!("name: {}", var_decl.name.name));
                self.line(&format!("type: <{}>", var_decl.type_annotation));
                if let Some(ref init) = var_decl.initializer {
                    self.line("initializer:");
                    self.indent();
                    self.format_expr(init);
                    self.dedent();
                }
                self.dedent();
            }
            Stmt::Assignment(assign) => {
                self.line(&format!("Assignment ({}) [{}]", assign.op, assign.span));
                self.indent();
                self.line("target:");
                self.indent();
                self.format_expr(&assign.target);
                self.dedent();
                self.line("value:");
                self.indent();
                self.format_expr(&assign.value);
                self.dedent();
                self.dedent();
            }
            Stmt::Update(up) => {
                let position = if up.is_prefix { "prefix" } else { "postfix" };
                self.line(&format!(
                    "UpdateStmt ({} {}) [{}]",
                    up.op, position, up.span
                ));
                self.indent();
                self.format_expr(&up.target);
                self.dedent();
            }
            Stmt::FunctionDecl(f) => {
                let ret = match &f.return_type {
                    Some(t) => format!(" -> <{}>", t),
                    None => String::new(),
                };
                self.line(&format!(
                    "FunctionDecl {}{}: [{}]",
                    f.name.name, ret, f.span
                ));
                self.indent();
                if !f.params.is_empty() {
                    self.line("params:");
                    self.indent();
                    for param in &f.params {
                        self.line(&format!(
                            "{}: <{}> [{}]",
                            param.name.name, param.type_annotation, param.span
                        ));
                    }
                    self.dedent();
                }
                self.line("body:");
                self.indent();
                self.format_block(&f.body);
                self.dedent();
                self.dedent();
            }
            Stmt::Return(ret) => {
                self.line(&format!("ReturnStmt [{}]", ret.span));
                if let Some(ref val) = ret.value {
                    self.indent();
                    self.format_expr(val);
                    self.dedent();
                }
            }
            Stmt::If(if_stmt) => {
                self.line(&format!("IfStmt [{}]", if_stmt.span));
                self.indent();
                self.line("condition:");
                self.indent();
                self.format_expr(&if_stmt.condition);
                self.dedent();
                self.line("then:");
                self.indent();
                self.format_block(&if_stmt.then_branch);
                self.dedent();
                for otherwise in &if_stmt.otherwise_branches {
                    self.line(&format!("otherwise [{}]", otherwise.span));
                    self.indent();
                    self.line("condition:");
                    self.indent();
                    self.format_expr(&otherwise.condition);
                    self.dedent();
                    self.line("body:");
                    self.indent();
                    self.format_block(&otherwise.body);
                    self.dedent();
                    self.dedent();
                }
                if let Some(ref else_block) = if_stmt.else_branch {
                    self.line("else:");
                    self.indent();
                    self.format_block(else_block);
                    self.dedent();
                }
                self.dedent();
            }
            Stmt::AsLoop(as_loop) => {
                self.line(&format!("AsLoop [{}]", as_loop.span));
                self.indent();
                self.line("condition:");
                self.indent();
                self.format_expr(&as_loop.condition);
                self.dedent();
                self.line("body:");
                self.indent();
                self.format_block(&as_loop.body);
                self.dedent();
                self.dedent();
            }
            Stmt::ForLoop(for_loop) => {
                self.line(&format!("ForLoop (fr) [{}]", for_loop.span));
                self.indent();
                if let Some(ref init) = for_loop.init {
                    self.line("init:");
                    self.indent();
                    self.format_stmt(init);
                    self.dedent();
                }
                if let Some(ref cond) = for_loop.condition {
                    self.line("condition:");
                    self.indent();
                    self.format_expr(cond);
                    self.dedent();
                }
                if let Some(ref update) = for_loop.update {
                    self.line("update:");
                    self.indent();
                    self.format_stmt(update);
                    self.dedent();
                }
                self.line("body:");
                self.indent();
                self.format_block(&for_loop.body);
                self.dedent();
                self.dedent();
            }
            Stmt::DoAsLoop(do_as) => {
                self.line(&format!("DoAsLoop (Do ... As) [{}]", do_as.span));
                self.indent();
                self.line("body:");
                self.indent();
                self.format_block(&do_as.body);
                self.dedent();
                self.line("condition:");
                self.indent();
                self.format_expr(&do_as.condition);
                self.dedent();
                self.dedent();
            }
            Stmt::Block(block) => {
                self.line(&format!("Block [{}]", block.span));
                self.indent();
                self.format_block(block);
                self.dedent();
            }
            Stmt::Expr(expr) => {
                self.line(&format!("ExprStmt [{}]", expr.span()));
                self.indent();
                self.format_expr(expr);
                self.dedent();
            }
            Stmt::ClassDecl(class_decl) => {
                let mut modifiers = String::new();
                if class_decl.is_abstract {
                    modifiers.push_str("abstract ");
                }
                if class_decl.is_final {
                    modifiers.push_str("final ");
                }
                let mut header = format!(
                    "ClassDecl {modifiers}{} [{}]",
                    class_decl.name.name, class_decl.span
                );
                if let Some(ref parent) = class_decl.parent {
                    header.push_str(&format!(" extends {}", parent.name));
                }
                if !class_decl.implements.is_empty() {
                    header.push_str(" implements ");
                    let ifaces: Vec<_> = class_decl
                        .implements
                        .iter()
                        .map(|i| i.name.as_str())
                        .collect();
                    header.push_str(&ifaces.join(", "));
                }
                self.line(&header);
                self.indent();
                for member in &class_decl.members {
                    match member {
                        ClassMember::Field(field) => {
                            let stat = if field.is_static { "static " } else { "" };
                            self.line(&format!(
                                "Field {} {}{}: <{}> [{}]",
                                field.access,
                                stat,
                                field.var_decl.name.name,
                                field.var_decl.type_annotation,
                                field.span
                            ));
                            if let Some(ref init) = field.var_decl.initializer {
                                self.indent();
                                self.format_expr(init);
                                self.dedent();
                            }
                        }
                        ClassMember::Method(method) => {
                            let stat = if method.is_static { "static " } else { "" };
                            let abst = if method.is_abstract { "abstract " } else { "" };
                            let fin = if method.is_final { "final " } else { "" };
                            let ret = match &method.function.return_type {
                                Some(t) => format!(": <{}>", t),
                                None => ": <VOID>".to_string(),
                            };
                            self.line(&format!(
                                "Method {} {}{}{}{}{} [{}]",
                                method.access,
                                stat,
                                abst,
                                fin,
                                method.function.name.name,
                                ret,
                                method.span
                            ));
                            self.indent();
                            if !method.function.params.is_empty() {
                                self.line("params:");
                                self.indent();
                                for param in &method.function.params {
                                    self.line(&format!(
                                        "{}: <{}> [{}]",
                                        param.name.name, param.type_annotation, param.span
                                    ));
                                }
                                self.dedent();
                            }
                            if !method.is_abstract {
                                self.line("body:");
                                self.indent();
                                self.format_block(&method.function.body);
                                self.dedent();
                            }
                            self.dedent();
                        }
                    }
                }
                self.dedent();
            }
            Stmt::InterfaceDecl(iface_decl) => {
                self.line(&format!(
                    "InterfaceDecl {} [{}]",
                    iface_decl.name.name, iface_decl.span
                ));
                self.indent();
                for method in &iface_decl.methods {
                    let ret = match &method.return_type {
                        Some(t) => format!(": <{}>", t),
                        None => ": <VOID>".to_string(),
                    };
                    self.line(&format!(
                        "InterfaceMethod {}{} [{}]",
                        method.name.name, ret, method.span
                    ));
                    if !method.params.is_empty() {
                        self.indent();
                        self.line("params:");
                        self.indent();
                        for param in &method.params {
                            self.line(&format!(
                                "{}: <{}> [{}]",
                                param.name.name, param.type_annotation, param.span
                            ));
                        }
                        self.dedent();
                        self.dedent();
                    }
                }
                self.dedent();
            }
            Stmt::Match(m) => {
                self.line(&format!("MatchStmt [{}]", m.span));
                self.indent();
                self.line("target:");
                self.indent();
                self.format_expr(&m.target);
                self.dedent();
                self.line("cases:");
                self.indent();
                for case in &m.cases {
                    self.line(&format!("Case [{}]", case.span));
                    self.indent();
                    self.line(&format!("pattern: {}", self.format_pattern(&case.pattern)));
                    if let Some(ref guard) = case.guard {
                        self.line("guard:");
                        self.indent();
                        self.format_expr(guard);
                        self.dedent();
                    }
                    self.line("body:");
                    self.indent();
                    for s in &case.body.statements {
                        self.format_stmt(s);
                    }
                    self.dedent();
                    self.dedent();
                }
                self.dedent();
                self.dedent();
            }
            Stmt::StructDecl(s) => {
                self.line(&format!("StructDecl {} [{}]", s.name.name, s.span));
                self.indent();
                for f in &s.fields {
                    self.line(&format!("{}: <{}> [{}]", f.name.name, f.field_type, f.span));
                }
                self.dedent();
            }
            Stmt::Unsafe(u) => {
                self.line(&format!("UnsafeBlock [{}]", u.span));
                self.indent();
                self.format_block(&u.body);
                self.dedent();
            }
            Stmt::ExternBlock(e) => {
                self.line(&format!("ExternBlock \"{}\" [{}]", e.abi, e.span));
                self.indent();
                for f in &e.functions {
                    let ret = match &f.return_type {
                        Some(t) => format!(": <{}>", t),
                        None => ": <VOID>".to_string(),
                    };
                    self.line(&format!("ExternFn {}{} [{}]", f.name.name, ret, f.span));
                    if !f.params.is_empty() {
                        self.indent();
                        for p in &f.params {
                            self.line(&format!(
                                "{}: <{}> [{}]",
                                p.name.name, p.type_annotation, p.span
                            ));
                        }
                        self.dedent();
                    }
                }
                self.dedent();
            }
        }
    }

    fn format_block(&mut self, block: &Block) {
        if block.statements.is_empty() {
            self.line("<empty block>");
        } else {
            for stmt in &block.statements {
                self.format_stmt(stmt);
            }
        }
    }

    fn format_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Literal(lit) => match lit {
                Literal::Integer { value, raw, span } => {
                    self.line(&format!("IntegerLiteral: {} ({}) [{}]", value, raw, span));
                }
                Literal::Float { raw, span } => {
                    self.line(&format!("FloatLiteral: {} [{}]", raw, span));
                }
                Literal::String { value, span } => {
                    self.line(&format!("StringLiteral: {:?} [{}]", value, span));
                }
                Literal::Char { value, span } => {
                    self.line(&format!("CharLiteral: {:?} [{}]", value, span));
                }
                Literal::Bool { value, span } => {
                    self.line(&format!("BoolLiteral: {} [{}]", value, span));
                }
                Literal::Null { span } => {
                    self.line(&format!("NullLiteral [{}]", span));
                }
            },
            Expr::Identifier(ident) => {
                self.line(&format!("Identifier: {} [{}]", ident.name, ident.span));
            }
            Expr::Unary { op, operand, span } => {
                self.line(&format!("UnaryExpr ({}) [{}]", op, span));
                self.indent();
                self.format_expr(operand);
                self.dedent();
            }
            Expr::Binary {
                op,
                left,
                right,
                span,
            } => {
                self.line(&format!("BinaryExpr ({}) [{}]", op, span));
                self.indent();
                self.format_expr(left);
                self.format_expr(right);
                self.dedent();
            }
            Expr::Update {
                op,
                target,
                is_prefix,
                span,
            } => {
                let pos = if *is_prefix { "prefix" } else { "postfix" };
                self.line(&format!("UpdateExpr ({} {}) [{}]", op, pos, span));
                self.indent();
                self.format_expr(target);
                self.dedent();
            }
            Expr::Call { callee, args, span } => {
                self.line(&format!("CallExpr [{}]", span));
                self.indent();
                self.line("callee:");
                self.indent();
                self.format_expr(callee);
                self.dedent();
                if !args.is_empty() {
                    self.line("arguments:");
                    self.indent();
                    for arg in args {
                        self.format_expr(arg);
                    }
                    self.dedent();
                }
                self.dedent();
            }
            Expr::Index {
                target,
                index,
                span,
            } => {
                self.line(&format!("IndexExpr [{}]", span));
                self.indent();
                self.line("target:");
                self.indent();
                self.format_expr(target);
                self.dedent();
                self.line("index:");
                self.indent();
                self.format_expr(index);
                self.dedent();
                self.dedent();
            }
            Expr::PropertyAccess {
                target,
                property,
                span,
            } => {
                self.line(&format!("PropertyAccess (.{}) [{}]", property.name, span));
                self.indent();
                self.format_expr(target);
                self.dedent();
            }
            Expr::ListLiteral { elements, span } => {
                self.line(&format!("ListLiteral [{}]", span));
                self.indent();
                for elem in elements {
                    self.format_expr(elem);
                }
                self.dedent();
            }
            Expr::MapLiteral { entries, span } => {
                self.line(&format!("MapLiteral [{}]", span));
                self.indent();
                for (key, val) in entries {
                    self.line("entry:");
                    self.indent();
                    self.line("key:");
                    self.indent();
                    self.format_expr(key);
                    self.dedent();
                    self.line("value:");
                    self.indent();
                    self.format_expr(val);
                    self.dedent();
                    self.dedent();
                }
                self.dedent();
            }
            Expr::Group { expr, span } => {
                self.line(&format!("GroupExpr [{}]", span));
                self.indent();
                self.format_expr(expr);
                self.dedent();
            }
            Expr::Borrow { expr, is_mut, span } => {
                let m = if *is_mut { "&mut" } else { "&" };
                self.line(&format!("BorrowExpr ({}) [{}]", m, span));
                self.indent();
                self.format_expr(expr);
                self.dedent();
            }
            Expr::Alloc { expr, span } => {
                self.line(&format!("AllocExpr [{}]", span));
                self.indent();
                self.format_expr(expr);
                self.dedent();
            }
            Expr::Free { target, span } => {
                self.line(&format!("FreeExpr [{}]", span));
                self.indent();
                self.format_expr(target);
                self.dedent();
            }
            Expr::StructLiteral { name, fields, span } => {
                self.line(&format!("StructLiteral {} [{}]", name.name, span));
                self.indent();
                for (fname, fval) in fields {
                    self.line(&format!("field {}:", fname.name));
                    self.indent();
                    self.format_expr(fval);
                    self.dedent();
                }
                self.dedent();
            }
        }
    }

    fn format_pattern(&self, pattern: &Pattern) -> String {
        match pattern {
            Pattern::Literal(lit) => match lit {
                Literal::Integer { value, .. } => format!("{}", value),
                Literal::Float { raw, .. } => raw.clone(),
                Literal::String { value, .. } => format!("{:?}", value),
                Literal::Char { value, .. } => format!("{:?}", value),
                Literal::Bool { value, .. } => format!("{}", value),
                Literal::Null { .. } => "null".to_string(),
            },
            Pattern::Wildcard(_) => "_".to_string(),
            Pattern::Binding(ident) => ident.name.clone(),
            Pattern::Alternative(alts, _) => {
                let parts: Vec<_> = alts.iter().map(|p| self.format_pattern(p)).collect();
                parts.join(" | ")
            }
        }
    }
}
