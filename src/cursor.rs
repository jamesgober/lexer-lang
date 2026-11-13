//! The [`Cursor`]: a zero-copy scanner over source text.

use core::str::Chars;

use intern_lang::{Interner, Symbol};
use source_lang::SourceFile;
use span_lang::{BytePos, Span};
use token_lang::Token;

/// A zero-copy cursor over a `&str`, the primitive every lexer is built on.
///
/// A `Cursor` walks source text one [`char`] at a time, tracking a byte position
/// and the start of the token currently being scanned. A lexer — hand-written or
/// generated — drives it with the peek/advance primitives ([`first`](Cursor::first),
/// [`second`](Cursor::second), [`bump`](Cursor::bump), [`bump_if`](Cursor::bump_if),
/// [`eat_while`](Cursor::eat_while)), then turns the scanned run into a
/// [`Token<K>`](Token) with [`emit`](Cursor::emit). token-lang says what a token is;
/// the cursor says where one ends.
///
/// It is zero-copy and allocation-free: it borrows the source for its whole life,
/// reports positions as [`BytePos`] and runs as [`Span`], and hands back lexemes as
/// borrowed `&str` slices. Advancing is `O(1)`; nothing here allocates.
///
/// # Positions
///
/// Positions are reported in a *global* space offset by a base (`0` by default).
/// Construct with [`for_source`](Cursor::for_source) to lex a [`SourceFile`] from a
/// `source-lang` map, and the spans land in that map's global position space — the
/// same space a `diag-lang` label points with. Use [`with_base`](Cursor::with_base)
/// to set the base directly, or [`new`](Cursor::new) for a standalone `&str` at base
/// `0`.
///
/// # Examples
///
/// A whole, if tiny, lexer: identifiers, `+`, and whitespace, terminated by EOF.
///
/// ```
/// use intern_lang::Interner;
/// use token_lang::{Symbol, Token, TokenKind};
/// use lexer_lang::Cursor;
///
/// #[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// enum Kind {
///     Ident(Symbol),
///     Plus,
///     Whitespace,
///     Eof,
/// }
/// impl TokenKind for Kind {
///     fn is_trivia(&self) -> bool { matches!(self, Kind::Whitespace) }
///     fn is_eof(&self) -> bool { matches!(self, Kind::Eof) }
///     fn symbol(&self) -> Option<Symbol> {
///         match self { Kind::Ident(s) => Some(*s), _ => None }
///     }
/// }
///
/// fn lex(src: &str, interner: &mut Interner) -> Vec<Token<Kind>> {
///     let mut cursor = Cursor::new(src);
///     let mut tokens = Vec::new();
///     while let Some(c) = cursor.first() {
///         let kind = if c.is_whitespace() {
///             cursor.eat_while(char::is_whitespace);
///             Kind::Whitespace
///         } else if c == '+' {
///             cursor.bump();
///             Kind::Plus
///         } else {
///             cursor.eat_while(|c| !c.is_whitespace() && c != '+');
///             Kind::Ident(cursor.intern_lexeme(interner))
///         };
///         tokens.push(cursor.emit(kind));
///     }
///     tokens.push(cursor.emit(Kind::Eof));
///     tokens
/// }
///
/// let mut interner = Interner::new();
/// let tokens = lex("foo + bar", &mut interner);
/// let significant = tokens.iter().filter(|t| !t.is_trivia() && !t.is_eof()).count();
/// assert_eq!(significant, 3); // foo, +, bar
/// assert!(tokens.last().unwrap().is_eof());
/// ```
#[derive(Clone, Debug)]
pub struct Cursor<'a> {
    /// The full source text, kept for slicing lexemes back out.
    text: &'a str,
    /// The unconsumed tail; its length gives the current offset.
    chars: Chars<'a>,
    /// Local byte offset where the in-progress token started. Kept as `usize` so
    /// slicing the lexeme back out is exact even for a text over 4 GiB.
    token_start: usize,
    /// Offset added to every reported position, placing spans in a global space.
    base: u32,
}

