//! The rewrite engine.
//!
//! Reduction is leftmost-outermost (so `यदि` stays lazy). Conflict resolution
//! is paratva: the latest-declared matching user rule wins. Beyond user rules,
//! the engine knows how to β-reduce lambdas, saturate function references, and
//! apply strict native builtins.

use crate::ast::{Bindings, Class, Program, Rule, SeqSystem, Term};
use crate::builtins;

pub const DEFAULT_FUEL: u64 = 5_000_000;

pub struct Engine<'a> {
    rules: &'a [Rule],
    seq: &'a [SeqSystem],
    classes: &'a [Class],
    pub fuel: u64,
}

pub struct Outcome {
    pub term: Term,
    pub steps: u64,
    pub out_of_fuel: bool,
}

impl<'a> Engine<'a> {
    pub fn new(prog: &'a Program, fuel: u64) -> Self {
        Engine { rules: &prog.rules, seq: &prog.seq, classes: &prog.classes, fuel }
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
                // A sequence-rewriting system applied to a list (strict in its
                // argument, like a builtin).
                if args.len() == 1 {
                    if let Some(system) = self.seq.iter().find(|s| &s.name == name) {
                        if let Some(a2) = self.step(&args[0]) {
                            return Some(Term::Sym(name.clone(), vec![a2]));
                        }
                        if let Some(elems) = list_items(&args[0]) {
                            let result = self.rewrite_seq(system, elems);
                            return Some(make_list(result));
                        }
                        return None; // argument is not a list ⇒ stuck
                    }
                }
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

    fn class_members(&self, name: &str) -> Option<&[Term]> {
        self.classes.iter().find(|c| c.name == name).map(|c| c.members.as_slice())
    }

    fn in_class(&self, name: &str, elem: &Term) -> bool {
        match self.class_members(name) {
            Some(members) => {
                let e = elem.strip();
                members.iter().any(|m| m == &e)
            }
            None => false,
        }
    }

    /// Match one क्रम pattern element against a list element, understanding गण
    /// classes: a bare class name matches any member; `?v:गण` (encoded as
    /// `@गण(?v, गण)`) matches a member and binds `?v` to it.
    fn seq_elem_match(&self, pat: &Term, elem: &Term, binds: &mut Bindings) -> bool {
        match pat {
            Term::Sym(tag, args) if tag == "@गण" && args.len() == 2 => {
                if let (Term::Var(v), Term::Sym(cls, _)) = (&args[0], &args[1]) {
                    if !self.in_class(cls, elem) {
                        return false;
                    }
                    match binds.get(v) {
                        Some(prev) => prev.strip() == elem.strip(),
                        None => {
                            binds.insert(v.clone(), elem.clone());
                            true
                        }
                    }
                } else {
                    false
                }
            }
            Term::Sym(name, a) if a.is_empty() && self.class_members(name).is_some() => {
                self.in_class(name, elem)
            }
            _ => Self::match_term(pat, elem, binds),
        }
    }

    /// Match a क्रम pattern against `elems` starting at absolute position
    /// `pos`, returning the end position on success. Handles anchors (`@आदि`
    /// at position 0, `@अन्त` at the end) and segment variables (`@तारा`,
    /// shortest-first with backtracking and non-linear consistency).
    fn seq_match_at(
        &self,
        pats: &[Term],
        elems: &[Term],
        pos: usize,
        binds: &mut Bindings,
    ) -> Option<usize> {
        let Some((first, rest)) = pats.split_first() else {
            return Some(pos);
        };
        match first {
            Term::Sym(n, a) if n == "@आदि" && a.is_empty() => {
                (pos == 0).then(|| self.seq_match_at(rest, elems, pos, binds)).flatten()
            }
            Term::Sym(n, a) if n == "@अन्त" && a.is_empty() => {
                (pos == elems.len()).then(|| self.seq_match_at(rest, elems, pos, binds)).flatten()
            }
            Term::Sym(n, args) if n == "@तारा" && !args.is_empty() => {
                let Term::Var(v) = &args[0] else { return None };
                let class = match args.get(1) {
                    Some(Term::Sym(c, _)) => Some(c.as_str()),
                    _ => None,
                };
                // A class-constrained segment can't extend past a non-member.
                let mut max_k = elems.len() - pos;
                if let Some(cls) = class {
                    max_k = elems[pos..]
                        .iter()
                        .take_while(|e| self.in_class(cls, e))
                        .count();
                }
                // Greedy: longest segment first, backtracking shorter.
                for k in (0..=max_k).rev() {
                    let segment = make_list(elems[pos..pos + k].to_vec());
                    let mut trial = binds.clone();
                    if let Some(prev) = trial.get(v) {
                        if prev.strip() != segment.strip() {
                            continue; // non-linear reuse must match the same segment
                        }
                    } else {
                        trial.insert(v.clone(), segment);
                    }
                    if let Some(end) = self.seq_match_at(rest, elems, pos + k, &mut trial) {
                        *binds = trial;
                        return Some(end);
                    }
                }
                None
            }
            pat => {
                if pos < elems.len() && self.seq_elem_match(pat, &elems[pos], binds) {
                    self.seq_match_at(rest, elems, pos + 1, binds)
                } else {
                    None
                }
            }
        }
    }

    /// Build the replacement for a matched क्रम rule: `@तारा(?v)` splices the
    /// captured segment's elements; everything else substitutes normally.
    fn expand_seq_rhs(rhs: &[Term], binds: &Bindings) -> Vec<Term> {
        let mut out = Vec::new();
        for t in rhs {
            if let Term::Sym(n, args) = t {
                if n == "@तारा" && args.len() == 1 {
                    if let Term::Var(v) = &args[0] {
                        if let Some(items) = binds.get(v).and_then(list_items) {
                            out.extend(items);
                            continue;
                        }
                    }
                }
            }
            out.push(Self::subst(t, binds));
        }
        out
    }

    /// Rewrite a sequence under a क्रम system: repeatedly replace the leftmost
    /// matching subsequence (latest-declared rule winning) until none applies.
    fn rewrite_seq(&self, system: &SeqSystem, mut elems: Vec<Term>) -> Vec<Term> {
        let mut budget = self.fuel;
        'scan: loop {
            for i in 0..=elems.len() {
                for rule in system.rules.iter().rev() {
                    let mut binds = Bindings::new();
                    if let Some(end) = self.seq_match_at(&rule.lhs, &elems, i, &mut binds) {
                        if end == i {
                            continue; // zero-width: never rewrite on nothing
                        }
                        let repl = Self::expand_seq_rhs(&rule.rhs, &binds);
                        if repl == elems[i..end] {
                            continue; // identity rewrite: no progress, skip
                        }
                        elems.splice(i..end, repl);
                        if budget == 0 {
                            return elems;
                        }
                        budget -= 1;
                        continue 'scan; // restart from the left after each rewrite
                    }
                }
            }
            return elems;
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

/// Collect the elements of a proper cons-list (looking through shares), or
/// `None` if `t` is not one.
fn list_items(t: &Term) -> Option<Vec<Term>> {
    let mut cur = t.peel();
    let mut out = Vec::new();
    loop {
        match cur {
            Term::Sym(ref n, ref a) if n == "रिक्त" && a.is_empty() => return Some(out),
            Term::Sym(ref n, ref a) if n == "युग्म" && a.len() == 2 => {
                out.push(a[0].clone());
                cur = a[1].peel();
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
