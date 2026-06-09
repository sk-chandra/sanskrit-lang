//! `sutra fmt` — a canonical formatter for `.sutra` source.
//!
//! Deliberately conservative: it normalizes **whitespace and indentation
//! only**. It preserves the author's line breaks, comments (including trailing
//! ones), and spelling choices — `fn` vs `सूत्र`, `->` vs `→`, `।` vs `;` —
//! via the raw text kept on tokens. Numeric literals are the one
//! canonicalization (rendered in ASCII digits).
//!
//! Safety: [`format_source`] verifies that the output lexes to exactly the
//! same token stream as the input and refuses to return otherwise, so it can
//! never change a program's meaning.

use crate::lexer::{lex, lex_full, LexError, Tok, Token};

/// Format Sūtra source. Returns an error string for unlexable input or
/// (should-never-happen) a formatting bug that altered the token stream.
pub fn format_source(src: &str) -> Result<String, String> {
    let toks = lex_full(src).map_err(|e: LexError| e.to_string())?;
    let out = render(&toks);
    // Verify: formatting must not change the program.
    let before: Vec<Tok> = lex(src).map_err(|e| e.to_string())?.into_iter().map(|t| t.tok).collect();
    let after: Vec<Tok> = lex(&out)
        .map_err(|e| format!("formatter produced unlexable output: {}", e))?
        .into_iter()
        .map(|t| t.tok)
        .collect();
    if before != after {
        return Err("internal error: formatting would change the token stream; refusing".into());
    }
    Ok(out)
}

fn render(toks: &[Token]) -> String {
    let toks: Vec<&Token> = toks.iter().filter(|t| t.tok != Tok::Eof).collect();
    let unary = unary_flags(&toks);

    let mut out = String::new();
    let mut depth: usize = 0;
    // For each open brace: is it a block (क्रम/do) rather than a map literal?
    let mut brace_blocks: Vec<bool> = Vec::new();
    // Bracket stack (true = brace), to know when we're inside a block.
    let mut stack: Vec<bool> = Vec::new();
    // Are we inside a declaration that started on an earlier line?
    let mut in_decl = false;

    let mut i = 0;
    let mut prev_line = 0usize; // 0 = nothing emitted yet
    while i < toks.len() {
        // Gather one source line of tokens.
        let line_no = toks[i].line;
        let mut j = i;
        while j < toks.len() && toks[j].line == line_no {
            j += 1;
        }
        let line = &toks[i..j];

        // Blank-line policy: keep at most one.
        if prev_line != 0 {
            out.push('\n');
            if line_no > prev_line + 1 {
                out.push('\n');
            }
        }

        // A declaration keyword at top level starts a new declaration.
        let starts_decl = depth == 0 && is_decl_keyword(&line[0].tok);
        if starts_decl {
            in_decl = true;
        }

        // Indent: bracket depth, plus one level for the continuation lines of
        // a declaration not inside a `{ }` block (whose depth already indents).
        let mut this_indent = depth;
        if matches!(line[0].tok, Tok::RParen | Tok::RBrack | Tok::RBrace) {
            this_indent = this_indent.saturating_sub(1);
        }
        if in_decl && !starts_decl && !stack.contains(&true) {
            this_indent += 1;
        }
        out.push_str(&"  ".repeat(this_indent));

        // Render the line's tokens with spacing.
        for (k, t) in line.iter().enumerate() {
            let idx = i + k;
            if k > 0 {
                let prev = &line[k - 1].tok;
                let prev2 = if k >= 2 { Some(&line[k - 2].tok) } else { None };
                if let Tok::Comment(_) = t.tok {
                    out.push_str("  "); // trailing comment: two spaces
                } else if needs_space(prev, prev2, &t.tok, unary[idx - 1], &brace_blocks) {
                    out.push(' ');
                }
            }
            out.push_str(&token_text(t));

            // Track bracket depth, brace kinds, and declaration ends.
            match t.tok {
                Tok::LParen | Tok::LBrack => {
                    depth += 1;
                    stack.push(false);
                }
                Tok::LBrace => {
                    depth += 1;
                    stack.push(true);
                    brace_blocks.push(is_block_brace(&toks, idx));
                }
                Tok::RParen | Tok::RBrack => {
                    depth = depth.saturating_sub(1);
                    stack.pop();
                }
                Tok::RBrace => {
                    depth = depth.saturating_sub(1);
                    stack.pop();
                    brace_blocks.pop();
                    if depth == 0 {
                        in_decl = false; // a क्रम block just closed
                    }
                }
                Tok::Danda if depth == 0 => in_decl = false,
                _ => {}
            }
        }

        prev_line = line_no;
        i = j;
    }
    out.push('\n');
    out
}

/// Is the `{` at `idx` a block brace (after `do`, or a `क्रम NAME` header)
/// rather than a map literal? Block braces get inner padding on one-liners.
fn is_block_brace(toks: &[&Token], idx: usize) -> bool {
    if idx == 0 {
        return false;
    }
    match &toks[idx - 1].tok {
        Tok::KwDo => true,
        Tok::Ident(_) => idx >= 2 && matches!(toks[idx - 2].tok, Tok::KwKrama),
        _ => false,
    }
}

