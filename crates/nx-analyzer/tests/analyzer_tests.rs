use nx_analyzer::analyze_source;
use nx_diagnostics::DiagnosticBag;

#[test]
fn test_valid_primitive_and_variable_types() {
    let src = r#"
var a:<INT8> = 127;
var b:<INT16> = 32767;
var c:<INT32> = 2147483647;
var d:<INT64> = 9223372036854775807;
var u:<UINT8> = 255;
var f:<FL64> = 3.14159;
var flag:<BOOL> = true;
var ch:<CHAR> = 'X';
var text:<STRING> = "Hello NextLang";
var dynamic_val:<ANY> = 123;
dynamic_val = "now a string";
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(!bag.has_errors(), "diagnostics: {:?}", bag.diagnostics());
}

#[test]
fn test_valid_collections_and_unions() {
    let src = r#"
var numbers:<LIST<INT32>> = [1, 2, 3];
var words:<LIST<STRING>> = ["alpha", "beta"];
var tags:<SET<STRING>> = ["tag1", "tag2"];
var ages:<MAP<STRING, INT32>> = { "Alice": 30, "Bob": 25 };
var union_val:<INT32 | FL64> = 10;
union_val = 3.14;
var union_list:<LIST<INT32 | FL64>> = [1, 2.5, 3];
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(!bag.has_errors(), "diagnostics: {:?}", bag.diagnostics());
}

#[test]
fn test_valid_functions_and_calls() {
    let src = r#"
fn add(a:<INT32>, b:<INT32>):<INT32> {
    return a + b;
}

fn log_msg(msg:<STRING>):<VOID> {
    vout(msg);
    return;
}

var sum:<INT32> = add(10, 20);
log_msg("done");
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(!bag.has_errors(), "diagnostics: {:?}", bag.diagnostics());
}

#[test]
fn test_valid_forward_reference_functions() {
    let src = r#"
fn caller():<INT32> {
    return callee(42);
}

fn callee(x:<INT32>):<INT32> {
    return x * 2;
}
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(!bag.has_errors(), "diagnostics: {:?}", bag.diagnostics());
}

#[test]
fn test_valid_control_flow_and_scopes() {
    let src = r#"
var score:<INT32> = 85;
if score >= 90 {
    vout("A");
} otherwise score >= 80 {
    vout("B");
} else {
    vout("C");
}

tem i:<INT32> = 0;
As i < 5 {
    i++;
}

fr<tem j:<INT32> = 0: j < 5: j++> {
    vout(j);
}

Do {
    i--;
} As i > 0;
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(!bag.has_errors(), "diagnostics: {:?}", bag.diagnostics());
}

#[test]
fn test_error_type_mismatch_assignment() {
    let src = r#"
var age:<INT32> = 20;
var name:<STRING> = "Alex";
age = name;
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    let diag = bag
        .diagnostics()
        .iter()
        .find(|d| d.code.as_deref() == Some("NX-TYPE-001"));
    assert!(diag.is_some(), "expected NX-TYPE-001 diagnostic");
    let msg = &diag.unwrap().message;
    assert!(msg.contains("expected INT32, found STRING"));
}

#[test]
fn test_error_const_reassignment() {
    let src = r#"
const MAX:<INT32> = 100;
MAX = 200;
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    let diag = bag
        .diagnostics()
        .iter()
        .find(|d| d.code.as_deref() == Some("NX-CONST-001"));
    assert!(diag.is_some(), "expected NX-CONST-001");
}

#[test]
fn test_error_undeclared_identifier() {
    let src = "vout(unknown_var);\n";
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    let diag = bag
        .diagnostics()
        .iter()
        .find(|d| d.code.as_deref() == Some("NX-RESOLVE-001"));
    assert!(diag.is_some(), "expected NX-RESOLVE-001");
}

