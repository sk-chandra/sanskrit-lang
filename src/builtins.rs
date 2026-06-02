//! Primitive (builtin) reductions over native values: arithmetic, comparison,
//! equality, and string/list operations. Builtins are *strict*: the engine only
//! calls them once their arguments are fully reduced.

use crate::ast::Term;

/// Is `name` the head of a builtin operation?
pub fn is_builtin(name: &str) -> bool {
    matches!(
        name,
        "+" | "-" | "*" | "/" | "%"
            | "neg"
            | "=="
            | "!="
            | "<"
            | "<="
            | ">"
            | ">="
            | "++"
            | "दीर्घ"
            | "रूप"
            | "अंश"
            | "अक्षर"
    )
}

fn boolean(b: bool) -> Term {
    Term::con(if b { "सत्य" } else { "असत्य" })
}

fn dosha(msg: &str) -> Term {
    Term::app("दोष", vec![Term::Str(msg.to_string())])
}

/// Apply a builtin to already-evaluated arguments. Returns `None` if the
/// operation does not apply (wrong arity or types) — the term is then left
/// stuck, surfacing the mismatch honestly.
pub fn apply(name: &str, args: &[Term]) -> Option<Term> {
    match name {
        "+" | "-" | "*" | "/" | "%" => arith(name, args),
        "neg" => negate(args),
        "==" => Some(boolean(structural_eq(args.get(0)?, args.get(1)?))),
        "!=" => Some(boolean(!structural_eq(args.get(0)?, args.get(1)?))),
        "<" | "<=" | ">" | ">=" => order(name, args),
        "++" => concat(args),
        "दीर्घ" => length(args),
        "रूप" => show(args),
        "अंश" => substr(args),
        "अक्षर" => chars(args),
        _ => None,
    }
}

fn as_f64(t: &Term) -> Option<f64> {
    match t {
        Term::Int(n) => Some(*n as f64),
        Term::Float(f) => Some(*f),
        _ => None,
    }
}

fn arith(op: &str, args: &[Term]) -> Option<Term> {
    if args.len() != 2 {
        return None;
    }
    match (&args[0], &args[1]) {
        (Term::Int(a), Term::Int(b)) => {
            let (a, b) = (*a, *b);
            Some(match op {
                "+" => Term::Int(a.wrapping_add(b)),
                "-" => Term::Int(a.wrapping_sub(b)),
                "*" => Term::Int(a.wrapping_mul(b)),
                "/" => {
                    if b == 0 {
                        return Some(dosha("शून्येन भागः (division by zero)"));
                    }
                    Term::Int(a / b)
                }
                "%" => {
                    if b == 0 {
                        return Some(dosha("शून्येन भागः (modulo by zero)"));
                    }
                    Term::Int(a % b)
                }
                _ => return None,
            })
        }
        _ => {
            // Promote to float if either side is numeric and at least one float.
            let a = as_f64(&args[0])?;
            let b = as_f64(&args[1])?;
            Some(match op {
                "+" => Term::Float(a + b),
                "-" => Term::Float(a - b),
                "*" => Term::Float(a * b),
                "/" => Term::Float(a / b),
                "%" => Term::Float(a % b),
                _ => return None,
            })
        }
    }
}

fn negate(args: &[Term]) -> Option<Term> {
    match args {
        [Term::Int(n)] => Some(Term::Int(-n)),
        [Term::Float(f)] => Some(Term::Float(-f)),
        _ => None,
    }
}

fn structural_eq(a: &Term, b: &Term) -> bool {
    // Compare numerically across Int/Float; otherwise structurally.
    if let (Some(x), Some(y)) = (as_f64(a), as_f64(b)) {
        return x == y;
    }
    a == b
}

fn order(op: &str, args: &[Term]) -> Option<Term> {
    if args.len() != 2 {
        return None;
    }
    let ord = match (&args[0], &args[1]) {
        (Term::Str(a), Term::Str(b)) => a.cmp(b),
        _ => {
            let a = as_f64(&args[0])?;
            let b = as_f64(&args[1])?;
            a.partial_cmp(&b)?
        }
    };
    use std::cmp::Ordering::*;
    let b = match op {
        "<" => ord == Less,
        "<=" => ord != Greater,
        ">" => ord == Greater,
        ">=" => ord != Less,
        _ => return None,
    };
    Some(boolean(b))
}

/// A list value: a chain of `युग्म`/`रिक्त`. Returns the elements if so.
fn list_items(mut t: &Term) -> Option<Vec<Term>> {
    let mut out = Vec::new();
    loop {
        match t {
            Term::Sym(n, a) if n == "रिक्त" && a.is_empty() => return Some(out),
            Term::Sym(n, a) if n == "युग्म" && a.len() == 2 => {
                out.push(a[0].clone());
                t = &a[1];
            }
            _ => return None,
        }
    }
}

fn make_list(items: Vec<Term>) -> Term {
    let mut t = Term::con("रिक्त");
    for it in items.into_iter().rev() {
        t = Term::app("युग्म", vec![it, t]);
    }
    t
}

fn concat(args: &[Term]) -> Option<Term> {
    if args.len() != 2 {
        return None;
    }
    match (&args[0], &args[1]) {
        (Term::Str(a), Term::Str(b)) => Some(Term::Str(format!("{}{}", a, b))),
        _ => {
            let mut a = list_items(&args[0])?;
            let b = list_items(&args[1])?;
            a.extend(b);
            Some(make_list(a))
        }
    }
}

fn length(args: &[Term]) -> Option<Term> {
    match args {
        [Term::Str(s)] => Some(Term::Int(s.chars().count() as i64)),
        _ => None,
    }
}

fn show(args: &[Term]) -> Option<Term> {
    match args {
        [t] => Some(Term::Str(crate::pretty::show(t, true))),
        _ => None,
    }
}

fn substr(args: &[Term]) -> Option<Term> {
    match args {
        [Term::Str(s), Term::Int(start), Term::Int(len)] => {
            let chars: Vec<char> = s.chars().collect();
            let start = (*start).max(0) as usize;
            let len = (*len).max(0) as usize;
            let slice: String = chars.iter().skip(start).take(len).collect();
            Some(Term::Str(slice))
        }
        _ => None,
    }
}

fn chars(args: &[Term]) -> Option<Term> {
    match args {
        [Term::Str(s)] => Some(make_list(
            s.chars().map(|c| Term::Str(c.to_string())).collect(),
        )),
        _ => None,
    }
}