impl<'a> Cursor<'a> {
    /// Creates a cursor over `text`, reporting positions from base `0`.
    ///
    /// # Examples
    ///
    /// ```
    /// use lexer_lang::Cursor;
    ///
    /// let mut cursor = Cursor::new("ab");
    /// assert_eq!(cursor.bump(), Some('a'));
    /// assert_eq!(cursor.bump(), Some('b'));
    /// assert!(cursor.is_eof());
    /// ```
    #[inline]
    #[must_use]
    pub fn new(text: &'a str) -> Self {
        Self::with_base(text, 0)
    }

    /// Creates a cursor over `text` whose reported positions are offset by `base`.
    ///
    /// Use this to lex a slice of a larger source while keeping spans in the larger
    /// source's coordinate space. [`for_source`](Cursor::for_source) is the usual
    /// way to get the base right for a `source-lang` file.
    ///
    /// `base + text.len()` is expected to fit in `u32`, the addressable envelope the
    /// `span-lang` / `source-lang` layers cap at. If it does not (a large `base`, or
    /// a text over 4 GiB), every reported position past the ceiling is `u32::MAX`:
    /// positions saturate rather than wrap, so a span is never off by `2^32`, and
    /// [`lexeme`](Cursor::lexeme) still returns the exact text.
    ///
    /// # Examples
    ///
    /// ```
    /// use lexer_lang::{BytePos, Cursor};
    ///
    /// let mut cursor = Cursor::with_base("xy", 100);
    /// assert_eq!(cursor.pos(), BytePos::new(100));
    /// cursor.bump();
    /// assert_eq!(cursor.pos(), BytePos::new(101));
    /// ```
    #[inline]
    #[must_use]
    pub fn with_base(text: &'a str, base: u32) -> Self {
        Self {
            text,
            chars: text.chars(),
            token_start: 0,
            base,
        }
    }

    /// Creates a cursor over a [`SourceFile`]'s text, with its base set so spans
    /// land in the owning [`SourceMap`](source_lang::SourceMap)'s global position
    /// space.
    ///
    /// This is the bridge from `source-lang` to the lexer: the spans the cursor
    /// emits resolve directly against the same map a diagnostic renders through, so
    /// a token's span points at the right file without any further arithmetic.
    ///
    /// # Examples
    ///
    /// ```
    /// use source_lang::SourceMap;
    /// use lexer_lang::Cursor;
    ///
    /// let mut map = SourceMap::new();
    /// map.add("a.txt", "first").expect("fits");      // 0..5
    /// let id = map.add("b.txt", "x").expect("fits"); // 5..6
    ///
    /// let file = map.source(id).unwrap();
    /// let mut cursor = Cursor::for_source(file);
    /// // The single token's span is in the map's global space, not 0-based.
    /// assert_eq!(cursor.pos().to_u32(), 5);
    /// cursor.bump();
    /// assert_eq!(cursor.token_span().start().to_u32(), 5);
    /// ```
    #[inline]
    #[must_use]
    pub fn for_source(file: &'a SourceFile) -> Self {
        Self::with_base(file.text(), file.span().start().to_u32())
    }