#[test]
fn test_error_duplicate_declaration_same_scope() {
    let src = r#"
var x:<INT32> = 1;
var x:<INT32> = 2;
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    let diag = bag
        .diagnostics()
        .iter()
        .find(|d| d.code.as_deref() == Some("NX-SCOPE-001"));
    assert!(diag.is_some(), "expected NX-SCOPE-001");
}

#[test]
fn test_error_scope_leakage_from_block() {
    let src = r#"
fn test_leak():<VOID> {
    var total:<INT32> = 100;
    if total > 50 {
        var bonus:<INT32> = 20;
        total = total + bonus;
    }
    vout(bonus);
}
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    let diag = bag
        .diagnostics()
        .iter()
        .find(|d| d.code.as_deref() == Some("NX-RESOLVE-001"));
    assert!(diag.is_some(), "bonus must not be visible outside if block");
}

#[test]
fn test_error_loop_variable_scope_leakage() {
    let src = r#"
fr<tem k:<INT32> = 0: k < 10: k++> {
    vout(k);
}
vout(k);
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    let diag = bag
        .diagnostics()
        .iter()
        .find(|d| d.code.as_deref() == Some("NX-RESOLVE-001"));
    assert!(
        diag.is_some(),
        "loop variable k must not leak outside fr header/body"
    );
}

#[test]
fn test_error_non_boolean_conditions() {
    let cases = [
        "var x:<INT32> = 10;\nif x { vout(x); }\n",
        "var s:<STRING> = \"hi\";\nAs s { vout(s); }\n",
        "var n:<INT32> = 5;\nDo { n--; } As n;\n",
    ];

    for case in cases {
        let mut bag = DiagnosticBag::new();
        let _ = analyze_source(case, &mut bag);
        assert!(bag.has_errors());
        let diag = bag
            .diagnostics()
            .iter()
            .find(|d| d.code.as_deref() == Some("NX-TYPE-002"));
        assert!(
            diag.is_some(),
            "condition must be of type BOOL in case: {}",
            case
        );
    }
}

#[test]
fn test_error_logical_operators_with_non_boolean() {
    let src = "var x:<INT32> = 1;\nvar y:<INT32> = 2;\nvar z:<BOOL> = x | y;\n";
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    let diag = bag
        .diagnostics()
        .iter()
        .find(|d| d.code.as_deref() == Some("NX-TYPE-002"));
    assert!(
        diag.is_some(),
        "logical operator | must require BOOL operands"
    );
}

#[test]
fn test_error_function_argument_count_mismatch() {
    let src = r#"
fn add(a:<INT32>, b:<INT32>):<INT32> {
    return a + b;
}
var res:<INT32> = add(1);
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    let diag = bag
        .diagnostics()
        .iter()
        .find(|d| d.code.as_deref() == Some("NX-FUNC-001"));
    assert!(
        diag.is_some(),
        "expected NX-FUNC-001 for argument count mismatch"
    );
}

#[test]
fn test_error_function_argument_type_mismatch() {
    let src = r#"
fn greet(name:<STRING>):<VOID> {
    vout(name);
}
greet(12345);
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    let diag = bag
        .diagnostics()
        .iter()
        .find(|d| d.code.as_deref() == Some("NX-FUNC-002"));
    assert!(
        diag.is_some(),
        "expected NX-FUNC-002 for argument type mismatch"
    );
}

#[test]
fn test_error_return_type_mismatch() {
    let src = r#"
fn get_number():<INT32> {
    return "not a number";
}
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    let diag = bag
        .diagnostics()
        .iter()
        .find(|d| d.code.as_deref() == Some("NX-FUNC-003"));
    assert!(
        diag.is_some(),
        "expected NX-FUNC-003 for return type mismatch"
    );
}

#[test]
fn test_error_return_outside_function() {
    let src = "return 42;\n";
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    let diag = bag
        .diagnostics()
        .iter()
        .find(|d| d.code.as_deref() == Some("NX-FUNC-004"));
    assert!(diag.is_some(), "return outside function must be rejected");
}

