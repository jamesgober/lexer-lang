//! Integration test: a small but realistic lexer driven entirely through the
//! public `Cursor` API, then wired to `source-lang` and `diag-lang` the way a real
//! front-end is — tokens carry global spans, and a lexical error becomes a rendered
//! diagnostic.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use diag_lang::{Diagnostic, Label, Renderer, Severity};
use intern_lang::Interner;
use lexer_lang::{Cursor, Span, Symbol, Token, TokenKind};
use source_lang::SourceMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Ident(Symbol),
    Int,
    Eq,
    EqEq,
    Str,
    /// An unterminated string literal: the run from the opening quote to end of
    /// input. The lexer records its span so the caller can diagnose it.
    UnterminatedStr,
    LineComment,
    Whitespace,
    Eof,
    Unknown,
}

impl TokenKind for Kind {
    fn is_trivia(&self) -> bool {
        matches!(self, Kind::Whitespace | Kind::LineComment)
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

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}
fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Scan one token from the cursor. Returns `None` at end of input.
fn next_token(cursor: &mut Cursor<'_>, interner: &mut Interner) -> Option<Token<Kind>> {
    let c = cursor.first()?;
    let kind = if c.is_whitespace() {
        cursor.eat_while(char::is_whitespace);
        Kind::Whitespace
    } else if c == '/' && cursor.second() == Some('/') {
        cursor.eat_while(|c| c != '\n');
        Kind::LineComment
    } else if is_ident_start(c) {
        cursor.eat_while(is_ident_continue);
        Kind::Ident(cursor.intern_lexeme(interner))
    } else if c.is_ascii_digit() {
        cursor.eat_while(|c| c.is_ascii_digit());
        Kind::Int
    } else if c == '=' {
        cursor.bump();
        if cursor.bump_if('=') {
            Kind::EqEq
        } else {
            Kind::Eq
        }
    } else if c == '"' {
        cursor.bump(); // opening quote
        cursor.eat_while(|c| c != '"');
        if cursor.bump_if('"') {
            Kind::Str
        } else {
            Kind::UnterminatedStr
        }
    } else {
        cursor.bump();
        Kind::Unknown
    };
    Some(cursor.emit(kind))
}

fn lex_all(src: &str, interner: &mut Interner) -> Vec<Token<Kind>> {
    let mut cursor = Cursor::new(src);
    let mut tokens = Vec::new();
    while let Some(tok) = next_token(&mut cursor, interner) {
        tokens.push(tok);
    }
    tokens.push(cursor.emit(Kind::Eof));
    tokens
}

#[test]
fn test_lexes_a_small_program_into_the_expected_kinds() {
    let mut interner = Interner::new();
    let tokens = lex_all("foo == 42 // note\n", &mut interner);

    let significant: Vec<Kind> = tokens
        .iter()
        .filter(|t| !t.is_trivia() && !t.is_eof())
        .map(|t| *t.kind())
        .collect();

    assert_eq!(
        significant,
        vec![Kind::Ident(interner.intern("foo")), Kind::EqEq, Kind::Int]
    );
    assert!(tokens.last().unwrap().is_eof());
}

#[test]
fn test_two_char_and_one_char_operators_are_distinguished() {
    let mut interner = Interner::new();
    let tokens = lex_all("= ==", &mut interner);
    let ops: Vec<Kind> = tokens
        .iter()
        .filter(|t| matches!(t.kind(), Kind::Eq | Kind::EqEq))
        .map(|t| *t.kind())
        .collect();
    assert_eq!(ops, vec![Kind::Eq, Kind::EqEq]);
}

#[test]
fn test_token_spans_tile_the_source() {
    let mut interner = Interner::new();
    let src = "a = \"hi\" 7";
    let tokens = lex_all(src, &mut interner);

    // Drop the zero-width EOF, then the spans cover [0, len) with no gaps.
    let spans: Vec<Span> = tokens
        .iter()
        .filter(|t| !t.is_eof())
        .map(Token::span)
        .collect();
    assert_eq!(spans.first().unwrap().start().to_u32(), 0);
    assert_eq!(spans.last().unwrap().end().to_u32(), src.len() as u32);
    for pair in spans.windows(2) {
        assert_eq!(pair[0].end(), pair[1].start());
    }
}

#[test]
fn test_identifiers_resolve_back_through_the_interner() {
    let mut interner = Interner::new();
    let tokens = lex_all("alpha beta alpha", &mut interner);
    let names: Vec<&str> = tokens
        .iter()
        .filter_map(|t| t.symbol())
        .map(|s| interner.resolve(s).unwrap())
        .collect();
    assert_eq!(names, vec!["alpha", "beta", "alpha"]);
    // `alpha` interned once: the two occurrences share a symbol.
    let alpha: Vec<Symbol> = tokens.iter().filter_map(|t| t.symbol()).collect();
    assert_eq!(alpha[0], alpha[2]);
}

#[test]
fn test_global_spans_drive_a_rendered_diagnostic() {
    // A lexer error path end to end: lex a file from a SourceMap (so spans are
    // global), find the unterminated string, and render a diagnostic pointing at it.
    let mut map = SourceMap::new();
    let id = map.add("greeting.txt", "say \"oops").expect("fits");

    let mut interner = Interner::new();
    let file = map.source(id).expect("just added");
    let mut cursor = Cursor::for_source(file);

    let mut bad: Option<Span> = None;
    while let Some(tok) = next_token(&mut cursor, &mut interner) {
        if *tok.kind() == Kind::UnterminatedStr {
            bad = Some(tok.span());
        }
    }
    let span = bad.expect("the string is unterminated");

    // The span is in the map's global space, so a label resolves straight away.
    let diag = Diagnostic::new(
        Severity::Error,
        "unterminated string literal",
        Label::new(span, "expected a closing quote"),
    );
    let rendered = Renderer::new().render(&diag, &map);
    assert!(rendered.contains("unterminated string literal"));
    assert!(rendered.contains("expected a closing quote"));
}
