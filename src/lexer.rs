//! The lexer (पद-विभाग) turns Sūtra source text into a token stream.
//!
//! Sūtra source is UTF-8 and identifiers may be written in Devanagari or Latin
//! (or mixed). Keywords are recognised in both scripts.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Tok {
    /// `सूत्र` / `sutra` — introduces a rewrite rule.
    KwSutra,
    /// `संज्ञा` / `samjna` — introduces a type / grammar production.
    KwSamjna,
    /// `अधिकार` / `adhikara` — a section header (organisational).
    KwAdhikara,
    /// `प्रयोग` / `prayoga` — an expression to evaluate and print.
    KwPrayoga,

    Ident(String),
    Var(String),
    Str(String),
    Numeral(u128),

    Arrow,  // -> or →
    Define, // :=
    Bar,    // |
    LParen,
    RParen,
    Comma,
    Danda, // । or ॥ or ;

    Eof,
}

#[derive(Clone, Debug)]
pub struct Token {
    pub tok: Tok,
    pub line: usize,
}

#[derive(Debug)]
pub struct LexError {
    pub msg: String,
    pub line: usize,
}

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "lex error (line {}): {}", self.line, self.msg)
    }
}

/// Devanagari digit `०`..`९` to its numeric value, if applicable.
fn devanagari_digit(c: char) -> Option<u32> {
    if ('\u{0966}'..='\u{096F}').contains(&c) {
        Some(c as u32 - 0x0966)
    } else {
        None
    }
}

fn is_digit(c: char) -> bool {
    c.is_ascii_digit() || devanagari_digit(c).is_some()
}

/// Can a character start an identifier?
fn ident_start(c: char) -> bool {
    c == '_' || c.is_alphabetic()
}

/// Can a character continue an identifier?
///
/// Besides ordinary alphanumerics we accept the Devanagari block (matras,
/// virama / halant for conjuncts, etc.) so that words like `रिक्त` or
/// `क्रमगुणित` lex as a single identifier — but we exclude the daṇḍa
/// punctuation `।`/`॥` which terminate declarations.
fn ident_continue(c: char) -> bool {
    if c == '_' || c == '\u{200C}' || c == '\u{200D}' {
        return true; // ZWNJ / ZWJ
    }
    if c == '\u{0964}' || c == '\u{0965}' {
        return false; // daṇḍa, double daṇḍa
    }
    if ('\u{0900}'..='\u{097F}').contains(&c) {
        return true; // rest of the Devanagari block
    }
    c.is_alphanumeric()
}

pub fn lex(src: &str) -> Result<Vec<Token>, LexError> {
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut line = 1;
    let mut out = Vec::new();

    while i < chars.len() {
        let c = chars[i];

        // Whitespace.
        if c == '\n' {
            line += 1;
            i += 1;
            continue;
        }
        if c.is_whitespace() {
            i += 1;
            continue;
        }

        // Comments: `#` to end of line.
        if c == '#' {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        // Single-character punctuation.
        match c {
            '(' => {
                out.push(Token { tok: Tok::LParen, line });
                i += 1;
                continue;
            }
            ')' => {
                out.push(Token { tok: Tok::RParen, line });
                i += 1;
                continue;
            }
            ',' => {
                out.push(Token { tok: Tok::Comma, line });
                i += 1;
                continue;
            }
            '|' => {
                out.push(Token { tok: Tok::Bar, line });
                i += 1;
                continue;
            }
            '।' | '॥' | ';' => {
                out.push(Token { tok: Tok::Danda, line });
                i += 1;
                continue;
            }
            '→' => {
                out.push(Token { tok: Tok::Arrow, line });
                i += 1;
                continue;
            }
            _ => {}
        }

        // Arrow `->`.
        if c == '-' {
            if i + 1 < chars.len() && chars[i + 1] == '>' {
                out.push(Token { tok: Tok::Arrow, line });
                i += 2;
                continue;
            }
            return Err(LexError {
                msg: "stray '-' (did you mean '->'?)".into(),
                line,
            });
        }

        // Define `:=`.
        if c == ':' {
            if i + 1 < chars.len() && chars[i + 1] == '=' {
                out.push(Token { tok: Tok::Define, line });
                i += 2;
                continue;
            }
            return Err(LexError {
                msg: "stray ':' (did you mean ':='?)".into(),
                line,
            });
        }

        // String literal.
        if c == '"' {
            i += 1;
            let mut s = String::new();
            loop {
                if i >= chars.len() {
                    return Err(LexError {
                        msg: "unterminated string literal".into(),
                        line,
                    });
                }
                let d = chars[i];
                if d == '"' {
                    i += 1;
                    break;
                }
                if d == '\\' && i + 1 < chars.len() {
                    let e = chars[i + 1];
                    s.push(match e {
                        'n' => '\n',
                        't' => '\t',
                        other => other,
                    });
                    i += 2;
                    continue;
                }
                if d == '\n' {
                    line += 1;
                }
                s.push(d);
                i += 1;
            }
            out.push(Token { tok: Tok::Str(s), line });
            continue;
        }

        // Variable `?name`.
        if c == '?' {
            i += 1;
            if i >= chars.len() || !ident_start(chars[i]) {
                return Err(LexError {
                    msg: "expected variable name after '?'".into(),
                    line,
                });
            }
            let mut name = String::new();
            while i < chars.len() && ident_continue(chars[i]) {
                name.push(chars[i]);
                i += 1;
            }
            out.push(Token { tok: Tok::Var(name), line });
            continue;
        }

        // Numeral (ASCII or Devanagari digits).
        if is_digit(c) {
            let mut val: u128 = 0;
            while i < chars.len() && is_digit(chars[i]) {
                let d = if let Some(v) = devanagari_digit(chars[i]) {
                    v
                } else {
                    chars[i].to_digit(10).unwrap()
                };
                val = val
                    .checked_mul(10)
                    .and_then(|v| v.checked_add(d as u128))
                    .ok_or_else(|| LexError {
                        msg: "numeral too large".into(),
                        line,
                    })?;
                i += 1;
            }
            out.push(Token { tok: Tok::Numeral(val), line });
            continue;
        }

        // Identifier or keyword.
        if ident_start(c) {
            let mut name = String::new();
            while i < chars.len() && ident_continue(chars[i]) {
                name.push(chars[i]);
                i += 1;
            }
            let tok = match name.as_str() {
                "sutra" | "सूत्र" => Tok::KwSutra,
                "samjna" | "संज्ञा" => Tok::KwSamjna,
                "adhikara" | "अधिकार" => Tok::KwAdhikara,
                "prayoga" | "प्रयोग" => Tok::KwPrayoga,
                _ => Tok::Ident(name),
            };
            out.push(Token { tok, line });
            continue;
        }

        return Err(LexError {
            msg: format!("unexpected character {:?}", c),
            line,
        });
    }

    out.push(Token { tok: Tok::Eof, line });
    Ok(out)
}
