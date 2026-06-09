//! Abstract syntax for Sūtra.
//!
//! The core is still term rewriting, but terms now include native data
//! (integers, floats, strings) and the two ingredients of higher-order
//! programming: lambdas and application.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// A term: the single syntactic category of the language.
#[derive(Clone, Debug, PartialEq)]
pub enum Term {
    /// A pattern variable, written `?name`.
    Var(String),
    /// A native integer literal.
    Int(i64),
    /// An arbitrary-precision integer (used once a value no longer fits `i64`).
    Big(crate::bigint::BigInt),
    /// A native floating-point literal.
    Float(f64),
    /// A native string literal.
    Str(String),
    /// A symbol applied to zero or more arguments: `name(args...)`. A nullary
    /// symbol is a constant / constructor.
    Sym(String, Vec<Term>),
    /// An anonymous function: `(params) => body`.
    Lam(Vec<String>, Box<Term>),
    /// Application of a function *value* (a lambda or a function reference) to
    /// arguments: `f(args...)` where `f` is not a literal symbol name.
    App(Box<Term>, Vec<Term>),
    /// A *shared thunk* — the call-by-need cell. A variable used more than once
    /// is bound to one of these, so reducing any occurrence updates them all.
    /// Shares are an internal runtime device; they never appear in source and
    /// are stripped from a normal form before it leaves the engine.
    Share(Rc<RefCell<Term>>),
    /// A native immutable map / record: key→value pairs kept sorted by key and
    /// deduplicated. Records are just maps with string (field-name) keys.
    Map(Vec<(Term, Term)>),
}

impl Term {
    pub fn con(name: &str) -> Term {
        Term::Sym(name.to_string(), vec![])
    }

    pub fn app(name: &str, args: Vec<Term>) -> Term {
        Term::Sym(name.to_string(), args)
    }

    /// Is this term a value (fully evaluated, no redex at the root)?
    /// Used by builtins to decide whether their arguments are ready.
    pub fn is_value_atom(&self) -> bool {
        matches!(self, Term::Int(_) | Term::Float(_) | Term::Str(_))
    }

    /// Wrap a term in a fresh shared thunk (idempotent: an existing Share is
    /// returned unchanged so cells are not nested).
    pub fn shared(t: Term) -> Term {
        match t {
            Term::Share(_) => t,
            other => Term::Share(Rc::new(RefCell::new(other))),
        }
    }

    /// Follow a chain of shared thunks to the current contents at the top
    /// (shallow): the result's head is not a Share, but its children may be.
    pub fn peel(&self) -> Term {
        match self {
            Term::Share(c) => c.borrow().peel(),
            other => other.clone(),
        }
    }

    /// A deep copy with every Share replaced by its current contents. Used to
    /// hand a clean, share-free normal form to the rest of the program.
    pub fn strip(&self) -> Term {
        match self {
            Term::Share(c) => c.borrow().strip(),
            Term::Sym(n, args) => Term::Sym(n.clone(), args.iter().map(|a| a.strip()).collect()),
            Term::App(f, args) => {
                Term::App(Box::new(f.strip()), args.iter().map(|a| a.strip()).collect())
            }
            Term::Lam(p, b) => Term::Lam(p.clone(), Box::new(b.strip())),
            Term::Map(entries) => {
                Term::Map(entries.iter().map(|(k, v)| (k.strip(), v.strip())).collect())
            }
            other => other.clone(),
        }
    }
}

/// Bindings produced by matching a pattern. Keys are variable names.
pub type Bindings = HashMap<String, Term>;

/// A rewrite rule (सूत्र): `lhs -> rhs`, optionally `lhs | guard -> rhs`.
#[derive(Clone, Debug)]
pub struct Rule {
    pub lhs: Term,
    /// An optional guard: the rule fires only if this (evaluated under the
    /// match's bindings) reduces to `सत्य`.
    pub guard: Option<Term>,
    pub rhs: Term,
    /// Source order, used to implement paratva (latest-declared wins).
    pub order: usize,
    /// The अधिकार (section) the rule was declared under, if any. Unqualified
    /// calls see all rules; a qualified call `म.f(…)` dispatches only to the
    /// rules of module `म`.
    pub module: Option<String>,
}

