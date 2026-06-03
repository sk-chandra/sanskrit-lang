//! Primitive (builtin) reductions over native values: arithmetic, comparison,
//! equality, and string/list operations. Builtins are *strict*: the engine only
//! calls them once their arguments are fully reduced.

use crate::ast::Term;
use crate::bigint::BigInt;

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
            | "रिक्तकोश"
            | "समावेश"
            | "प्राप्ति"
            | "अस्ति"
            | "निष्कास"
            | "कुञ्जिकाः"
            | "मूल्यानि"
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
    // Arguments are strict and fully reduced here; peel any shared thunks so the
    // operations below work on plain values.
    let owned: Vec<Term> = args.iter().map(|a| a.strip()).collect();
    let args: &[Term] = &owned;
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
        "रिक्तकोश" => Some(Term::Map(vec![])),
        "समावेश" => map_insert(args),
        "प्राप्ति" => map_get(args),
        "अस्ति" => map_has(args),
        "निष्कास" => map_remove(args),
        "कुञ्जिकाः" => map_keys(args),
        "मूल्यानि" => map_values(args),
        _ => None,
    }
}

/// A total order on key terms, used to keep maps sorted and deduplicated.
fn key_cmp(a: &Term, b: &Term) -> std::cmp::Ordering {
    crate::pretty::show(a, true).cmp(&crate::pretty::show(b, true))
}

fn map_insert(args: &[Term]) -> Option<Term> {
    match args {
        [Term::Map(entries), k, v] => {
            let mut out = entries.clone();
            match out.binary_search_by(|(ek, _)| key_cmp(ek, k)) {
                Ok(i) => out[i].1 = v.clone(),
                Err(i) => out.insert(i, (k.clone(), v.clone())),
            }
            Some(Term::Map(out))
        }
        _ => None,
    }
}

fn map_lookup<'a>(entries: &'a [(Term, Term)], k: &Term) -> Option<&'a Term> {
    entries
        .binary_search_by(|(ek, _)| key_cmp(ek, k))
        .ok()
        .map(|i| &entries[i].1)
}

fn map_get(args: &[Term]) -> Option<Term> {
    match args {
        [Term::Map(entries), k] => Some(match map_lookup(entries, k) {
            Some(v) => v.clone(),
            None => dosha("कुञ्जिका न प्राप्ता (key not found)"),
        }),
        // प्राप्ति(map, key, default) — return default when absent.
        [Term::Map(entries), k, default] => {
            Some(map_lookup(entries, k).cloned().unwrap_or_else(|| default.clone()))
        }
        _ => None,
    }
}

fn map_has(args: &[Term]) -> Option<Term> {
    match args {
        [Term::Map(entries), k] => Some(boolean(map_lookup(entries, k).is_some())),
        _ => None,
    }
}

fn map_remove(args: &[Term]) -> Option<Term> {
    match args {
        [Term::Map(entries), k] => {
            let mut out = entries.clone();
            if let Ok(i) = out.binary_search_by(|(ek, _)| key_cmp(ek, k)) {
                out.remove(i);
            }
            Some(Term::Map(out))
        }
        _ => None,
    }
}

fn map_keys(args: &[Term]) -> Option<Term> {
    match args {
        [Term::Map(entries)] => Some(make_list(entries.iter().map(|(k, _)| k.clone()).collect())),
        _ => None,
    }
}

fn map_values(args: &[Term]) -> Option<Term> {
    match args {
        [Term::Map(entries)] => Some(make_list(entries.iter().map(|(_, v)| v.clone()).collect())),
        _ => None,
    }
}

fn as_f64(t: &Term) -> Option<f64> {
    match t {
        Term::Int(n) => Some(*n as f64),
        Term::Big(b) => Some(b.to_f64()),
        Term::Float(f) => Some(*f),
        _ => None,
    }
}

fn is_integer(t: &Term) -> bool {
    matches!(t, Term::Int(_) | Term::Big(_))
}

fn as_big(t: &Term) -> Option<BigInt> {
    match t {
        Term::Int(n) => Some(BigInt::from_i64(*n)),
        Term::Big(b) => Some(b.clone()),
        _ => None,
    }
}

/// Demote an arbitrary-precision result back to a native `Int` when it fits, so
/// that a `Big` value is never used for something representable as `i64` (which
/// keeps equality between the two representations correct).
fn from_big(b: BigInt) -> Term {
    match b.to_i64() {
        Some(n) => Term::Int(n),
        None => Term::Big(b),
    }
}

fn big_arith(op: &str, x: &BigInt, y: &BigInt) -> Option<Term> {
    Some(match op {
        "+" => from_big(x.add(y)),
        "-" => from_big(x.sub(y)),
        "*" => from_big(x.mul(y)),
        "/" => match x.div_rem(y) {
            Some((q, _)) => from_big(q),
            None => dosha("शून्येन भागः (division by zero)"),
        },
        "%" => match x.div_rem(y) {
            Some((_, r)) => from_big(r),
            None => dosha("शून्येन भागः (modulo by zero)"),
        },
        _ => return None,
    })
}

fn arith(op: &str, args: &[Term]) -> Option<Term> {
    if args.len() != 2 {
        return None;
    }
    match (&args[0], &args[1]) {
        // Fast path: both fit i64, with overflow promoting to big integers.
        (Term::Int(a), Term::Int(b)) => {
            let (a, b) = (*a, *b);
            let checked = match op {
                "+" => a.checked_add(b),
                "-" => a.checked_sub(b),
                "*" => a.checked_mul(b),
                "/" => {
                    if b == 0 {
                        return Some(dosha("शून्येन भागः (division by zero)"));
                    }
                    a.checked_div(b)
                }
                "%" => {
                    if b == 0 {
                        return Some(dosha("शून्येन भागः (modulo by zero)"));
                    }
                    a.checked_rem(b)
                }
                _ => return None,
            };
            Some(match checked {
                Some(v) => Term::Int(v),
                None => big_arith(op, &BigInt::from_i64(a), &BigInt::from_i64(b))?,
            })
        }
        // Either side is an arbitrary-precision integer.
        (a, b) if is_integer(a) && is_integer(b) => {
            big_arith(op, &as_big(a)?, &as_big(b)?)
        }
        // Otherwise floating point (if at least one side is a float).
        _ => {
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
        [Term::Int(n)] => Some(match n.checked_neg() {
            Some(v) => Term::Int(v),
            None => from_big(BigInt::from_i64(*n).neg()),
        }),
        [Term::Big(b)] => Some(from_big(b.neg())),
        [Term::Float(f)] => Some(Term::Float(-f)),
        _ => None,
    }
}

fn structural_eq(a: &Term, b: &Term) -> bool {
    // Integers compare exactly (never via lossy f64); a float on either side
    // compares numerically; everything else compares structurally.
    if is_integer(a) && is_integer(b) {
        return as_big(a) == as_big(b);
    }
    if let (Some(x), Some(y)) = (as_f64(a), as_f64(b)) {
        return x == y;
    }
    a == b
}

fn order(op: &str, args: &[Term]) -> Option<Term> {
    if args.len() != 2 {
        return None;
    }
    let (a, b) = (&args[0], &args[1]);
    let ord = match (a, b) {
        (Term::Str(x), Term::Str(y)) => x.cmp(y),
        _ if is_integer(a) && is_integer(b) => as_big(a)?.cmp(&as_big(b)?),
        _ => as_f64(a)?.partial_cmp(&as_f64(b)?)?,
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
        [Term::Map(entries)] => Some(Term::Int(entries.len() as i64)),
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
