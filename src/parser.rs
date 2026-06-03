//! The parser builds a [`Program`] from tokens. The surface language has
//! ergonomic sugar — infix operators, `let`, lambdas, `if`, list literals — all
//! of which desugar here into the small core (`Sym` / `App` / `Lam` / literals).

use crate::ast::{Program, Rule, Samjna, Term};
use crate::lexer::{lex, Tok, Token};
use crate::names::canonical;

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

type PResult<T> = Result<T, ParseError>;

/// Precedence and associativity of an infix operator.
fn binding_power(op: &str) -> Option<(u8, bool)> {
    // (precedence, right_associative)
    let bp = match op {
        "|>" => (1, false),
        ">>" | ">>=" => (2, false),
        "||" => (3, false),
        "&&" => (4, false),
        "==" | "!=" | "<" | "<=" | ">" | ">=" => (5, false),
        "::" => (6, true),
        "++" => (7, true),
        "+" | "-" => (8, false),
        "*" | "/" | "%" => (9, false),
        _ => return None,
    };
    Some(bp)
}

/// Build a cons list from elements.
fn cons_list(items: Vec<Term>) -> Term {
    let mut t = Term::con("रिक्त");
    for it in items.into_iter().rev() {
        t = Term::app("युग्म", vec![it, t]);
    }
    t
}

/// Desugar an infix application.
fn binop(op: &str, l: Term, r: Term) -> Term {
    match op {
        "::" => Term::app("युग्म", vec![l, r]),
        "&&" => Term::app("च", vec![l, r]),
        "||" => Term::app("वा", vec![l, r]),
        ">>" => Term::app("अनुक्रम", vec![l, r]),
        ">>=" => Term::app("बन्ध", vec![l, r]),
        "|>" => apply_value(r, vec![l]),
        other => Term::app(other, vec![l, r]),
    }
}

