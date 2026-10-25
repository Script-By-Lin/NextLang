use nx_ast::*;
use nx_diagnostics::DiagnosticBag;
use nx_parser::{check_source, parse_source};

#[test]
fn test_var_const_tem_declarations() {
    let src = r#"
var a:<INT32> = 1;
const B:<FL64> = 2.0;
tem c:<STRING> = "hello";
"#;
    let mut bag = DiagnosticBag::new();
    let prog = parse_source(src, &mut bag);
    assert!(!bag.has_errors());
    assert_eq!(prog.statements.len(), 3);

    if let Stmt::VarDecl(ref v) = prog.statements[0] {
        assert_eq!(v.kind, VarKind::Var);
        assert_eq!(v.name.name, "a");
    } else {
        panic!("expected VarDecl");
    }

    if let Stmt::VarDecl(ref v) = prog.statements[1] {
        assert_eq!(v.kind, VarKind::Const);
        assert_eq!(v.name.name, "B");
    } else {
        panic!("expected VarDecl");
    }

    if let Stmt::VarDecl(ref v) = prog.statements[2] {
        assert_eq!(v.kind, VarKind::Tem);
        assert_eq!(v.name.name, "c");
    } else {
        panic!("expected VarDecl");
    }
}

#[test]
fn test_complex_types() {
    let src = r#"
var list:<LIST<INT32>> = [1, 2];
var map:<MAP<STRING, INT32>> = { "a": 1 };
var nested:<LIST<LIST<INT32>>> = [[1]];
var union:<LIST<INT32 | FL64>> = [1, 2.5];
"#;
    let mut bag = DiagnosticBag::new();
    let prog = parse_source(src, &mut bag);
    assert!(!bag.has_errors());
    assert_eq!(prog.statements.len(), 4);
}

#[test]
fn test_precedence_arithmetic_and_logical() {
    let src = r#"
var res:<INT32> = 2 + 3 * 4;
var flag:<BOOL> = 1 > 0 & 2 < 5 | not false;
"#;
    let mut bag = DiagnosticBag::new();
    let prog = parse_source(src, &mut bag);
    assert!(!bag.has_errors());

    // 2 + (3 * 4)
    if let Stmt::VarDecl(ref v) = prog.statements[0] {
        if let Some(Expr::Binary {
            op, left, right, ..
        }) = &v.initializer
        {
            assert_eq!(*op, BinaryOp::Add);
            assert!(matches!(
                **left,
                Expr::Literal(Literal::Integer { value: 2, .. })
            ));
            assert!(matches!(
                **right,
                Expr::Binary {
                    op: BinaryOp::Mul,
                    ..
                }
            ));
        } else {
            panic!("expected binary add expr");
        }
    }
}

#[test]
fn test_function_declaration_and_call() {
    let src = r#"
fn multiply(x:<INT32>, y:<INT32>):<INT32> {
    return x * y;
}

var result:<INT32> = multiply(3, 4);
vout(result);
"#;
    let mut bag = DiagnosticBag::new();
    let prog = check_source(src, &mut bag);
    assert!(!bag.has_errors(), "diagnostics: {:?}", bag.diagnostics());
    assert_eq!(prog.statements.len(), 3);
}

#[test]
fn test_if_otherwise_else() {
    let src = r#"
var val:<INT32> = 50;
if val > 100 {
    vout("high");
} otherwise val > 40 {
    vout("medium");
} otherwise val > 10 {
    vout("low");
} else {
    vout("zero");
}
"#;
    let mut bag = DiagnosticBag::new();
    let prog = check_source(src, &mut bag);
    assert!(!bag.has_errors());

    if let Stmt::If(ref if_stmt) = prog.statements[1] {
        assert_eq!(if_stmt.otherwise_branches.len(), 2);
        assert!(if_stmt.else_branch.is_some());
    } else {
        panic!("expected if statement");
    }
}

#[test]
fn test_all_loop_forms() {
    let src = r#"
tem i:<INT32> = 0;
As i < 5 {
    i++;
}

fr<tem k:<INT32> = 0: k < 10: k++> {
    vout(k);
}

Do {
    i--;
} As i > 0;
"#;
    let mut bag = DiagnosticBag::new();
    let prog = check_source(src, &mut bag);
    assert!(!bag.has_errors(), "diagnostics: {:?}", bag.diagnostics());
    assert_eq!(prog.statements.len(), 4);
}

