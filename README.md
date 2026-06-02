# सूत्र · Sūtra

A small, pure **term-rewriting programming language** in the spirit of Pāṇini's
*Aṣṭādhyāyī*. A program is a set of rewrite rules (सूत्र) over terms; running it
means rewriting a term to a normal form. There is no mutable state and no
built-in control flow — conditionals, arithmetic, lists and even error handling
are ordinary rules, most of them in a standard library that is itself written in
Sūtra.

> *vipratiṣedhe paraṃ kāryam* — "in conflict, the later rule prevails."

See **[DESIGN.md](DESIGN.md)** for the full language design and specification.

## Quickstart

```sh
# Build
cargo build --release

# Evaluate an expression (uses the standard library by default)
cargo run -q -- eval "क्रमगुणित(५)"          # ⇒ १२०   (5! )
cargo run -q -- eval "क्रमगुणित(५)" --ascii   # ⇒ 120

# Run a file of प्रयोग (evaluate-and-print) expressions
cargo run -q -- run examples/ganita.sutra

# Interactive REPL
cargo run -q -- repl
```

CLI options: `--fuel N` (step limit), `--ascii` (Latin numerals), `--no-prelude`,
`--check` (report which saṃjñās a result inhabits), `--steps`.

## A taste of the language

```
# A rule (सूत्र): the left side rewrites to the right side.
सूत्र क्रमगुणित(०) -> उत्तर(०)।
सूत्र क्रमगुणित(उत्तर(?न)) -> गुणन(उत्तर(?न), क्रमगुणित(?न))।

# A type / grammar production (संज्ञा).
संज्ञा संख्या := ० | उत्तर(संख्या)।

# An expression to evaluate (प्रयोग).
प्रयोग क्रमगुणित(५)।     # ⇒ १२०
```

Key ideas (all detailed in [DESIGN.md](DESIGN.md)):

* **Numerals are sugar** for Peano terms (`५` ⇄ `उत्तर(उत्तर(उत्तर(उत्तर(उत्तर(०)))))`).
* **Control flow is library code.** `यदि` (if) is a 3-argument rule; outermost
  (lazy) reduction means the unused branch is never evaluated.
* **Conflict resolution is *paratva*:** when two rules match, the later-declared
  one wins, so a specific rule after a general one acts as an exception.
* **Failure is pure:** unmatched terms simply get *stuck* (the normal form is the
  irreducible term), and recoverable errors are returned as `दोष` (doṣa) values.
* **Types (saṃjñā) are optional and structural** — they classify terms, they
  don't gate evaluation. Try `--check` or `:type` in the REPL.

## Examples

| File | Shows |
|------|-------|
| [`examples/ganita.sutra`](examples/ganita.sutra) | arithmetic |
| [`examples/suchi.sutra`](examples/suchi.sutra)   | lists (length, append, reverse, head, member) |
| [`examples/sandhi.sutra`](examples/sandhi.sutra) | Sanskrit vowel sandhi as rewriting |
| [`examples/dosha.sutra`](examples/dosha.sutra)   | doṣa error-values and stuck terms |
| [`examples/paratva.sutra`](examples/paratva.sutra) | later-rule-wins conflict resolution |

## Project layout

```
src/        the interpreter (lexer, parser, engine, samjna checker, pretty, CLI)
std/        the standard library, written in Sūtra (embedded into the binary)
examples/   runnable example programs
tests/      end-to-end tests
DESIGN.md   language design & specification
```

## Tests

```sh
cargo test
```

## Status

v0.1 — a working prototype. First-order rewriting only; higher-order functions,
true subsequence/contextual matching, and `अधिकार` scoping are future work (see
the limitations section of [DESIGN.md](DESIGN.md)).

## License

MIT — see [LICENSE](LICENSE).