#[test]
fn test_error_missing_return_statement() {
    let src = r#"
fn calculate(x:<INT32>):<INT32> {
    vout(x);
}
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    let diag = bag
        .diagnostics()
        .iter()
        .find(|d| d.code.as_deref() == Some("NX-FUNC-005"));
    assert!(
        diag.is_some(),
        "missing return path must be reported for non-VOID function"
    );
}

#[test]
fn test_error_duplicate_parameter_name() {
    let src = r#"
fn compute(a:<INT32>, a:<INT32>):<INT32> {
    return a;
}
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    let diag = bag
        .diagnostics()
        .iter()
        .find(|d| d.code.as_deref() == Some("NX-FUNC-006"));
    assert!(diag.is_some(), "duplicate parameter names must be rejected");
}

#[test]
fn test_error_out_of_range_integer_literal() {
    let src = "var small:<INT8> = 300;\n";
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    let diag = bag
        .diagnostics()
        .iter()
        .find(|d| d.code.as_deref() == Some("NX-TYPE-003"));
    assert!(
        diag.is_some(),
        "integer 300 out of range for INT8 must produce NX-TYPE-003"
    );
}

#[test]
fn test_error_invalid_collection_element_type() {
    let src = "var numbers:<LIST<INT32>> = [1, \"two\", 3];\n";
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    let diag = bag
        .diagnostics()
        .iter()
        .find(|d| d.code.as_deref() == Some("NX-TYPE-005"));
    assert!(
        diag.is_some(),
        "string inside LIST<INT32> must produce NX-TYPE-005"
    );
}

#[test]
fn test_error_static_division_by_zero() {
    let src = "var x:<INT32> = 10 / 0;\n";
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    let diag = bag
        .diagnostics()
        .iter()
        .find(|d| d.code.as_deref() == Some("NX-SAFETY-001"));
    assert!(
        diag.is_some(),
        "division by zero must produce NX-SAFETY-001"
    );
}

#[test]
fn test_error_static_out_of_bounds_indexing() {
    let src = r#"
var nums:<LIST<INT32>> = [10, 20, 30];
var bad:<INT32> = [10, 20, 30][5];
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    let diag = bag
        .diagnostics()
        .iter()
        .find(|d| d.code.as_deref() == Some("NX-SAFETY-002"));
    assert!(
        diag.is_some(),
        "index 5 on list of length 3 must produce NX-SAFETY-002"
    );
}

// =========================================================================
// Phase 3: OOP System Tests
// =========================================================================

#[test]
fn test_valid_basic_class_and_methods() {
    let src = r#"
class Counter {
    private var count:<INT32> = 0;

    public fn init(start:<INT32>):<VOID> {
        self.count = start;
        return;
    }

    public fn increment():<VOID> {
        self.count = self.count + 1;
        return;
    }

    public fn get_count():<INT32> {
        return self.count;
    }
}

var c:<Counter> = Counter(10);
c.increment();
var val:<INT32> = c.get_count();
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(!bag.has_errors(), "diagnostics: {:?}", bag.diagnostics());
}

#[test]
fn test_valid_inheritance_and_polymorphism() {
    let src = r#"
class Animal {
    protected var name:<STRING> = "animal";

    public fn init(n:<STRING>):<VOID> {
        self.name = n;
        return;
    }

    public fn speak():<STRING> {
        return self.name;
    }
}

class Dog extends Animal {
    public fn init(n:<STRING>):<VOID> {
        self.name = n;
        return;
    }

    public fn speak():<STRING> {
        return "Woof: " + self.name;
    }
}

var pet:<Animal> = Dog("Rex");
var sound:<STRING> = pet.speak();
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(!bag.has_errors(), "diagnostics: {:?}", bag.diagnostics());
}

#[test]
fn test_valid_interface_implementation() {
    let src = r#"
interface Printable {
    fn print_info():<STRING>;
}

class Document implements Printable {
    private var title:<STRING> = "Report";

    public fn print_info():<STRING> {
        return self.title;
    }
}

var p:<Printable> = Document();
var info:<STRING> = p.print_info();
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(!bag.has_errors(), "diagnostics: {:?}", bag.diagnostics());
}

