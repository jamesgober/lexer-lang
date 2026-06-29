# lexer-lang &mdash; Engineering Directives

> Engineering standards and the definition of done for this project. Read alongside `REPS.md` (root, authoritative) and `dev/ROADMAP.md` (current phase). If anything here conflicts with `REPS.md`, `REPS.md` wins.

---

## 0. Philosophy

This library is built and maintained to a production standard and treated as a flagship piece of work. Plan the full path, then build one verified step at a time. "Good enough" is treated as a defect. lexer-lang is the first MAIN-tier crate and the first real HQL handoff: every front-end built on the family scans through it, so the cursor's per-character cost is multiplied across every byte of every compilation, and a position it reports wrong points every later diagnostic at the wrong place.

---

## 1. What this is

lexer-lang is the scanner core. The `Cursor` walks a `&str` one character at a time, tracks a byte position and the start of the current token, and turns a scanned run into a `Token<K>` — where `K` is the language's own kind. It is the primitive a hand-written lexer drives directly and a generated lexer targets. It owns scanning and nothing else: it does not define a language's keywords (that is `K`, from `token-lang`), collect diagnostics (the lexer author does, through `diag-lang`, from the spans the cursor reports), or store source text (that is `source-lang`).

---

## 2. Engineering law (non-negotiable)

- **Performance** — peak is the baseline; the cursor is zero-copy and allocation-free; advancing one character is `O(1)` and peeking does not rescan; lexemes are borrowed `&str`, never copied; no "faster" claim without `criterion` numbers.
- **Correctness** — the invariants in section 4 are covered by property tests over arbitrary UTF-8, so multi-byte characters and empty input are exercised, not assumed.
- **Security** — every input is hostile text; the cursor must never panic, slice off a `char` boundary, or read out of bounds on any `&str`, and position arithmetic stays within the `u32` envelope the source layers cap at.
- **Architecture** — SOLID, KISS, YAGNI; one responsibility; the cursor stays a primitive (peek / advance / emit) rather than growing into a language-specific or rule-driven tokenizer.
- **Cross-platform** — Linux/macOS/Windows first-class, verified by CI.
- **Error handling** — scanning is total: every primitive is defined at end of input (`None`, `false`, or a no-op), so there is no fallible path of the cursor's own to mishandle. A lexical *error* is the language's judgement, built by the author from a span.
- **Production-ready** — `#![forbid(unsafe_code)]` and `#![deny(missing_docs)]` from the first commit; no stray `println!`/`dbg!`; every public item has rustdoc with a runnable example.

---

## 3. Definition of done

1. Compiles clean on Linux/macOS/Windows, stable and MSRV 1.85.
2. `fmt`, `clippy -D warnings`, `test --all-features`, `cargo doc -D warnings` clean.
3. `cargo audit` + `cargo deny check` pass.
4. No `unwrap`/`expect`/`todo!`/`dbg!` in shipping code.
5. A Tier-1 API exists and headlines the docs.
6. Property tests cover every section-4 invariant.
7. Hot-path changes carry benchmarks; no regression over 5%.
8. Docs and `CHANGELOG.md` updated; the matching `docs/release/vX.Y.Z.md` written before the tag.

---

## 4. Project-specific invariants

- `bump` returns the next `char` and advances the reported position by exactly that character's UTF-8 length; over a whole source the positions are strictly monotonic and end at `text.len()`.
- The cursor never panics and never slices off a `char` boundary on any `&str`; a lexeme is always a valid sub-`&str` of the source.
- `first` agrees with the head of the remaining text and `is_eof` with the remaining text being empty, at every step.
- Lexing a whole source tiles it: the emitted token spans cover `[0, len)` with no gap or overlap, and slicing the source by those spans reproduces the input exactly.
- A base offset shifts every reported position and span by exactly the base and changes nothing else; `for_source` sets the base so spans land in the `SourceMap`'s global position space.
- The cursor allocates nothing: it borrows the source for its whole life and reports positions, spans, and lexemes by value or by borrow.
