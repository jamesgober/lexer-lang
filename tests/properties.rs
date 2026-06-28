//! Property tests for the cursor invariants in `dev/DIRECTIVES.md`, checked over
//! arbitrary UTF-8 input (so multi-byte characters and empty input are covered).

#![allow(clippy::unwrap_used)]

use lexer_lang::{BytePos, Cursor};
use proptest::prelude::*;

proptest! {
    /// Each `bump` returns the next `char` and advances the position by exactly that
    /// character's UTF-8 length; draining ends at `text.len()` and at end of input.
    #[test]
    fn bump_advances_by_char_utf8_len(text in ".*") {
        let mut cursor = Cursor::new(&text);
        let mut expected = 0u32;
        for ch in text.chars() {
            prop_assert_eq!(cursor.pos(), BytePos::new(expected));
            prop_assert_eq!(cursor.bump(), Some(ch));
            expected += ch.len_utf8() as u32;
        }
        prop_assert_eq!(cursor.bump(), None);
        prop_assert_eq!(cursor.pos(), BytePos::new(text.len() as u32));
        prop_assert!(cursor.is_eof());
    }

    /// `first` always agrees with the head of the remaining text, and `is_eof`
    /// agrees with the remaining text being empty — at every step of a drain.
    #[test]
    fn first_and_is_eof_track_the_remaining_text(text in ".*") {
        let mut cursor = Cursor::new(&text);
        loop {
            prop_assert_eq!(cursor.first(), cursor.remaining().chars().next());
            prop_assert_eq!(cursor.is_eof(), cursor.remaining().is_empty());
            if cursor.bump().is_none() {
                break;
            }
        }
    }

    /// Before any `emit`, the lexeme is exactly the consumed prefix of the source.
    #[test]
    fn lexeme_is_the_consumed_prefix(text in ".*") {
        let mut cursor = Cursor::new(&text);
        let mut consumed = String::new();
        while let Some(c) = cursor.bump() {
            consumed.push(c);
            prop_assert_eq!(cursor.lexeme(), consumed.as_str());
        }
    }

    /// Emitting after every single character tiles the source: the token spans cover
    /// `[0, len)` with no gap or overlap, and slicing the source by those spans and
    /// concatenating reproduces the input exactly.
    #[test]
    fn emitting_each_char_tiles_the_source(text in ".*") {
        let mut cursor = Cursor::new(&text);
        let mut joined = String::new();
        let mut prev_end = 0u32;
        while cursor.first().is_some() {
            let _ = cursor.bump();
            let tok = cursor.emit(());
            prop_assert_eq!(tok.span().start().to_u32(), prev_end);
            let lo = tok.span().start().to_usize();
            let hi = tok.span().end().to_usize();
            joined.push_str(&text[lo..hi]);
            prev_end = tok.span().end().to_u32();
        }
        prop_assert_eq!(prev_end, text.len() as u32);
        prop_assert_eq!(joined, text);
    }

    /// A base offset shifts every reported position by exactly the base, and never
    /// changes which characters are produced.
    #[test]
    fn base_offsets_every_position(text in ".*", raw_base in any::<u32>()) {
        // Keep `base + text.len()` within `u32`, the addressable envelope.
        let headroom = (u32::MAX - text.len() as u32).max(1);
        let base = raw_base % headroom;

        let mut plain = Cursor::new(&text);
        let mut based = Cursor::with_base(&text, base);
        loop {
            prop_assert_eq!(based.pos().to_u32(), plain.pos().to_u32() + base);
            prop_assert_eq!(based.token_span().start().to_u32(),
                            plain.token_span().start().to_u32() + base);
            let a = plain.bump();
            let b = based.bump();
            prop_assert_eq!(a, b);
            if a.is_none() {
                break;
            }
        }
    }
}