#[test]
fn test_valid_abstract_class_hierarchy() {
    let src = r#"
abstract class Shape {
    public abstract fn area():<FL64>;
}

class Circle extends Shape {
    private var radius:<FL64> = 1.0;

    public fn init(r:<FL64>):<VOID> {
        self.radius = r;
        return;
    }

    public fn area():<FL64> {
        return 3.14159 * self.radius * self.radius;
    }
}

var s:<Shape> = Circle(2.5);
var a:<FL64> = s.area();
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(!bag.has_errors(), "diagnostics: {:?}", bag.diagnostics());
}

#[test]
fn test_valid_static_members() {
    let src = r#"
class MathUtil {
    public static var pi:<FL64> = 3.14159;

    public static fn square(x:<INT32>):<INT32> {
        return x * x;
    }
}

var sq:<INT32> = MathUtil.square(5);
var p:<FL64> = MathUtil.pi;
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(!bag.has_errors(), "diagnostics: {:?}", bag.diagnostics());
}

#[test]
fn test_error_nx_oop_001_unknown_parent() {
    let src = "class Sub extends NonExistentParent {}\n";
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    assert!(bag
        .diagnostics()
        .iter()
        .any(|d| d.code.as_deref() == Some("NX-OOP-001")));
}

#[test]
fn test_error_nx_oop_002_circular_inheritance() {
    let src = r#"
class A extends B {}
class B extends A {}
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    assert!(bag
        .diagnostics()
        .iter()
        .any(|d| d.code.as_deref() == Some("NX-OOP-002")));
}

#[test]
fn test_error_nx_oop_003_abstract_instantiation() {
    let src = r#"
abstract class Shape {
    public abstract fn area():<FL64>;
}
var s:<Shape> = Shape();
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    assert!(bag
        .diagnostics()
        .iter()
        .any(|d| d.code.as_deref() == Some("NX-OOP-003")));
}

#[test]
fn test_error_nx_oop_004_interface_instantiation() {
    let src = r#"
interface Printable {
    fn print_info():<STRING>;
}
var p:<Printable> = Printable();
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    assert!(bag
        .diagnostics()
        .iter()
        .any(|d| d.code.as_deref() == Some("NX-OOP-004")));
}

#[test]
fn test_error_nx_oop_005_private_access() {
    let src = r#"
class SecretVault {
    private var code:<INT32> = 1234;
}
var vault:<SecretVault> = SecretVault();
var x:<INT32> = vault.code;
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    assert!(bag
        .diagnostics()
        .iter()
        .any(|d| d.code.as_deref() == Some("NX-OOP-005")));
}

#[test]
fn test_error_nx_oop_006_protected_access() {
    let src = r#"
class Base {
    protected var secret:<INT32> = 42;
}
var b:<Base> = Base();
var x:<INT32> = b.secret;
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    assert!(bag
        .diagnostics()
        .iter()
        .any(|d| d.code.as_deref() == Some("NX-OOP-006")));
}

#[test]
fn test_error_nx_oop_007_extend_final_class() {
    let src = r#"
final class Locked {}
class Sub extends Locked {}
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    assert!(bag
        .diagnostics()
        .iter()
        .any(|d| d.code.as_deref() == Some("NX-OOP-007")));
}

#[test]
fn test_error_nx_oop_008_override_final_method() {
    let src = r#"
class Base {
    public final fn frozen():<VOID> {
        return;
    }
}
class Sub extends Base {
    public fn frozen():<VOID> {
        return;
    }
}
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    assert!(bag
        .diagnostics()
        .iter()
        .any(|d| d.code.as_deref() == Some("NX-OOP-008")));
}

