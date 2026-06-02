//! The rewrite engine.
//!
//! Reduction is leftmost-outermost (so `यदि` stays lazy). Conflict resolution
//! is paratva: the latest-declared matching user rule wins. Beyond user rules,
//! the engine knows how to β-reduce lambdas, saturate function references, and
//! apply strict native builtins.

use crate::ast::{Bindings, Program, Rule, Term};
use crate::builtins;

pub const DEFAULT_FUEL: u64 = 5_000_000;

pub struct Engine<'a> {
    rules: &'a [Rule],
    pub fuel: u64,
}

pub struct Outcome {
    pub term: Term,
    pub steps: u64,
    pub out_of_fuel: bool,
}

impl<'a> Engine<'a> {
    pub fn new(prog: &'a Program, fuel: u64) -> Self {
        Engine { rules: &prog.rules, fuel }
    }

    /// Match a pattern against a concrete term (one-way, non-linear).
    fn match_term(pat: &Term, term: &Term, binds: &mut Bindings) -> bool {
        match pat {
            Term::Var(v) => {
                if let Some(prev) = binds.get(v) {
                    prev == term
                } else {
                    binds.insert(v.clone(), term.clone());
                    true
                }
            }
            Term::Int(a) => matches!(term, Term::Int(b) if a == b),
            Term::Float(a) => matches!(term, Term::Float(b) if a == b),
            Term::Str(a) => matches!(term, Term::Str(b) if a == b),
            Term::Sym(name, pargs) => match term {
                Term::Sym(tname, targs) if name == tname && pargs.len() == targs.len() => pargs
                    .iter()
                    .zip(targs.iter())
                    .all(|(p, t)| Self::match_term(p, t, binds)),
                _ => false,
            },
            // Lambdas / applications in patterns are matched structurally.
            other => other == term,
        }
    }

    /// Capture-avoiding (shadow-aware) substitution.
    pub fn subst(t: &Term, binds: &Bindings) -> Term {
        match t {
            Term::Var(v) => binds.get(v).cloned().unwrap_or_else(|| Term::Var(v.clone())),
            Term::Int(_) | Term::Float(_) | Term::Str(_) => t.clone(),
            Term::Sym(name, args) => {
                Term::Sym(name.clone(), args.iter().map(|a| Self::subst(a, binds)).collect())
            }
            Term::Lam(params, body) => {
                // Parameters shadow outer bindings of the same name.
                let mut inner = binds.clone();
                for p in params {
                    inner.remove(p);
                }
                Term::Lam(params.clone(), Box::new(Self::subst(body, &inner)))
            }
            Term::App(f, args) => Term::App(
                Box::new(Self::subst(f, binds)),
                args.iter().map(|a| Self::subst(a, binds)).collect(),
            ),
        }
    }

    /// β-reduce a lambda applied to arguments, supporting currying and
    /// over-application.
    fn beta(params: &[String], body: &Term, args: &[Term]) -> Term {
        let n = params.len();
        let m = args.len();
        if m < n {
            // Partial application: bind the first m, return a smaller lambda.
            let mut binds = Bindings::new();
            for (p, a) in params.iter().zip(args.iter()) {
                binds.insert(p.clone(), a.clone());
            }
            let rest = params[m..].to_vec();
            Term::Lam(rest, Box::new(Self::subst(body, &binds)))
        } else {
            let mut binds = Bindings::new();
            for (p, a) in params.iter().zip(args.iter()) {
                binds.insert(p.clone(), a.clone());
            }
            let reduced = Self::subst(body, &binds);
            if m == n {
                reduced
            } else {
                // Over-application: apply the result to the leftover args.
                Term::App(Box::new(reduced), args[n..].to_vec())
            }
        }
    }

    /// Try a root user-rule rewrite (paratva: latest-declared match wins).
    fn try_user_rules(&self, t: &Term) -> Option<Term> {
        for rule in self.rules.iter().rev() {
            let mut binds = Bindings::new();
            if Self::match_term(&rule.lhs, t, &mut binds) {
                return Some(Self::subst(&rule.rhs, &binds));
            }
        }
        None
    }

    fn step_arg<F>(args: &[Term], step_one: F, rebuild: impl Fn(Vec<Term>) -> Term) -> Option<Term>
    where
        F: Fn(&Term) -> Option<Term>,
    {
        for (i, a) in args.iter().enumerate() {
            if let Some(a2) = step_one(a) {
                let mut new_args = args.to_vec();
                new_args[i] = a2;
                return Some(rebuild(new_args));
            }
        }
        None
    }

    /// One leftmost-outermost rewrite step.
    fn step(&self, t: &Term) -> Option<Term> {
        // A root user rule always takes precedence (outermost, paratva).
        if let Term::Sym(..) = t {
            if let Some(r) = self.try_user_rules(t) {
                return Some(r);
            }
        }
        match t {
            Term::Sym(name, args) => {
                if builtins::is_builtin(name) {
                    // Builtins are strict: reduce the leftmost reducible
                    // argument first; only apply once every argument is a value.
                    let n = name.clone();
                    if let Some(r) =
                        Self::step_arg(args, |a| self.step(a), move |na| Term::Sym(n.clone(), na))
                    {
                        return Some(r);
                    }
                    return builtins::apply(name, args);
                }
                // Non-builtin symbol: outermost — descend into arguments.
                let n = name.clone();
                Self::step_arg(args, |a| self.step(a), move |na| Term::Sym(n.clone(), na))
            }
            Term::App(f, args) => {
                // β-reduce a lambda immediately.
                if let Term::Lam(params, body) = f.as_ref() {
                    return Some(Self::beta(params, body, args));
                }
                // Reduce the function position to a value first.
                if let Some(f2) = self.step(f) {
                    return Some(Term::App(Box::new(f2), args.clone()));
                }
                // The function is now irreducible: applying a symbol (a function
                // reference or partial application) saturates it into a call.
                if let Term::Sym(name, prev) = f.as_ref() {
                    let mut call = prev.clone();
                    call.extend(args.iter().cloned());
                    return Some(Term::Sym(name.clone(), call));
                }
                let fc = f.clone();
                Self::step_arg(args, |a| self.step(a), move |na| Term::App(fc.clone(), na))
            }
            // We do not reduce under a lambda until it is applied.
            _ => None,
        }
    }

    pub fn normalize(&self, t: &Term) -> Outcome {
        let mut cur = t.clone();
        let mut steps = 0u64;
        while steps < self.fuel {
            match self.step(&cur) {
                Some(next) => {
                    cur = next;
                    steps += 1;
                }
                None => return Outcome { term: cur, steps, out_of_fuel: false },
            }
        }
        Outcome { term: cur, steps, out_of_fuel: true }
    }
}
