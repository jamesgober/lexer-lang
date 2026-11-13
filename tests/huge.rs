//! A text over 4 GiB: positions saturate at `u32::MAX` and the lexeme is exact.
//!
//! Needs about 4 GiB of memory, so it is ignored by default. Run it with
//! `cargo test --release --test huge -- --ignored`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use lexer_lang::{BytePos, Cursor, Span};

#[test]
#[ignore = "allocates 4 GiB"]
#[cfg(target_pointer_width = "64")]
fn test_token_across_the_4_gib_mark() {
    let lead = u32::MAX as usize - 5; // the first token ends 6 bytes before 2^32
    let mut text = String::with_capacity(lead + 16);
    text.extend(core::iter::repeat_n('a', lead));
    text.push_str("bbbbbbbbbbbbbbbb"); // 16 bytes, crossing 2^32

    let mut cursor = Cursor::new(&text);
    cursor.eat_while(|c| c == 'a');
    let first = cursor.emit("a");
    assert_eq!(first.span(), Span::new(0, u32::MAX - 5));

    cursor.eat_while(|c| c == 'b');
    // Before, the local offset wrapped to 10 here: the lexeme sliced
    // `text[4294967290..10]` and panicked, and the span came out as 10..4294967290.
    assert_eq!(cursor.lexeme(), "bbbbbbbbbbbbbbbb");
    assert_eq!(cursor.token_span(), Span::new(u32::MAX - 5, u32::MAX));
    assert_eq!(cursor.pos(), BytePos::new(u32::MAX));
    assert!(cursor.is_eof());
}
