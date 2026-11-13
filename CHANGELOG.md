<h1 align="center">
    <img width="90px" height="auto" src="https://raw.githubusercontent.com/jamesgober/jamesgober/main/media/icons/hexagon-3.svg" alt="Triple Hexagon">
    <br><b>CHANGELOG</b>
</h1>
<p>
  All notable changes to <code>lexer-lang</code> will be documented in this file. The format is based on <a href="https://keepachangelog.com/en/1.1.0/">Keep a Changelog</a>,
  and this project adheres to <a href="https://semver.org/spec/v2.0.0.html/">Semantic Versioning</a>.
</p>

---

## [Unreleased]

## [1.0.1] - 2026-11-12

### Fixed

- Positions past 4 GiB now saturate. The cursor turned its local byte offset
  into a `u32` with a plain cast, so on a text over 4 GiB the offset wrapped:
  a token that crossed the 4 GiB mark got a wrong span, and `lexeme` sliced
  backwards and panicked. Only the `base + offset` addition saturated, though
  the docs said all position arithmetic did. The cursor now keeps local offsets
  as `usize`, so `lexeme` is exact at any size, and every reported position
  past `u32::MAX` is `u32::MAX`. Texts that fit in 4 GiB behave exactly as
  before.

## [1.0.0] - 2026-06-28

API freeze. `1.0.0` declares the `Cursor` surface stable under Semantic Versioning
until `2.0`: no breaking changes before `2.0`, additions only in minor releases, and
the MSRV (Rust 1.85) rising only in a minor.

### Added

- `Cursor::reset_token` — discards the in-progress run without emitting a token, so a
  lexer can drop trivia (whitespace, comments) instead of folding it into the next
  token's span. Complements `emit` for the discard-style lexers that do not emit
  trivia as tokens.

### Changed

- Position arithmetic (`pos`, `token_span`) now saturates rather than wrapping if a
  user-supplied base plus the source length somehow exceeds the `u32` addressing
  envelope — a span is never silently off by `2^32`.
- Bumped the `token-lang` dependency to `1` (its frozen 1.0 release).
- `docs/API.md` and the crate docs document the surface as the stable `1.0` contract,
  with the SemVer promise recorded.

## [0.2.0] - 2026-06-28

The core: the scanner that turns source text into a token stream. Wires `span-lang`,
`token-lang`, `intern-lang`, and `source-lang` — the first MAIN-tier crate.

### Added

- `Cursor<'a>` — a zero-copy scanner over a `&str`. Construct with `new`,
  `with_base`, or `for_source` (spans in a `SourceMap`'s global space). Look ahead
  with `first`, `second`, `is_eof`, `remaining`, `pos`; consume with `bump`,
  `bump_if`, `eat_while`; read the run with `lexeme`, `token_span`, `intern_lexeme`;
  finish a token with `emit`, which builds a `Token<K>` and starts the next. Borrows
  the source for its whole life and allocates nothing.
- Re-exports of `Token`, `TokenKind` (`token-lang`), `Interner`, `Symbol`
  (`intern-lang`), `BytePos`, `Span` (`span-lang`), and `SourceFile` (`source-lang`).
- Property tests over arbitrary UTF-8 (position/UTF-8 advance, source tiling, base
  offsetting), an integration test that drives a small lexer and renders a
  `diag-lang` diagnostic from a cursor span, and Criterion benchmarks for full-lex
  and raw-`bump` throughput.
- `dev/DIRECTIVES.md` and `docs/API.md`.

### Changed

- Aligned `clippy.toml` MSRV to Rust 1.85 (matching `Cargo.toml` and the sibling
  crates).

### Removed

- The no-op `serde` feature stub: a transient `Cursor` borrow has nothing to
  serialise.

### Fixed

- `Cargo.toml` `keywords` and `categories` were unquoted bare words — invalid TOML
  that prevented the crate from building; they are now quoted strings.

---

## [0.1.0] - 2026-06-18

Initial scaffold and repository bootstrap. No domain logic yet &mdash; this release establishes the structure, tooling, and quality gates the implementation will be built on.

### Added

- `Cargo.toml` with crate metadata, Rust 2024 edition, MSRV 1.85.
- Dual `Apache-2.0 OR MIT` license files.
- `README.md`, `CHANGELOG.md`, and a documentation skeleton.
- `REPS.md` compliance baseline.
- `.github/workflows/ci.yml` CI matrix; `deny.toml`, `clippy.toml`, `rustfmt.toml`.
- `dev/DIRECTIVES.md` and `dev/ROADMAP.md` (committed engineering standards + plan).

[Unreleased]: https://github.com/jamesgober/lexer-lang/compare/v1.0.1...HEAD
[1.0.1]: https://github.com/jamesgober/lexer-lang/compare/v1.0.0...v1.0.1
[1.0.0]: https://github.com/jamesgober/lexer-lang/compare/v0.2.0...v1.0.0
[0.2.0]: https://github.com/jamesgober/lexer-lang/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/jamesgober/lexer-lang/releases/tag/v0.1.0
