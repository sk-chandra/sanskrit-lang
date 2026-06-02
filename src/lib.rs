//! Sūtra (सूत्र) — a pure term-rewriting programming language in the spirit of
//! Pāṇini's Aṣṭādhyāyī.
//!
//! A program is a set of rewrite rules (सूत्र) over terms (पद). Evaluation is
//! repeated rewriting to a normal form. There are no statements, no mutable
//! state, and no built-in control flow: conditionals, arithmetic and data
//! structures are all ordinary rules, most of them living in the standard
//! library (which is itself written in Sūtra).

pub mod ast;
pub mod engine;
pub mod lexer;
pub mod parser;
pub mod pretty;
pub mod samjna;

pub use ast::{Program, Term};
pub use engine::{Engine, Outcome};

/// The standard library, embedded into the binary so it is always available.
pub const PRELUDE_SOURCES: &[(&str, &str)] = &[
    ("ganita", include_str!("../std/ganita.sutra")),
    ("tarka", include_str!("../std/tarka.sutra")),
    ("suchi", include_str!("../std/suchi.sutra")),
    ("sandhi", include_str!("../std/sandhi.sutra")),
];

/// Parse and combine all standard-library modules into one program.
pub fn load_prelude() -> Result<Program, String> {
    let mut prog = Program::default();
    for (name, src) in PRELUDE_SOURCES {
        let module = parser::parse_program(src)
            .map_err(|e| format!("error in stdlib module '{}': {}", name, e))?;
        prog.extend(module);
    }
    Ok(prog)
}