    /// The unconsumed source text.
    ///
    /// # Examples
    ///
    /// ```
    /// use lexer_lang::Cursor;
    ///
    /// let mut cursor = Cursor::new("abc");
    /// cursor.bump();
    /// assert_eq!(cursor.remaining(), "bc");
    /// ```
    #[inline]
    #[must_use]
    pub fn remaining(&self) -> &'a str {
        self.chars.as_str()
    }

    /// Whether the cursor has reached the end of the source.
    #[inline]
    #[must_use]
    pub fn is_eof(&self) -> bool {
        self.chars.as_str().is_empty()
    }

    /// The current local byte offset: how far into `text` the cursor has advanced,
    /// ignoring the base.
    #[inline]
    fn offset(&self) -> usize {
        // `text` is the full source and `chars.as_str()` its unconsumed tail, so the
        // difference is the consumed prefix length, always within `text.len()`.
        self.text.len() - self.chars.as_str().len()
    }

    /// Converts a local byte offset to a global position: `base + local`,
    /// saturating at `u32::MAX` both when `local` itself does not fit in `u32`
    /// (a text over 4 GiB) and when the addition overflows.
    #[inline]
    fn global(&self, local: usize) -> u32 {
        let local = u32::try_from(local).unwrap_or(u32::MAX);
        self.base.saturating_add(local)
    }

    /// The current position, in the global space set by the base.
    ///
    /// The base plus the source length should fit in `u32` (the addressable
    /// envelope the source layers cap at). If it does not, the position saturates
    /// at `u32::MAX` rather than wrapping, so it is never silently wrong by `2^32`.
    ///
    /// # Examples
    ///
    /// ```
    /// use lexer_lang::{BytePos, Cursor};
    ///
    /// let mut cursor = Cursor::new("hi");
    /// assert_eq!(cursor.pos(), BytePos::new(0));
    /// cursor.bump();
    /// assert_eq!(cursor.pos(), BytePos::new(1));
    /// ```
    #[inline]
    #[must_use]
    pub fn pos(&self) -> BytePos {
        BytePos::new(self.global(self.offset()))
    }

    /// Peeks the next character without consuming it, or `None` at end of input.
    ///
    /// # Examples
    ///
    /// ```
    /// use lexer_lang::Cursor;
    ///
    /// let cursor = Cursor::new("xy");
    /// assert_eq!(cursor.first(), Some('x'));
    /// ```
    #[inline]
    #[must_use]
    pub fn first(&self) -> Option<char> {
        self.chars.clone().next()
    }

    /// Peeks the character after [`first`](Cursor::first) without consuming
    /// anything — the second character of lookahead, for tokens like `==`, `//`, or
    /// `..`.
    ///
    /// # Examples
    ///
    /// ```
    /// use lexer_lang::Cursor;
    ///
    /// let cursor = Cursor::new("==");
    /// assert_eq!(cursor.first(), Some('='));
    /// assert_eq!(cursor.second(), Some('='));
    /// ```
    #[inline]
    #[must_use]
    pub fn second(&self) -> Option<char> {
        let mut chars = self.chars.clone();
        let _ = chars.next();
        chars.next()
    }

    /// Consumes and returns the next character, or `None` at end of input.
    ///
    /// # Examples
    ///
    /// ```
    /// use lexer_lang::Cursor;
    ///
    /// let mut cursor = Cursor::new("ab");
    /// assert_eq!(cursor.bump(), Some('a'));
    /// assert_eq!(cursor.bump(), Some('b'));
    /// assert_eq!(cursor.bump(), None);
    /// ```
    #[inline]
    pub fn bump(&mut self) -> Option<char> {
        self.chars.next()
    }

    /// Consumes the next character only if it equals `expected`, returning whether
    /// it did. Handy for two-character operators after the first is known.
    ///
    /// # Examples
    ///
    /// ```
    /// use lexer_lang::Cursor;
    ///
    /// let mut cursor = Cursor::new("=>");
    /// assert!(cursor.bump_if('='));
    /// assert!(cursor.bump_if('>'));
    /// assert!(!cursor.bump_if('!')); // no match: nothing consumed
    /// assert!(cursor.is_eof());
    /// ```
    #[inline]
    pub fn bump_if(&mut self, expected: char) -> bool {
        if self.first() == Some(expected) {
            let _ = self.bump();
            true
        } else {
            false
        }
    }

    /// Consumes characters while `pred` holds.
    ///
    /// Stops at the first character for which `pred` is `false`, or at end of input,
    /// leaving that character unconsumed. This is the workhorse for scanning runs —
    /// identifier bodies, digit sequences, whitespace. Read what was consumed back
    /// with [`lexeme`](Cursor::lexeme); it returns nothing so a side-effect-only call
    /// stays clean under `unused_results`.
    ///
    /// # Examples
    ///
    /// ```
    /// use lexer_lang::Cursor;
    ///
    /// let mut cursor = Cursor::new("abc123");
    /// cursor.eat_while(|c| c.is_ascii_alphabetic());
    /// assert_eq!(cursor.lexeme(), "abc");
    /// assert_eq!(cursor.remaining(), "123");
    /// ```
    #[inline]
    pub fn eat_while(&mut self, mut pred: impl FnMut(char) -> bool) {
        while let Some(c) = self.first() {
            if !pred(c) {
                break;
            }
            let _ = self.bump();
        }
    }

    /// The lexeme of the in-progress token: the source text consumed since the last
    /// [`emit`](Cursor::emit) (or since construction).
    ///
    /// # Examples
    ///
    /// ```
    /// use lexer_lang::Cursor;
    ///
    /// let mut cursor = Cursor::new("let x");
    /// cursor.eat_while(|c| c.is_ascii_alphabetic());
    /// assert_eq!(cursor.lexeme(), "let");
    /// ```
    #[inline]
    #[must_use]
    pub fn lexeme(&self) -> &'a str {
        &self.text[self.token_start..self.offset()]
    }

    /// The [`Span`] of the in-progress token, in the global space set by the base:
    /// from where the last [`emit`](Cursor::emit) left off to the current position.
    ///
    /// # Examples
    ///
    /// ```
    /// use lexer_lang::{Cursor, Span};
    ///
    /// let mut cursor = Cursor::with_base("let", 10);
    /// cursor.eat_while(|c| c.is_ascii_alphabetic());
    /// assert_eq!(cursor.token_span(), Span::new(10, 13));
    /// ```
    #[inline]
    #[must_use]
    pub fn token_span(&self) -> Span {
        Span::new(self.global(self.token_start), self.global(self.offset()))
    }

    /// Interns the current [`lexeme`](Cursor::lexeme) into `interner`, returning its
    /// [`Symbol`] — the cheap handle an identifier or keyword token carries.
    ///
    /// # Examples
    ///
    /// ```
    /// use intern_lang::Interner;
    /// use lexer_lang::Cursor;
    ///
    /// let mut interner = Interner::new();
    /// let mut cursor = Cursor::new("name rest");
    /// cursor.eat_while(|c| c.is_ascii_alphabetic());
    /// let sym = cursor.intern_lexeme(&mut interner);
    /// assert_eq!(interner.resolve(sym), Some("name"));
    /// ```
    #[inline]
    pub fn intern_lexeme(&self, interner: &mut Interner) -> Symbol {
        interner.intern(self.lexeme())
    }

    /// Ends the in-progress token, returning a [`Token<K>`](Token) of `kind` spanning
    /// the consumed run, and starts the next token at the current position.
    ///
    /// After `emit`, [`lexeme`](Cursor::lexeme) is empty and
    /// [`token_span`](Cursor::token_span) is a zero-width span at the current
    /// position, until more is consumed. Emitting without having consumed anything —
    /// at end of input, say — yields a token with an empty span, the natural shape
    /// for an end-of-input marker.
    ///
    /// # Examples
    ///
    /// ```
    /// use lexer_lang::{Cursor, Span, Token};
    ///
    /// let mut cursor = Cursor::new("ab");
    /// cursor.bump();
    /// let first = cursor.emit("a");
    /// assert_eq!(first, Token::new("a", Span::new(0, 1)));
    ///
    /// // The next token starts where the last one ended.
    /// cursor.bump();
    /// let second = cursor.emit("b");
    /// assert_eq!(second, Token::new("b", Span::new(1, 2)));
    /// ```
    #[inline]
    pub fn emit<K>(&mut self, kind: K) -> Token<K> {
        let token = Token::new(kind, self.token_span());
        self.token_start = self.offset();
        token
    }

    /// Discards the in-progress run without producing a token, starting the next
    /// token at the current position.
    ///
    /// This is the counterpart to [`emit`](Cursor::emit) for trivia a lexer drops
    /// rather than keeps: consume the whitespace or comment, then `reset_token` so it
    /// is not folded into the span of the token that follows. (A lexer that instead
    /// *emits* trivia — for a formatter or a lossless tree — uses `emit` with a
    /// trivia kind and never needs this.)
    ///
    /// # Examples
    ///
    /// ```
    /// use lexer_lang::{Cursor, Span};
    ///
    /// let mut cursor = Cursor::new("  x");
    /// cursor.eat_while(char::is_whitespace);
    /// cursor.reset_token(); // drop the leading spaces
    ///
    /// cursor.eat_while(|c| c.is_ascii_alphabetic());
    /// // The identifier's span covers only `x`, not the spaces before it.
    /// assert_eq!(cursor.token_span(), Span::new(2, 3));
    /// assert_eq!(cursor.lexeme(), "x");
    /// ```
    #[inline]
    pub fn reset_token(&mut self) {
        self.token_start = self.offset();
    }
}

