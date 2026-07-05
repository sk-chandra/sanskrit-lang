//! The effect runtime — pure effect-as-data I/O.
//!
//! A Sūtra program is pure: `मुख्य` (main) evaluates to an *action*, a data
//! term built from effect constructors. The program only *builds* this tree;
//! the runtime here performs the effects, so purity is preserved.
//!
//! Core actions:
//!   शुद्ध(x)      pure: yield x, do nothing
//!   मुद्रण(s)     print s, yield एकक
//!   पठन          read a line, yield it
//!   बन्ध(m, k)    run m, then run k(result)
//!
//! World actions (Phase 2):
//!   सञ्चिकापाठ(p)      read file p → its contents (or a दोष)
//!   सञ्चिकालेख(p, s)   write s to file p → एकक (or a दोष)
//!   प्राचलाः           the program's command-line arguments (a list of strings)
//!   पर्यावरण(name)     environment variable → its value (or a दोष)
//!   काल               current Unix time in seconds
//!   यादृच्छिक(n)       a random integer in [0, n)

use std::cell::Cell;
use std::io::{BufRead, Write};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::ast::Term;
use crate::engine::Engine;
use crate::pretty;

pub struct Runner<'a, 'e> {
    pub engine: &'a Engine<'e>,
    pub ascii: bool,
    args: Vec<String>,
    rng: Cell<u64>,
}

fn dosha(msg: impl Into<String>) -> Term {
    Term::app("दोष", vec![Term::Str(msg.into())])
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

impl<'a, 'e> Runner<'a, 'e> {
    pub fn new(engine: &'a Engine<'e>, ascii: bool, args: Vec<String>) -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E3779B97F4A7C15)
            | 1; // never zero
        Runner { engine, ascii, args, rng: Cell::new(seed) }
    }

    fn next_rand(&self) -> u64 {
        // xorshift64
        let mut x = self.rng.get();
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng.set(x);
        x
    }

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
                Term::Str(line.trim_end_matches(['\n', '\r']).to_string())
            }

            Term::Sym(n, args) if n == "बन्ध" && args.len() == 2 => {
                let result = self.run(args[0].clone());
                let next = Term::App(Box::new(args[1].clone()), vec![result]);
                self.run(next)
            }

            Term::Sym(n, args) if n == "सञ्चिकापाठ" && args.len() == 1 => match &args[0] {
                Term::Str(path) => match std::fs::read_to_string(path) {
                    Ok(s) => Term::Str(s),
                    Err(e) => dosha(format!("सञ्चिकापाठ: {}", e)),
                },
                _ => dosha("सञ्चिकापाठ: path must be a string"),
            },

            Term::Sym(n, args) if n == "सञ्चिकालेख" && args.len() == 2 => {
                match (&args[0], &args[1]) {
                    (Term::Str(path), Term::Str(content)) => match std::fs::write(path, content) {
                        Ok(()) => Term::con("एकक"),
                        Err(e) => dosha(format!("सञ्चिकालेख: {}", e)),
                    },
                    _ => dosha("सञ्चिकालेख: expects (path, contents) strings"),
                }
            }

            Term::Sym(n, args) if n == "प्राचलाः" && args.is_empty() => {
                make_list(self.args.iter().map(|a| Term::Str(a.clone())).collect())
            }

            Term::Sym(n, args) if n == "पर्यावरण" && args.len() == 1 => match &args[0] {
                Term::Str(name) => match std::env::var(name) {
                    Ok(v) => Term::Str(v),
                    Err(_) => dosha(format!("पर्यावरण: {} अनुपलब्ध", name)),
                },
                _ => dosha("पर्यावरण: name must be a string"),
            },

            Term::Sym(n, args) if n == "काल" && args.is_empty() => Term::Int(now_secs() as i64),

            Term::Sym(n, args) if n == "यादृच्छिक" && args.len() == 1 => match &args[0] {
                Term::Int(m) if *m > 0 => Term::Int((self.next_rand() % (*m as u64)) as i64),
                _ => dosha("यादृच्छिक: argument must be a positive integer"),
            },

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

fn make_list(items: Vec<Term>) -> Term {
    let mut t = Term::con("रिक्त");
    for it in items.into_iter().rev() {
        t = Term::app("युग्म", vec![it, t]);
    }
    t
}
