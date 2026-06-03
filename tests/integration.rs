//! End-to-end tests for Sūtra v0.2: native data, higher-order functions,
//! ergonomic sugar, bilingual aliases, and pure effect-as-data I/O.

use sutra::effect::Runner;
use sutra::engine::{Engine, DEFAULT_FUEL};
use sutra::{load_prelude, parser, pretty, samjna, Program, Term};

fn eval(expr: &str) -> String {
    eval_in(&load_prelude().unwrap(), expr)
}

fn eval_in(prog: &Program, expr: &str) -> String {
    let term = parser::parse_expr(expr).expect("should parse");
    let engine = Engine::new(prog, DEFAULT_FUEL);
    pretty::show(&engine.normalize(&term).term, true)
}

fn nf(prog: &Program, expr: &str) -> Term {
    let term = parser::parse_expr(expr).unwrap();
    Engine::new(prog, DEFAULT_FUEL).normalize(&term).term
}

#[test]
fn arithmetic_and_precedence() {
    assert_eq!(eval("2 + 3 * 4"), "14");
    assert_eq!(eval("(2 + 3) * 4"), "20");
    assert_eq!(eval("10 - 3 - 2"), "5"); // left associative
    assert_eq!(eval("7 / 2"), "3"); // integer division
    assert_eq!(eval("7 % 3"), "1");
    assert_eq!(eval("3.0 / 2.0"), "1.5"); // float
    assert_eq!(eval("-5 + 8"), "3");
}

#[test]
fn factorial_is_fast_and_exact() {
    assert_eq!(eval("क्रमगुणित(20)"), "2432902008176640000");
}

#[test]
fn arbitrary_precision_integers() {
    // 100! — exact, far beyond i64.
    assert_eq!(
        eval("क्रमगुणित(100)"),
        "9332621544394415268169923885626670049071596826438162146859296389\
521759999322991560894146397615651828625369792082722375825118521091686400\
0000000000000000000000"
    );
    // Overflow promotes; results that fit demote back to Int.
    assert_eq!(eval("9223372036854775807 + 1"), "9223372036854775808");
    assert_eq!(eval("क्रमगुणित(21) / क्रमगुणित(20)"), "21");
    assert_eq!(eval("क्रमगुणित(30) % 7"), "0");
    // Exact equality where f64 would lie (both round to the same double).
    assert_eq!(eval("क्रमगुणित(25) == क्रमगुणित(25) + 1"), "असत्य");
    assert_eq!(eval("क्रमगुणित(30) > क्रमगुणित(29)"), "सत्य");
    // Huge literals parse.
    assert_eq!(
        eval("1000000000000000000000 * 2"),
        "2000000000000000000000"
    );
}

#[test]
fn comparison_and_logic() {
    assert_eq!(eval("2 < 3 && 5 == 5"), "सत्य");
    assert_eq!(eval("!(1 > 2) || असत्य"), "सत्य");
    assert_eq!(eval("3 != 4"), "सत्य");
    assert_eq!(eval("\"a\" < \"b\""), "सत्य");
}

#[test]
fn yadi_is_lazy() {
    assert_eq!(eval("यदि(सत्य, 7, क्रमगुणित(लूप))"), "7");
    assert_eq!(eval("if 3 > 2 then 100 else क्रमगुणित(लूप)"), "100");
}

#[test]
fn lists_literals_and_operators() {
    assert_eq!(eval("[1, 2, 3]"), "[1, 2, 3]");
    assert_eq!(eval("0 :: [1, 2]"), "[0, 1, 2]");
    assert_eq!(eval("[1, 2] ++ [3, 4]"), "[1, 2, 3, 4]");
    assert_eq!(eval("विपर्यय([1, 2, 3])"), "[3, 2, 1]");
    assert_eq!(eval("दीर्घ([10, 20, 30])"), "3");
    assert_eq!(eval("सदस्य(2, [1, 2, 3])"), "सत्य");
}