/// A named class of elements (गण) used in क्रम patterns, where a class name
/// matches any of its members. Members are either listed literally or derived
/// as a pratyāhāra span over the program's śivasūtra inventory.
#[derive(Clone, Debug)]
pub struct Class {
    pub name: String,
    pub members: Vec<Term>,
    /// `गण X := प्रत्याहार(start, marker)।` — members are the sounds from
    /// `start` up to the row ended by `marker`, resolved against the
    /// śivasūtra inventory when the engine is built.
    pub derive: Option<(String, String)>,
}

/// One row of the śivasūtra inventory: an ordered group of sounds closed by a
/// marker consonant (इत्), e.g. `[अ, इ, उ] -> ण्।`.
#[derive(Clone, Debug)]
pub struct SivaRow {
    pub sounds: Vec<Term>,
    pub marker: String,
}

/// One rule of a sequence-rewriting system: a contiguous subsequence `lhs`
/// (element patterns) is replaced by `rhs` wherever it occurs in a list.
#[derive(Clone, Debug)]
pub struct SeqRule {
    pub lhs: Vec<Term>,
    pub rhs: Vec<Term>,
    pub order: usize,
}

/// A named sequence-rewriting system (क्रम) — the Pāṇinian contextual frontier.
/// Applied as `NAME(list)`, it rewrites the list by repeatedly replacing the
/// leftmost matching subsequence (latest-declared rule winning) until none
/// applies. Context is expressed simply by including neighbouring elements in
/// the pattern (`L A R -> L B R`).
#[derive(Clone, Debug)]
pub struct SeqSystem {
    pub name: String,
    pub rules: Vec<SeqRule>,
}

/// A saṃjñā (संज्ञा) — a named class of terms (an algebraic data type /
/// grammar production).
#[derive(Clone, Debug)]
pub struct Samjna {
    pub name: String,
    pub params: Vec<String>,
    pub alts: Vec<Term>,
}

/// A whole parsed program.
#[derive(Clone, Debug, Default)]
pub struct Program {
    pub rules: Vec<Rule>,
    pub samjnas: Vec<Samjna>,
    /// Named element classes (गण) for क्रम patterns.
    pub classes: Vec<Class>,
    /// Śivasūtra inventories: each `शिवसूत्र { … }` block is one self-contained
    /// inventory. Pratyāhāra resolution tries the latest block first (paratva),
    /// so a program's own inventory shadows the stdlib's rather than extending
    /// it.
    pub siva: Vec<Vec<SivaRow>>,
    /// Named sequence-rewriting systems (क्रम).
    pub seq: Vec<SeqSystem>,
    /// `प्रयोग EXPR।` declarations: expressions to evaluate and print.
    pub prayogas: Vec<Term>,
    /// Imported file paths (`उपयोग "..."`), resolved by the loader.
    pub imports: Vec<String>,
}

impl Program {
    /// Append another program's declarations, keeping rule order stable so that
    /// later declarations keep higher `order` (and thus win under paratva).
    pub fn extend(&mut self, other: Program) {
        let base = self.rules.len();
        for mut r in other.rules {
            r.order = base + r.order;
            self.rules.push(r);
        }
        self.samjnas.extend(other.samjnas);
        self.classes.extend(other.classes);
        self.siva.extend(other.siva);
        self.seq.extend(other.seq);
        self.prayogas.extend(other.prayogas);
        self.imports.extend(other.imports);
    }

    pub fn samjna(&self, name: &str) -> Option<&Samjna> {
        self.samjnas.iter().find(|s| s.name == name)
    }

    /// The right-hand side of the `मुख्य` (main) rule, if defined.
    pub fn main_action(&self) -> Option<Term> {
        self.rules
            .iter()
            .rev()
            .find(|r| matches!(&r.lhs, Term::Sym(n, a) if n == "मुख्य" && a.is_empty()))
            .map(|r| r.rhs.clone())
    }
}