#[test]
fn test_malformed_syntax_no_panic() {
    let malformed_cases = [
        "var",
        "var x",
        "var x:",
        "var x:<INT32",
        "var x:<INT32> =",
        "if",
        "if {",
        "otherwise {",
        "As {",
        "fr<i < 10> {",
        "Do { }",
        "fn",
        "fn (",
        "{ unclosed block",
        "(1 + )",
        "age = ;",
    ];

    for case in malformed_cases {
        let mut bag = DiagnosticBag::new();
        // Crucial test: parser must never panic on malformed input!
        let _ = parse_source(case, &mut bag);
        assert!(
            bag.has_errors(),
            "expected errors for malformed case: '{}'",
            case
        );
    }
}

#[test]
fn test_scope_validation_errors() {
    // 1. Undeclared variable assignment
    let mut bag = DiagnosticBag::new();
    let _ = check_source("x = 10;\n", &mut bag);
    assert!(bag.has_errors());
    assert!(bag
        .diagnostics()
        .iter()
        .any(|d| d.message.contains("undeclared variable 'x'")));

    // 2. Reassignment of constant
    let mut bag = DiagnosticBag::new();
    let _ = check_source("const C:<INT32> = 1;\nC = 2;\n", &mut bag);
    assert!(bag.has_errors());
    assert!(bag
        .diagnostics()
        .iter()
        .any(|d| d.message.contains("cannot reassign constant 'C'")));

    // 3. Duplicate declaration in same scope
    let mut bag = DiagnosticBag::new();
    let _ = check_source("var a:<INT32> = 1;\nvar a:<INT32> = 2;\n", &mut bag);
    assert!(bag.has_errors());
    assert!(bag
        .diagnostics()
        .iter()
        .any(|d| d.message.contains("already declared in this scope")));
}

#[test]
fn test_class_and_interface_parsing() {
    let src = r#"
interface Speaker {
    fn speak():<VOID>;
}

abstract class Animal implements Speaker {
    public var name:<STRING>;
    public abstract fn speak():<VOID>;
}

class Dog extends Animal {
    private var age:<INT32>;

    public fn init(name:<STRING>, age:<INT32>):<VOID> {
        self.name = name;
        self.age = age;
    }

    public fn speak():<VOID> {
        vout("Woof");
    }
}
"#;
    let mut bag = DiagnosticBag::new();
    let prog = parse_source(src, &mut bag);
    assert!(!bag.has_errors());
    assert_eq!(prog.statements.len(), 3);

    // 1. Interface Speaker
    if let Stmt::InterfaceDecl(ref iface) = prog.statements[0] {
        assert_eq!(iface.name.name, "Speaker");
        assert_eq!(iface.methods.len(), 1);
        assert_eq!(iface.methods[0].name.name, "speak");
    } else {
        panic!("expected InterfaceDecl");
    }

    // 2. Abstract class Animal
    if let Stmt::ClassDecl(ref cls) = prog.statements[1] {
        assert_eq!(cls.name.name, "Animal");
        assert!(cls.is_abstract);
        assert!(!cls.is_final);
        assert_eq!(cls.implements.len(), 1);
        assert_eq!(cls.implements[0].name, "Speaker");
        assert_eq!(cls.members.len(), 2);
    } else {
        panic!("expected ClassDecl Animal");
    }

    // 3. Concrete class Dog extends Animal
    if let Stmt::ClassDecl(ref cls) = prog.statements[2] {
        assert_eq!(cls.name.name, "Dog");
        assert!(!cls.is_abstract);
        assert_eq!(cls.parent.as_ref().unwrap().name, "Animal");
        assert_eq!(cls.members.len(), 3);
    } else {
        panic!("expected ClassDecl Dog");
    }
}

#[test]
fn test_match_statement_parsing() {
    let src = r#"
var status:<INT32> = 200;
match status {
    case 200 {
        vout("OK");
    }
    case 404 | 403 {
        vout("Client Error");
    }
    case code if code >= 500 {
        vout("Server Error");
    }
    case _ {
        vout("Unknown");
    }
}
"#;
    let mut bag = DiagnosticBag::new();
    let prog = parse_source(src, &mut bag);
    assert!(!bag.has_errors(), "diagnostics: {:?}", bag.diagnostics());
    assert_eq!(prog.statements.len(), 2);

    if let Stmt::Match(ref m) = prog.statements[1] {
        assert_eq!(m.cases.len(), 4);
        assert!(matches!(m.cases[0].pattern, Pattern::Literal(_)));
        assert!(matches!(m.cases[1].pattern, Pattern::Alternative(_, _)));
        assert!(matches!(m.cases[2].pattern, Pattern::Binding(_)));
        assert!(m.cases[2].guard.is_some());
        assert!(matches!(m.cases[3].pattern, Pattern::Wildcard(_)));
    } else {
        panic!("expected Stmt::Match");
    }
}

