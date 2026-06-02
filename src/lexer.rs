//! The lexer turns Sūtra source text into a token stream. Source is UTF-8;
//! identifiers may be Devanagari or Latin, and keywords are bilingual.

#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    // Keywords.
    KwSutra,
    KwSamjna,
    KwAdhikara,
    KwPrayoga,
    KwImport,
    KwLet,
    KwIn,
    KwIf,
    KwThen,
    KwElse,

    // Literals & names.
    Ident(String),
    Var(String),
    Str(String),
    Int(i64),
    Float(f64),

    // Fixed punctuation.
    Arrow,    // ->  →   (rule)
    FatArrow, // =>       (lambda)
    Define,   // :=       (saṃjñā)
    Eq,       // =        (let binding)
    Bar,      // |        (saṃjñā alternation)
    LParen,
    RParen,
    LBrack,
    RBrack,
    LBrace,
    RBrace,
    Colon, // :  (map/record key separator)
    Dot,   // .  (field access)
    Comma,
    Danda, // । ॥ ;

    /// A binary/unary operator lexeme (handled by the Pratt parser).
    Op(String),

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

fn devanagari_digit(c: char) -> Option<u32> {
    if ('\u{0966}'..='\u{096F}').contains(&c) {
        Some(c as u32 - 0x0966)
    } else {
        None
    }
}

fn digit_val(c: char) -> Option<u32> {
    c.to_digit(10).or_else(|| devanagari_digit(c))
}

fn is_digit(c: char) -> bool {
    digit_val(c).is_some()
}

fn ident_start(c: char) -> bool {
    c == '_' || c.is_alphabetic()
}

/// Identifier continuation: accept the Devanagari block (mātrās, virama for
/// conjuncts) but not the daṇḍa punctuation.
fn ident_continue(c: char) -> bool {
    if c == '_' || c == '\u{200C}' || c == '\u{200D}' {
        return true;
    }
    if c == '\u{0964}' || c == '\u{0965}' {
        return false; // daṇḍa, double daṇḍa
    }
    if ('\u{0900}'..='\u{097F}').contains(&c) {
        return true;
    }
    c.is_alphanumeric()
}

/// Multi-character operators, longest first.
const MULTI_OPS: &[&str] = &[
    ">>=", "->", "=>", ":=", "==", "!=", "<=", ">=", "&&", "||", "++", "::", "|>", ">>",
];

pub fn lex(src: &str) -> Result<Vec<Token>, LexError> {
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut line = 1;
    let mut out = Vec::new();

    while i < chars.len() {
        let c = chars[i];

        if c == '\n' {
            line += 1;
            i += 1;
            continue;
        }
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c == '#' {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        // Multi-character operators / fixed lexemes.
        let mut matched = false;
        for op in MULTI_OPS {
            let opn = op.chars().count();
            if i + opn <= chars.len() && chars[i..i + opn].iter().collect::<String>() == **op {
                let tok = match *op {
                    "->" => Tok::Arrow,
                    "=>" => Tok::FatArrow,
                    ":=" => Tok::Define,
                    other => Tok::Op(other.to_string()),
                };
                out.push(Token { tok, line });
                i += opn;
                matched = true;
                break;
            }
        }
        if matched {
            continue;
        }

        // Single-character punctuation & operators.
        let single: Option<Tok> = match c {
            '(' => Some(Tok::LParen),
            ')' => Some(Tok::RParen),
            '[' => Some(Tok::LBrack),
            ']' => Some(Tok::RBrack),
            '{' => Some(Tok::LBrace),
            '}' => Some(Tok::RBrace),
            ':' => Some(Tok::Colon),
            '.' => Some(Tok::Dot),
            ',' => Some(Tok::Comma),
            '|' => Some(Tok::Bar),
            '=' => Some(Tok::Eq),
            '।' | '॥' | ';' => Some(Tok::Danda),
            '→' => Some(Tok::Arrow),
            '+' | '-' | '*' | '/' | '%' | '<' | '>' | '!' => Some(Tok::Op(c.to_string())),
            _ => None,
        };
        if let Some(tok) = single {
            out.push(Token { tok, line });
            i += 1;
            continue;
        }

        // String literal.
        if c == '"' {
            i += 1;
            let mut s = String::new();
            loop {
                if i >= chars.len() {
                    return Err(LexError { msg: "unterminated string literal".into(), line });
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
                        '\\' => '\\',
                        '"' => '"',
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
                return Err(LexError { msg: "expected variable name after '?'".into(), line });
            }
            let mut name = String::new();
            while i < chars.len() && ident_continue(chars[i]) {
                name.push(chars[i]);
                i += 1;
            }
            out.push(Token { tok: Tok::Var(name), line });
            continue;
        }

        // Numeric literal (Int or Float).
        if is_digit(c) {
            let mut int_part = String::new();
            while i < chars.len() && is_digit(chars[i]) {
                let d = digit_val(chars[i]).unwrap();
                int_part.push(char::from_digit(d, 10).unwrap());
                i += 1;
            }
            // Float? a '.' followed by a digit.
            if i + 1 < chars.len() && chars[i] == '.' && is_digit(chars[i + 1]) {
                let mut frac = String::new();
                i += 1; // consume '.'
                while i < chars.len() && is_digit(chars[i]) {
                    let d = digit_val(chars[i]).unwrap();
                    frac.push(char::from_digit(d, 10).unwrap());
                    i += 1;
                }
                let f: f64 = format!("{}.{}", int_part, frac)
                    .parse()
                    .map_err(|_| LexError { msg: "invalid float".into(), line })?;
                out.push(Token { tok: Tok::Float(f), line });
            } else {
                let n: i64 = int_part
                    .parse()
                    .map_err(|_| LexError { msg: "integer literal out of range".into(), line })?;
                out.push(Token { tok: Tok::Int(n), line });
            }
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
                "sutra" | "सूत्र" | "fn" => Tok::KwSutra,
                "samjna" | "संज्ञा" | "type" => Tok::KwSamjna,
                "adhikara" | "अधिकार" | "section" => Tok::KwAdhikara,
                "prayoga" | "प्रयोग" | "eval" => Tok::KwPrayoga,
                "upayoga" | "उपयोग" | "import" | "use" => Tok::KwImport,
                "astu" | "अस्तु" | "let" => Tok::KwLet,
                "atah" | "अतः" | "in" => Tok::KwIn,
                "cet" | "चेत्" | "if" => Tok::KwIf,
                "tarhi" | "तर्हि" | "then" => Tok::KwThen,
                "anyatha" | "अन्यथा" | "else" => Tok::KwElse,
                _ => Tok::Ident(name),
            };
            out.push(Token { tok, line });
            continue;
        }

        return Err(LexError { msg: format!("unexpected character {:?}", c), line });
    }

    out.push(Token { tok: Tok::Eof, line });
    Ok(out)
}
