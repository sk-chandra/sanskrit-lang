//! The `sutra` command-line interface.
//!
//! Usage:
//!   sutra run FILE.sutra [options]   run FILE (executes मुख्य/main if present,
//!                                    otherwise evaluates every प्रयोग)
//!   sutra FILE.sutra     [options]   (same as run)
//!   sutra eval "EXPR"    [options]   evaluate a single expression
//!   sutra repl           [options]   interactive session
//!
//! Options: --fuel N  --ascii  --no-prelude  --check  --steps

use std::path::Path;
use std::process::exit;

use sutra::check::{self, Severity};
use sutra::effect::Runner;
use sutra::engine::{Engine, DEFAULT_FUEL};
use sutra::{ast::Program, load_file, load_prelude, parser, pretty, samjna};

struct Options {
    fuel: u64,
    ascii: bool,
    no_prelude: bool,
    check: bool,
    steps: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options { fuel: DEFAULT_FUEL, ascii: false, no_prelude: false, check: false, steps: false }
    }
}

const USAGE: &str = "\
सूत्र — Sūtra interpreter

usage:
  sutra run FILE.sutra [options]   run FILE (executes मुख्य/main, else प्रयोग)
  sutra FILE.sutra     [options]   (same as run)
  sutra eval \"EXPR\"    [options]   evaluate a single expression
  sutra check FILE.sutra           statically check FILE for likely mistakes
  sutra fmt FILE.sutra [--write]   format FILE (print to stdout, or rewrite in place)
  sutra repl           [options]   interactive session

options:
  --fuel N        max rewrite steps before giving up
  --ascii         Latin digits instead of Devanagari
  --no-prelude    do not load the standard library
  --check         report which saṃjñās each result inhabits
  --steps         report the number of rewrite steps taken";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("{}", USAGE);
        exit(2);
    }

    let mut opts = Options::default();
    let mut write = false;
    let mut positional: Vec<String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--write" | "-w" => write = true,
            "--ascii" => opts.ascii = true,
            "--no-prelude" => opts.no_prelude = true,
            "--check" => opts.check = true,
            "--steps" => opts.steps = true,
            "--fuel" => {
                i += 1;
                match args.get(i).and_then(|s| s.parse::<u64>().ok()) {
                    Some(n) => opts.fuel = n,
                    None => {
                        eprintln!("--fuel requires a number");
                        exit(2);
                    }
                }
            }
            "-h" | "--help" => {
                println!("{}", USAGE);
                return;
            }
            a => positional.push(a.to_string()),
        }
        i += 1;
    }

    if positional.is_empty() {
        eprintln!("{}", USAGE);
        exit(2);
    }

    let (cmd, rest) = match positional[0].as_str() {
        "run" => ("run", &positional[1..]),
        "eval" => ("eval", &positional[1..]),
        "check" => ("check", &positional[1..]),
        "fmt" => ("fmt", &positional[1..]),
        "repl" => ("repl", &positional[1..]),
        _ => ("run", &positional[..]),
    };

    let result = match cmd {
        "run" => cmd_run(rest, &opts),
        "eval" => cmd_eval(rest, &opts),
        "check" => cmd_check(rest, &opts),
        "fmt" => cmd_fmt(rest, write),
        "repl" => cmd_repl(&opts),
        _ => unreachable!(),
    };

    if let Err(e) = result {
        eprintln!("{}", e);
        exit(1);
    }
}

fn base_program(opts: &Options) -> Result<Program, String> {
    if opts.no_prelude {
        Ok(Program::default())
    } else {
        load_prelude()
    }
}

fn cmd_run(rest: &[String], opts: &Options) -> Result<(), String> {
    let path = rest.first().ok_or("run: expected a file path")?;
    let mut prog = base_program(opts)?;
    let file_prog = load_file(Path::new(path))?;
    let prayogas = file_prog.prayogas.clone();
    prog.extend(file_prog);

    // If the program defines मुख्य (main), execute it as an action. Any
    // positional arguments after the file are passed to the program (प्राचलाः).
    if let Some(action) = prog.main_action() {
        let engine = Engine::new(&prog, opts.fuel);
        let prog_args: Vec<String> = rest.iter().skip(1).cloned().collect();
        let runner = Runner::new(&engine, opts.ascii, prog_args);
        runner.run(action);
        return Ok(());
    }

    if prayogas.is_empty() {
        eprintln!(
            "(no मुख्य and no प्रयोग in {} — nothing to run. Add `सूत्र मुख्य -> …।` or `प्रयोग …।`)",
            path
        );
        return Ok(());
    }

    let engine = Engine::new(&prog, opts.fuel);
    for expr in &prayogas {
        print_result(&prog, expr, &engine.normalize(expr), opts);
    }
    Ok(())
}