#[cfg(test)]
mod tests {
    // Tests assert on known-present peeks and just-added sources; unwrapping them
    // is the clearest form and is confined to test code. `bump`/`add` are also
    // called for their side effect, so their results are intentionally dropped.
    #![allow(clippy::unwrap_used, clippy::expect_used, unused_results)]

    extern crate alloc;
    use alloc::string::String;
    use alloc::vec::Vec;

    use super::*;

    #[test]
    fn test_bump_walks_every_char_then_eof() {
        let mut cursor = Cursor::new("abc");
        assert_eq!(cursor.bump(), Some('a'));
        assert_eq!(cursor.bump(), Some('b'));
        assert_eq!(cursor.bump(), Some('c'));
        assert_eq!(cursor.bump(), None);
        assert!(cursor.is_eof());
    }

    #[test]
    fn test_first_and_second_peek_without_consuming() {
        let cursor = Cursor::new("=>x");
        assert_eq!(cursor.first(), Some('='));
        assert_eq!(cursor.second(), Some('>'));
        // Peeking did not advance.
        assert_eq!(cursor.pos(), BytePos::new(0));
    }

    #[test]
    fn test_second_is_none_past_end() {
        let cursor = Cursor::new("a");
        assert_eq!(cursor.first(), Some('a'));
        assert_eq!(cursor.second(), None);
    }

