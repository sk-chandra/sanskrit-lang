# सूत्र — Sūtra: a Pāṇinian term-rewriting language

> *vipratiṣedhe paraṃ kāryam* — "in conflict, the later rule prevails." (Aṣṭādhyāyī 1.4.2)

Sūtra is a small, pure programming language whose only computational mechanism is
**term rewriting**, in the spirit of Pāṇini's *Aṣṭādhyāyī* — the ~2,500-year-old
Sanskrit grammar that is, in effect, a formal rewriting system. A program is a
set of rules (सूत्र); running a program means rewriting a term to a normal form.

This document is both the design rationale and the specification for v0.1. It is
the authoritative record of the decisions taken during design.

---

## 1. Philosophy

Pāṇini described Sanskrit not with prose but with ~4,000 *sūtras*: ordered,
context-sensitive rewrite rules over phonological strings, complete with named
classes (*saṃjñā*), abbreviatory devices (*pratyāhāra*), meta-rules
(*paribhāṣā*), and an explicit conflict-resolution principle (*paratva*). It is
uncannily close to a modern term-rewriting system.

Sūtra takes that correspondence literally. The guiding commitments are:

1. **Purity.** No mutable state, no side effects, no I/O in the language core.
   A program is a mathematical object: a rewrite relation.
2. **Uniformity.** Everything is a *term*. There are no statements and no
   special forms. Arithmetic, booleans, conditionals, lists and even error
   handling are ordinary rules — most of them library code written in Sūtra.
3. **Authenticity.** Keywords, the standard library and idioms are Sanskrit;
   source may be written in Devanagari or Latin. The showcase library module
   implements (a fragment of) Sanskrit *sandhi* — the language using Pāṇini's
   mechanism to encode Pāṇini's phonology.

Non-termination is *allowed*: the language is Turing-complete and a program may
fail to reach a normal form. This is a deliberate research-flavoured choice, not
an oversight (see §7).

---

## 2. Lexical structure

Source is UTF-8. Identifiers may be Devanagari, Latin, or mixed; keywords are
recognised in both scripts.

| Element        | Form                                                            |
|----------------|-----------------------------------------------------------------|
| Comment        | `#` … end of line                                               |
| Daṇḍa (terminator) | `।` (U+0964); also `॥` and ASCII `;` as aliases             |
| Rewrite arrow  | `->` or `→`                                                     |
| Saṃjñā define  | `:=`                                                            |
| Alternation    | `\|`                                                            |
| Application    | `name(arg, …)`                                                  |
| Variable       | `?name` (the `?` sigil; script-neutral)                         |
| Numeral        | ASCII `0-9` or Devanagari `०-९`                                  |
| String         | `"…"` with `\n`, `\t`, `\"`, `\\` escapes                       |
| Keywords       | `सूत्र`/`sutra`, `संज्ञा`/`samjna`, `अधिकार`/`adhikara`, `प्रयोग`/`prayoga` |

Identifiers accept the full Devanagari block (so conjuncts and *mātrā*s — e.g.
`क्रमगुणित`, `रिक्त` — lex as a single token) but exclude the daṇḍa punctuation.
Hyphens are **not** identifier characters (use compounds or `_`).

---

## 3. Grammar (EBNF)

```ebnf
program      = { declaration } ;
declaration  = sutra_decl | samjna_decl | adhikara_decl | prayoga_decl ;

sutra_decl   = ("सूत्र" | "sutra") term "->" term DANDA ;
samjna_decl  = ("संज्ञा" | "samjna") IDENT [ "(" param {"," param} ")" ]
                 ":=" term { "|" term } DANDA ;
adhikara_decl= ("अधिकार" | "adhikara") IDENT DANDA ;   (* section header *)
prayoga_decl = ("प्रयोग" | "prayoga") term DANDA ;       (* expr to evaluate *)

term         = VAR | STRING | NUMERAL | IDENT [ "(" term {"," term} ")" ] ;
```

* A bare `IDENT` is a **nullary symbol** (a constant / constructor).
* `IDENT(args)` is an **application**.
* There is no distinction between "functions" and "constructors": a symbol
  reduces iff some rule's left-hand side matches it.
