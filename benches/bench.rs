//! Criterion benchmarks for the cursor's scanning throughput.
//!
//! The path that matters is a full lex: walking a whole source, classifying each
//! character, consuming runs, and emitting tokens. A second benchmark isolates raw
//! `bump` throughput — the lower bound the classification work sits on top of.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use intern_lang::{Interner, Symbol};
use lexer_lang::{Cursor, Token, TokenKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Ident(Symbol),
    Int,
    Plus,
    Whitespace,
    Eof,
}

impl TokenKind for Kind {
    fn is_trivia(&self) -> bool {
        matches!(self, Kind::Whitespace)
    }
    fn is_eof(&self) -> bool {
        matches!(self, Kind::Eof)
    }
    fn symbol(&self) -> Option<Symbol> {
        match self {
            Kind::Ident(s) => Some(*s),
            _ => None,
        }
    }
}

/// `n` repetitions of `nameNN 42 + ` — a representative mix of identifiers,
/// numbers, an operator, and whitespace.
fn program(n: usize) -> String {
    let mut s = String::with_capacity(n * 12);
    for i in 0..n {
        s.push_str("name");
        s.push_str(&i.to_string());
        s.push_str(" 42 + ");
    }
    s
}

fn lex_all(src: &str, interner: &mut Interner) -> usize {
    let mut cursor = Cursor::new(src);
    let mut count = 0usize;
    while let Some(c) = cursor.first() {
        let kind = if c.is_whitespace() {
            cursor.eat_while(char::is_whitespace);
            Kind::Whitespace
        } else if c.is_ascii_digit() {
            cursor.eat_while(|c| c.is_ascii_digit());
            Kind::Int
        } else if c == '+' {
            let _ = cursor.bump();
            Kind::Plus
        } else {
            cursor.eat_while(|c| c.is_ascii_alphanumeric());
            Kind::Ident(cursor.intern_lexeme(interner))
        };
        let tok: Token<Kind> = cursor.emit(kind);
        count += black_box(&tok).is_trivia() as usize;
    }
    let _ = cursor.emit(Kind::Eof);
    count
}

fn bench_lex_program(c: &mut Criterion) {
    let src = program(1_000); // ~6k tokens
    c.bench_function("lex_program/1000", |b| {
        b.iter(|| {
            let mut interner = Interner::new();
            black_box(lex_all(black_box(&src), &mut interner))
        });
    });
}

fn bench_bump_throughput(c: &mut Criterion) {
    let src = program(1_000);
    c.bench_function("bump_drain/1000", |b| {
        b.iter(|| {
            let mut cursor = Cursor::new(black_box(&src));
            let mut n = 0u64;
            while cursor.bump().is_some() {
                n += 1;
            }
            black_box(n)
        });
    });
}

criterion_group!(benches, bench_lex_program, bench_bump_throughput);
criterion_main!(benches);