    #[test]
    fn test_bump_if_only_consumes_on_match() {
        let mut cursor = Cursor::new("=>");
        assert!(!cursor.bump_if('!'));
        assert_eq!(cursor.pos(), BytePos::new(0));
        assert!(cursor.bump_if('='));
        assert_eq!(cursor.pos(), BytePos::new(1));
    }

    #[test]
    fn test_eat_while_stops_at_predicate_boundary() {
        let mut cursor = Cursor::new("123abc");
        cursor.eat_while(|c| c.is_ascii_digit());
        assert_eq!(cursor.lexeme(), "123");
        assert_eq!(cursor.remaining(), "abc");
    }

    #[test]
    fn test_lexeme_and_span_track_the_current_run() {
        let mut cursor = Cursor::with_base("let x", 5);
        cursor.eat_while(|c| c.is_ascii_alphabetic());
        assert_eq!(cursor.lexeme(), "let");
        assert_eq!(cursor.token_span(), Span::new(5, 8));
    }

    #[test]
    fn test_emit_resets_the_token_start() {
        let mut cursor = Cursor::new("ab");
        cursor.bump();
        assert_eq!(cursor.emit("a"), Token::new("a", Span::new(0, 1)));
        // Lexeme and span are now empty at the boundary.
        assert_eq!(cursor.lexeme(), "");
        assert_eq!(cursor.token_span(), Span::new(1, 1));
        cursor.bump();
        assert_eq!(cursor.emit("b"), Token::new("b", Span::new(1, 2)));
    }

    #[test]
    fn test_emit_at_eof_is_an_empty_span() {
        let mut cursor = Cursor::new("a");
        cursor.bump();
        let _ = cursor.emit("a");
        let eof = cursor.emit("eof");
        assert_eq!(eof, Token::new("eof", Span::new(1, 1)));
        assert!(eof.span().is_empty());
    }

    #[test]
    fn test_reset_token_drops_the_run_without_emitting() {
        let mut cursor = Cursor::new("  x");
        cursor.eat_while(char::is_whitespace);
        cursor.reset_token();
        cursor.eat_while(|c| c.is_ascii_alphabetic());
        // The whitespace is not folded into the identifier's span.
        assert_eq!(cursor.lexeme(), "x");
        assert_eq!(cursor.token_span(), Span::new(2, 3));
    }

    #[test]
    fn test_reset_token_then_emit_spans_only_the_kept_run() {
        let mut cursor = Cursor::new("// c\nv");
        cursor.eat_while(|c| c != '\n'); // a comment to drop
        cursor.reset_token();
        cursor.bump(); // the '\n'
        cursor.reset_token();
        cursor.bump(); // 'v'
        assert_eq!(cursor.emit("ident"), Token::new("ident", Span::new(5, 6)));
    }