#[test]
fn higher_order_functions() {
    assert_eq!(eval("प्रति((?x) => ?x * ?x, [1, 2, 3, 4])"), "[1, 4, 9, 16]");
    assert_eq!(eval("छन्न((?x) => ?x % 2 == 0, श्रेणी(0, 8))"), "[0, 2, 4, 6]");
    assert_eq!(eval("संहार((?a, ?b) => ?a + ?b, 0, [1, 2, 3, 4, 5])"), "15");
    // Named function passed as a value.
    assert_eq!(eval("प्रति(वर्ग, [1, 2, 3])"), "[1, 4, 9]");
    // Partial application of a builtin used as a function.
    assert_eq!(eval("प्रति(योग(10), [1, 2, 3])"), "[11, 12, 13]");
}

#[test]
fn let_lambda_pipe() {
    assert_eq!(eval("let ?x = 21 in ?x * 2"), "42");
    assert_eq!(eval("((?x, ?y) => ?x + ?y)(3, 4)"), "7");
    assert_eq!(eval("[1, 2, 3, 4] |> दीर्घ"), "4");
    assert_eq!(eval("5 |> वर्ग |> द्विगुण"), "50");
}

#[test]
fn strings() {
    assert_eq!(eval("\"नम\" ++ \"स्ते\""), "\"नमस्ते\"");
    assert_eq!(eval("दीर्घ(\"hello\")"), "5");
    assert_eq!(eval("रूप(क्रमगुणित(5))"), "\"120\"");
    assert_eq!(eval("अंश(\"abcdef\", 1, 3)"), "\"bcd\"");
}

#[test]
fn bilingual_latin_aliases() {
    assert_eq!(eval("map((?x) => add(?x, 1), [1, 2, 3])"), "[2, 3, 4]");
    assert_eq!(eval("fold((?a, ?b) => mul(?a, ?b), 1, [1, 2, 3, 4])"), "24");
    assert_eq!(eval("filter((?x) => gt(?x, 2), [1, 2, 3, 4])"), "[3, 4]");
}

#[test]
fn paratva_later_rule_wins() {
    let src = "\
        सूत्र जाति(?क्ष) -> अन्य।\n\
        सूत्र जाति(अ) -> स्वर।\n";
    let prog = parser::parse_program(src).unwrap();
    assert_eq!(eval_in(&prog, "जाति(अ)"), "स्वर");
    assert_eq!(eval_in(&prog, "जाति(ग)"), "अन्य");
}

#[test]
fn nonlinear_matching() {
    assert_eq!(eval("यमल(1 + 0, 1)"), "समान"); // equal after reduction
    assert_eq!(eval("यमल(1, 2)"), "यमल(1, 2)"); // unequal ⇒ stuck
}

#[test]
fn failure_model() {
    assert_eq!(eval("शीर्ष([])"), "दोष(\"रिक्ता सूची\")");
    assert!(eval("5 / 0").starts_with("दोष(")); // error as value
    assert_eq!(eval("शीर्ष(5)"), "शीर्ष(5)"); // stuck term
}

#[test]
fn samjna_classification() {
    let prog = load_prelude().unwrap();
    assert!(samjna::inhabits(&prog, &nf(&prog, "क्रमगुणित(5)"), "संख्या"));
    assert!(samjna::inhabits(&prog, &nf(&prog, "3.14"), "दशांश"));
    assert!(samjna::inhabits(&prog, &nf(&prog, "\"hi\""), "अक्षरमाला"));
    assert!(samjna::inhabits(&prog, &nf(&prog, "[1, 2, 3]"), "सूची"));
    assert!(samjna::inhabits(&prog, &nf(&prog, "न(सत्य)"), "सत्यता"));
    assert!(samjna::inhabits(&prog, &nf(&prog, "शीर्ष([])"), "दोष"));
}

#[test]
fn numeral_printing() {
    let prog = load_prelude().unwrap();
    let t = nf(&prog, "6 * 7");
    assert_eq!(pretty::show(&t, false), "४२"); // Devanagari default
    assert_eq!(pretty::show(&t, true), "42"); // ASCII
}

#[test]
fn fuel_limit_stops_nontermination() {
    let prog = parser::parse_program("सूत्र चक्र(?क) -> चक्र(?क)।").unwrap();
    let term = parser::parse_expr("चक्र(0)").unwrap();
    let out = Engine::new(&prog, 1000).normalize(&term);
    assert!(out.out_of_fuel);
}