* `अधिकार` (section header) is organisational only in v1 (no scoping yet — see
  §9). `प्रयोग E।` registers an expression to be evaluated and printed.

---

## 4. Saṃjñā = types = grammar productions

A **saṃjñā** (संज्ञा, "technical name / class") simultaneously serves as an
algebraic data type and a grammar production:

```
संज्ञा संख्या := ० | उत्तर(संख्या)।                    # natural numbers
संज्ञा सत्यता := सत्य | असत्य।                          # booleans
संज्ञा सूची(?क) := रिक्त | युग्म(?क, सूची(?क))।         # polymorphic lists
संज्ञा दोष := दोष(?सन्देश)।                              # error values
```

The reading: an alternative's **head is a data constructor**, and each
**argument is a field type** (a parameter, a reference to another saṃjñā, or a
nested constructor). This separation is what allows a type whose constructor
shares its own name (`दोष := दोष(?सन्देश)`) to be checked without looping —
type references only ever recurse into strictly smaller sub-terms.

**Typing is optional and structural** (decision: *optional / structural*). The
engine never enforces types; membership is computed on demand. The checker
answers "does term *t* inhabit saṃjñā *S*?" and is exposed via:

* the `--check` CLI flag (reports the saṃjñās each result inhabits), and
* `:type EXPR` in the REPL.

This matches the fluid nature of pure rewriting: types document and classify,
they do not gate evaluation.

---

## 5. Control flow is library code

There are no built-in conditionals. Branching is pattern-matching on boolean
*terms*, defined entirely in `std/tarka.sutra`:

```
सूत्र यदि(सत्य, ?तदा, ?अन्यथा) -> ?तदा।
सूत्र यदि(असत्य, ?तदा, ?अन्यथा) -> ?अन्यथा।
```

Because reduction is **outermost / lazy** (§6), `यदि` does not evaluate the
branch it discards — so an ordinary three-argument rule behaves like a proper
conditional, and recursion guarded by `यदि` terminates.

---

## 6. The rewrite engine

Evaluation = repeated rewriting to a normal form. Two decisions define its
behaviour.

### 6.1 Reduction order: leftmost-outermost

A single step tries to rewrite the **root** redex first; only if no rule matches
the root does it descend left-to-right into arguments. This yields lazy
evaluation:

* `यदि(सत्य, t, e)` fires at the root immediately → `t`, never touching `e`.
* When the root *cannot* fire (e.g. `क्रमगुणित(योग(१,२))` — `योग(१,२)` is not yet
  a `उत्तर`/`०` constructor), descent reduces the argument just enough for the
  root to match on the next step. Arguments are thus forced only as demanded.

### 6.2 Conflict resolution: paratva (परत्व)

When several rules match the same redex, the **latest-declared** rule wins
(*vipratiṣedhe paraṃ kāryam*). This implements *apavāda* (exceptions) directly:
write the general rule first, then the specific overrides after it.

```
सूत्र वर्ग(?क्ष) -> अन्य।      # general (utsarga)
सूत्र वर्ग(अ)   -> स्वर।       # exception (apavāda) — declared later, so it wins
```

Implementation: rules are scanned in reverse declaration order; the first match
is applied. Standard-library modules are loaded in a fixed order so their
indices compose predictably; a user file is appended after the prelude.

> **Interaction note.** Under outermost reduction a *general fallback* rule fires
> before its arguments are evaluated — which is exactly the desired apavāda
> behaviour for dispatch on constructors/constants. For value-dependent
> decisions, write rules whose patterns require constructors (e.g. structural
> `तुल्य` on `०`/`उत्तर`); those force their arguments via descent and stay sound.

### 6.3 Matching

Matching is one-way (the subject term has no variables) and **non-linear**: a
variable occurring twice in a left-hand side must bind structurally equal
sub-terms.

```
सूत्र यमल(?क, ?क) -> समान।   # fires only when both arguments are equal
```

`यमल(योग(१,१), २)` → `समान` (after both sides reduce to `२`); `यमल(१, २)` is
left stuck.

---

## 7. Failure model

Decision: **stuck terms + doṣa values** — both are pure, and there are no
exceptions or panics in the language.

