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

    /// Match a pattern against a concrete term (one-way, non-linear). The term
    /// may contain shared thunks, which are looked through transparently.
    fn match_term(pat: &Term, term: &Term, binds: &mut Bindings) -> bool {
        // See through a shared thunk to its current contents.
        if let Term::Share(c) = term {
            return Self::match_term(pat, &c.borrow(), binds);
        }
        match pat {
            Term::Var(v) => {
                if let Some(prev) = binds.get(v) {
                    // Non-linear use: compare share-free snapshots.
                    prev.strip() == term.strip()
                } else {
                    binds.insert(v.clone(), term.clone());
                    true
                }
            }
            Term::Int(a) => matches!(term, Term::Int(b) if a == b),
            Term::Big(a) => matches!(term, Term::Big(b) if a == b),
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

    /// Count occurrences of variable `v` in `t`, saturating at 2 (enough to know
    /// whether a binding is used more than once).
    fn occurs(t: &Term, v: &str) -> usize {
        match t {
            Term::Var(x) => (x == v) as usize,
            Term::Sym(_, args) | Term::App(_, args) => {
                let mut n = match t {
                    Term::App(f, _) => Self::occurs(f, v),
                    _ => 0,
                };
                for a in args {
                    n += Self::occurs(a, v);
                    if n >= 2 {
                        return 2;
                    }
                }
                n
            }
            Term::Lam(params, body) => {
                if params.iter().any(|p| p == v) {
                    0 // shadowed
                } else {
                    Self::occurs(body, v)
                }
            }
            _ => 0,
        }
    }

    /// Wrap each binding that is used more than once in the template into a
    /// single shared thunk, so it is reduced at most once (call-by-need).
    fn share_binds(binds: Bindings, template: &Term) -> Bindings {
        binds
            .into_iter()
            .map(|(k, val)| {
                let v = if Self::occurs(template, &k) >= 2 { Term::shared(val) } else { val };
                (k, v)
            })
            .collect()
    }

    /// Capture-avoiding (shadow-aware) substitution.
    pub fn subst(t: &Term, binds: &Bindings) -> Term {
        match t {
            Term::Var(v) => binds.get(v).cloned().unwrap_or_else(|| Term::Var(v.clone())),
            Term::Int(_) | Term::Big(_) | Term::Float(_) | Term::Str(_) => t.clone(),
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
            // A shared thunk is a runtime value with no free template variables.
            Term::Share(_) => t.clone(),
            // Map entries may mention variables when written as a literal that
            // has not yet been built into a Map value.
            Term::Map(entries) => Term::Map(
                entries
                    .iter()
                    .map(|(k, v)| (Self::subst(k, binds), Self::subst(v, binds)))
                    .collect(),
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
            let binds = Self::share_binds(binds, body);
            Term::Lam(rest, Box::new(Self::subst(body, &binds)))
        } else {
            let mut binds = Bindings::new();
            for (p, a) in params.iter().zip(args.iter()) {
                binds.insert(p.clone(), a.clone());
            }
            let binds = Self::share_binds(binds, body);
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
                // A guard must reduce to सत्य for the rule to fire; otherwise we
                // fall through to earlier-declared rules.
                if let Some(guard) = &rule.guard {
                    let g = self.normalize(&Self::subst(guard, &binds)).term;
                    if g != Term::con("सत्य") {
                        continue;
                    }
                }
                let binds = Self::share_binds(binds, &rule.rhs);
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
                // Look through shared thunks in the function position.
                let fp = f.peel();
                // β-reduce a lambda immediately.
                if let Term::Lam(params, body) = &fp {
                    return Some(Self::beta(params, body, args));
                }
                // Reduce the function position to a value first.
                if let Some(f2) = self.step(f) {
                    return Some(Term::App(Box::new(f2), args.clone()));
                }
                // The function is now irreducible: applying a symbol (a function
                // reference or partial application) saturates it into a call.
                if let Term::Sym(name, prev) = &fp {
                    let mut call = prev.clone();
                    call.extend(args.iter().cloned());
                    return Some(Term::Sym(name.clone(), call));
                }
                let fc = f.clone();
                Self::step_arg(args, |a| self.step(a), move |na| Term::App(fc.clone(), na))
            }
            // A shared thunk: advance its contents in place so that every other
            // reference to the same cell sees the progress (call-by-need).
            Term::Share(c) => {
                let stepped = self.step(&c.borrow());
                match stepped {
                    Some(next) => {
                        *c.borrow_mut() = next;
                        Some(Term::Share(std::rc::Rc::clone(c)))
                    }
                    None => None,
                }
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
                // Hand a clean, share-free normal form to the rest of the program.
                None => return Outcome { term: cur.strip(), steps, out_of_fuel: false },
            }
        }
        Outcome { term: cur.strip(), steps, out_of_fuel: true }
    }
}
