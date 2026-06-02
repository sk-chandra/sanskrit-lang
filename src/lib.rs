//! Sūtra (सूत्र) — a term-rewriting language with native data, higher-order
//! functions, ergonomic sugar, and pure effect-as-data I/O.

pub mod ast;
pub mod builtins;
pub mod effect;
pub mod engine;
pub mod lexer;
pub mod names;
pub mod parser;
pub mod pretty;
pub mod samjna;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub use ast::{Program, Term};
pub use engine::{Engine, Outcome};

/// The standard library, embedded into the binary so it is always available.
pub const PRELUDE_SOURCES: &[(&str, &str)] = &[
    ("core", include_str!("../std/core.sutra")),
    ("tarka", include_str!("../std/tarka.sutra")),
    ("ganita", include_str!("../std/ganita.sutra")),
    ("suchi", include_str!("../std/suchi.sutra")),
    ("io", include_str!("../std/io.sutra")),
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

/// Load a `.sutra` file and recursively resolve its `उपयोग "..."` imports.
/// Imported declarations are merged *before* the importing file's, so a file
/// can override what it imports (paratva).
pub fn load_file(path: &Path) -> Result<Program, String> {
    let mut visited = HashSet::new();
    load_file_inner(path, &mut visited)
}

fn load_file_inner(path: &Path, visited: &mut HashSet<PathBuf>) -> Result<Program, String> {
    let canon = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if !visited.insert(canon) {
        return Ok(Program::default()); // already loaded; break cycles
    }
    let src = std::fs::read_to_string(path).map_err(|e| format!("cannot read {}: {}", path.display(), e))?;
    let parsed = parser::parse_program(&src).map_err(|e| format!("{}: {}", path.display(), e))?;

    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let mut prog = Program::default();
    for imp in &parsed.imports {
        let imp_path = dir.join(imp);
        let sub = load_file_inner(&imp_path, visited)?;
        prog.extend(sub);
    }
    // The file's own declarations come after its imports.
    prog.extend(Program {
        rules: parsed.rules,
        samjnas: parsed.samjnas,
        prayogas: parsed.prayogas,
        imports: vec![],
    });
    Ok(prog)
}
