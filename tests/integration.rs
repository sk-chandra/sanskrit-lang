//! End-to-end tests for Sūtra v0.2: native data, higher-order functions,
//! ergonomic sugar, bilingual aliases, and pure effect-as-data I/O.

use sutra::check::{self, Severity};
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
fn tuples() {
    assert_eq!(eval("(1, 2)"), "(1, 2)");
    assert_eq!(eval("(1, \"two\", सत्य)"), "(1, \"two\", सत्य)");
    assert_eq!(eval("प्रथम((10, 20))"), "10");
    assert_eq!(eval("द्वितीय((10, 20))"), "20");
    // Tuples pattern-match like any constructor.
    let prog = parser::parse_program("सूत्र अदल((?a, ?b)) -> (?b, ?a)।").unwrap();
    let mut full = load_prelude().unwrap();
    full.extend(prog);
    assert_eq!(eval_in(&full, "अदल((1, 2))"), "(2, 1)");
    // A pair inhabits युग्मक.
    let p = load_prelude().unwrap();
    assert!(samjna::inhabits(&p, &nf(&p, "(1, 2)"), "युग्मक"));
}

#[test]
fn do_notation() {
    // do { ?x <- शुद्ध(10); ?y <- शुद्ध(5); शुद्ध(?x * ?y) } ⇒ 50, no I/O.
    let prog = load_prelude().unwrap();
    let engine = Engine::new(&prog, DEFAULT_FUEL);
    let runner = Runner::new(&engine, true, vec![]);
    let action =
        parser::parse_expr("क्रिया { ?x <- शुद्ध(10); ?y <- शुद्ध(5); शुद्ध(?x * ?y) }").unwrap();
    assert_eq!(pretty::show(&runner.run(action), true), "50");
}

#[test]
fn pattern_guards() {
    // Guarded base case declared last (so paratva tries it first); when its
    // guard fails it falls through to the recursive clause.
    let src = "सूत्र फिबो(?n) -> फिबो(?n - 1) + फिबो(?n - 2)।\n\
               सूत्र फिबो(?n) | ?n < 2 -> ?n।";
    let mut prog = load_prelude().unwrap();
    prog.extend(parser::parse_program(src).unwrap());
    assert_eq!(eval_in(&prog, "फिबो(10)"), "55");

    let src2 = "सूत्र चिह्न(?n) -> शून्यम्।\n\
                सूत्र चिह्न(?n) | ?n > 0 -> धनम्।\n\
                सूत्र चिह्न(?n) | ?n < 0 -> ऋणम्।";
    let mut prog2 = load_prelude().unwrap();
    prog2.extend(parser::parse_program(src2).unwrap());
    assert_eq!(eval_in(&prog2, "चिह्न(5)"), "धनम्");
    assert_eq!(eval_in(&prog2, "चिह्न(-2)"), "ऋणम्");
    assert_eq!(eval_in(&prog2, "चिह्न(0)"), "शून्यम्");
}

#[test]
fn sequence_rewriting() {
    let src = "क्रम संधि { [अ, इ] -> [ए]। [अ, उ] -> [ओ]। }\n\
               क्रम संक्षेप { [?x, ?x] -> [?x]। }";
    let mut prog = load_prelude().unwrap();
    prog.extend(parser::parse_program(src).unwrap());
    // Combines a vowel junction anywhere in the sequence.
    assert_eq!(eval_in(&prog, "संधि([क, अ, इ, त])"), "[क, ए, त]");
    // Variable patterns: collapse runs of equal elements.
    assert_eq!(eval_in(&prog, "संक्षेप([1, 1, 2, 2, 3])"), "[1, 2, 3]");
    // A non-list argument leaves the application stuck (honest failure).
    assert_eq!(eval_in(&prog, "संधि(5)"), "संधि(5)");
}