#[test]
fn test_error_nx_oop_009_incompatible_override() {
    let src = r#"
class Base {
    public fn compute(x:<INT32>):<INT32> {
        return x;
    }
}
class Sub extends Base {
    public fn compute(x:<STRING>):<INT32> {
        return 0;
    }
}
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    assert!(bag
        .diagnostics()
        .iter()
        .any(|d| d.code.as_deref() == Some("NX-OOP-009")));
}

#[test]
fn test_error_nx_oop_010_unimplemented_abstract_method() {
    let src = r#"
abstract class Base {
    public abstract fn calculate():<INT32>;
}
class ConcreteSub extends Base {}
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    assert!(bag
        .diagnostics()
        .iter()
        .any(|d| d.code.as_deref() == Some("NX-OOP-010")));
}

#[test]
fn test_error_nx_oop_011_unimplemented_interface_method() {
    let src = r#"
interface Worker {
    fn work():<VOID>;
}
class Robot implements Worker {}
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    assert!(bag
        .diagnostics()
        .iter()
        .any(|d| d.code.as_deref() == Some("NX-OOP-011")));
}

#[test]
fn test_error_nx_oop_012_unknown_member() {
    let src = r#"
class Person {
    public var name:<STRING> = "Alex";
}
var p:<Person> = Person();
var x:<INT32> = p.unknown_field;
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    assert!(bag
        .diagnostics()
        .iter()
        .any(|d| d.code.as_deref() == Some("NX-OOP-012")));
}

#[test]
fn test_error_nx_oop_013_instance_static_mismatch() {
    let src = r#"
class Helper {
    public fn instance_method():<VOID> {
        return;
    }
}
Helper.instance_method();
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    assert!(bag
        .diagnostics()
        .iter()
        .any(|d| d.code.as_deref() == Some("NX-OOP-013")));
}

#[test]
fn test_error_nx_oop_014_constructor_argument_mismatch() {
    let src = r#"
class Point {
    public fn init(x:<INT32>, y:<INT32>):<VOID> {
        return;
    }
}
var p:<Point> = Point(10);
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    assert!(bag
        .diagnostics()
        .iter()
        .any(|d| d.code.as_deref() == Some("NX-OOP-014")));
}

#[test]
fn test_error_nx_oop_015_duplicate_type() {
    let src = r#"
class Widget {}
class Widget {}
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    assert!(bag
        .diagnostics()
        .iter()
        .any(|d| d.code.as_deref() == Some("NX-OOP-015")));
}

#[test]
fn test_error_nx_oop_016_duplicate_member() {
    let src = r#"
class Account {
    public var balance:<INT32> = 0;
    public var balance:<INT32> = 100;
}
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    assert!(bag
        .diagnostics()
        .iter()
        .any(|d| d.code.as_deref() == Some("NX-OOP-016")));
}

#[test]
fn test_error_nx_oop_017_abstract_method_in_non_abstract_class() {
    let src = r#"
class Concrete {
    public abstract fn must_fail():<VOID>;
}
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    assert!(bag
        .diagnostics()
        .iter()
        .any(|d| d.code.as_deref() == Some("NX-OOP-017")));
}

#[test]
fn test_valid_super_method_call() {
    let src = r#"
class Base {
    public fn greet():<STRING> {
        return "Hello";
    }
}

class Derived extends Base {
    public fn greet():<STRING> {
        return super.greet() + " World";
    }
}

var d:<Derived> = Derived();
var g:<STRING> = d.greet();
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(!bag.has_errors(), "diagnostics: {:?}", bag.diagnostics());
}

#[test]
fn test_error_nx_own_001_use_after_move() {
    let src = r#"
var s1:<STRING> = "Hello";
var s2:<STRING> = s1;
var s3:<STRING> = s1;
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    assert!(bag
        .diagnostics()
        .iter()
        .any(|d| d.code.as_deref() == Some("NX-OWN-001")));
}

#[test]
fn test_error_nx_own_002_use_after_free() {
    let src = r#"
var s:<STRING> = alloc("Hello");
free(s);
var s2:<STRING> = s;
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    assert!(bag
        .diagnostics()
        .iter()
        .any(|d| d.code.as_deref() == Some("NX-OWN-002")));
}

