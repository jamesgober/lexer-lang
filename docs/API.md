# lexer-lang &mdash; API Reference

> Complete reference for every public item in `lexer-lang`, with examples.
> **Status: stable (1.0).** The surface below is the `1.0` contract; it follows
> [Semantic Versioning](#semver-promise) and will not change in a breaking way before
> `2.0`. See [`dev/ROADMAP.md`](../dev/ROADMAP.md).

## SemVer promise

As of `1.0.0` the public API documented here is **stable**. The crate follows
[Semantic Versioning](https://semver.org):

- No item in this reference will be **removed or changed in a breaking way** within
  the `1.x` series. Breaking changes wait for `2.0`.
- New functionality arrives in **minor** releases (`1.1`, `1.2`, …) and is additive —
  a new cursor primitive, a new constructor, a new convenience.
- Bug fixes, documentation, and internal changes are **patch** releases.
- The **MSRV** is Rust `1.85`. Raising it is a minor change, called out in the
  changelog; it is never a patch.

Anything not in this reference — the private fields, exact `Debug` output — is not
part of the contract and may change at any time.

## Table of Contents

- [Overview](#overview)
- [Installation](#installation)
- [Quick start](#quick-start)
- [The model](#the-model)
- [`Cursor`](#cursor)
  - [Construction: `new` / `with_base` / `for_source`](#construction)
  - [Looking ahead: `first` / `second` / `is_eof` / `remaining` / `pos`](#looking-ahead)
  - [Consuming: `bump` / `bump_if` / `eat_while`](#consuming)
  - [Reading the run: `lexeme` / `token_span` / `intern_lexeme`](#reading-the-run)
  - [Finishing a token: `emit` / `reset_token`](#finishing-a-token)
- [Re-exported types](#re-exported-types)
- [Diagnostics](#diagnostics)
- [Feature flags](#feature-flags)

---

## Overview

lexer-lang is the scanner that turns source text into a token stream. Its one type,
[`Cursor`](#cursor), is a zero-copy scanner over a `&str`: the primitive every lexer
is built on, hand-written or generated. A lexer drives the cursor with peek/advance
steps and turns each scanned run into a [`Token<K>`](#re-exported-types), where `K`
is the language's own token kind.

The cursor owns scanning and nothing else. It does not decide a language's keywords
(that is `K`, from [`token-lang`](https://docs.rs/token-lang)), collect diagnostics
(the lexer author does, through [`diag-lang`](https://docs.rs/diag-lang), from the
spans the cursor reports), or store sources (that is
[`source-lang`](https://docs.rs/source-lang)). It is the seam where a front-end
stops handling raw bytes and starts handling tokens.

---

## Installation

```toml
[dependencies]
lexer-lang = "1"
```

Or from the terminal:

```bash
cargo add lexer-lang
```

The crate is `no_std`-friendly and allocation-free: the cursor borrows its source
and needs neither the standard library nor `alloc`. The default `std` feature only
forwards to the standard-library builds of the family crates; with
`default-features = false` the crate stays `no_std` and the whole surface works as
documented here.

---

## Quick start

```rust
use intern_lang::Interner;
use token_lang::{Symbol, TokenKind};
use lexer_lang::Cursor;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Ident(Symbol),
    Number,
    Whitespace,
    Eof,
}
impl TokenKind for Kind {
    fn is_trivia(&self) -> bool { matches!(self, Kind::Whitespace) }
    fn is_eof(&self) -> bool { matches!(self, Kind::Eof) }
    fn symbol(&self) -> Option<Symbol> {
        match self { Kind::Ident(s) => Some(*s), _ => None }
    }
}

let mut interner = Interner::new();
let mut cursor = Cursor::new("x42 7");
let mut kinds = Vec::new();
while let Some(c) = cursor.first() {
    let kind = if c.is_whitespace() {
        cursor.eat_while(char::is_whitespace);
        Kind::Whitespace
    } else if c.is_ascii_digit() {
        cursor.eat_while(|c| c.is_ascii_digit());
        Kind::Number
    } else {
        cursor.eat_while(|c| c.is_ascii_alphanumeric());
        Kind::Ident(cursor.intern_lexeme(&mut interner))
    };
    kinds.push(cursor.emit(kind).into_kind());
}
assert!(matches!(kinds[0], Kind::Ident(_)));
assert_eq!(kinds[2], Kind::Number);
```

---

## The model

A lexer is a loop: look at the next character, decide which kind of token starts
there, consume its run, emit a token. The cursor supplies exactly the primitives
that loop needs and tracks two positions for you — the current scan position, and
the start of the token in progress. After [`emit`](#finishing-a-token) the token
start jumps to the current position, so the next token begins cleanly where the last
ended.

Everything is zero-copy: the cursor borrows the source, reports positions as
[`BytePos`](#re-exported-types) and runs as [`Span`](#re-exported-types), and hands
lexemes back as borrowed `&str`. Nothing allocates.

---

## `Cursor`

```rust
pub struct Cursor<'a> { /* private */ }
```

A zero-copy cursor over a `&'a str`. Derives `Clone` and `Debug`. Cloning is cheap
(two pointers and two integers) and yields an independent cursor at the same
position — useful for bounded backtracking.

### Construction

```rust
pub fn new(text: &'a str) -> Cursor<'a>
pub fn with_base(text: &'a str, base: u32) -> Cursor<'a>
pub fn for_source(file: &'a SourceFile) -> Cursor<'a>
```

- **`new`** — a cursor over `text`, reporting positions from base `0`.
- **`with_base`** — positions offset by `base`, for lexing a slice of a larger source
  while keeping spans in the larger source's coordinate space.
- **`for_source`** — a cursor over a [`SourceFile`](#re-exported-types)'s text, with
  the base set so the spans it emits land in the owning
  [`SourceMap`](https://docs.rs/source-lang)'s global position space — the same space
  a `diag-lang` label points with.

```rust
use source_lang::SourceMap;
use lexer_lang::Cursor;

let mut map = SourceMap::new();
map.add("a.txt", "first").expect("fits");      // 0..5
let id = map.add("b.txt", "x").expect("fits"); // 5..6

let cursor = Cursor::for_source(map.source(id).unwrap());
assert_eq!(cursor.pos().to_u32(), 5); // global, not 0-based
```

### Looking ahead

```rust
pub fn first(&self) -> Option<char>
pub fn second(&self) -> Option<char>
pub fn is_eof(&self) -> bool
pub fn remaining(&self) -> &'a str
pub fn pos(&self) -> BytePos
```

- **`first`** — the next character without consuming it, or `None` at end of input.
- **`second`** — the character after `first`, the second of two-character lookahead
  (for `==`, `//`, `..`).
- **`is_eof`** — whether the end of the source is reached.
- **`remaining`** — the unconsumed source text.
- **`pos`** — the current position, in the global space set by the base.

```rust
use lexer_lang::Cursor;

let cursor = Cursor::new("==x");
assert_eq!(cursor.first(), Some('='));
assert_eq!(cursor.second(), Some('='));
assert_eq!(cursor.remaining(), "==x");
assert!(!cursor.is_eof());
```

### Consuming

```rust
pub fn bump(&mut self) -> Option<char>
pub fn bump_if(&mut self, expected: char) -> bool
pub fn eat_while(&mut self, pred: impl FnMut(char) -> bool)
```

- **`bump`** — consume and return the next character, or `None` at end of input.
- **`bump_if`** — consume the next character only if it equals `expected`, returning
  whether it did; for two-character operators after the first is known.
- **`eat_while`** — consume characters while `pred` holds, stopping at the first that
  fails or at end of input. It returns nothing — read what was consumed with
  [`lexeme`](#reading-the-run) — so a side-effect-only call stays clean under
  `unused_results`.

```rust
use lexer_lang::Cursor;

let mut cursor = Cursor::new("if=42");
cursor.eat_while(|c| c.is_ascii_alphabetic()); // "if"
assert_eq!(cursor.lexeme(), "if");
assert!(cursor.bump_if('='));
assert_eq!(cursor.bump(), Some('4'));
```

### Reading the run

```rust
pub fn lexeme(&self) -> &'a str
pub fn token_span(&self) -> Span
pub fn intern_lexeme(&self, interner: &mut Interner) -> Symbol
```

- **`lexeme`** — the source text consumed since the last [`emit`](#finishing-a-token)
  (or since construction): the in-progress token's text.
- **`token_span`** — that run's [`Span`](#re-exported-types), in the global space set
  by the base.
- **`intern_lexeme`** — interns the current lexeme into `interner` and returns its
  [`Symbol`](#re-exported-types), the cheap handle an identifier or keyword token
  carries.

```rust
use intern_lang::Interner;
use lexer_lang::{Cursor, Span};

let mut interner = Interner::new();
let mut cursor = Cursor::with_base("total", 10);
cursor.eat_while(|c| c.is_ascii_alphabetic());
assert_eq!(cursor.lexeme(), "total");
assert_eq!(cursor.token_span(), Span::new(10, 15));
let sym = cursor.intern_lexeme(&mut interner);
assert_eq!(interner.resolve(sym), Some("total"));
```

### Finishing a token

```rust
pub fn emit<K>(&mut self, kind: K) -> Token<K>
pub fn reset_token(&mut self)
```

- **`emit`** — ends the in-progress token, returning a
  [`Token<K>`](#re-exported-types) of `kind` spanning the consumed run, and starts
  the next token at the current position. The `kind` is the language's own — `emit`
  is generic over it, so the cursor never needs to know a language's token set.
  Emitting without having consumed anything (at end of input, say) yields a token
  with an empty span, the natural shape for an end-of-input marker.
- **`reset_token`** — discards the in-progress run *without* producing a token, and
  starts the next token at the current position. This is the counterpart to `emit`
  for trivia a lexer drops rather than keeps: consume the whitespace or comment, then
  `reset_token` so it is not folded into the following token's span. (A lexer that
  *emits* trivia — for a formatter or a lossless tree — uses `emit` with a trivia
  kind and never needs this.)

```rust
use lexer_lang::{Cursor, Span, Token};

let mut cursor = Cursor::new("ab");
cursor.bump();
assert_eq!(cursor.emit("a"), Token::new("a", Span::new(0, 1)));
// The next token starts where the last one ended.
cursor.bump();
assert_eq!(cursor.emit("b"), Token::new("b", Span::new(1, 2)));
```

Dropping leading whitespace instead of emitting it:

```rust
use lexer_lang::{Cursor, Span};

let mut cursor = Cursor::new("  x");
cursor.eat_while(char::is_whitespace);
cursor.reset_token(); // the spaces are not part of any token
cursor.eat_while(|c| c.is_ascii_alphabetic());
assert_eq!(cursor.token_span(), Span::new(2, 3)); // just `x`
```

---

## Re-exported types

lexer-lang re-exports the family types its API speaks, so a lexer can name them
without depending on each crate directly:

```rust
pub use token_lang::{Token, TokenKind};
pub use intern_lang::{Interner, Symbol};
pub use span_lang::{BytePos, Span};
pub use source_lang::SourceFile;
```

- **`Token<K>` / `TokenKind`** — the token type [`emit`](#finishing-a-token) returns
  and the trait a language's kind implements. See
  [`token-lang`](https://docs.rs/token-lang).
- **`Interner` / `Symbol`** — the interner [`intern_lexeme`](#reading-the-run) writes
  to and the handle it returns. See [`intern-lang`](https://docs.rs/intern-lang).
- **`BytePos` / `Span`** — the position and run the cursor reports. See
  [`span-lang`](https://docs.rs/span-lang).
- **`SourceFile`** — the source [`for_source`](#construction) scans. See
  [`source-lang`](https://docs.rs/source-lang).

---

## Diagnostics

lexer-lang does **not** depend on `diag-lang`, and the cursor never builds or
collects diagnostics — that is the lexer author's job, and it is a one-liner. The
cursor reports the span of any run; when a run is a lexical error (an unterminated
string, a stray character), the author builds a `diag_lang::Diagnostic` from that
span. Because [`for_source`](#construction) puts spans in the map's global space, the
label resolves with no further arithmetic:

```rust,ignore
use diag_lang::{Diagnostic, Label, Renderer, Severity};

// `span` came from `cursor.token_span()` on the offending run.
let diag = Diagnostic::new(
    Severity::Error,
    "unterminated string literal",
    Label::new(span, "expected a closing quote"),
);
let rendered = Renderer::new().render(&diag, &map);
```

Keeping the cursor diagnostic-free is deliberate: scanning stays total and
allocation-free, and the same separation `diag-lang` documents — the front-end
decides *what* is wrong, `diag-lang` decides how it *looks* — holds here too.

---

## Feature flags

| Feature | Default | Description |
|---------|---------|-------------|
| `std`   | yes     | Forwards to `std` on the family crates (`span-lang`, `token-lang`, `intern-lang`, `source-lang`). The cursor needs no `std` itself; this only affects the dependency builds. |

There is no `serde` feature: a `Cursor` is a transient borrow of source text with
nothing to serialise.

Disabling `std` keeps the crate `no_std` (it needs no `alloc` either):

```toml
[dependencies]
lexer-lang = { version = "1", default-features = false }
```

---

<sub>Copyright &copy; 2026 <strong>James Gober</strong>.</sub>
