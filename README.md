<h1 align="center">
    <img width="99" alt="Rust logo" src="https://raw.githubusercontent.com/jamesgober/rust-collection/72baabd71f00e14aa9184efcb16fa3deddda3a0a/assets/rust-logo.svg">
    <br>
    <b>lexer-lang</b>
    <br>
    <sub><sup>LEXER & TOKENIZER</sup></sub>
</h1>

<div align="center">
    <a href="https://crates.io/crates/lexer-lang"><img alt="Crates.io" src="https://img.shields.io/crates/v/lexer-lang"></a>
    <a href="https://crates.io/crates/lexer-lang"><img alt="Downloads" src="https://img.shields.io/crates/d/lexer-lang?color=%230099ff"></a>
    <a href="https://docs.rs/lexer-lang"><img alt="docs.rs" src="https://img.shields.io/docsrs/lexer-lang"></a>
    <a href="https://github.com/jamesgober/lexer-lang/actions"><img alt="CI" src="https://github.com/jamesgober/lexer-lang/actions/workflows/ci.yml/badge.svg"></a>
    <a href="https://github.com/rust-lang/rfcs/blob/master/text/2495-min-rust-version.md"><img alt="MSRV" src="https://img.shields.io/badge/MSRV-1.85%2B-blue"></a>
</div>

<br>

<div align="left">
    <p>
        lexer-lang is the MAIN-tier crate that follows: The scanner that turns source text into a token stream. First real HQL handoff. Part of the -lang language-construction family; see _strategy/LANG_COLLECTION.md for the master plan.
    </p>
    <br>
    <hr>
    <p>
        <strong>MSRV is 1.85+</strong> (Rust 2024 edition).
    </p>
    <blockquote>
        <strong>Status: stable (1.0).</strong> The public API is frozen under Semantic Versioning — no breaking changes before <code>2.0</code>. See <a href="./CHANGELOG.md"><code>CHANGELOG.md</code></a> and <a href="./docs/API.md"><code>docs/API.md</code></a>.
    </blockquote>
</div>

<hr>
<br>

## Installation

```toml
[dependencies]
lexer-lang = "1"
```

<br>

## Usage

A lexer is a loop over a `Cursor`: peek at the next character, consume its run, and
emit a `Token<K>` of the language's own kind.

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
    let _token = cursor.emit(kind);
}
```

See <a href="./docs/API.md"><code>docs/API.md</code></a> for the full surface.

<br>

## Status

<strong>Status: stable (1.0).</strong> <code>v1.0.0</code> freezes the `Cursor`
scanner under Semantic Versioning: no breaking changes before <code>2.0</code>. See
the <a href="./dev/ROADMAP.md"><code>ROADMAP</code></a> and <a href="./docs/API.md"><code>docs/API.md</code></a>.

<hr>
<br>

## Contributing

See <a href="./dev/DIRECTIVES.md"><code>dev/DIRECTIVES.md</code></a> for engineering standards and the definition of done. Before a PR: `cargo fmt --all`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all-features` must be clean.

<br>

<div id="license">
    <h2>License</h2>
    <p>Licensed under either of</p>
    <ul>
        <li><b>Apache License, Version 2.0</b> &mdash; <a href="./LICENSE-APACHE">LICENSE-APACHE</a></li>
        <li><b>MIT License</b> &mdash; <a href="./LICENSE-MIT">LICENSE-MIT</a></li>
    </ul>
    <p>at your option.</p>
</div>

<div align="center">
  <h2></h2>
  <sup>COPYRIGHT <small>&copy;</small> 2026 <strong>James Gober <me@jamesgober.com>.</strong></sup>
</div>
