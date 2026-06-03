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
    let mut positional: Vec<String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
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
        "repl" => ("repl", &positional[1..]),
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

fn cmd_repl(opts: &Options) -> Result<(), String> {
    use std::io::{self, BufRead, Write};
    let prog = base_program(opts)?;
    let engine = Engine::new(&prog, opts.fuel);

    println!("सूत्र REPL — type a term; `:help` for commands, `:quit` to exit.");
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    loop {
        print!("सूत्र> ");
        stdout.flush().ok();
        let mut line = String::new();
        if stdin.lock().read_line(&mut line).map_err(|e| e.to_string())? == 0 {
            println!();
            break;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix(':') {
            let mut parts = rest.splitn(2, char::is_whitespace);
            let cmd = parts.next().unwrap_or("");
            let arg = parts.next().unwrap_or("").trim();
            match cmd {
                "quit" | "q" => break,
                "help" | "h" => println!(
                    ":quit  :rules  :samjnas  :type EXPR  (evaluate by typing a term)"
                ),
                "rules" => {
                    for r in &prog.rules {
                        println!("  {} -> {}", pretty::show(&r.lhs, opts.ascii), pretty::show(&r.rhs, opts.ascii));
                    }
                }
                "samjnas" => {
                    for s in &prog.samjnas {
                        println!("  {}", s.name);
                    }
                }
                "type" => match parser::parse_expr(arg) {
                    Ok(t) => {
                        let nf = engine.normalize(&t).term;
                        let names = samjna::classify(&prog, &nf);
                        let ty = if names.is_empty() { "(none)".into() } else { names.join(", ") };
                        println!("  {} : {}", pretty::show(&nf, opts.ascii), ty);
                    }
                    Err(e) => println!("  {}", e),
                },
                other => println!("  unknown command {:?} (try :help)", other),
            }
            continue;
        }
        match parser::parse_expr(line) {
            Ok(t) => print_result(&prog, &t, &engine.normalize(&t), opts),
            Err(e) => println!("  {}", e),
        }
    }
    Ok(())
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
