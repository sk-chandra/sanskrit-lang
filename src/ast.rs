//! Abstract syntax for Sūtra.
//!
//! The core is still term rewriting, but terms now include native data
//! (integers, floats, strings) and the two ingredients of higher-order
//! programming: lambdas and application.

use std::collections::HashMap;

/// A term: the single syntactic category of the language.
#[derive(Clone, Debug, PartialEq)]
pub enum Term {
    /// A pattern variable, written `?name`.
    Var(String),
    /// A native integer literal.
    Int(i64),
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
}

/// Bindings produced by matching a pattern. Keys are variable names.
pub type Bindings = HashMap<String, Term>;

/// A rewrite rule (सूत्र): `lhs -> rhs`.
#[derive(Clone, Debug)]
pub struct Rule {
    pub lhs: Term,
    pub rhs: Term,
    /// Source order, used to implement paratva (latest-declared wins).
    pub order: usize,
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
