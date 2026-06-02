//! The parser (व्याकरण) builds a [`Program`] from a token stream.

use crate::ast::{Program, Rule, Samjna, Term};
use crate::lexer::{lex, Tok, Token};

#[derive(Debug)]
pub struct ParseError {
    pub msg: String,
    pub line: usize,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "parse error (line {}): {}", self.line, self.msg)
    }
}

impl From<crate::lexer::LexError> for ParseError {
    fn from(e: crate::lexer::LexError) -> Self {
        ParseError { msg: e.msg, line: e.line }
    }
}

struct Parser {
    toks: Vec<Token>,
    pos: usize,
    order: usize,
}

impl Parser {
    fn peek(&self) -> &Tok {
        &self.toks[self.pos].tok
    }

    fn line(&self) -> usize {
        self.toks[self.pos].line
    }

    fn bump(&mut self) -> Tok {
        let t = self.toks[self.pos].tok.clone();
        if self.pos + 1 < self.toks.len() {
            self.pos += 1;
        }
        t
    }

    fn err<T>(&self, msg: impl Into<String>) -> Result<T, ParseError> {
        Err(ParseError { msg: msg.into(), line: self.line() })
    }

    fn expect(&mut self, want: &Tok, what: &str) -> Result<(), ParseError> {
        if self.peek() == want {
            self.bump();
            Ok(())
        } else {
            self.err(format!("expected {}, found {:?}", what, self.peek()))
        }
    }

    fn program(&mut self) -> Result<Program, ParseError> {
        let mut prog = Program::default();
        loop {
            match self.peek() {
                Tok::Eof => break,
                Tok::KwSutra => {
                    let r = self.rule()?;
                    prog.rules.push(r);
                }
                Tok::KwSamjna => {
                    let s = self.samjna()?;
                    prog.samjnas.push(s);
                }
                Tok::KwAdhikara => {
                    // Section header: `अधिकार name।` — organisational only in v1.
                    self.bump();
                    self.atom_name()?;
                    self.expect(&Tok::Danda, "daṇḍa '।' after section name")?;
                }
                Tok::KwPrayoga => {
                    self.bump();
                    let t = self.term()?;
                    self.expect(&Tok::Danda, "daṇḍa '।' after प्रयोग expression")?;
                    prog.prayogas.push(t);
                }
                other => {
                    return self.err(format!(
                        "expected a declaration (सूत्र / संज्ञा / अधिकार / प्रयोग), found {:?}",
                        other
                    ))
                }
            }
        }
        Ok(prog)
    }

    fn rule(&mut self) -> Result<Rule, ParseError> {
        self.expect(&Tok::KwSutra, "सूत्र")?;
        let lhs = self.term()?;
        self.expect(&Tok::Arrow, "arrow '->'")?;
        let rhs = self.term()?;
        self.expect(&Tok::Danda, "daṇḍa '।' after rule")?;
        let order = self.order;
        self.order += 1;
        Ok(Rule { lhs, rhs, order })
    }

    fn samjna(&mut self) -> Result<Samjna, ParseError> {
        self.expect(&Tok::KwSamjna, "संज्ञा")?;
        let name = self.atom_name()?;
        let mut params = Vec::new();
        if self.peek() == &Tok::LParen {
            self.bump();
            if self.peek() != &Tok::RParen {
                loop {
                    match self.bump() {
                        Tok::Var(v) => params.push(v),
                        Tok::Ident(v) => params.push(v),
                        other => return self.err(format!("expected type parameter, found {:?}", other)),
                    }
                    if self.peek() == &Tok::Comma {
                        self.bump();
                    } else {
                        break;
                    }
                }
            }
            self.expect(&Tok::RParen, "')'")?;
        }
        self.expect(&Tok::Define, "':='")?;
        let mut alts = vec![self.term()?];
        while self.peek() == &Tok::Bar {
            self.bump();
            alts.push(self.term()?);
        }
        self.expect(&Tok::Danda, "daṇḍa '।' after संज्ञा")?;
        Ok(Samjna { name, params, alts })
    }

    /// Parse a bare identifier name (used for section / saṃjñā names).
    fn atom_name(&mut self) -> Result<String, ParseError> {
        match self.bump() {
            Tok::Ident(s) => Ok(s),
            other => self.err(format!("expected a name, found {:?}", other)),
        }
    }

    fn term(&mut self) -> Result<Term, ParseError> {
        match self.bump() {
            Tok::Var(v) => Ok(Term::Var(v)),
            Tok::Str(s) => Ok(Term::Str(s)),
            Tok::Numeral(n) => Ok(Term::nat(n)),
            Tok::Ident(name) => {
                if self.peek() == &Tok::LParen {
                    self.bump();
                    let mut args = Vec::new();
                    if self.peek() != &Tok::RParen {
                        loop {
                            args.push(self.term()?);
                            if self.peek() == &Tok::Comma {
                                self.bump();
                            } else {
                                break;
                            }
                        }
                    }
                    self.expect(&Tok::RParen, "')'")?;
                    Ok(Term::Sym(name, args))
                } else {
                    Ok(Term::Sym(name, vec![]))
                }
            }
            other => self.err(format!("expected a term, found {:?}", other)),
        }
    }
}

/// Parse a full program (a sequence of declarations).
pub fn parse_program(src: &str) -> Result<Program, ParseError> {
    let toks = lex(src)?;
    let mut p = Parser { toks, pos: 0, order: 0 };
    p.program()
}

/// Parse a single expression (used by the REPL and `-e`).
pub fn parse_expr(src: &str) -> Result<Term, ParseError> {
    let toks = lex(src)?;
    let mut p = Parser { toks, pos: 0, order: 0 };
    let t = p.term()?;
    if p.peek() != &Tok::Eof {
        // Allow a trailing daṇḍa.
        if p.peek() == &Tok::Danda {
            p.bump();
        }
        if p.peek() != &Tok::Eof {
            return p.err(format!("unexpected trailing input: {:?}", p.peek()));
        }
    }
    Ok(t)
}