#[test]
fn element_classes() {
    let src = "गण अवर्ण := [अ, आ]।\n\
               गण इवर्ण := [इ, ई]।\n\
               क्रम संधि { [अवर्ण, इवर्ण] -> [ए]। }\n\
               गण स्वर := [अ, आ, इ, ई, उ, ऊ]।\n\
               क्रम लोप { [स्वर, ?v:स्वर] -> [?v]। }";
    let mut prog = load_prelude().unwrap();
    prog.extend(parser::parse_program(src).unwrap());
    // A class matches any member (आ ∈ अवर्ण, ई ∈ इवर्ण).
    assert_eq!(eval_in(&prog, "संधि([क, आ, ई, त])"), "[क, ए, त]");
    // Bare class as context + bound class member reproduced in the output.
    assert_eq!(eval_in(&prog, "लोप([क, अ, इ, त])"), "[क, इ, त]");
    assert_eq!(eval_in(&prog, "लोप([आ, उ])"), "[उ]");
}

#[test]
fn seq_anchors_and_segments() {
    let src = "गण स्वर := [अ, आ, इ]।\n\
               क्रम आदि { [^, अ] -> [आ]। }\n\
               क्रम विसर्ग { [स, $] -> [ः]। }\n\
               क्रम विभाग { [?a*, मध्य, ?b*] -> [रचना(?a, ?b)]। }\n\
               क्रम अग्रलोप { [^, ?v:स्वर*, ?c] -> [?c]। }\n\
               क्रम युगल { [^, ?x*, सम, ?x*, $] -> [?x*]। }";
    let mut prog = load_prelude().unwrap();
    prog.extend(parser::parse_program(src).unwrap());
    // ^ fires only at the start.
    assert_eq!(eval_in(&prog, "आदि([अ, क, अ])"), "[आ, क, अ]");
    // $ fires only at the end.
    assert_eq!(eval_in(&prog, "विसर्ग([र, म, स])"), "[र, म, ः]");
    assert_eq!(eval_in(&prog, "विसर्ग([स, थ])"), "[स, थ]");
    // ?v* captures segments (greedy); plain ?v in the RHS gets the list value.
    assert_eq!(eval_in(&prog, "विभाग([1, 2, मध्य, 3])"), "[([1, 2], [3])]");
    // Class-constrained segment: a run of vowels.
    assert_eq!(eval_in(&prog, "अग्रलोप([अ, इ, आ, क])"), "[क]");
    // Non-linear segments with anchors: the WHOLE word must be x-सम-x.
    assert_eq!(eval_in(&prog, "युगल([प, द, सम, प, द])"), "[प, द]");
    assert_eq!(eval_in(&prog, "युगल([प, सम, द])"), "[प, सम, द]");
}

#[test]
fn shivasutra_pratyahara() {
    // The stdlib ships Pāṇini's śivasūtras: iko yaṇ aci (6.1.77) end-to-end.
    let src = "सूत्र यणादेश(इ) -> य। सूत्र यणादेश(उ) -> व।\n\
               क्रम संधि { [?i:इक्, ?a:अच्] -> [यणादेश(?i), ?a]। }";
    let mut prog = load_prelude().unwrap();
    prog.extend(parser::parse_program(src).unwrap());
    assert_eq!(eval_in(&prog, "संधि([द, ध, इ, अ, त, र])"), "[द, ध, य, अ, त, र]");
    // ऐ is a vowel (अच्) but not इक्; हल् is the consonant span.
    let cls = "क्रम क1 { [?v:अच्] -> [स्वरः]। }\nक्रम क2 { [?v:हल्] -> [व्य]। }";
    let mut p2 = load_prelude().unwrap();
    p2.extend(parser::parse_program(cls).unwrap());
    assert_eq!(eval_in(&p2, "क1([ऐ])"), "[स्वरः]");
    assert_eq!(eval_in(&p2, "क2([ख])"), "[व्य]");
    assert_eq!(eval_in(&p2, "क2([औ])"), "[औ]"); // a vowel is not in हल्

    // A user-defined inventory and span.
    let own = "शिवसूत्र { [क, ख] -> म्। [ग] -> न्। }\n\
               गण कन् := प्रत्याहार(ख, न्)।\n\
               क्रम प { [?v:कन्] -> [हित]। }";
    let p3 = {
        let mut p = load_prelude().unwrap();
        p.extend(parser::parse_program(own).unwrap());
        p
    };
    assert_eq!(eval_in(&p3, "प([ग])"), "[हित]");
    assert_eq!(eval_in(&p3, "प([क])"), "[क]"); // क is before the span's start
}

