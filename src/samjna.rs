//! Optional structural typing via saṃjñā (संज्ञा).
//!
//! Types are not enforced; membership is computed on demand and exposed through
//! `--check` and the REPL's `:type`. Native types (Int/Float/String) are
//! recognised under built-in saṃjñā names so that e.g. `42 : संख्या`.

use crate::ast::{Program, Term};

/// Built-in saṃjñā names for native values: (name, predicate).
const NATIVE: &[&str] = &["संख्या", "दशांश", "अक्षरमाला", "कोश"];

fn native_inhabits(name: &str, term: &Term) -> Option<bool> {
    match name {
        "संख्या" => Some(matches!(term, Term::Int(_) | Term::Big(_))),
        "दशांश" => Some(matches!(term, Term::Float(_))),
        "अक्षरमाला" => Some(matches!(term, Term::Str(_))),
        "कोश" => Some(matches!(term, Term::Map(_))),
        _ => None,
    }
}

/// Does `term` inhabit the saṃjñā named `name`?
pub fn inhabits(prog: &Program, term: &Term, name: &str) -> bool {
    if let Some(b) = native_inhabits(name, term) {
        return b;
    }
    let s = match prog.samjna(name) {
        Some(s) => s,
        None => return false,
    };
    s.alts.iter().any(|alt| cons_matches(prog, alt, term, &s.params))
}

fn cons_matches(prog: &Program, alt: &Term, term: &Term, params: &[String]) -> bool {
    match alt {
        Term::Sym(c, field_types) => match term {
            Term::Sym(tc, subs) if tc == c && field_types.len() == subs.len() => field_types
                .iter()
                .zip(subs.iter())
                .all(|(ft, sub)| type_matches(prog, ft, sub, params)),
            _ => false,
        },
        Term::Var(_) => true,
        _ => alt == term,
    }
}

fn type_matches(prog: &Program, ty: &Term, term: &Term, params: &[String]) -> bool {
    match ty {
        Term::Var(_) => true,
        Term::Sym(n, _) if params.iter().any(|p| p == n) => true,
        Term::Sym(n, _) if native_inhabits(n, term).is_some() => {
            native_inhabits(n, term).unwrap()
        }
        Term::Sym(n, _) if prog.samjna(n).is_some() => inhabits(prog, term, n),
        Term::Sym(n, fields) => match term {
            Term::Sym(tn, subs) if tn == n && fields.len() == subs.len() => fields
                .iter()
                .zip(subs.iter())
                .all(|(f, sub)| type_matches(prog, f, sub, params)),
            _ => false,
        },
        _ => ty == term,
    }
}

/// Names of all saṃjñās (declared + native) that `term` inhabits.
pub fn classify(prog: &Program, term: &Term) -> Vec<String> {
    let mut out = Vec::new();
    for n in NATIVE {
        if inhabits(prog, term, n) {
            out.push((*n).to_string());
        }
    }
    for s in &prog.samjnas {
        if inhabits(prog, term, &s.name) {
            out.push(s.name.clone());
        }
    }
    out
}