#[test]
fn test_ownership_syntax_parsing() {
    let src = r#"
var num:<INT32> = 42;
var r:<&INT32> = &num;
var mr:<&mut INT32> = &mut num;
var boxed:<INT32> = alloc(100);
free(boxed);
"#;
    let mut bag = DiagnosticBag::new();
    let prog = parse_source(src, &mut bag);
    assert!(!bag.has_errors(), "diagnostics: {:?}", bag.diagnostics());
    assert_eq!(prog.statements.len(), 5);
}

#[test]
fn test_phase8_systems_syntax() {
    let src = r#"
struct Point {
    x:<FL64>,
    y:<FL64>
}

extern "C" {
    fn puts(s:<STRING>):<INT32>;
}

unsafe {
    var p:<Point> = Point { x: 10.5, y: 20.5 };
    var bitwise:<INT32> = (1 ^ 2) << 3 >> 1;
    var inverted:<INT32> = ~bitwise;
}
"#;
    let mut bag = DiagnosticBag::new();
    let prog = parse_source(src, &mut bag);
    assert!(!bag.has_errors(), "diagnostics: {:?}", bag.diagnostics());
    assert_eq!(prog.statements.len(), 3);

    // 1. StructDecl
    match &prog.statements[0] {
        Stmt::StructDecl(s) => {
            assert_eq!(s.name.name, "Point");
            assert_eq!(s.fields.len(), 2);
            assert_eq!(s.fields[0].name.name, "x");
            assert_eq!(s.fields[1].name.name, "y");
        }
        _ => panic!("expected StructDecl"),
    }

    // 2. ExternBlock
    match &prog.statements[1] {
        Stmt::ExternBlock(e) => {
            assert_eq!(e.abi, "C");
            assert_eq!(e.functions.len(), 1);
            assert_eq!(e.functions[0].name.name, "puts");
        }
        _ => panic!("expected ExternBlock"),
    }

    // 3. Unsafe block
    match &prog.statements[2] {
        Stmt::Unsafe(u) => {
            assert_eq!(u.body.statements.len(), 3);
        }
        _ => panic!("expected Unsafe block"),
    }
}

#[test]
fn test_phase9_import_and_test_parsing() {
    let src = r#"
import math;
import "utils.nx";

test "basic assertion" {
    assert(true, "must be true");
}
"#;
    let mut bag = DiagnosticBag::new();
    let prog = parse_source(src, &mut bag);
    assert!(!bag.has_errors());
    assert_eq!(prog.statements.len(), 3);

    match &prog.statements[0] {
        Stmt::Import(i) => assert_eq!(i.path, "math"),
        _ => panic!("expected Stmt::Import"),
    }

    match &prog.statements[1] {
        Stmt::Import(i) => assert_eq!(i.path, "utils.nx"),
        _ => panic!("expected Stmt::Import"),
    }

    match &prog.statements[2] {
        Stmt::TestDecl(t) => {
            assert_eq!(t.name, "basic assertion");
            assert_eq!(t.body.statements.len(), 1);
        }
        _ => panic!("expected Stmt::TestDecl"),
    }
}

#[test]
fn test_phase9_module_resolution_and_cycle_detection() {
    use std::path::Path;

    // 1. Stdlib import resolution
    let src = r#"
import math;
var x:<FL64> = sqrt(25.0);
"#;
    let mut bag = DiagnosticBag::new();
    let prog = nx_parser::parse_source_with_modules(Path::new("main.nx"), src, &mut bag);
    assert!(!bag.has_errors());
    // prog should contain the merged math declarations plus the entry statements
    assert!(prog.statements.len() > 2);

    // 2. Circular import detection
    let tmp_dir = std::env::temp_dir().join(format!("nx_mod_test_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&tmp_dir);
    let mod_a = tmp_dir.join("a.nx");
    let mod_b = tmp_dir.join("b.nx");

    let _ = std::fs::write(&mod_a, "import \"b.nx\";\nfn fa():<VOID> {}\n");
    let _ = std::fs::write(&mod_b, "import \"a.nx\";\nfn fb():<VOID> {}\n");

    let mut cycle_bag = DiagnosticBag::new();
    let content = std::fs::read_to_string(&mod_a).unwrap();
    let _ = nx_parser::parse_source_with_modules(&mod_a, &content, &mut cycle_bag);
    assert!(cycle_bag.has_errors());
    assert!(cycle_bag
        .diagnostics()
        .iter()
        .any(|d| d.message.contains("circular dependency detected")));

    let _ = std::fs::remove_dir_all(&tmp_dir);
}
