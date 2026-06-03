//! A lightweight static checker (`sutra check`). It catches likely mistakes —
//! without imposing a type system on what is fundamentally an untyped rewriting
//! language — by reporting three things:
//!
//! 1. **Unbound variables** in a rule's guard/right-hand side (or in a प्रयोग):
//!    a `?x` that is not bound by the left-hand side or an enclosing lambda.
//!    This is almost always a typo and is reported as an error.
//! 2. **Constructor-arity mismatches**: using a saṃjñā-declared constructor with
//!    the wrong number of arguments (a warning).
//! 3. **Non-exhaustive matches**: a one-argument function whose clauses match
//!    constructors of a single saṃjñā but miss some of them, with no catch-all
//!    (a warning).

use std::collections::{HashMap, HashSet};

use crate::ast::{Program, Term};

#[derive(Clone, Copy, PartialEq)]
pub enum Severity {
    Error,
    Warning,
}

pub struct Diagnostic {
    pub severity: Severity,
    pub msg: String,
}

impl Diagnostic {
    fn error(msg: impl Into<String>) -> Self {
        Diagnostic { severity: Severity::Error, msg: msg.into() }
    }
    fn warning(msg: impl Into<String>) -> Self {
        Diagnostic { severity: Severity::Warning, msg: msg.into() }
    }
}

/// Check `target`'s declarations, using `context` (target merged with the
/// prelude and imports) to resolve constructor and saṃjñā information.
pub fn check(context: &Program, target: &Program) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    let ctors = constructor_arities(context);

    for rule in &target.rules {
        let head = rule_head(&rule.lhs);
        let mut bound = HashSet::new();
        collect_pattern_vars(&rule.lhs, &mut bound);

        let mut unbound = Vec::new();
        if let Some(g) = &rule.guard {
            free_unbound(g, &mut bound.clone(), &mut unbound);
        }
        free_unbound(&rule.rhs, &mut bound, &mut unbound);
        unbound.sort();
        unbound.dedup();
        for v in unbound {
            diags.push(Diagnostic::error(format!(
                "rule `{}`: variable ?{} is used but never bound",
                head, v
            )));
        }

        check_arities(&rule.lhs, &ctors, &mut diags);
        check_arities(&rule.rhs, &ctors, &mut diags);
    }

    for (i, p) in target.prayogas.iter().enumerate() {
        let mut unbound = Vec::new();
        free_unbound(p, &mut HashSet::new(), &mut unbound);
        unbound.sort();
        unbound.dedup();
        for v in unbound {
            diags.push(Diagnostic::error(format!(
                "प्रयोग #{}: variable ?{} is unbound",
                i + 1,
                v
            )));
        }
        check_arities(p, &ctors, &mut diags);
    }

    exhaustiveness(target, context, &ctors, &mut diags);
    diags
}

fn rule_head(lhs: &Term) -> String {
    match lhs {
        Term::Sym(n, _) => n.clone(),
        other => crate::pretty::show(other, true),
    }
}

/// Map each saṃjñā-declared constructor name to its arity.
fn constructor_arities(prog: &Program) -> HashMap<String, usize> {
    let mut m = HashMap::new();
    for s in &prog.samjnas {
        for alt in &s.alts {
            if let Term::Sym(name, args) = alt {
                m.insert(name.clone(), args.len());
            }
        }
    }
    m
}

fn collect_pattern_vars(t: &Term, out: &mut HashSet<String>) {
    match t {
        Term::Var(v) => {
            out.insert(v.clone());
        }
        Term::Sym(_, args) | Term::App(_, args) => {
            if let Term::App(f, _) = t {
                collect_pattern_vars(f, out);
            }
            for a in args {
                collect_pattern_vars(a, out);
            }
        }
        Term::Map(entries) => {
            for (k, v) in entries {
                collect_pattern_vars(k, out);
                collect_pattern_vars(v, out);
            }
        }
        _ => {}
    }
}

