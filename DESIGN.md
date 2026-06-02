# सूत्र — Sūtra: design & specification (v0.2)

> *vipratiṣedhe paraṃ kāryam* — "in conflict, the later rule prevails." (Aṣṭādhyāyī 1.4.2)

Sūtra is a programming language whose computational core is **term rewriting**,
in the spirit of Pāṇini's *Aṣṭādhyāyī*. A program is a set of rules (सूत्र) over
terms (पद); running a program rewrites a term to a normal form. On top of that
core, v0.2 adds the machinery of a practical general-purpose language: native
data, higher-order functions, ergonomic syntax, pure I/O, and modules — while
keeping the rewriting soul intact.

See [ROADMAP.md](ROADMAP.md) for the phase plan. This document specifies the
language as it stands.

---

## 1. Philosophy

Pāṇini described Sanskrit with ordered, context-sensitive rewrite rules, named
classes (*saṃjñā*), and an explicit conflict-resolution principle (*paratva*).
Sūtra takes that correspondence literally and builds outward from it. Commitments:

1. **Rewriting is the core.** Functions are rules; evaluation is reduction.
2. **Purity.** No mutable state and no hidden effects. Even I/O is pure data
   (§8): a program *describes* effects that the runtime performs.
3. **Sanskrit-first, but bilingual.** Devanagari names are canonical; every
   keyword, operator-word, builtin and stdlib function has a Latin alias, so the
   same language is writable in either script (§2).
4. **Sugar desugars.** Operators, `let`, lambdas, `if`, list literals are
   surface conveniences that elaborate into the small core (§3).

Non-termination is permitted (the language is Turing-complete); a configurable
*fuel* guards runaway reductions as tooling, not semantics.

---

## 2. Lexical structure & bilingual naming

Source is UTF-8. Identifiers may be Devanagari, Latin, or mixed.

| Element            | Form                                                        |
|--------------------|-------------------------------------------------------------|
| Comment            | `#` … end of line                                           |
| Daṇḍa (terminator) | `।` (also `॥`, `;`)                                         |
| Variable           | `?name`                                                     |
| Integer / Float    | `42`, `-3`, `3.14` (ASCII or Devanagari digits)             |
| String             | `"…"` with `\n \t \" \\` escapes                            |
| Rule arrow         | `->` or `→`                                                 |
| Lambda arrow       | `=>`                                                        |
| Saṃjñā define      | `:=`                                                        |
| Operators          | `+ - * / %  == != < <= > >=  && \|\|  ++  ::  \|>  >> >>=`   |

**Keywords** (each bilingual): `सूत्र`/`fn`, `संज्ञा`/`type`, `अधिकार`/`section`,
`प्रयोग`/`eval`, `उपयोग`/`import`, `अस्तु`/`let`, `अतः`/`in`, `चेत्`/`if`,
`तर्हि`/`then`, `अन्यथा`/`else`.

**Aliases.** The parser canonicalises identifiers, so `add`/`योग` → `+`,
`map` → `प्रति`, `print` → `मुद्रण`, `true` → `सत्य`, etc. Programs written in
Latin and in Devanagari are the same program.

---

## 3. Surface syntax (EBNF, sugar shown)

```ebnf
program     = { decl } ;
decl        = rule | samjna | section | prayoga | import ;
rule        = ("सूत्र"|"fn")   expr "->" expr DANDA ;
samjna      = ("संज्ञा"|"type") IDENT [ "(" params ")" ] ":=" expr {"|" expr} DANDA ;
prayoga     = ("प्रयोग"|"eval") expr DANDA ;
import      = ("उपयोग"|"import") STRING DANDA ;

expr        = "let" VAR "=" expr "in" expr
            | "if" expr "then" expr "else" expr
            | infix ;
infix       = (* Pratt parsing of the operators above, by precedence *) ;
unary       = ["-"|"!"] postfix ;
postfix     = primary { "(" [ args ] ")" } ;          (* application *)
primary     = INT | FLOAT | STRING | VAR | IDENT
            | "[" [ args ] "]"                         (* list literal *)
            | "(" params ")" "=>" expr                 (* lambda *)
            | "(" expr ")" ;                           (* grouping *)
```

Desugaring (every construct elaborates to `Sym` / `App` / `Lam` / literals):

| Surface                  | Core                                              |
|--------------------------|---------------------------------------------------|
| `a + b`, `a == b`, …     | `Sym("+", [a, b])`, `Sym("==", …)`               |
| `a :: b`                 | `युग्म(a, b)`                                      |
| `[x, y, z]`              | `युग्म(x, युग्म(y, युग्म(z, रिक्त)))`             |
| `a && b` / `a \|\| b` / `!a` | `च(a,b)` / `वा(a,b)` / `न(a)`                  |
| `x \|> f`                | `f(x)` (appends `x` as the last argument)        |
| `let ?x = e in b`        | `((?x) => b)(e)`                                  |
| `if c then a else b`     | `यदि(c, a, b)`                                     |
| `a >> b` / `a >>= k`     | `अनुक्रम(a, b)` / `बन्ध(a, k)`                    |

---

## 4. The core term language