#[test]
fn test_error_nx_own_002_double_free() {
    let src = r#"
var s:<STRING> = alloc("Hello");
free(s);
free(s);
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    assert!(bag
        .diagnostics()
        .iter()
        .any(|d| d.code.as_deref() == Some("NX-OWN-002")));
}

#[test]
fn test_error_nx_own_003_tem_escape() {
    let src = r#"
fn test_tem():<INT32> {
    tem x:<INT32> = 42;
    return x;
}
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    assert!(bag
        .diagnostics()
        .iter()
        .any(|d| d.code.as_deref() == Some("NX-OWN-003")));
}

#[test]
fn test_error_nx_borrow_001_mutate_while_borrowed() {
    let src = r#"
var x:<STRING> = "Hello";
var r:<&STRING> = &x;
x = "World";
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    assert!(bag
        .diagnostics()
        .iter()
        .any(|d| d.code.as_deref() == Some("NX-BORROW-001")));
}

#[test]
fn test_error_nx_borrow_002_move_while_borrowed() {
    let src = r#"
var x:<STRING> = "Hello";
var r:<&STRING> = &x;
var y:<STRING> = x;
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    assert!(bag
        .diagnostics()
        .iter()
        .any(|d| d.code.as_deref() == Some("NX-BORROW-002")));
}

#[test]
fn test_error_nx_borrow_003_multiple_mutable_borrows() {
    let src = r#"
var x:<STRING> = "Hello";
var m1:<&mut STRING> = &mut x;
var m2:<&mut STRING> = &mut x;
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    assert!(bag
        .diagnostics()
        .iter()
        .any(|d| d.code.as_deref() == Some("NX-BORROW-003")));
}

#[test]
fn test_error_nx_borrow_003_immutable_as_mutable() {
    let src = r#"
const x:<STRING> = "Hello";
var m:<&mut STRING> = &mut x;
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    assert!(bag
        .diagnostics()
        .iter()
        .any(|d| d.code.as_deref() == Some("NX-BORROW-003")));
}

#[test]
fn test_error_nx_match_001_type_mismatch() {
    let src = r#"
var x:<STRING> = "hello";
match x {
    case 123 {
        var y:<INT32> = 1;
    }
}
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    assert!(bag
        .diagnostics()
        .iter()
        .any(|d| d.code.as_deref() == Some("NX-MATCH-001")));
}

#[test]
fn test_error_nx_match_002_non_boolean_guard() {
    let src = r#"
var x:<INT32> = 5;
match x {
    case y if y + 1 {
        var z:<INT32> = y;
    }
}
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    assert!(bag
        .diagnostics()
        .iter()
        .any(|d| d.code.as_deref() == Some("NX-MATCH-002")));
}

#[test]
fn test_error_nx_match_003_duplicate_case() {
    let src = r#"
var x:<INT32> = 5;
match x {
    case 1 { var a:<INT32> = 1; }
    case 1 { var b:<INT32> = 2; }
}
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    assert!(bag
        .diagnostics()
        .iter()
        .any(|d| d.code.as_deref() == Some("NX-MATCH-003")));
}

#[test]
fn test_error_nx_match_004_wildcard_not_last() {
    let src = r#"
var x:<INT32> = 5;
match x {
    case _ { var a:<INT32> = 0; }
    case 1 { var b:<INT32> = 1; }
}
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    assert!(bag
        .diagnostics()
        .iter()
        .any(|d| d.code.as_deref() == Some("NX-MATCH-004")));
}

#[test]
fn test_valid_match_and_borrowing() {
    let src = r#"
var x:<INT32> = 10;
match x {
    case 1 | 2 {
        var a:<INT32> = 1;
    }
    case v if v > 5 {
        var b:<INT32> = v;
    }
    case _ {
        var c:<INT32> = 0;
    }
}
var s:<STRING> = "test";
var r:<&STRING> = &s;
var allocated:<STRING> = alloc("heap string");
free(allocated);
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(!bag.has_errors(), "diagnostics: {:?}", bag.diagnostics());
}

