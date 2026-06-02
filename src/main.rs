//! The `sutra` command-line interface.
//!
//! Usage:
//!   sutra run FILE.sutra [options]   evaluate every `प्रयोग` in FILE
//!   sutra FILE.sutra      [options]   (same as `run`)
//!   sutra eval "EXPR"     [options]   evaluate a single expression
//!   sutra repl            [options]   start an interactive session
//!
//! Options:
//!   --fuel N        max rewrite steps before giving up (default 1000000)
//!   --ascii         print numerals with Latin digits instead of Devanagari
//!   --no-prelude    do not load the standard library
//!   --check         after each result, report which saṃjñās it inhabits
//!   --steps         report the number of rewrite steps taken

use std::io::{self, BufRead, Write};
use std::process::exit;

use sutra::engine::{Engine, DEFAULT_FUEL};
use sutra::{ast::Program, load_prelude, parser, pretty, samjna};

struct Options {
    fuel: u64,
    ascii: bool,
    no_prelude: bool,
    check: bool,
    steps: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            fuel: DEFAULT_FUEL,
            ascii: false,
            no_prelude: false,
            check: false,
            steps: false,
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("{}", USAGE);
        exit(2);
    }

    let mut opts = Options::default();
    let mut positional: Vec<String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        match a.as_str() {
            "--ascii" => opts.ascii = true,
            "--no-prelude" => opts.no_prelude = true,
            "--check" => opts.check = true,
            "--steps" => opts.steps = true,
            "--fuel" => {
                i += 1;
                let v = args.get(i).and_then(|s| s.parse::<u64>().ok());
                match v {
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
            _ => positional.push(a.clone()),
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
        "repl" => ("repl", &positional[1..]),
        // Bare argument: treat as a file to run.
        _ => ("run", &positional[..]),
    };

    let result = match cmd {
        "run" => cmd_run(rest, &opts),
        "eval" => cmd_eval(rest, &opts),
        "repl" => cmd_repl(&opts),
        _ => unreachable!(),
    };

    if let Err(e) = result {
        eprintln!("{}", e);
        exit(1);
    }
}

const USAGE: &str = "\
सूत्र — Sūtra interpreter

usage:
  sutra run FILE.sutra [options]   evaluate every प्रयोग in FILE
  sutra FILE.sutra     [options]   (same as run)
  sutra eval \"EXPR\"    [options]   evaluate a single expression
  sutra repl           [options]   start an interactive session

options:
  --fuel N        max rewrite steps before giving up (default 1000000)
  --ascii         print numerals with Latin digits
  --no-prelude    do not load the standard library
  --check         report which saṃjñās each result inhabits
  --steps         report the number of rewrite steps taken";

fn base_program(opts: &Options) -> Result<Program, String> {
    if opts.no_prelude {
        Ok(Program::default())
    } else {
        load_prelude()
    }
}

fn cmd_run(rest: &[String], opts: &Options) -> Result<(), String> {
    let path = rest
        .first()
        .ok_or_else(|| "run: expected a file path".to_string())?;
    let src = std::fs::read_to_string(path).map_err(|e| format!("cannot read {}: {}", path, e))?;

    let mut prog = base_program(opts)?;
    let file_prog = parser::parse_program(&src).map_err(|e| e.to_string())?;
    let prayogas = file_prog.prayogas.clone();
    prog.extend(file_prog);

    if prayogas.is_empty() {
        eprintln!(
            "(no प्रयोग expressions in {} — nothing to evaluate. Add e.g. `प्रयोग क्रमगुणित(५)।`)",
            path
        );
        return Ok(());
    }

    let engine = Engine::new(&prog, opts.fuel);
    for expr in &prayogas {
        let outcome = engine.normalize(expr);
        print_result(&prog, expr, &outcome, opts);
    }
    Ok(())
}

fn cmd_eval(rest: &[String], opts: &Options) -> Result<(), String> {
    let expr_src = rest
        .first()
        .ok_or_else(|| "eval: expected an expression".to_string())?;
    let prog = base_program(opts)?;
    let expr = parser::parse_expr(expr_src).map_err(|e| e.to_string())?;
    let engine = Engine::new(&prog, opts.fuel);
    let outcome = engine.normalize(&expr);
    print_result(&prog, &expr, &outcome, opts);
    Ok(())
}

fn cmd_repl(opts: &Options) -> Result<(), String> {
    let prog = base_program(opts)?;
    let engine = Engine::new(&prog, opts.fuel);

    println!("सूत्र REPL — type a term and press enter. `:help` for commands, `:quit` to exit.");
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    loop {
        print!("सूत्र> ");
        stdout.flush().ok();
        let mut line = String::new();
        if stdin.lock().read_line(&mut line).map_err(|e| e.to_string())? == 0 {
            println!();
            break; // EOF
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with(':') {
            let mut parts = line.splitn(2, char::is_whitespace);
            let cmd = parts.next().unwrap();
            let arg = parts.next().unwrap_or("").trim();
            match cmd {
                ":quit" | ":q" => break,
                ":help" | ":h" => {
                    println!(
                        ":quit  exit   |  :rules  list rules  |  :samjnas  list types  |  \
                         :type EXPR  classify a term"
                    );
                }
                ":rules" => {
                    for r in &prog.rules {
                        println!(
                            "  {} -> {}",
                            pretty::show(&r.lhs, opts.ascii),
                            pretty::show(&r.rhs, opts.ascii)
                        );
                    }
                }
                ":samjnas" => {
                    for s in &prog.samjnas {
                        println!("  {}", s.name);
                    }
                }
                ":type" => match parser::parse_expr(arg) {
                    Ok(t) => {
                        let nf = engine.normalize(&t).term;
                        let names = samjna::classify(&prog, &nf);
                        if names.is_empty() {
                            println!("  (inhabits no declared saṃjñā)");
                        } else {
                            println!("  {} : {}", pretty::show(&nf, opts.ascii), names.join(", "));
                        }
                    }
                    Err(e) => println!("  {}", e),
                },
                other => println!("  unknown command {:?} (try :help)", other),
            }
            continue;
        }
        match parser::parse_expr(line) {
            Ok(t) => {
                let outcome = engine.normalize(&t);
                print_result(&prog, &t, &outcome, opts);
            }
            Err(e) => println!("  {}", e),
        }
    }
    Ok(())
}

fn print_result(prog: &Program, input: &sutra::Term, outcome: &sutra::Outcome, opts: &Options) {
    let lhs = pretty::show(input, opts.ascii);
    let rhs = pretty::show(&outcome.term, opts.ascii);
    print!("{}  ⇒  {}", lhs, rhs);
    if opts.steps {
        print!("   [{} steps]", outcome.steps);
    }
    if outcome.out_of_fuel {
        print!("   ⚠ out of fuel (possible non-termination)");
    }
    println!();
    if opts.check {
        let names = samjna::classify(prog, &outcome.term);
        if names.is_empty() {
            println!("    : (inhabits no declared saṃjñā)");
        } else {
            println!("    : {}", names.join(", "));
        }
    }
}
