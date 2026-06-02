//! End-to-end tests: parse an expression, evaluate it against the standard
//! library, and check the pretty-printed normal form.

use sutra::engine::Engine;
use sutra::{load_prelude, parser, pretty, samjna, Program};

/// Evaluate `expr` against the prelude and return its normal form (ASCII).
fn eval(expr: &str) -> String {
    let prog = load_prelude().expect("prelude should load");
    eval_in(&prog, expr)
}

fn eval_in(prog: &Program, expr: &str) -> String {
    let term = parser::parse_expr(expr).expect("expression should parse");
    let engine = Engine::new(prog, sutra::engine::DEFAULT_FUEL);
    let outcome = engine.normalize(&term);
    pretty::show(&outcome.term, true)
}

#[test]
fn arithmetic() {
    assert_eq!(eval("योग(२, ३)"), "5");
    assert_eq!(eval("गुणन(४, ५)"), "20");
    assert_eq!(eval("क्रमगुणित(५)"), "120");
    assert_eq!(eval("क्रमगुणित(०)"), "1");
    assert_eq!(eval("पूर्व(०)"), "0");
    assert_eq!(eval("वियोग(७, १०)"), "0"); // monus never goes negative
    assert_eq!(eval("वियोग(१०, ३)"), "7");
}

#[test]
fn comparisons_and_logic() {
    assert_eq!(eval("तुल्य(गुणन(२, ३), ६)"), "सत्य");
    assert_eq!(eval("तुल्य(२, ३)"), "असत्य");
    assert_eq!(eval("न्यूनसम(३, ३)"), "सत्य");
    assert_eq!(eval("न्यूनसम(४, ३)"), "असत्य");
    assert_eq!(eval("न(सत्य)"), "असत्य");
    assert_eq!(eval("च(सत्य, असत्य)"), "असत्य");
    assert_eq!(eval("वा(असत्य, सत्य)"), "सत्य");
}

#[test]
fn yadi_is_lazy() {
    // The discarded branch is never evaluated, so a divergent expression there
    // does not cause non-termination.
    assert_eq!(eval("यदि(सत्य, ७, क्रमगुणित(पुनरावृत्ति))"), "7");
    assert_eq!(eval("यदि(असत्य, क्रमगुणित(पुनरावृत्ति), ९)"), "9");
}

#[test]
fn paratva_makes_later_rules_win() {
    let src = "\
        सूत्र वर्ग(?क्ष) -> अन्य।\n\
        सूत्र वर्ग(अ) -> स्वर।\n\
        सूत्र वर्ग(क) -> व्यञ्जन।\n";
    let prog = parser::parse_program(src).unwrap();
    assert_eq!(eval_in(&prog, "वर्ग(अ)"), "स्वर"); // specific (later) wins
    assert_eq!(eval_in(&prog, "वर्ग(क)"), "व्यञ्जन");
    assert_eq!(eval_in(&prog, "वर्ग(ग)"), "अन्य"); // only the general rule matches
}

#[test]
fn nonlinear_matching() {
    // यमल(x, x) -> समान only when the two arguments are structurally equal,
    // which holds after they reduce to the same normal form.
    assert_eq!(eval("यमल(योग(१, १), २)"), "समान");
    // Unequal arguments: the term is stuck (no rule applies).
    assert_eq!(eval("यमल(१, २)"), "यमल(1, 2)");
}

#[test]
fn lists() {
    assert_eq!(eval("दीर्घ(युग्म(१, युग्म(२, युग्म(३, रिक्त))))"), "3");
    assert_eq!(
        eval("विपर्यय(युग्म(१, युग्म(२, युग्म(३, रिक्त))))"),
        "युग्म(3, युग्म(2, युग्म(1, रिक्त)))"
    );
    assert_eq!(
        eval("योजन(युग्म(१, रिक्त), युग्म(२, रिक्त))"),
        "युग्म(1, युग्म(2, रिक्त))"
    );
    assert_eq!(eval("सदस्य(२, युग्म(१, युग्म(२, रिक्त)))"), "सत्य");
    assert_eq!(eval("सदस्य(९, युग्म(१, युग्म(२, रिक्त)))"), "असत्य");
}

#[test]
fn errors_as_values_and_stuck_terms() {
    // An error is ordinary data.
    assert_eq!(eval("शीर्ष(रिक्त)"), "दोष(\"रिक्ता सूची\")");
    // A term with no applicable rule simply stops reducing.
    assert_eq!(eval("शीर्ष(५)"), "शीर्ष(5)");
}

#[test]
fn sandhi() {
    assert_eq!(eval("संधि(युग्म(अ, युग्म(इ, रिक्त)))"), "युग्म(ए, रिक्त)");
    assert_eq!(eval("संधि(युग्म(अ, युग्म(उ, रिक्त)))"), "युग्म(ओ, रिक्त)");
    assert_eq!(eval("संधि(युग्म(अ, युग्म(अ, रिक्त)))"), "युग्म(आ, रिक्त)");
    assert_eq!(
        eval("संधि(युग्म(क, युग्म(अ, युग्म(इ, रिक्त))))"),
        "युग्म(क, युग्म(ए, रिक्त))"
    );
}

#[test]
fn numeral_sugar_roundtrips() {
    // Devanagari numerals in, Devanagari numerals out (default printing).
    let prog = load_prelude().unwrap();
    let term = parser::parse_expr("योग(१२, ८)").unwrap();
    let engine = Engine::new(&prog, sutra::engine::DEFAULT_FUEL);
    let out = engine.normalize(&term);
    assert_eq!(pretty::show(&out.term, false), "२०"); // Devanagari
    assert_eq!(pretty::show(&out.term, true), "20"); // ASCII
}

#[test]
fn fuel_limit_stops_nontermination() {
    // A rule that rewrites forever should hit the fuel limit rather than hang.
    let prog = parser::parse_program("सूत्र चक्र(?क) -> चक्र(?क)।").unwrap();
    let term = parser::parse_expr("चक्र(०)").unwrap();
    let engine = Engine::new(&prog, 1000);
    let out = engine.normalize(&term);
    assert!(out.out_of_fuel);
    assert_eq!(out.steps, 1000);
}

#[test]
fn samjna_classification() {
    let prog = load_prelude().unwrap();
    let nf = |e: &str| {
        let t = parser::parse_expr(e).unwrap();
        Engine::new(&prog, sutra::engine::DEFAULT_FUEL)
            .normalize(&t)
            .term
    };
    assert!(samjna::inhabits(&prog, &nf("क्रमगुणित(३)"), "संख्या"));
    assert!(!samjna::inhabits(&prog, &nf("सत्य"), "संख्या"));
    assert!(samjna::inhabits(&prog, &nf("न(असत्य)"), "सत्यता"));
    assert!(samjna::inhabits(&prog, &nf("युग्म(१, रिक्त)"), "सूची"));
    assert!(samjna::inhabits(&prog, &nf("शीर्ष(रिक्त)"), "दोष"));
}
