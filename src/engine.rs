//! The rewrite engine (प्रक्रिया, "the process / derivation").
//!
//! Evaluation is repeated rewriting to a normal form. Two design choices give
//! Sūtra its character:
//!
//! * **Reduction order is leftmost-outermost.** We always try to rewrite the
//!   outermost (root) redex first, descending into arguments only when the root
//!   cannot fire. This makes evaluation lazy enough that `यदि` (if) — an
//!   ordinary three-argument rule — does not evaluate the branch it discards.
//!
//! * **Conflict resolution is paratva** (परत्व, *vipratiṣedhe paraṃ kāryam*:
//!   "in conflict, the later operation prevails"). When several rules match the
//!   same redex, the **latest-declared** one wins. This makes a specific rule
//!   written *after* a general one behave as an apavāda (exception).

use crate::ast::{Bindings, Program, Rule, Term};

pub const DEFAULT_FUEL: u64 = 1_000_000;

pub struct Engine<'a> {
    rules: &'a [Rule],
    pub fuel: u64,
}

/// The outcome of normalising a term.
pub struct Outcome {
    pub term: Term,
    pub steps: u64,
    /// True if reduction stopped because it ran out of fuel (possible
    /// non-termination) rather than reaching a normal form.
    pub out_of_fuel: bool,
}

impl<'a> Engine<'a> {
    pub fn new(prog: &'a Program, fuel: u64) -> Self {
        Engine { rules: &prog.rules, fuel }
    }

    /// Match a pattern against a concrete term, extending `binds`.
    ///
    /// Matching is one-way (the term contains no variables) and supports
    /// non-linear patterns: a variable occurring twice must bind structurally
    /// equal subterms.
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
            Term::Str(a) => matches!(term, Term::Str(b) if a == b),
            Term::Sym(name, pargs) => match term {
                Term::Sym(tname, targs) if name == tname && pargs.len() == targs.len() => pargs
                    .iter()
                    .zip(targs.iter())
                    .all(|(p, t)| Self::match_term(p, t, binds)),
                _ => false,
            },
        }
    }

    /// Substitute bound variables into a rule's right-hand side.
    fn subst(t: &Term, binds: &Bindings) -> Term {
        match t {
            Term::Var(v) => binds.get(v).cloned().unwrap_or_else(|| Term::Var(v.clone())),
            Term::Str(s) => Term::Str(s.clone()),
            Term::Sym(name, args) => {
                Term::Sym(name.clone(), args.iter().map(|a| Self::subst(a, binds)).collect())
            }
        }
    }

    /// Try to rewrite `t` at its root. Under paratva the latest-declared
    /// matching rule wins, so we scan rules in reverse declaration order.
    fn rewrite_root(&self, t: &Term) -> Option<Term> {
        for rule in self.rules.iter().rev() {
            let mut binds = Bindings::new();
            if Self::match_term(&rule.lhs, t, &mut binds) {
                return Some(Self::subst(&rule.rhs, &binds));
            }
        }
        None
    }

    /// Perform a single leftmost-outermost rewrite step, if any redex exists.
    fn step(&self, t: &Term) -> Option<Term> {
        // Outermost: try the root first.
        if let Some(t2) = self.rewrite_root(t) {
            return Some(t2);
        }
        // Otherwise descend left-to-right into arguments.
        if let Term::Sym(name, args) = t {
            for (i, a) in args.iter().enumerate() {
                if let Some(a2) = self.step(a) {
                    let mut new_args = args.clone();
                    new_args[i] = a2;
                    return Some(Term::Sym(name.clone(), new_args));
                }
            }
        }
        None
    }

    /// Reduce `t` to a normal form (or until fuel is exhausted).
    pub fn normalize(&self, t: &Term) -> Outcome {
        let mut cur = t.clone();
        let mut steps = 0u64;
        while steps < self.fuel {
            match self.step(&cur) {
                Some(next) => {
                    cur = next;
                    steps += 1;
                }
                None => {
                    return Outcome { term: cur, steps, out_of_fuel: false };
                }
            }
        }
        Outcome { term: cur, steps, out_of_fuel: true }
    }
}