fn cmd_eval(rest: &[String], opts: &Options) -> Result<(), String> {
    let expr_src = rest.first().ok_or("eval: expected an expression")?;
    let prog = base_program(opts)?;
    let expr = parser::parse_expr(expr_src).map_err(|e| e.to_string())?;
    let engine = Engine::new(&prog, opts.fuel);
    print_result(&prog, &expr, &engine.normalize(&expr), opts);
    Ok(())
}

fn cmd_check(rest: &[String], opts: &Options) -> Result<(), String> {
    let path = rest.first().ok_or("check: expected a file path")?;
    // Parse the file on its own (what we report on), and build a context with
    // the prelude + imports so constructors/saṃjñās are known.
    let src = std::fs::read_to_string(path).map_err(|e| format!("cannot read {}: {}", path, e))?;
    let target = parser::parse_program(&src).map_err(|e| e.to_string())?;

    let mut context = if opts.no_prelude { Program::default() } else { load_prelude()? };
    context.extend(load_file(Path::new(path))?);

    let diags = check::check(&context, &target);
    let mut errors = 0;
    for d in &diags {
        let tag = match d.severity {
            Severity::Error => {
                errors += 1;
                "error"
            }
            Severity::Warning => "warning",
        };
        eprintln!("{}: {}", tag, d.msg);
    }
    if diags.is_empty() {
        println!("{}: no problems found.", path);
    } else {
        eprintln!(
            "{}: {} error(s), {} warning(s).",
            path,
            errors,
            diags.len() - errors
        );
    }
    if errors > 0 {
        exit(1);
    }
    Ok(())
}

fn cmd_fmt(rest: &[String], write: bool) -> Result<(), String> {
    let path = rest.first().ok_or("fmt: expected a file path")?;
    let src = std::fs::read_to_string(path).map_err(|e| format!("cannot read {}: {}", path, e))?;
    let formatted = sutra::fmt::format_source(&src)?;
    if write {
        if formatted != src {
            std::fs::write(path, &formatted)
                .map_err(|e| format!("cannot write {}: {}", path, e))?;
            println!("formatted {}", path);
        } else {
            println!("{} already formatted", path);
        }
    } else {
        print!("{}", formatted);
    }
    Ok(())
}