    #[test]
    fn test_positions_saturate_instead_of_wrapping() {
        // A base one byte below the ceiling: advancing past it saturates at u32::MAX
        // rather than wrapping around to a tiny offset.
        let mut cursor = Cursor::with_base("ab", u32::MAX - 1);
        assert_eq!(cursor.pos(), BytePos::new(u32::MAX - 1));
        cursor.bump();
        assert_eq!(cursor.pos(), BytePos::new(u32::MAX));
        cursor.bump();
        assert_eq!(cursor.pos(), BytePos::new(u32::MAX)); // saturated, not wrapped
    }

    #[test]
    fn test_local_offsets_past_u32_saturate() {
        // A text over 4 GiB is too big for a unit test, so check the conversion
        // directly: a local offset that does not fit in u32 pins at u32::MAX
        // instead of wrapping to a small number.
        let cursor = Cursor::with_base("", 0);
        assert_eq!(cursor.global(5), 5);
        assert_eq!(cursor.global(u32::MAX as usize), u32::MAX);
        #[cfg(target_pointer_width = "64")]
        {
            assert_eq!(cursor.global(u32::MAX as usize + 1), u32::MAX);
            assert_eq!(cursor.global(u32::MAX as usize + 11), u32::MAX);
        }
        let based = Cursor::with_base("", 7);
        assert_eq!(based.global(u32::MAX as usize - 3), u32::MAX);
    }

    #[test]
    fn test_multibyte_chars_advance_by_byte_length() {
        // "αβ" is four bytes: each Greek letter is two.
        let mut cursor = Cursor::new("αβ");
        assert_eq!(cursor.bump(), Some('α'));
        assert_eq!(cursor.pos(), BytePos::new(2));
        assert_eq!(cursor.bump(), Some('β'));
        assert_eq!(cursor.pos(), BytePos::new(4));
        assert!(cursor.is_eof());
    }

    #[test]
    fn test_lexeme_slices_on_char_boundaries() {
        let mut cursor = Cursor::new("αβγ");
        cursor.bump();
        cursor.bump();
        assert_eq!(cursor.lexeme(), "αβ");
    }

    #[test]
    fn test_for_source_offsets_spans_into_global_space() {
        use source_lang::SourceMap;

        let mut map = SourceMap::new();
        map.add("a", "first").expect("fits");
        let id = map.add("b", "xy").expect("fits");
        let file = map.source(id).expect("just added");

        let mut cursor = Cursor::for_source(file);
        assert_eq!(cursor.pos().to_u32(), 5);
        cursor.eat_while(|_| true);
        assert_eq!(cursor.token_span(), Span::new(5, 7));
    }

    #[test]
    fn test_intern_lexeme_interns_the_current_run() {
        let mut interner = Interner::new();
        let mut cursor = Cursor::new("foo bar");
        cursor.eat_while(|c| c.is_ascii_alphabetic());
        let foo = cursor.intern_lexeme(&mut interner);
        assert_eq!(interner.resolve(foo), Some("foo"));
    }

    #[test]
    fn test_full_lex_tiles_the_source() {
        // Lexing the whole input, the emitted spans cover it with no gap or overlap.
        let src = "ab cd";
        let mut cursor = Cursor::new(src);
        let mut spans: Vec<Span> = Vec::new();
        let mut lexemes = String::new();
        while !cursor.is_eof() {
            let c = cursor.first().unwrap();
            if c.is_whitespace() {
                cursor.eat_while(char::is_whitespace);
            } else {
                cursor.eat_while(|c| !c.is_whitespace());
            }
            let tok = cursor.emit(());
            lexemes.push_str(&src[tok.span().start().to_usize()..tok.span().end().to_usize()]);
            spans.push(tok.span());
        }
        // Spans tile [0, len) exactly.
        assert_eq!(spans.first().unwrap().start(), BytePos::new(0));
        assert_eq!(spans.last().unwrap().end(), BytePos::new(src.len() as u32));
        for pair in spans.windows(2) {
            assert_eq!(pair[0].end(), pair[1].start());
        }
        assert_eq!(lexemes, src);
    }
}
