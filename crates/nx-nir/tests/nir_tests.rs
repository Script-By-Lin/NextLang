use nx_diagnostics::DiagnosticBag;
use nx_nir::*;
use nx_parser::parse_source;

fn lower(src: &str) -> String {
    let mut bag = DiagnosticBag::new();
    let prog = parse_source(src, &mut bag);
    assert!(!bag.has_errors(), "parse error: {:?}", bag.diagnostics());
    let lowering = AstLowering::new("test");
    let mut nir = lowering.lower_program(&prog);
    optimize_module(&mut nir, OptLevel::O1);
    print_module(&nir)
}

#[test]
fn test_nir_conditions_and_loops() {
    let src = r#"
var x:<INT32> = 10;
if x > 5 {
    vout("greater");
} otherwise x == 5 {
    vout("equal");
} else {
    vout("less");
}

As x > 0 {
    x--;
}
"#;
    let nir = lower(src);
    assert!(nir.contains("br %t"));
    assert!(nir.contains("if_then"));
    assert!(nir.contains("as_header"));
    assert!(nir.contains("as_body"));
}

#[test]
fn test_nir_functions_and_calls() {
    let src = r#"
fn add(a:<INT32>, b:<INT32>):<INT32> {
    return a + b;
}

fn main() {
    var res:<INT32> = add(5, 7);
    vout("result:", res);
}
"#;
    let nir = lower(src);
    assert!(nir.contains("fn add(a: I32, b: I32) -> I32"));
    assert!(nir.contains("fn main() -> I32 [entry]"));
    assert!(nir.contains("call add("));
}

#[test]
fn test_nir_oop_class_and_methods() {
    let src = r#"
class Shape {
    var area:<INT64> = 0;
    fn init(a:<INT64>) {
        self.area = a;
        return;
    }
    fn get_area():<INT64> {
        return self.area;
    }
}

var s:<Shape> = Shape(42);
vout(s.get_area());
"#;
    let nir = lower(src);
    assert!(nir.contains("class Shape"));
    assert!(nir.contains("fn Shape_init("));
    assert!(nir.contains("fn Shape_get_area("));
}

#[test]
fn test_nir_match_case() {
    let src = r#"
var status:<INT32> = 200;
match status {
    case 200 { vout("OK"); }
    case 404 | 500 { vout("Error"); }
    case _ { vout("Other"); }
}
"#;
    let nir = lower(src);
    assert!(nir.contains("case_"));
    assert!(nir.contains("case_body_"));
    assert!(nir.contains("match_merge_"));
}

#[test]
fn test_nir_phase8_systems() {
    let src = r#"
struct Point {
    x:<INT32>,
    y:<INT32>
}

extern "C" {
    fn abs(n:<INT32>):<INT32>;
}

fn main() {
    var a:<INT32> = 12;
    var b:<INT32> = 5;
    var bit_and:<INT32> = a & b;
    var bit_xor:<INT32> = a ^ b;
    var bit_shl:<INT32> = a << 2;
    var bit_not:<INT32> = ~a;

    var p:<Point> = Point { x: 10, y: 20 };
    vout("point x:", p.x);

    unsafe {
        var res:<INT32> = abs(-42);
        vout("abs:", res);
    }
}
"#;
    let nir = lower(src);
    assert!(nir.contains("class Point"));
    assert!(nir.contains("binop BitAnd"));
    assert!(nir.contains("binop BitXor"));
    assert!(nir.contains("binop Shl"));
    assert!(nir.contains("unop BitNot"));
    assert!(nir.contains("call abs("));
}
