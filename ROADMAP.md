# सूत्र — Roadmap toward a general-purpose language

Sūtra began as a pure, first-order term-rewriting core (v0.1). The goal now is a
**general-purpose practice language** that keeps the Pāṇinian rewriting soul but
is practical to write real programs in. This file tracks the phased plan; see
[DESIGN.md](DESIGN.md) for the current specification.

Locked-in design decisions:

* **I/O = pure effect-as-data.** Programs stay pure: `मुख्य` (main) evaluates to
  an *action* built from `शुद्ध`/`बन्ध`/`मुद्रण`/`पठन`; the runtime executes it.
* **Ergonomic sugar.** Infix operators, `let`, lambdas, list literals, `if` — all
  desugar to the rewriting core.
* **Bilingual, Sanskrit-first.** Devanagari names are canonical; every keyword,
  operator-word, builtin and stdlib function has a Latin/ASCII alias.

---

## Phase 0 — pure rewriting core *(done, v0.1)*

Lexer/parser, leftmost-outermost reduction, paratva conflict resolution,
non-linear matching, optional structural saṃjñā types, stuck-term + doṣa failure
model, Peano numerals, basic stdlib (ganita/tarka/suchi/sandhi).

## Phase 1 — the practical foundation *(this pass, v0.2)*

* **Native data types:** `Int`, `Float`, `String`, `Bool` (सत्य/असत्य), `Unit`
  (एकक), and cons `List`. Numerals are now native integers, not Peano.
* **Builtins:** arithmetic (`+ - * / %`), comparison (`== != < <= > >=`),
  string/list `++`, plus conversions — primitive reductions over native values.
* **Higher-order functions:** lambdas `(x) => e`, closures by substitution,
  first-class function references, and application/β-reduction (`?f(?x)`).
* **Ergonomic surface syntax:** infix operators with precedence, prefix `-`/`!`,
  list literals `[a, b, c]`, `let x = e in b`, lambdas, `if/then/else`, and the
  pipe `|>` — all desugaring to core terms.
* **Pure effect-as-data I/O:** `शुद्ध` (pure), `बन्ध` (bind), `मुद्रण` (print),
  `पठन` (read), with `>>=`/`>>` operators and a runtime driver that executes the
  action returned by `मुख्य` (main).
* **Modules:** `उपयोग "file"` (import) to compose programs across files.
* **Bilingual aliases:** Devanagari canonical + Latin spellings everywhere.
* **Refreshed stdlib:** native `ganita`, `tarka`, `suchi` with `map`/`fold`/
  `filter`, `sutra` (string ops), `io` (action helpers).
* Updated examples, tests, DESIGN and README.

## Phase 2 — performance & richer data *(next)*

* Call-by-need (graph reduction / sharing) for predictable performance.
* Native `Map`/dictionary and records; tuples.
* More effects: file read/write, program args, time, randomness, environment.
* `do`-notation sugar over `बन्ध`/`शुद्ध`.
* Arbitrary-precision integers.

## Phase 3 — scale & tooling

* Real module namespacing/scoping for `अधिकार`; an imports/package story.
* Pattern guards; exhaustiveness and optional static type checking over saṃjñā.
* Tooling: formatter, language server, better REPL (history, completion).
* The Pāṇinian frontier: true subsequence / contextual (`_` environment)
  matching and *pratyāhāra*, so real sandhi/grammar rules can be expressed.
