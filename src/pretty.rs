//! Pretty-printing of terms. Integers print in Devanagari by default (pass
//! `ascii = true` for Latin digits). Cons-lists print with `[...]` sugar.

use crate::ast::Term;
use crate::names::devanagari_digits;

pub fn show(term: &Term, ascii: bool) -> String {
    match term {
        Term::Share(c) => show(&c.borrow(), ascii),
        Term::Int(n) => {
            let s = n.to_string();
            if ascii {
                s
            } else {
                devanagari_digits(&s)
            }
        }
        Term::Big(b) => {
            let s = b.to_decimal_string();
            if ascii {
                s
            } else {
                devanagari_digits(&s)
            }
        }
        Term::Float(f) => {
            let mut s = format!("{}", f);
            // Keep floats visibly distinct from integers (7.0, not 7).
            if f.is_finite() && !s.contains(['.', 'e', 'E']) {
                s.push_str(".0");
            }
            if ascii {
                s
            } else {
                devanagari_digits(&s)
            }
        }
        Term::Str(s) => format!("\"{}\"", s),
        Term::Var(v) => format!("?{}", v),
        Term::Lam(params, body) => {
            let ps: Vec<String> = params.iter().map(|p| format!("?{}", p)).collect();
            format!("({}) => {}", ps.join(", "), show(body, ascii))
        }
        Term::App(f, args) => {
            let a: Vec<String> = args.iter().map(|x| show(x, ascii)).collect();
            let fs = match f.as_ref() {
                Term::Lam(..) => format!("({})", show(f, ascii)),
                _ => show(f, ascii),
            };
            format!("{}({})", fs, a.join(", "))
        }
        Term::Map(entries) => {
            let inner: Vec<String> = entries
                .iter()
                .map(|(k, v)| {
                    // Print string keys bare (record style), other keys quoted.
                    let ks = match k {
                        Term::Str(s) => s.clone(),
                        other => show(other, ascii),
                    };
                    format!("{}: {}", ks, show(v, ascii))
                })
                .collect();
            format!("{{{}}}", inner.join(", "))
        }
        Term::Sym(name, args) => {
            // List sugar.
            if let Some(items) = as_list(term) {
                let inner: Vec<String> = items.iter().map(|x| show(x, ascii)).collect();
                return format!("[{}]", inner.join(", "));
            }
            // Infix sugar for binary operators.
            if args.len() == 2 && is_operator(name) {
                return format!("{} {} {}", show(&args[0], ascii), name, show(&args[1], ascii));
            }
            if args.is_empty() {
                name.clone()
            } else {
                let inner: Vec<String> = args.iter().map(|x| show(x, ascii)).collect();
                format!("{}({})", name, inner.join(", "))
            }
        }
    }
}

fn is_operator(name: &str) -> bool {
    matches!(
        name,
        "+" | "-" | "*" | "/" | "%" | "==" | "!=" | "<" | "<=" | ">" | ">=" | "++" | "::"
    )
}

/// If `term` is a proper cons-list, return its elements.
fn as_list(term: &Term) -> Option<Vec<Term>> {
    let mut t = term;
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