1. **Stuck terms.** If no rule applies and a term is not a value, it simply stops
   rewriting. The normal form *is* the irreducible term. `शीर्ष(५)` (head of a
   number) reduces to `शीर्ष(५)` — the term itself signals "no rule here."
2. **doṣa (दोष) — errors as values.** Library rules may *return* an error term
   for recoverable failures, which is ordinary data that can be inspected or
   pattern-matched:

   ```
   सूत्र शीर्ष(रिक्त) -> दोष("रिक्ता सूची")।   # head of empty list → fault
   सूत्र शीर्ष(युग्म(?क, ?श)) -> ?क।
   ```

The only failures the *runtime* (the Rust host) reports are **parse errors** and
**possible non-termination**. Because non-termination is permitted by the
semantics, it is handled as tooling, not language: a configurable **fuel**
(step limit, default 1,000,000) stops runaway reductions and reports
`out of fuel`. Purity is preserved.

---

## 8. Numerals and the standard library

Numerals are **sugar** for Peano terms: `०` is `Term::con("०")`, `उत्तर(n)` is the
successor, and the numeral `५` parses to `उत्तर(उत्तर(उत्तर(उत्तर(उत्तर(०)))))`.
The pretty-printer collapses Peano terms back to numerals (Devanagari by
default; `--ascii` for Latin).

The standard library is **written in Sūtra** (`std/*.sutra`, embedded in the
binary) and loaded as a prelude:

| Module           | Contents                                                        |
|------------------|-----------------------------------------------------------------|
| `गणित` ganita    | `योग` +, `गुणन` ×, `पूर्व` pred, `वियोग` monus, `क्रमगुणित` factorial, `तुल्य` =, `न्यूनसम` ≤ |
| `तर्क` tarka     | `न` not, `च` and, `वा` or, `यदि` if, `यमल` (nonlinear demo)      |
| `सूची` suchi     | `दीर्घ` length, `योजन` append, `विपर्यय` reverse, `शीर्ष` head, `पुच्छ` tail, `सदस्य` member |
| `संधि` sandhi    | vowel sandhi as rewrite rules (the dogfooding showcase)         |

Higher-order list functions (`map`/`fold`) are intentionally **out of scope for
v1** — the term language is first-order. See §9.

---

## 9. Known limitations & future work

* **First-order only.** No higher-order functions yet; `map`/`fold` need an
  `apply`/defunctionalisation layer.
* **No true subsequence/associative matching.** Pāṇini's rules operate on flat
  phoneme *strings* with context. v1 uses first-order matching over cons-lists;
  sandhi is modelled on `युग्म`/`रिक्त` lists. Real *pratyāhāra* and contextual
  ( `_` environment) rules are future work.
* **`अधिकार` is organisational only.** No namespacing/scoping is enforced yet;
  the symbol table is flat.
* **No sharing.** Outermost reduction without graph sharing can recompute
  duplicated sub-terms; fine for the examples, but a real implementation would
  want call-by-need.
* **Structural typing is loose** for polymorphism (type arguments to saṃjñā
  references are not unified, only the head saṃjñā is checked).

---

## 10. A worked reduction trace

`क्रमगुणित(२)` under outermost reduction + paratva (numerals shown sugared):

```
क्रमगुणित(२)
  → गुणन(२, क्रमगुणित(१))                 # क्रमगुणित(उत्तर(?क)) rule, ?क = १
  → योग(२, गुणन(१, क्रमगुणित(१)))         # गुणन(उत्तर(?क),?ख) rule
  → उत्तर(योग(१, गुणन(१, क्रमगुणित(१))))   # योग(उत्तर(?क),?ख) rule (root fires)
  → … (inner योग / गुणन / क्रमगुणित forced by descent as needed) …
  → २                                       # normal form
```

The discarded-branch laziness of `यदि`, the apavāda behaviour of paratva, and
the "stuck term vs doṣa" failure model can all be observed directly via the CLI:

```sh
sutra eval "यदि(सत्य, ७, क्रमगुणित(लूप))"   # ⇒ ७   (else branch never evaluated)
sutra run examples/paratva.sutra            # specific rules override the general one
sutra run examples/dosha.sutra              # doṣa values and a stuck term
sutra eval "क्रमगुणित(६)" --check            # ⇒ ७२० : संख्या
```