/// Apply a (possibly value) function term to arguments, choosing `Sym` vs `App`.
fn apply_value(f: Term, args: Vec<Term>) -> Term {
    match f {
        // A bare name: a direct symbol application / call.
        Term::Sym(name, prev) if prev.is_empty() => Term::Sym(name, args),
        // Appending to an existing partial application (e.g. for `|>`).
        Term::Sym(name, mut prev) => {
            prev.extend(args);
            Term::Sym(name, prev)
        }
        other => Term::App(Box::new(other), args),
    }
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
    fn err<T>(&self, msg: impl Into<String>) -> PResult<T> {
        Err(ParseError { msg: msg.into(), line: self.line() })
    }
    fn expect(&mut self, want: &Tok, what: &str) -> PResult<()> {
        if self.peek() == want {
            self.bump();
            Ok(())
        } else {
            self.err(format!("expected {}, found {:?}", what, self.peek()))
        }
    }

    // ---- declarations ----

    fn program(&mut self) -> PResult<Program> {
        let mut prog = Program::default();
        loop {
            match self.peek() {
                Tok::Eof => break,
                Tok::KwSutra => prog.rules.push(self.rule()?),
                Tok::KwSamjna => prog.samjnas.push(self.samjna()?),
                Tok::KwAdhikara => {
                    self.bump();
                    self.ident_name()?;
                    self.expect(&Tok::Danda, "daṇḍa after section name")?;
                }
                Tok::KwPrayoga => {
                    self.bump();
                    let t = self.expr()?;
                    self.expect(&Tok::Danda, "daṇḍa after प्रयोग")?;
                    prog.prayogas.push(t);
                }
                Tok::KwImport => {
                    self.bump();
                    match self.bump() {
                        Tok::Str(path) => prog.imports.push(path),
                        other => return self.err(format!("उपयोग expects a \"path\", found {:?}", other)),
                    }
                    self.expect(&Tok::Danda, "daṇḍa after उपयोग")?;
                }
                other => {
                    return self.err(format!("expected a declaration, found {:?}", other))
                }
            }
        }
        Ok(prog)
    }

    fn rule(&mut self) -> PResult<Rule> {
        self.expect(&Tok::KwSutra, "सूत्र")?;
        let lhs = self.expr()?;
        self.expect(&Tok::Arrow, "'->'")?;
        let rhs = self.expr()?;
        self.expect(&Tok::Danda, "daṇḍa after rule")?;
        let order = self.order;
        self.order += 1;
        Ok(Rule { lhs, rhs, order })
    }

    fn samjna(&mut self) -> PResult<Samjna> {
        self.expect(&Tok::KwSamjna, "संज्ञा")?;
        let name = self.ident_name()?;
        let mut params = Vec::new();
        if self.peek() == &Tok::LParen {
            self.bump();
            if self.peek() != &Tok::RParen {
                loop {
                    match self.bump() {
                        Tok::Var(v) | Tok::Ident(v) => params.push(v),
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
        let mut alts = vec![self.expr()?];
        while self.peek() == &Tok::Bar {
            self.bump();
            alts.push(self.expr()?);
        }
        self.expect(&Tok::Danda, "daṇḍa after संज्ञा")?;
        Ok(Samjna { name, params, alts })
    }

    fn ident_name(&mut self) -> PResult<String> {
        match self.bump() {
            Tok::Ident(s) => Ok(canonical(&s)),
            other => self.err(format!("expected a name, found {:?}", other)),
        }
    }

    // ---- expressions (Pratt) ----

    fn expr(&mut self) -> PResult<Term> {
        self.expr_bp(0)
    }

    fn expr_bp(&mut self, min_bp: u8) -> PResult<Term> {
        let mut lhs = self.unary()?;
        loop {
            let op = match self.peek() {
                Tok::Op(o) => o.clone(),
                _ => break,
            };
            let (prec, right) = match binding_power(&op) {
                Some(bp) => bp,
                None => break,
            };
            if prec < min_bp {
                break;
            }
            self.bump(); // operator
            let next_min = if right { prec } else { prec + 1 };
            let rhs = self.expr_bp(next_min)?;
            lhs = binop(&op, lhs, rhs);
        }
        Ok(lhs)
    }

    fn unary(&mut self) -> PResult<Term> {
        if let Tok::Op(o) = self.peek() {
            let o = o.clone();
            if o == "-" || o == "!" {
                self.bump();
                let e = self.unary()?;
                return Ok(match (o.as_str(), e) {
                    ("-", Term::Int(n)) => Term::Int(-n),
                    ("-", Term::Float(f)) => Term::Float(-f),
                    ("-", e) => Term::app("neg", vec![e]),
                    ("!", e) => Term::app("न", vec![e]),
                    _ => unreachable!(),
                });
            }
        }
        self.postfix()
    }

    /// A primary followed by zero or more application argument lists or
    /// field accesses (`f(args)`, `r.field`).
    fn postfix(&mut self) -> PResult<Term> {
        let mut e = self.primary()?;
        loop {
            match self.peek() {
                Tok::LParen => {
                    self.bump();
                    let args = self.arg_list()?;
                    self.expect(&Tok::RParen, "')'")?;
                    e = apply_value(e, args);
                }
                Tok::Dot => {
                    self.bump();
                    let field = match self.bump() {
                        Tok::Ident(s) => s,
                        other => return self.err(format!("expected a field name after '.', found {:?}", other)),
                    };
                    e = Term::app("प्राप्ति", vec![e, Term::Str(field)]);
                }
                _ => break,
            }
        }
        Ok(e)
    }

    fn arg_list(&mut self) -> PResult<Vec<Term>> {
        let mut args = Vec::new();
        if self.peek() != &Tok::RParen {
            loop {
                args.push(self.expr()?);
                if self.peek() == &Tok::Comma {
                    self.bump();
                } else {
                    break;
                }
            }
        }
        Ok(args)
    }

    fn primary(&mut self) -> PResult<Term> {
        match self.peek().clone() {
            Tok::Int(n) => {
                self.bump();
                Ok(Term::Int(n))
            }
            Tok::Big(b) => {
                self.bump();
                Ok(Term::Big(b))
            }
            Tok::Float(f) => {
                self.bump();
                Ok(Term::Float(f))
            }
            Tok::Str(s) => {
                self.bump();
                Ok(Term::Str(s))
            }
            Tok::Var(v) => {
                self.bump();
                Ok(Term::Var(v))
            }
            Tok::Ident(name) => {
                self.bump();
                Ok(Term::Sym(canonical(&name), vec![]))
            }
            Tok::LBrack => self.list_literal(),
            Tok::LBrace => self.map_literal(),
            Tok::LParen => self.paren_or_lambda(),
            Tok::KwLet => self.let_expr(),
            Tok::KwIf => self.if_expr(),
            other => self.err(format!("expected an expression, found {:?}", other)),
        }
    }

    fn list_literal(&mut self) -> PResult<Term> {
        self.expect(&Tok::LBrack, "'['")?;
        let mut items = Vec::new();
        if self.peek() != &Tok::RBrack {
            loop {
                items.push(self.expr()?);
                if self.peek() == &Tok::Comma {
                    self.bump();
                } else {
                    break;
                }
            }
        }
        self.expect(&Tok::RBrack, "']'")?;
        Ok(cons_list(items))
    }

    /// A map / record literal: `{ key: value, … }`. A bare identifier key is a
    /// field name (string key); `{}` is the empty map. Desugars to a chain of
    /// `समावेश` (insert) over `रिक्तकोश` (the empty map).
    fn map_literal(&mut self) -> PResult<Term> {
        self.expect(&Tok::LBrace, "'{'")?;
        let mut entries: Vec<(Term, Term)> = Vec::new();
        if self.peek() != &Tok::RBrace {
            loop {
                let key = match self.peek().clone() {
                    // A bare identifier names a field ⇒ a string key.
                    Tok::Ident(name) => {
                        self.bump();
                        Term::Str(name)
                    }
                    _ => self.expr()?,
                };
                self.expect(&Tok::Colon, "':' in map entry")?;
                let value = self.expr()?;
                entries.push((key, value));
                if self.peek() == &Tok::Comma {
                    self.bump();
                } else {
                    break;
                }
            }
        }
        self.expect(&Tok::RBrace, "'}'")?;
        let mut t = Term::con("रिक्तकोश");
        for (k, v) in entries {
            t = Term::app("समावेश", vec![t, k, v]);
        }
        Ok(t)
    }

    /// Either a parenthesised expression or a lambda `(params) => body`.
    fn paren_or_lambda(&mut self) -> PResult<Term> {
        self.expect(&Tok::LParen, "'('")?;
        // Empty params: `() => body`.
        if self.peek() == &Tok::RParen {
            self.bump();
            self.expect(&Tok::FatArrow, "'=>' (lambda)")?;
            let body = self.expr()?;
            return Ok(Term::Lam(vec![], Box::new(body)));
        }
        let mut items = vec![self.expr()?];
        while self.peek() == &Tok::Comma {
            self.bump();
            items.push(self.expr()?);
        }
        self.expect(&Tok::RParen, "')'")?;
        if self.peek() == &Tok::FatArrow {
            self.bump();
            // Lambda: every item must be a variable.
            let mut params = Vec::new();
            for it in items {
                match it {
                    Term::Var(v) => params.push(v),
                    other => {
                        return self.err(format!(
                            "lambda parameters must be variables like ?x, found {:?}",
                            other
                        ))
                    }
                }
            }
            let body = self.expr()?;
            Ok(Term::Lam(params, Box::new(body)))
        } else if items.len() == 1 {
            Ok(items.into_iter().next().unwrap())
        } else {
            self.err("parenthesised tuples are not supported (use a constructor)")
        }
    }

    /// `let ?x = e in body`  ⇒  `((?x) => body)(e)`.
    fn let_expr(&mut self) -> PResult<Term> {
        self.expect(&Tok::KwLet, "let")?;
        let name = match self.bump() {
            Tok::Var(v) | Tok::Ident(v) => v,
            other => return self.err(format!("let expects a binder name, found {:?}", other)),
        };
        self.expect(&Tok::Eq, "'='")?;
        let bound = self.expr()?;
        self.expect(&Tok::KwIn, "'in'")?;
        let body = self.expr()?;
        Ok(Term::App(
            Box::new(Term::Lam(vec![name], Box::new(body))),
            vec![bound],
        ))
    }

    /// `if c then a else b`  ⇒  `यदि(c, a, b)`.
    fn if_expr(&mut self) -> PResult<Term> {
        self.expect(&Tok::KwIf, "if")?;
        let cond = self.expr()?;
        self.expect(&Tok::KwThen, "then")?;
        let then_e = self.expr()?;
        self.expect(&Tok::KwElse, "else")?;
        let else_e = self.expr()?;
        Ok(Term::app("यदि", vec![cond, then_e, else_e]))
    }
}

pub fn parse_program(src: &str) -> PResult<Program> {
    let toks = lex(src)?;
    let mut p = Parser { toks, pos: 0, order: 0 };
    p.program()
}

pub fn parse_expr(src: &str) -> PResult<Term> {
    let toks = lex(src)?;
    let mut p = Parser { toks, pos: 0, order: 0 };
    let t = p.expr()?;
    if p.peek() == &Tok::Danda {
        p.bump();
    }
    if p.peek() != &Tok::Eof {
        return p.err(format!("unexpected trailing input: {:?}", p.peek()));
    }
    Ok(t)
}
