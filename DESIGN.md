# सूत्र — Sūtra: design & specification (v0.3)

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
rule        = ("सूत्र"|"fn")   expr [ "|" expr ] "->" expr DANDA ;  (* optional guard *)
samjna      = ("संज्ञा"|"type") IDENT [ "(" params ")" ] ":=" expr {"|" expr} DANDA ;
prayoga     = ("प्रयोग"|"eval") expr DANDA ;
import      = ("उपयोग"|"import") STRING DANDA ;
gana        = ("गण"|"class") IDENT ":=" "[" args "]" DANDA ;
krama       = ("क्रम"|"seq") IDENT "{" { "[" elems "]" "->" "[" args "]" DANDA } "}" ;
            (* a क्रम pattern element may be a class-bound variable  ?v:गण *)

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
| `(a, b, c)`              | `रचना(a, b, c)` (a tuple constructor)            |
| `{k: v, …}` / `r.f`      | `समावेश(…, k, v)` / `प्राप्ति(r, "f")`           |
| `a >> b` / `a >>= k`     | `अनुक्रम(a, b)` / `बन्ध(a, k)`                    |
| `do { ?x <- m; … ; r }`  | `बन्ध(m, (?x) => …)` … ending in `r`             |

---

## 4. The core term language

A term is one of: a variable `?x`; a native `Int`, `Float`, or `Str`; a symbol
application `name(args…)` (nullary = constant/constructor); a lambda
`(params) => body`; an application `f(args…)` of a function *value*; or a native
`Map`. The parser produces a `Sym` when the head is a literal name and an `App`
when the head is a variable, lambda, or parenthesised expression — that
distinction is what makes both named functions and lambdas first-class.

### Maps & records

A **map** is a native immutable key→value table (kept sorted and deduplicated,
so equality is order-independent). A **record** is simply a map whose keys are
field-name strings. Both share one literal and recognise as the `कोश` saṃjñā:

```
{नाम: "पाणिनि", सूत्राणि: 3959}        # bare-identifier keys are field strings
{"a": 1, 2: "two"}                     # arbitrary value keys
{भाषा: "संस्कृत"}.भाषा                  # dot access ⇒ "संस्कृत"
```

`{k: v, …}` desugars to `समावेश(… समावेश(रिक्तकोश, k, v) …)` and `r.f` to
`प्राप्ति(r, "f")`. Operations (`समावेश` insert, `प्राप्ति` get — with an
optional default, `अस्ति` has, `निष्कास` remove, `कुञ्जिकाः` keys, `मूल्यानि`
values, `दीर्घ` size) are native builtins; they are functional (return new
maps). Lookup/membership are O(log n); update is O(n) (a persistent HAMT is
future work).

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
* **Call-by-need sharing.** When a rule fires or a lambda β-reduces, a variable
  used more than once in the body is bound to a single shared thunk, so the
  expression behind it is reduced at most once no matter how often it is used.
  For example `let ?y = e in ?y + ?y` evaluates `e` once; `मन्द(n,1) = 2ⁿ` runs
  in O(n) steps rather than O(2ⁿ). Shares are an internal device, stripped from
  a normal form before it is shown.
* **Builtins are strict.** Native operations (`+`, `==`, `++`, …) reduce their
  arguments to values first, then compute. They are tried only after user rules.
* **Higher-order reduction.** Applying a lambda β-reduces (with currying and
  over-application); applying a function reference or partial application
  saturates it into a call.
