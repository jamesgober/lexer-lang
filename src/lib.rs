//! # lexer_lang
//!
//! The scanner that turns source text into a token stream.
//!
//! lexer-lang provides the [`Cursor`] — a zero-copy scanner over a `&str` — the
//! primitive every lexer is built on, hand-written or generated. A lexer drives the
//! cursor with peek/advance steps and turns each scanned run into a
//! [`Token<K>`](Token), where `K` is the language's own token kind. The cursor owns
//! scanning and nothing else: it does not decide a language's keywords (that is `K`),
//! collect diagnostics (that is the lexer author's, through `diag-lang`), or store
//! sources (that is `source-lang`).
//!
//! It is the first MAIN-tier crate: the seam where a front-end stops handling raw
//! bytes and starts handling tokens.
//!
//! ## The model
//!
//! A lexer is a loop. Look at the next character, decide what kind of token starts
//! there, consume its run, and [`emit`](Cursor::emit) a token. The cursor supplies
//! exactly the primitives that loop needs:
//!
//! - look ahead — [`first`](Cursor::first), [`second`](Cursor::second),
//!   [`is_eof`](Cursor::is_eof);
//! - consume — [`bump`](Cursor::bump), [`bump_if`](Cursor::bump_if),
//!   [`eat_while`](Cursor::eat_while);
//! - read the run — [`lexeme`](Cursor::lexeme), [`token_span`](Cursor::token_span),
//!   [`intern_lexeme`](Cursor::intern_lexeme);
//! - finish a token — [`emit`](Cursor::emit), which builds the `Token<K>` and starts
//!   the next, or [`reset_token`](Cursor::reset_token), which drops the run (for
//!   trivia the lexer discards rather than emits).
//!
//! Everything is zero-copy: the cursor borrows the source, reports positions as
//! [`BytePos`] and [`Span`], and hands lexemes back as borrowed `&str`. Nothing here
//! allocates.
//!
//! ## Positions
//!
//! The spans a cursor emits live in a global position space offset by a base.
//! [`Cursor::for_source`] sets that base from a [`SourceFile`], so the spans resolve
//! directly against the [`SourceMap`](source_lang::SourceMap) a diagnostic renders
//! through — token spans and error labels share one coordinate space.
//!
//! ## Quickstart
//!
//! ```
//! use intern_lang::Interner;
//! use token_lang::{Symbol, TokenKind};
//! use lexer_lang::Cursor;
//!
//! #[derive(Clone, Copy, Debug, PartialEq, Eq)]
//! enum Kind {
//!     Ident(Symbol),
//!     Number,
//!     Whitespace,
//!     Eof,
//! }
//! impl TokenKind for Kind {
//!     fn is_trivia(&self) -> bool { matches!(self, Kind::Whitespace) }
//!     fn is_eof(&self) -> bool { matches!(self, Kind::Eof) }
//!     fn symbol(&self) -> Option<Symbol> {
//!         match self { Kind::Ident(s) => Some(*s), _ => None }
//!     }
//! }
//!
//! let mut interner = Interner::new();
//! let mut cursor = Cursor::new("x42 7");
//! let mut kinds = Vec::new();
//! while let Some(c) = cursor.first() {
//!     let kind = if c.is_whitespace() {
//!         cursor.eat_while(char::is_whitespace);
//!         Kind::Whitespace
//!     } else if c.is_ascii_digit() {
//!         cursor.eat_while(|c| c.is_ascii_digit());
//!         Kind::Number
//!     } else {
//!         cursor.eat_while(|c| c.is_ascii_alphanumeric());
//!         Kind::Ident(cursor.intern_lexeme(&mut interner))
//!     };
//!     kinds.push(cursor.emit(kind).into_kind());
//! }
//! assert!(matches!(kinds[0], Kind::Ident(_)));
//! assert_eq!(kinds[1], Kind::Whitespace);
//! assert_eq!(kinds[2], Kind::Number);
//! ```
//!
//! ## `no_std`
//!
//! The crate is `no_std`-compatible and allocation-free: the cursor borrows its
//! source and needs neither the standard library nor `alloc`. The default `std`
//! feature only forwards to the standard-library builds of the family crates.
//! Disable default features to build for a `no_std` target.
//!
//! ## Stability
//!
//! As of `1.0.0` the public surface is **stable** and follows Semantic Versioning:
//! no breaking changes before `2.0`, additions arrive in minor releases, and the
//! MSRV (Rust 1.85) only rises in a minor. The frozen surface is catalogued in
//! [`docs/API.md`](https://github.com/jamesgober/lexer-lang/blob/main/docs/API.md).

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(unused_must_use)]
#![deny(unused_results)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::print_stdout)]
#![deny(clippy::print_stderr)]
#![deny(clippy::dbg_macro)]
#![deny(clippy::unreachable)]

mod cursor;

pub use cursor::Cursor;

// Re-exported so a lexer can name the types the cursor's API speaks without also
// depending on the family crates directly.
pub use intern_lang::{Interner, Symbol};
pub use source_lang::SourceFile;
pub use span_lang::{BytePos, Span};
pub use token_lang::{Token, TokenKind};