#[test]
fn underivable_pratyahara_is_reported() {
    let src = "गण भ्रम := प्रत्याहार(क, ज़्)।";
    let target = parser::parse_program(src).unwrap();
    let mut ctx = load_prelude().unwrap();
    ctx.extend(target.clone());
    let diags = check::check(&ctx, &target);
    assert!(diags
        .iter()
        .any(|d| d.severity == Severity::Error && d.msg.contains("भ्रम")));
}

#[test]
fn static_checker() {
    let src = "संज्ञा रंग := लाल | हरित | नील।\n\
               सूत्र दुगुना(?x) -> ?x + ?y।\n\
               सूत्र नाम(लाल) -> \"r\"।\n\
               सूत्र नाम(हरित) -> \"g\"।";
    let target = parser::parse_program(src).unwrap();
    let mut ctx = load_prelude().unwrap();
    ctx.extend(target.clone());
    let diags = check::check(&ctx, &target);

    assert!(diags
        .iter()
        .any(|d| d.severity == Severity::Error && d.msg.contains("?y")));
    assert!(diags
        .iter()
        .any(|d| d.msg.contains("non-exhaustive") && d.msg.contains("नील")));

    // A correct program is clean.
    let good = parser::parse_program("सूत्र वर्ग2(?x) -> ?x * ?x।").unwrap();
    let mut ctx2 = load_prelude().unwrap();
    ctx2.extend(good.clone());
    assert!(check::check(&ctx2, &good).is_empty());
}

#[test]
fn formatter() {
    let messy = "सूत्र   वर्ग2(?क)->?क*?क।    # comment\n\
                 fn dbl( ?x ) ->?x   * 2 ;\n\
                 गण स्वर:=[अ ,आ]।\n\
                 क्रम लोप{\n[स्वर,?v:स्वर]->[?v]।\n}\n\
                 प्रयोग {नाम:\"क\"}.नाम।\n";
    let formatted = sutra::fmt::format_source(messy).unwrap();
    // Canonical spacing; spelling (fn, ;) and comments preserved.
    assert!(formatted.contains("सूत्र वर्ग2(?क) -> ?क * ?क।  # comment"));
    assert!(formatted.contains("fn dbl(?x) -> ?x * 2;"));
    assert!(formatted.contains("गण स्वर := [अ, आ]।"));
    assert!(formatted.contains("  [स्वर, ?v:स्वर] -> [?v]।")); // block indented, ?v:गण tight
    assert!(formatted.contains("प्रयोग {नाम: \"क\"}.नाम।"));
    // Idempotent.
    assert_eq!(sutra::fmt::format_source(&formatted).unwrap(), formatted);
    // Token stream unchanged (the formatter's own invariant, double-checked).
    let toks = |s: &str| -> Vec<_> {
        sutra::lexer::lex(s).unwrap().into_iter().map(|t| t.tok).collect()
    };
    assert_eq!(toks(messy), toks(&formatted));
    // Multi-line declaration bodies get a continuation indent.
    let multi = "सूत्र मुख्य ->\nमुद्रण(\"a\") >>\nमुद्रण(\"b\")।\n";
    let f2 = sutra::fmt::format_source(multi).unwrap();
    assert!(f2.contains("\n  मुद्रण(\"a\") >>"));
}

#[test]
fn parse_int_builtin() {
    assert_eq!(eval("पूर्णांक(\"42\") + 8"), "50");
    assert!(eval("पूर्णांक(\"नहीं\")").starts_with("दोष("));
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