#[test]
fn test_phase8_types_and_bitwise() {
    let src = r#"
var u:<USIZE> = 100;
var i:<ISIZE> = -50;
var b8:<UINT8> = 255;
var a:<INT32> = 5 ^ 3;
var b:<INT32> = 1 << 4;
var c:<INT32> = 16 >> 2;
var d:<INT32> = ~5;
var e:<UINT32> = 1 | 4;
var f:<UINT32> = 7 & 3;
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(!bag.has_errors(), "diagnostics: {:?}", bag.diagnostics());
}

#[test]
fn test_phase8_struct_and_unsafe_ffi() {
    let src = r#"
struct Point {
    x: <FL64>,
    y: <FL64>
}
var p: <Point> = Point { x: 10.0, y: 20.0 };
var px: <FL64> = p.x;

extern "C" {
    fn puts(s: <STRING>): <INT32>;
}

unsafe {
    puts("hello native");
}
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(!bag.has_errors(), "diagnostics: {:?}", bag.diagnostics());
}

#[test]
fn test_phase8_error_extern_outside_unsafe() {
    let src = r#"
extern "C" {
    fn abs(n: <INT32>): <INT32>;
}
var res: <INT32> = abs(-42);
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    assert!(bag
        .diagnostics()
        .iter()
        .any(|d| d.code.as_deref() == Some("NX-SYS-001")));
}

#[test]
fn test_phase10_valid_async_await() {
    let src = r#"
async fn compute_value(x: <INT32>): <INT32> {
    return x * 2;
}

async fn run(): <INT32> {
    var fut: <FUTURE<INT32>> = compute_value(21);
    var res: <INT32> = await fut;
    return res;
}
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(!bag.has_errors(), "diagnostics: {:?}", bag.diagnostics());
}

#[test]
fn test_phase10_error_cannot_await_non_future() {
    let src = r#"
async fn run(): <VOID> {
    var n: <INT32> = 42;
    var x: <INT32> = await n;
}
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    assert!(bag
        .diagnostics()
        .iter()
        .any(|d| d.message.contains("cannot await non-future type 'INT32'")));
}

#[test]
fn test_phase10_error_await_with_active_mutable_borrow() {
    let src = r#"
async fn get_num(): <INT32> {
    return 10;
}

async fn run(): <VOID> {
    var val: <INT32> = 5;
    var b_mut: <&mut INT32> = &mut val;
    var res: <INT32> = await get_num();
}
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    assert!(bag
        .diagnostics()
        .iter()
        .any(|d| d.code.as_deref() == Some("NX-OWN-004")));
}

#[test]
fn test_modern_syntax_and_inference_valid() {
    let src = r#"
var age: Int32 = 25;
const PI: Float64 = 3.14159;
var x = 10;
var s = "hello";

fn add(a: Int32, b: Int32) -> Int32 {
    return a + b;
}

fn greet(name: String) -> Void {
    vout("Hello", name);
}

class User {
    var name: String;
    var age: Int32;

    fn init(name: String, age: Int32) -> Void {
        self.name = name;
        self.age = age;
    }

    fn introduce() -> Void {
        vout("Hello, I am", self.name);
    }
}

var user = User("Alex", 25);
user.introduce();
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(!bag.has_errors(), "diagnostics: {:?}", bag.diagnostics());
}

#[test]
fn test_modern_syntax_inference_type_mismatch_rejected() {
    let src = r#"
var x = 10;
x = "not an int";
"#;
    let mut bag = DiagnosticBag::new();
    let _ = analyze_source(src, &mut bag);
    assert!(bag.has_errors());
    assert!(bag
        .diagnostics()
        .iter()
        .any(|d| d.code.as_deref() == Some("NX-TYPE-001")));
}