fn cmd_repl(opts: &Options) -> Result<(), String> {
    use std::io::{self, BufRead, Write};

    // Two programs: the immutable base (prelude) and the user's session
    // definitions. The active program is base + session, rebuilt on demand.
    let base = base_program(opts)?;
    let mut session = Program::default();
    let mut history: Vec<String> = Vec::new();
    // The अधिकार currently in effect, carried across inputs so a section and
    // its rules need not be typed in one go.
    let mut cur_module: Option<String> = None;

    println!("सूत्र REPL — type a term to evaluate, or a declaration (सूत्र/गण/क्रम/…)");
    println!("to define one. `:help` for commands, `:quit` to exit.");
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        // Read a (possibly multi-line) input.
        let mut buf = String::new();
        let mut cancelled = false;
        loop {
            print!("{}", if buf.is_empty() { "सूत्र> " } else { "  ...> " });
            stdout.flush().ok();
            let mut line = String::new();
            if stdin.lock().read_line(&mut line).map_err(|e| e.to_string())? == 0 {
                if buf.trim().is_empty() {
                    println!();
                    return Ok(());
                }
                break;
            }
            // A blank line while continuing cancels the partial input.
            if !buf.is_empty() && line.trim().is_empty() && !input_complete(&buf) {
                cancelled = true;
                break;
            }
            buf.push_str(&line);
            if input_complete(&buf) {
                break;
            }
        }
        if cancelled {
            println!("  (cancelled)");
            continue;
        }
        let input = buf.trim().to_string();
        if input.is_empty() {
            continue;
        }
        history.push(input.clone());

        if let Some(rest) = input.strip_prefix(':') {
            let mut parts = rest.splitn(2, char::is_whitespace);
            let cmd = parts.next().unwrap_or("");
            let arg = parts.next().unwrap_or("").trim();
            match cmd {
                "quit" | "q" => break,
                "help" | "h" => print_repl_help(),
                "rules" => list_rules(&session, opts),
                "samjnas" => {
                    for s in &session.samjnas {
                        println!("  {}", s.name);
                    }
                    if session.samjnas.is_empty() {
                        println!("  (no session saṃjñās; the stdlib ones are still in scope)");
                    }
                }
                "classes" => {
                    for c in &session.classes {
                        println!("  गण {}", c.name);
                    }
                    for s in &session.seq {
                        println!("  क्रम {}", s.name);
                    }
                    if session.classes.is_empty() && session.seq.is_empty() {
                        println!("  (no session गण/क्रम; stdlib परात्याहार classes अच्/हल्/… still in scope)");
                    }
                }
                "type" => {
                    let prog = combined(&base, &session);
                    match parser::parse_expr(arg) {
                        Ok(t) => {
                            let nf = Engine::new(&prog, opts.fuel).normalize(&t).term;
                            let names = samjna::classify(&prog, &nf);
                            let ty = if names.is_empty() { "(none)".into() } else { names.join(", ") };
                            println!("  {} : {}", pretty::show(&nf, opts.ascii), ty);
                        }
                        Err(e) => println!("  {}", e),
                    }
                }
                "load" => match load_file(Path::new(arg)) {
                    Ok(p) => {
                        session.extend(p);
                        println!("  loaded {} ({} rules now in session)", arg, session.rules.len());
                    }
                    Err(e) => println!("  {}", e),
                },
                "check" => {
                    let prog = combined(&base, &session);
                    let diags = sutra::check::check(&prog, &session);
                    if diags.is_empty() {
                        println!("  no problems found.");
                    }
                    for d in &diags {
                        let tag = if d.severity == Severity::Error { "error" } else { "warning" };
                        println!("  {}: {}", tag, d.msg);
                    }
                }
                "reset" => {
                    session = Program::default();
                    cur_module = None;
                    println!("  session definitions cleared.");
                }
                "history" => {
                    for (i, h) in history.iter().enumerate() {
                        println!("  {:>3}  {}", i + 1, h);
                    }
                }
                other => println!("  unknown command {:?} (try :help)", other),
            }
            continue;
        }

        // A declaration (or block of them) vs. a bare expression.
        if starts_declaration(&input) {
            match parser::parse_program_in(&input, cur_module.clone()) {
                Ok(mut parsed) => {
                    // A section opened in this input stays in effect afterwards.
                    if let Some(m) = parser::trailing_module(&input) {
                        cur_module = Some(m);
                    }
                    let new_prayogas = std::mem::take(&mut parsed.prayogas);
                    session.extend(parsed);
                    let prog = combined(&base, &session);
                    let engine = Engine::new(&prog, opts.fuel);
                    for p in &new_prayogas {
                        print_result(&prog, p, &engine.normalize(p), opts);
                    }
                    if new_prayogas.is_empty() {
                        println!("  defined.");
                    }
                }
                Err(e) => println!("  {}", e),
            }
        } else {
            let prog = combined(&base, &session);
            match parser::parse_expr(&input) {
                Ok(t) => {
                    let engine = Engine::new(&prog, opts.fuel);
                    print_result(&prog, &t, &engine.normalize(&t), opts);
                }
                Err(e) => println!("  {}", e),
            }
        }
    }
    Ok(())
}

fn combined(base: &Program, session: &Program) -> Program {
    let mut p = base.clone();
    p.extend(session.clone());
    p
}

