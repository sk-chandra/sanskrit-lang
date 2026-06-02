//! The effect runtime — pure effect-as-data I/O.
//!
//! A Sūtra program is pure: `मुख्य` (main) evaluates to an *action*, a data
//! term built from these constructors:
//!
//! * `शुद्ध(x)`      — an action that yields `x` and does nothing.
//! * `मुद्रण(s)`     — print `s`, yielding `एकक` (unit).
//! * `पठन`          — read a line, yielding it as a string.
//! * `बन्ध(m, k)`    — run `m`, then run `k(result)`.
//!
//! The program only *builds* this tree; the runtime here is what actually
//! performs the effects, so purity is preserved.

use std::io::{BufRead, Write};

use crate::ast::Term;
use crate::engine::Engine;
use crate::pretty;

pub struct Runner<'a, 'e> {
    pub engine: &'a Engine<'e>,
    pub ascii: bool,
}

impl<'a, 'e> Runner<'a, 'e> {
    /// Execute an action term, returning the value it yields.
    pub fn run(&self, action: Term) -> Term {
        let nf = self.engine.normalize(&action).term;
        match &nf {
            Term::Sym(n, args) if n == "शुद्ध" && args.len() == 1 => args[0].clone(),
            Term::Sym(n, args) if n == "मुद्रण" && args.len() == 1 => {
                self.print_value(&args[0]);
                Term::con("एकक")
            }
            Term::Sym(n, args) if n == "पठन" && args.is_empty() => {
                let mut line = String::new();
                let stdin = std::io::stdin();
                let _ = stdin.lock().read_line(&mut line);
                let line = line.trim_end_matches(['\n', '\r']).to_string();
                Term::Str(line)
            }
            Term::Sym(n, args) if n == "बन्ध" && args.len() == 2 => {
                let result = self.run(args[0].clone());
                let next = Term::App(Box::new(args[1].clone()), vec![result]);
                self.run(next)
            }
            // Not an action: treat main's value as the final result.
            other => other.clone(),
        }
    }

    fn print_value(&self, t: &Term) {
        let nf = self.engine.normalize(t).term;
        let out = match &nf {
            Term::Str(s) => s.clone(),
            other => pretty::show(other, self.ascii),
        };
        let stdout = std::io::stdout();
        let mut h = stdout.lock();
        let _ = writeln!(h, "{}", out);
    }
}