#[test]
fn call_by_need_shares_work() {
    // मन्द(n,1) = 2^n by binding a recursive call once and using it twice.
    // Under call-by-name this needs 2^n reductions; sharing makes it linear, so
    // a large n is only reachable at all because work is shared.
    let src = "सूत्र मन्द(?n, ?x) -> यदि(?n == 0, ?x, let ?y = मन्द(?n - 1, ?x) in ?y + ?y)।";
    let prog = parser::parse_program(src).unwrap();
    let mut full = load_prelude().unwrap();
    full.extend(prog);
    assert_eq!(eval_in(&full, "मन्द(40, 1)"), "1099511627776"); // 2^40
}

#[test]
fn maps_and_records() {
    assert_eq!(eval("{a: 1, b: 2, c: 3}"), "{a: 1, b: 2, c: 3}");
    assert_eq!(eval("प्राप्ति({नाम: \"क\"}, \"नाम\")"), "\"क\"");
    assert_eq!(eval("{x: 1, y: 2}.y"), "2"); // dot access
    assert_eq!(eval("समावेश({a: 1}, \"b\", 2)"), "{a: 1, b: 2}");
    assert_eq!(eval("समावेश({a: 1}, \"a\", 9)"), "{a: 9}"); // overwrite
    assert_eq!(eval("अस्ति({a: 1}, \"a\")"), "सत्य");
    assert_eq!(eval("अस्ति({a: 1}, \"z\")"), "असत्य");
    assert_eq!(eval("निष्कास({a: 1, b: 2}, \"a\")"), "{b: 2}");
    assert_eq!(eval("कुञ्जिकाः({x: 1, y: 2})"), "[\"x\", \"y\"]");
    assert_eq!(eval("दीर्घ({a: 1, b: 2, c: 3})"), "3");
    assert_eq!(eval("प्राप्ति({a: 1}, \"z\", 0)"), "0"); // default
    // Maps are value-equal regardless of insertion order.
    assert_eq!(eval("{a: 1, b: 2} == {b: 2, a: 1}"), "सत्य");
    // Bilingual.
    assert_eq!(eval("get(insert(emptymap, \"k\", 42), \"k\")"), "42");
}

#[test]
fn map_is_a_kosha() {
    let prog = load_prelude().unwrap();
    assert!(samjna::inhabits(&prog, &nf(&prog, "{a: 1}"), "कोश"));
}

#[test]
fn effect_as_data_runs_purely() {
    // बन्ध(शुद्ध(10), (?x) => शुद्ध(?x + 5)) executes to the value 15 with no I/O.
    let prog = load_prelude().unwrap();
    let engine = Engine::new(&prog, DEFAULT_FUEL);
    let runner = Runner::new(&engine, true, vec![]);
    let action = parser::parse_expr("बन्ध(शुद्ध(10), (?x) => शुद्ध(?x + 5))").unwrap();
    let result = runner.run(action);
    assert_eq!(pretty::show(&result, true), "15");
}

#[test]
fn world_effects() {
    let prog = load_prelude().unwrap();
    let engine = Engine::new(&prog, DEFAULT_FUEL);
    let runner = Runner::new(&engine, true, vec!["a".into(), "b".into()]);

    // Program arguments.
    let args = parser::parse_expr("प्राचलाः").unwrap();
    assert_eq!(pretty::show(&runner.run(args), true), "[\"a\", \"b\"]");

    // File write → read roundtrip.
    let path = std::env::temp_dir().join("sutra_world_effects_test.txt");
    let p = path.to_string_lossy().replace('\\', "/");
    let src = format!(
        "बन्ध(सञ्चिकालेख(\"{p}\", \"नमस्ते\"), (?z) => सञ्चिकापाठ(\"{p}\"))"
    );
    let action = parser::parse_expr(&src).unwrap();
    assert_eq!(pretty::show(&runner.run(action), true), "\"नमस्ते\"");

    // Randomness in range, and the time effect yields a positive integer.
    assert_eq!(pretty::show(&runner.run(parser::parse_expr("यादृच्छिक(1)").unwrap()), true), "0");
    let t = parser::parse_expr("बन्ध(काल, (?t) => शुद्ध(?t > 0))").unwrap();
    assert_eq!(pretty::show(&runner.run(t), true), "सत्य");
}