A term is one of: a variable `?x`; a native `Int`, `Float`, or `Str`; a symbol
application `name(args…)` (nullary = constant/constructor); a lambda
`(params) => body`; or an application `f(args…)` of a function *value*. The
parser produces a `Sym` when the head is a literal name and an `App` when the
head is a variable, lambda, or parenthesised expression — that distinction is
what makes both named functions and lambdas first-class.

---

## 5. Saṃjñā = types = grammar productions (optional, structural)

A saṃjñā doubles as an algebraic data type and a grammar production:

```
संज्ञा सत्यता := सत्य | असत्य।
संज्ञा सूची(?त) := रिक्त | युग्म(?त, सूची(?त))।
संज्ञा दोष := दोष(?सन्देश)।
```

Native values are recognised under built-in saṃjñā names: `संख्या` (Int),
`दशांश` (Float), `अक्षरमाला` (String). Typing is **optional and structural** —
never enforced; membership is computed on demand and surfaced via `--check` and
the REPL's `:type`.

---

## 6. Reduction & conflict resolution

* **Leftmost-outermost reduction.** The root redex is tried first; arguments are
  reduced only as demanded. This keeps `यदि` (a library rule) lazy — the unused
  branch is never evaluated.
* **Builtins are strict.** Native operations (`+`, `==`, `++`, …) reduce their
  arguments to values first, then compute. They are tried only after user rules.
* **Higher-order reduction.** Applying a lambda β-reduces (with currying and
  over-application); applying a function reference or partial application
  saturates it into a call.
* **Paratva.** When several user rules match a redex, the **latest-declared**
  wins (`vipratiṣedhe paraṃ kāryam`) — so a specific rule after a general one is
  an apavāda (exception).
* **Non-linear matching.** A variable repeated in a left-hand side must bind
  structurally equal sub-terms.

---

## 7. Failure model

There are no exceptions. Failure is pure and takes two forms:

1. **Stuck terms.** A term with no applicable rule simply stops reducing; the
   normal form *is* the irreducible term (e.g. `शीर्ष(5)`).
2. **doṣa values.** Recoverable errors are returned as ordinary `दोष` data —
   `शीर्ष([])` ⇒ `दोष("रिक्ता सूची")`, `5 / 0` ⇒ `दोष("शून्येन भागः …")` — which
   can be inspected and pattern-matched.

The runtime itself only reports parse errors and possible non-termination (via
the fuel limit).

---

## 8. Pure effect-as-data I/O

A program stays pure: `मुख्य` (main) evaluates to an **action**, a data term
built from these constructors, which the runtime then performs:

| Constructor      | Meaning                                          |
|------------------|--------------------------------------------------|
| `शुद्ध(x)`        | yield `x`, do nothing (`pure`/`return`)          |
| `मुद्रण(s)`       | print `s`, yield `एकक` (`print`)                 |
| `पठन`            | read a line, yield it (`read`)                   |
| `बन्ध(m, k)`      | run `m`, then run `k(result)` (`bind`, `>>=`)    |
| `अनुक्रम(a, b)`   | run `a`, then `b` (`seq`, `>>`)                  |

```
सूत्र मुख्य ->
  मुद्रण("नाम?") >> बन्ध(पठन, (?न) => मुद्रण("नमस्ते, " ++ ?न))।
```

The program only *builds* the action tree; `effect.rs` walks it performing the
actual effects. Purity is preserved.

---

## 9. Modules

`उपयोग "path"।` (import) loads another `.sutra` file (relative to the importer)
and merges its declarations *before* the importing file's, so a file can
override what it imports (paratva). Cycles are broken automatically. Namespacing
remains flat for now (see roadmap).

---

## 10. The standard library (written in Sūtra)

| Module           | Highlights                                                     |
|------------------|-----------------------------------------------------------------|
| `core`           | saṃjñās सत्यता / सूची / दोष                                     |
| `तर्क` tarka     | `न` not, `च` and, `वा` or, `यदि` if, `यमल` (nonlinear)          |
| `गणित` ganita    | `वर्ग`, `द्विगुण`, `क्रमगुणित`, `सम`/`विषम`, `महत्तम`/`अल्पतम`, `श्रेणी` |
| `सूची` suchi     | `दीर्घ`, `योजन`, `विपर्यय`, `शीर्ष`, `पुच्छ`, `प्रति` map, `छन्न` filter, `संहार` fold, `सदस्य`, `समष्टि` |
| `io`             | `अनुक्रम` sequencing                                            |

Arithmetic/comparison/`++`/`दीर्घ`(string)/`रूप`(show)/`अंश`/`अक्षर` are native
builtins.

---

## 11. Limitations & future work

* **Call-by-name** β/`let`: arguments are substituted unevaluated, so a value
  bound and used many times can be recomputed. Call-by-need (sharing) is the
  next performance step.
* **First-order matching only** — no true subsequence/contextual (`_`) rules
  yet, so real Pāṇinian sandhi is approximated over cons-lists.
* **Capture-avoidance** in substitution is shadow-aware but does not α-rename;
  in practice rule RHSs close their lambdas before β so this is rarely visible.
* **Flat namespaces** (`अधिकार` is organisational). No `Map`/records/tuples yet,
  no file/args/time effects yet, no static type checking. See the roadmap.