/// Does the input begin with a declaration keyword (so it should be parsed as
/// declarations rather than an expression)?
fn starts_declaration(input: &str) -> bool {
    let kw = input.split_whitespace().next().unwrap_or("");
    matches!(
        kw,
        "सूत्र" | "fn" | "sutra"
            | "संज्ञा" | "type" | "samjna"
            | "गण" | "class" | "gana"
            | "क्रम" | "seq" | "sequence" | "krama"
            | "शिवसूत्र" | "shivasutra" | "sivasutra" | "inventory"
            | "अधिकार" | "section" | "adhikara"
            | "प्रयोग" | "eval" | "prayoga"
            | "उपयोग" | "import" | "use" | "upayoga"
    )
}

/// Heuristic completeness for multi-line input: all brackets balanced, no
/// unterminated string, and a declaration has reached its terminator.
fn input_complete(buf: &str) -> bool {
    let toks = match sutra::lexer::lex(buf) {
        Ok(t) => t,
        Err(_) => return false, // e.g. an unterminated string literal
    };
    use sutra::lexer::Tok;
    let mut depth: i32 = 0;
    let mut has_danda = false;
    let mut has_close_brace = false;
    for t in &toks {
        match &t.tok {
            Tok::LParen | Tok::LBrack | Tok::LBrace => depth += 1,
            Tok::RParen | Tok::RBrack => depth -= 1,
            Tok::RBrace => {
                depth -= 1;
                has_close_brace = true;
            }
            Tok::Danda => has_danda = true,
            _ => {}
        }
    }
    if depth > 0 {
        return false;
    }
    // A trailing token that cannot end an input means more is coming, e.g.
    // `2 +`, `f(x) ->`, `[1,`.
    if let Some(last) = toks.iter().rev().find(|t| !matches!(t.tok, Tok::Comment(_) | Tok::Eof)) {
        if matches!(
            last.tok,
            Tok::Op(_)
                | Tok::Arrow
                | Tok::FatArrow
                | Tok::LArrow
                | Tok::Define
                | Tok::Eq
                | Tok::Bar
                | Tok::Comma
                | Tok::Colon
                | Tok::Dot
        ) {
            return false;
        }
    }
    // A daṇḍa-terminated declaration is incomplete until its daṇḍa appears;
    // a क्रम/शिवसूत्र block is complete once its brace closes.
    if starts_declaration(buf.trim()) && !has_danda && !has_close_brace {
        return false;
    }
    true
}

fn print_repl_help() {
    println!("  commands:");
    println!("    :help            this message");
    println!("    :type EXPR       evaluate EXPR and show the saṃjñās it inhabits");
    println!("    :rules           list session rules (your definitions)");
    println!("    :samjnas         list saṃjñā types");
    println!("    :classes         list गण classes and क्रम systems");
    println!("    :load FILE       load a .sutra file into the session");
    println!("    :check           statically check the session definitions");
    println!("    :reset           clear session definitions");
    println!("    :history         show input history");
    println!("    :quit            exit");
    println!("  otherwise: type a term to evaluate, or a declaration to define one.");
    println!("  multi-line input continues until brackets balance and a daṇḍा (।) is typed.");
}

fn list_rules(session: &Program, opts: &Options) {
    if session.rules.is_empty() {
        println!("  (no session rules yet — define one, e.g. `सूत्र द्विगुण(?x) -> ?x * 2।`)");
        return;
    }
    for r in &session.rules {
        let m = r.module.as_deref().map(|m| format!("[{}] ", m)).unwrap_or_default();
        let guard = match &r.guard {
            Some(g) => format!(" | {}", pretty::show(g, opts.ascii)),
            None => String::new(),
        };
        println!(
            "  {}{}{} -> {}",
            m,
            pretty::show(&r.lhs, opts.ascii),
            guard,
            pretty::show(&r.rhs, opts.ascii)
        );
    }
}

fn print_result(prog: &Program, input: &sutra::Term, outcome: &sutra::Outcome, opts: &Options) {
    print!("{}  ⇒  {}", pretty::show(input, opts.ascii), pretty::show(&outcome.term, opts.ascii));
    if opts.steps {
        print!("   [{} steps]", outcome.steps);
    }
    if outcome.out_of_fuel {
        print!("   ⚠ out of fuel");
    }
    println!();
    if opts.check {
        let names = samjna::classify(prog, &outcome.term);
        println!("    : {}", if names.is_empty() { "(none)".into() } else { names.join(", ") });
    }
}
