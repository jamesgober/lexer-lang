# lexer-lang - Roadmap

> Path from scaffold to a stable 1.0. Hard parts are front-loaded; each phase has hard exit criteria.
> Master plan: ../../_strategy/LANG_COLLECTION.md
>
> **Anti-deferral rule:** no listed hard task moves to a later phase unless this file records the move and the reason.

## v0.1.0 - Scaffold (DONE)
Compiles, CI green, structure correct, no domain logic.
- [x] Manifest, README, CHANGELOG, REPS, dual license, CI, deny, clippy, rustfmt.

## v0.2.0 - Core (DONE)
The scanner that turns source text into a token stream. First real HQL handoff.
Dependencies (wires token, source, diag, intern) are wired here, when first used.
Exit criteria:
- [x] Every public item has rustdoc + a runnable example.
- [x] Core invariants property-tested (full DIRECTIVES + API authored at this stage).

Delivered: `Cursor`, a zero-copy scanner over `&str` — peek (`first`/`second`),
advance (`bump`/`bump_if`/`eat_while`), read (`lexeme`/`token_span`/`intern_lexeme`),
and `emit` to build a `Token<K>`. Wires `span` (positions), `token` (output),
`intern` (`intern_lexeme`), and `source` (`for_source`, global-space spans).
`diag` is wired by the lexer *author*, not the cursor — see `dev/NOTES.md` for that
decision and the deferred token-stream iterator. Property tests cover every
section-4 invariant over arbitrary UTF-8; `dev/DIRECTIVES.md` and `docs/API.md`
authored.

## v1.0.0 - API freeze
Public surface stable and frozen until 2.0.
- [ ] docs/API.md marked stable; SemVer promise recorded.
- [ ] Full test + benchmark suite green on all three platforms.