/// Collect variables used in `t` that are not in `bound`, threading lambda
/// binders into scope.
fn free_unbound(t: &Term, bound: &mut HashSet<String>, out: &mut Vec<String>) {
    match t {
        Term::Var(v) => {
            if !bound.contains(v) {
                out.push(v.clone());
            }
        }
        Term::Sym(_, args) => {
            for a in args {
                free_unbound(a, bound, out);
            }
        }
        Term::App(f, args) => {
            free_unbound(f, bound, out);
            for a in args {
                free_unbound(a, bound, out);
            }
        }
        Term::Lam(params, body) => {
            let added: Vec<String> =
                params.iter().filter(|p| bound.insert((*p).clone())).cloned().collect();
            free_unbound(body, bound, out);
            for p in added {
                bound.remove(&p);
            }
        }
        Term::Map(entries) => {
            for (k, v) in entries {
                free_unbound(k, bound, out);
                free_unbound(v, bound, out);
            }
        }
        _ => {}
    }
}

fn check_arities(t: &Term, ctors: &HashMap<String, usize>, diags: &mut Vec<Diagnostic>) {
    match t {
        Term::Sym(name, args) => {
            if let Some(&arity) = ctors.get(name) {
                if !args.is_empty() && args.len() != arity {
                    diags.push(Diagnostic::warning(format!(
                        "constructor `{}` expects {} argument(s) but got {}",
                        name,
                        arity,
                        args.len()
                    )));
                }
            }
            for a in args {
                check_arities(a, ctors, diags);
            }
        }
        Term::App(f, args) => {
            check_arities(f, ctors, diags);
            for a in args {
                check_arities(a, ctors, diags);
            }
        }
        Term::Lam(_, body) => check_arities(body, ctors, diags),
        Term::Map(entries) => {
            for (k, v) in entries {
                check_arities(k, ctors, diags);
                check_arities(v, ctors, diags);
            }
        }
        _ => {}
    }
}

/// Conservative exhaustiveness check for one-argument functions.
fn exhaustiveness(
    target: &Program,
    context: &Program,
    ctors: &HashMap<String, usize>,
    diags: &mut Vec<Diagnostic>,
) {
    // Constructor -> saṃjñā name, and saṃjñā -> its constructor set.
    let mut owner: HashMap<String, String> = HashMap::new();
    let mut members: HashMap<String, HashSet<String>> = HashMap::new();
    for s in &context.samjnas {
        for alt in &s.alts {
            if let Term::Sym(name, _) = alt {
                owner.insert(name.clone(), s.name.clone());
                members.entry(s.name.clone()).or_default().insert(name.clone());
            }
        }
    }

    // Group single-argument clauses by function name.
    let mut groups: HashMap<String, Vec<&crate::ast::Rule>> = HashMap::new();
    for r in &target.rules {
        if let Term::Sym(name, args) = &r.lhs {
            if args.len() == 1 {
                groups.entry(name.clone()).or_default().push(r);
            }
        }
    }

    for (fname, rules) in groups {
        // Any var pattern or guard makes the match potentially total → skip.
        let mut covered: HashSet<String> = HashSet::new();
        let mut total = false;
        for r in &rules {
            if r.guard.is_some() {
                total = true;
                break;
            }
            if let Term::Sym(_, args) = &r.lhs {
                match &args[0] {
                    Term::Sym(c, _) if ctors.contains_key(c) => {
                        covered.insert(c.clone());
                    }
                    Term::Sym(..) => {} // a constructor not from any saṃjñā; ignore
                    _ => {
                        total = true; // a variable / literal acts as a catch-all
                        break;
                    }
                }
            }
        }
        if total || covered.is_empty() {
            continue;
        }
        // All covered constructors must belong to one saṃjñā.
        let sajna: HashSet<&String> = covered.iter().filter_map(|c| owner.get(c)).collect();
        if sajna.len() != 1 {
            continue;
        }
        let sname = sajna.into_iter().next().unwrap();
        if let Some(all) = members.get(sname) {
            let missing: Vec<String> = all.difference(&covered).cloned().collect();
            if !missing.is_empty() {
                let mut missing = missing;
                missing.sort();
                diags.push(Diagnostic::warning(format!(
                    "function `{}` may be non-exhaustive over {}: no clause for {}",
                    fname,
                    sname,
                    missing.join(", ")
                )));
            }
        }
    }
}
