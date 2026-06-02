//! Optional structural typing via saṃjñā (संज्ञा).
//!
//! A saṃjñā is a grammar production / algebraic data type. Types are *not*
//! enforced by the engine — they are structural and opt-in. This module answers
//! the question "does term `t` inhabit saṃjñā `S`?" by checking whether `t` is
//! derivable from `S`'s alternatives.
//!
//! A production alternative is read as a *data constructor* whose arguments are
//! *field types*. For example in
//!
//! ```text
//! संज्ञा सूची(?क) := रिक्त | युग्म(?क, सूची(?क))।
//! ```
//!
//! `युग्म` is a constructor; its first field has type `?क` (the type
//! parameter) and its second field has type `सूची(?क)` (a reference back to the
//! list type). Keeping the constructor role (the alternative head) separate from
//! the type role (the field positions) is what lets a type whose constructor
//! shares its own name — e.g. `संज्ञा दोष := दोष(?सन्देश)।` — be checked without
//! looping: type references only ever recurse into strictly smaller sub-terms.

use crate::ast::{Program, Term};

/// Does `term` inhabit the saṃjñā named `name`?
pub fn inhabits(prog: &Program, term: &Term, name: &str) -> bool {
    let s = match prog.samjna(name) {
        Some(s) => s,
        None => return false,
    };
    s.alts
        .iter()
        .any(|alt| cons_matches(prog, alt, term, &s.params))
}

/// Match a production alternative (a data constructor) against a term.
fn cons_matches(prog: &Program, alt: &Term, term: &Term, params: &[String]) -> bool {
    match alt {
        Term::Sym(c, field_types) => match term {
            Term::Sym(tc, subs) if tc == c && field_types.len() == subs.len() => field_types
                .iter()
                .zip(subs.iter())
                .all(|(ft, sub)| type_matches(prog, ft, sub, params)),
            _ => false,
        },
        // A bare variable alternative acts as a catch-all.
        Term::Var(_) => true,
        Term::Str(_) => matches!(term, Term::Str(_)),
    }
}

/// Check a field *type* expression against the corresponding sub-term.
fn type_matches(prog: &Program, ty: &Term, term: &Term, params: &[String]) -> bool {
    match ty {
        // A type variable (`?क`) matches anything.
        Term::Var(_) => true,
        // A reference to a type parameter matches anything.
        Term::Sym(n, _) if params.iter().any(|p| p == n) => true,
        // A reference to another saṃjñā: recurse (always on a smaller sub-term).
        Term::Sym(n, _) if prog.samjna(n).is_some() => inhabits(prog, term, n),
        // Otherwise a constructor literal used as a type: match structurally.
        Term::Sym(n, fields) => match term {
            Term::Sym(tn, subs) if tn == n && fields.len() == subs.len() => fields
                .iter()
                .zip(subs.iter())
                .all(|(f, sub)| type_matches(prog, f, sub, params)),
            _ => false,
        },
        Term::Str(_) => matches!(term, Term::Str(_)),
    }
}

/// Return the names of all declared saṃjñās that `term` inhabits.
pub fn classify(prog: &Program, term: &Term) -> Vec<String> {
    prog.samjnas
        .iter()
        .filter(|s| inhabits(prog, term, &s.name))
        .map(|s| s.name.clone())
        .collect()
}
