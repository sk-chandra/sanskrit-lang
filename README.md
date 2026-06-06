# सूत्र · Sūtra

A programming language whose computational core is **term rewriting**, in the
spirit of Pāṇini's *Aṣṭādhyāyī* — grown into a practical, general-purpose
language. Programs are sets of rewrite rules (सूत्र) over terms; running one
rewrites a term to a normal form. On top of that pure core sit native data,
higher-order functions, ergonomic syntax, modules, and pure effect-as-data I/O.

> *vipratiṣedhe paraṃ kāryam* — "in conflict, the later rule prevails."

Sanskrit-first but **bilingual**: every keyword, operator-word, builtin and
stdlib function has a Latin alias, so the same program can be written in
Devanagari or ASCII.

See **[DESIGN.md](DESIGN.md)** for the specification and **[ROADMAP.md](ROADMAP.md)**
for where it's going.

## Quickstart

```sh
cargo build --release

# Evaluate an expression
cargo run -q -- eval "क्रमगुणित(20)"                     # ⇒ 2432902008176640000
cargo run -q -- eval "प्रति((?x) => ?x * ?x, [1,2,3,4])"  # ⇒ [1, 4, 9, 16]

# The same, in Latin (bilingual aliases)
cargo run -q -- eval "map((?x) => mul(?x, ?x), [1,2,3,4])" --ascii

# Run a program (executes मुख्य/main if present, else every प्रयोग)
cargo run -q -- run examples/fizzbuzz.sutra
echo "विश्व" | cargo run -q -- run examples/hello.sutra

# Statically check a file for likely mistakes (unbound vars, arity, exhaustiveness)
cargo run -q -- check examples/guards.sutra

# Interactive REPL
cargo run -q -- repl
```

Options: `--fuel N`, `--ascii`, `--no-prelude`, `--check` (report saṃjñās),
`--steps`.

## A taste

```
# Functions are rewrite rules. Native ints, operators, and `if` are sugar.
सूत्र क्रमगुणित(?न) -> यदि(?न == 0, 1, ?न * क्रमगुणित(?न - 1))।

# Higher-order functions, lambdas, list literals, pipes:
प्रयोग [1, 2, 3, 4, 5] |> प्रति((?x) => ?x * ?x) |> समष्टि।   # ⇒ 55

# Maps and records (a record is a map with field-name keys):
प्रयोग {नाम: "पाणिनि", सूत्राणि: 3959}.सूत्राणि।             # ⇒ 3959

# क्रम — sequence rewriting (the Pāṇinian frontier) with गण element classes:
गण अवर्ण := [अ, आ]।  गण इवर्ण := [इ, ई]।
क्रम संधि { [अवर्ण, इवर्ण] -> [ए]। }                        # गुण: a/ā + i/ī → e
प्रयोग संधि([क, आ, ई, त])।                                  # ⇒ [क, ए, त]

# Pure effect-as-data I/O with do-notation: मुख्य builds an action, the runtime runs it.
सूत्र मुख्य -> क्रिया {
  मुद्रण("तव नाम किम्?");
  ?न <- पठन;
  मुद्रण("नमस्ते, " ++ ?न ++ "!")
}।
```

What makes Sūtra Sūtra (all detailed in [DESIGN.md](DESIGN.md)):

* **Everything is rewriting.** `यदि` (if), `&&`, `::` etc. are ordinary rules /
  sugar, not built-in control flow. Reduction is outermost, so `यदि` is lazy.
* **Sequence rewriting (क्रम) with element classes (गण).** Pāṇini also rewrote
  *sequences* with context and named classes of sounds; a `क्रम` system rewrites
  subsequences of a list (leftmost-first, paratva, variables, cascading) and a
  `गण` lets one rule range over a whole class — so real sandhi expresses
  directly, not as recursion.
* **Paratva conflict resolution.** When two rules match, the later one wins — a
  specific rule after a general one is an exception (apavāda).
* **Pattern guards.** `सूत्र f(?n) | ?n < 2 -> ?n।` — a rule fires only when its
  guard holds, otherwise it falls through to an earlier rule.
* **Pure failure.** Unmatched terms get *stuck* (the normal form is the term);
  recoverable errors are returned as `दोष` (doṣa) values. No exceptions.
* **Pure I/O.** Effects are data (`शुद्ध`/`बन्ध`/`मुद्रण`/`पठन`) executed by the
  runtime, so programs stay pure.
* **Optional structural types (saṃjñā).** They classify terms; they don't gate
  evaluation. Try `--check` or `:type` in the REPL.

## Examples

| File | Shows |
|------|-------|
| [`examples/ganita.sutra`](examples/ganita.sutra) | native arithmetic, `श्रेणी`, `समष्टि` |
| [`examples/suchi.sutra`](examples/suchi.sutra)   | lists, literals, `map`/`filter`/`fold` |
| [`examples/kosha.sutra`](examples/kosha.sutra)   | maps & records: literals, dot access, queries |
| [`examples/fizzbuzz.sutra`](examples/fizzbuzz.sutra) | FizzBuzz as a folded, pure action |
| [`examples/hello.sutra`](examples/hello.sutra)   | interactive I/O (print/read/bind) |
| [`examples/effects.sutra`](examples/effects.sutra) | files, args, time, randomness as effects |
| [`examples/samvada.sutra`](examples/samvada.sutra) | do-notation dialogue (read & add numbers) |
| [`examples/guards.sutra`](examples/guards.sutra) | pattern guards (fibonacci, sign, fizzbuzz) |
| [`examples/sandhi.sutra`](examples/sandhi.sutra) | Sanskrit vowel sandhi as a क्रम sequence system |
| [`examples/dosha.sutra`](examples/dosha.sutra)   | doṣa error-values and stuck terms |
| [`examples/paratva.sutra`](examples/paratva.sutra) | later-rule-wins conflict resolution |

## Project layout

```
src/        interpreter: lexer, parser, engine, builtins, bigint, effect, samjna, check, pretty, names, CLI
std/        standard library, written in Sūtra (embedded into the binary)
examples/   runnable example programs
tests/      end-to-end tests
DESIGN.md   language specification        ROADMAP.md   phase plan
```

## Tests

```sh
cargo test
```

## Status

v0.4 (Phase 3 in progress) — a general-purpose practice language. Native data
(maps & records, tuples, **arbitrary-precision integers**), higher-order
functions, **call-by-need sharing**, ergonomic sugar (`do`-notation,
**pattern guards**), pure effect-as-data I/O (console, files, args, env, time,
randomness), modules, bilingual syntax, **`क्रम` sequence rewriting with `गण`
element classes** (the Pāṇinian frontier), and a **static checker** (`sutra
check`). Next (see [ROADMAP.md](ROADMAP.md)): module namespacing, regex-style
क्रम environments, and more tooling.

## License

MIT — see [LICENSE](LICENSE).