/// Precompute, for every token, whether it is a *unary* `-`/`!` (no space
/// after it). Comments are transparent to this judgement.
fn unary_flags(toks: &[&Token]) -> Vec<bool> {
    let mut flags = vec![false; toks.len()];
    for (i, t) in toks.iter().enumerate() {
        let is_candidate = matches!(&t.tok, Tok::Op(o) if o == "-" || o == "!");
        if !is_candidate {
            continue;
        }
        let prev = toks[..i].iter().rev().find(|p| !matches!(p.tok, Tok::Comment(_)));
        flags[i] = match prev.map(|p| &p.tok) {
            None => true,
            Some(
                Tok::Op(_)
                | Tok::Arrow
                | Tok::FatArrow
                | Tok::LArrow
                | Tok::Define
                | Tok::Eq
                | Tok::Bar
                | Tok::Comma
                | Tok::Colon
                | Tok::LParen
                | Tok::LBrack
                | Tok::LBrace
                | Tok::Danda,
            ) => true,
            Some(t) if is_keyword(t) => true,
            _ => false,
        };
    }
    flags
}

fn is_decl_keyword(t: &Tok) -> bool {
    matches!(
        t,
        Tok::KwSutra
            | Tok::KwSamjna
            | Tok::KwAdhikara
            | Tok::KwPrayoga
            | Tok::KwImport
            | Tok::KwKrama
            | Tok::KwGana
    )
}

fn is_keyword(t: &Tok) -> bool {
    matches!(
        t,
        Tok::KwSutra
            | Tok::KwSamjna
            | Tok::KwAdhikara
            | Tok::KwPrayoga
            | Tok::KwImport
            | Tok::KwKrama
            | Tok::KwGana
            | Tok::KwLet
            | Tok::KwIn
            | Tok::KwIf
            | Tok::KwThen
            | Tok::KwElse
            | Tok::KwDo
    )
}

fn needs_space(
    prev: &Tok,
    prev2: Option<&Tok>,
    cur: &Tok,
    prev_is_unary: bool,
    brace_blocks: &[bool],
) -> bool {
    // Tight before closers and separators.
    match cur {
        Tok::Comma | Tok::Danda | Tok::RParen | Tok::RBrack | Tok::Colon | Tok::Dot => return false,
        Tok::RBrace => return brace_blocks.last().copied().unwrap_or(false),
        _ => {}
    }
    // Tight after openers and field access.
    match prev {
        Tok::LParen | Tok::LBrack | Tok::Dot => return false,
        Tok::LBrace => return brace_blocks.last().copied().unwrap_or(false),
        // `?v:गण` stays tight; a map key's `:` gets a space after.
        Tok::Colon => return !matches!(prev2, Some(Tok::Var(_))),
        _ => {}
    }
    // Calls and postfix application: `f(x)`, `?f(x)`, `g(1)(2)`.
    if matches!(cur, Tok::LParen)
        && matches!(prev, Tok::Ident(_) | Tok::Var(_) | Tok::RParen | Tok::RBrack)
    {
        return false;
    }
    // A unary - or ! hugs its operand.
    if prev_is_unary {
        return false;
    }
    true
}

fn token_text(t: &Token) -> String {
    if let Some(raw) = &t.raw {
        return raw.clone();
    }
    match &t.tok {
        Tok::Ident(s) => s.clone(),
        Tok::Var(v) => format!("?{}", v),
        Tok::Str(s) => {
            let mut out = String::from("\"");
            for c in s.chars() {
                match c {
                    '\\' => out.push_str("\\\\"),
                    '"' => out.push_str("\\\""),
                    '\n' => out.push_str("\\n"),
                    '\t' => out.push_str("\\t"),
                    other => out.push(other),
                }
            }
            out.push('"');
            out
        }
        Tok::Int(n) => n.to_string(),
        Tok::Big(b) => b.to_decimal_string(),
        Tok::Float(f) => {
            let s = format!("{}", f);
            if s.contains(['.', 'e', 'E']) {
                s
            } else {
                format!("{}.0", s)
            }
        }
        Tok::Comment(text) => {
            if text.is_empty() {
                "#".to_string()
            } else {
                format!("# {}", text)
            }
        }
        Tok::LParen => "(".into(),
        Tok::RParen => ")".into(),
        Tok::LBrack => "[".into(),
        Tok::RBrack => "]".into(),
        Tok::LBrace => "{".into(),
        Tok::RBrace => "}".into(),
        Tok::Comma => ",".into(),
        Tok::Dot => ".".into(),
        Tok::Colon => ":".into(),
        Tok::Bar => "|".into(),
        Tok::Eq => "=".into(),
        Tok::Danda => "।".into(),
        Tok::Arrow => "->".into(),
        Tok::FatArrow => "=>".into(),
        Tok::LArrow => "<-".into(),
        Tok::Define => ":=".into(),
        Tok::Op(o) => o.clone(),
        // Keywords always carry raw; canonical fallbacks for safety.
        Tok::KwSutra => "सूत्र".into(),
        Tok::KwSamjna => "संज्ञा".into(),
        Tok::KwAdhikara => "अधिकार".into(),
        Tok::KwPrayoga => "प्रयोग".into(),
        Tok::KwImport => "उपयोग".into(),
        Tok::KwKrama => "क्रम".into(),
        Tok::KwGana => "गण".into(),
        Tok::KwLet => "अस्तु".into(),
        Tok::KwIn => "अतः".into(),
        Tok::KwIf => "चेत्".into(),
        Tok::KwThen => "तर्हि".into(),
        Tok::KwElse => "अन्यथा".into(),
        Tok::KwDo => "क्रिया".into(),
        Tok::Eof => String::new(),
    }
}