* **Guards.** A rule may carry a guard, `lhs | cond -> rhs`; it fires only if
  `cond` (evaluated under the match's bindings) reduces to `सत्य`, otherwise the
  search falls through to earlier-declared rules. Because the guard forces its
  own arguments, numeric base cases are written directly (`फिबो(?n) | ?n < 2 ->
  ?n`) — declare guarded special cases *last* so paratva tries them first.
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
built from these constructors, which the runtime then performs. The
`do { … }` (`क्रिया`) block is the idiomatic way to sequence them:

```
सूत्र मुख्य -> क्रिया {
  मुद्रण("नाम?");
  ?नाम <- पठन;
  मुद्रण("नमस्ते, " ++ ?नाम)
}।
```

The core action constructors are:

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

Beyond the console there are *world* actions: `सञ्चिकापाठ(p)` / `सञ्चिकालेख(p, s)`
(read / write a file), `प्राचलाः` (command-line arguments, a list of strings),
`पर्यावरण(name)` (an environment variable), `काल` (current Unix time, seconds),
and `यादृच्छिक(n)` (a random integer in `[0, n)`). Failing world actions yield a
`दोष` value rather than aborting.

The program only *builds* the action tree; `effect.rs` walks it performing the
actual effects. Purity is preserved. (Effects only run when a program is
executed via `मुख्य`; in a `प्रयोग` they remain inert data.)

---

## 9. Sequence rewriting (क्रम) — the Pāṇinian frontier

Term rewriting matches whole terms; Pāṇini's rules also rewrite *sequences*
with context (`A → B / L _ R`). A **क्रम** (krama) system captures this: a named
block of subsequence-rewrite rules over lists.

```
क्रम संधि {
  [अ, इ] -> [ए]।      # अ + इ → ए  (गुण)
  [अ, उ] -> [ओ]।
  [अ, अ] -> [आ]।
  [आ, इ] -> [ऐ]।
}
```

Applied as a function, `संधि(list)` rewrites the list by repeatedly replacing
the **leftmost** matching contiguous subsequence (the latest-declared rule
winning, as with paratva) until none applies — so junctions combine anywhere in
a word and even cascade (`[अ,अ,इ] → [आ,इ] → [ऐ]`). Patterns may contain
variables (`[?x, ?x] -> [?x]` collapses equal neighbours, with non-linear
matching), and *context* is just neighbouring elements in the pattern
(`[क,अ,त] -> [क,आ,त]`). The system is strict in its argument (the list is
reduced first); a non-list argument leaves the call stuck.

**Element classes (गण).** As Pāṇini named classes of sounds (pratyāhāra such as
*अच्* = vowels), a `गण` declares a named set of atoms:

```
गण अवर्ण := [अ, आ]।   गण इवर्ण := [इ, ई]।
क्रम संधि { [अवर्ण, इवर्ण] -> [ए]। }   # गुण: a/ā + i/ī → e
```

In a क्रम pattern, a bare class name matches **any** member (useful as context),
while `?v:गण` matches a member **and binds** `?v` to it for reuse in the output.
So a single rule covers an entire class instead of enumerating every phoneme,
and rules still cascade (`[अ,अ,इ] → [आ,इ] → [ए]`).

## 10. Modules

`उपयोग "path"।` (import) loads another `.sutra` file (relative to the importer)
and merges its declarations *before* the importing file's, so a file can
override what it imports (paratva). Cycles are broken automatically. Namespacing
remains flat for now (see roadmap).

---

## 11. The standard library (written in Sūtra)

| Module           | Highlights                                                     |
|------------------|-----------------------------------------------------------------|
| `core`           | saṃjñās सत्यता / सूची / दोष                                     |
| `तर्क` tarka     | `न` not, `च` and, `वा` or, `यदि` if, `यमल` (nonlinear)          |
| `गणित` ganita    | `वर्ग`, `द्विगुण`, `क्रमगुणित`, `सम`/`विषम`, `महत्तम`/`अल्पतम`, `श्रेणी` |
| `सूची` suchi     | `दीर्घ`, `योजन`, `विपर्यय`, `शीर्ष`, `पुच्छ`, `प्रति` map, `छन्न` filter, `संहार` fold, `सदस्य`, `समष्टि` |
| `io`             | `अनुक्रम` sequencing                                            |

Arithmetic/comparison/`++`/`दीर्घ`(string & map)/`रूप`(show)/`अंश`/`अक्षर` and
the map operations (`रिक्तकोश`/`समावेश`/`प्राप्ति`/`अस्ति`/`निष्कास`/`कुञ्जिकाः`/
`मूल्यानि`) are native builtins.

---

## 12. Static checking (`sutra check`)

Sūtra stays untyped, but `sutra check FILE` runs a lightweight linter that
catches the most common mistakes without changing the language:

* **unbound variables** — a `?x` used in a guard / right-hand side / प्रयोग that
  no left-hand side or enclosing lambda binds (an error);
* **constructor-arity mismatches** against saṃjñā declarations (a warning);
* **non-exhaustive matches** — a one-argument function whose clauses cover some
  but not all constructors of a single saṃjñā, with no catch-all (a warning).

It exits non-zero if any error is found, so it can gate CI.

## 13. Limitations & future work

* **Numeric base cases need a guard.** Because a catch-all rule `f(?n)` matches
  an unevaluated argument before a literal-pattern rule `f(0)` can force it,
  numeric recursion should test with `?n == 0` (as `क्रमगुणित` does) rather than
  rely on a `f(0) -> …` clause. Constructor patterns (e.g. `युग्म`/`रिक्त`) do
  not have this issue.
* **64-bit integers** that wrap on overflow (so `क्रमगुणित(100)` overflows);
  arbitrary precision is planned.
* **Sequence rewriting is list-based** — क्रम systems rewrite cons-lists by
  contiguous subsequence with गण classes, but there is no regex-style
  environment (optional/repeated elements) or śivasūtra *pratyāhāra* encoding
  yet, and term-level matching remains first-order.
* **Capture-avoidance** in substitution is shadow-aware but does not α-rename;
  in practice rule RHSs close their lambdas before β so this is rarely visible.
* **Map update is O(n)** (persistent sorted vector); a HAMT is future work.
* **Flat namespaces** (`अधिकार` is organisational); no static type checking yet.
  See the roadmap.
