//! Abstract syntax for Sūtra.
//!
//! Everything in Sūtra is a *term* (पद). There are no statements and no
//! special forms — programs are sets of rewrite rules (सूत्र) over terms.

use std::collections::BTreeMap;

/// The canonical name of the zero constructor.
///
/// We use the Devanagari digit zero so that the numeral `०` a user writes and
/// the internal nullary constructor are literally the same symbol.
pub const ZERO: &str = "०";

/// The canonical name of the successor constructor (uttara, "the next").
pub const SUCC: &str = "उत्तर";

/// A term: either a variable, a string literal, or a symbol application.
///
/// A nullary application `Sym(name, [])` is a *constant* / constructor; a
/// non-empty one is an application `name(arg, ...)`. There is no separate
/// notion of function vs. constructor — whether a symbol reduces depends only
/// on whether any rule's left-hand side matches it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Term {
    /// A pattern variable, written `?name`. Only meaningful inside rules.
    Var(String),
    /// A string literal, e.g. doṣa (error) messages.
    Str(String),
    /// A symbol applied to zero or more arguments: `name(args...)`.
    Sym(String, Vec<Term>),
}

impl Term {
    /// A nullary constant / constructor.
    pub fn con(name: &str) -> Term {
        Term::Sym(name.to_string(), vec![])
    }

    /// An application `name(args...)`.
    pub fn app(name: &str, args: Vec<Term>) -> Term {
        Term::Sym(name.to_string(), args)
    }

    /// Build the Peano encoding of a natural number: `उत्तर(उत्तर(...(०)))`.
    pub fn nat(mut n: u128) -> Term {
        let mut t = Term::con(ZERO);
        while n > 0 {
            t = Term::Sym(SUCC.to_string(), vec![t]);
            n -= 1;
        }
        t
    }

    /// If this term is a Peano numeral, return its value.
    pub fn as_nat(&self) -> Option<u128> {
        let mut t = self;
        let mut n: u128 = 0;
        loop {
            match t {
                Term::Sym(s, args) if s == ZERO && args.is_empty() => return Some(n),
                Term::Sym(s, args) if s == SUCC && args.len() == 1 => {
                    n = n.checked_add(1)?;
                    t = &args[0];
                }
                _ => return None,
            }
        }
    }
}

/// A bound set of pattern variables produced by matching.
pub type Bindings = BTreeMap<String, Term>;

/// A rewrite rule (सूत्र): `lhs -> rhs`.
#[derive(Clone, Debug)]
pub struct Rule {
    pub lhs: Term,
    pub rhs: Term,
    /// Source order index, used to implement paratva (latest-declared wins).
    pub order: usize,
}

/// A saṃjñā (संज्ञा) — a named class of terms, i.e. an algebraic data type /
/// grammar production: `name(params) := alt | alt | ...`.
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
}

impl Program {
    /// Append another program's declarations to this one, keeping rule order
    /// stable (later declarations get higher `order`, so they win under paratva).
    pub fn extend(&mut self, other: Program) {
        let base = self.rules.len();
        for mut r in other.rules {
            r.order = base + r.order;
            self.rules.push(r);
        }
        self.samjnas.extend(other.samjnas);
        self.prayogas.extend(other.prayogas);
    }

    pub fn samjna(&self, name: &str) -> Option<&Samjna> {
        self.samjnas.iter().find(|s| s.name == name)
    }
}
