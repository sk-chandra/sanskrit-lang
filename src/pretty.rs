//! Pretty-printing of terms, with numeral sugar.
//!
//! Peano numbers (`उत्तर(उत्तर(...(०)))`) are collapsed back to numerals so
//! that results read naturally. By default numerals print in Devanagari; pass
//! `ascii = true` for Latin digits.

use crate::ast::Term;

fn to_devanagari(n: u128) -> String {
    let ascii = n.to_string();
    ascii
        .chars()
        .map(|c| {
            if let Some(d) = c.to_digit(10) {
                char::from_u32(0x0966 + d).unwrap()
            } else {
                c
            }
        })
        .collect()
}

pub fn show(term: &Term, ascii: bool) -> String {
    if let Some(n) = term.as_nat() {
        return if ascii { n.to_string() } else { to_devanagari(n) };
    }
    match term {
        Term::Var(v) => format!("?{}", v),
        Term::Str(s) => format!("\"{}\"", s),
        Term::Sym(name, args) if args.is_empty() => name.clone(),
        Term::Sym(name, args) => {
            let inner: Vec<String> = args.iter().map(|a| show(a, ascii)).collect();
            format!("{}({})", name, inner.join(", "))
        }
    }
}
