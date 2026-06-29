# lexer-lang — build notes

> Working notes. Not part of the shipped crate; left **uncommitted** (REPS forbids
> committing planning/handoff notes). `dev/DIRECTIVES.md` and `dev/ROADMAP.md` are
> committed; this file is not.

## STATUS (2026-06-28) — v1.0.0, ready for you to commit/push

lexer-lang is taken through its **v1.0.0 API freeze**, solid and production-ready.
All changes are in the working tree, **uncommitted** — you said you'd handle the
git commit/push.

Currently on branch `feature/core-cursor` (the v0.2.0 commit `b37de4a` is there; main
is still the scaffold). Suggested finish: commit the freeze, fast-forward `main` to
it, tag `v0.2.0` (at b37de4a) and `v1.0.0`, push — mirroring how token-lang was
landed. Then `cargo publish` when ready (token-lang 1.0.0 is already on crates.io, so
the `token-lang = "1"` dep resolves).

### What changed this turn

- **`token-lang` dep bumped `0.2` → `1`** (its frozen 1.0; API-identical, builds green).
- **Polish before freezing (caught in review):**
  - **`Cursor::reset_token`** — discards the in-progress run without emitting, so a
    lexer can *drop* trivia rather than being forced to emit it as tokens. Both
    styles are now first-class. (Without this, a discard-style lexer would fold
    leading whitespace into the next token's span — a real limitation to lock in.)
  - **Saturating position arithmetic** in `pos`/`token_span` — a user-supplied base
    plus length over the `u32` envelope saturates instead of wrapping (REPS integer
    safety). `for_source` already stays in-envelope.
- **v1.0.0 freeze:** version → 1.0.0; `docs/API.md` + crate docs marked the stable
  contract with the SemVer promise; `docs/release/v1.0.0.md`; CHANGELOG `[1.0.0]`;
  ROADMAP checked; README to stable.

### Green (stable + MSRV 1.85)

fmt; clippy `-D warnings` default / all-features / no-default-features; tests
(16 unit + 5 property + 5 integration + 17 doctests); `cargo doc -D warnings`;
`--no-default-features` (`no_std`); `cargo deny check`; benches compile.

## Resolved at the freeze: the cursor is diagnostic-free (now part of the contract)

The one open decision from v0.2.0 is now **frozen as shipped**: the cursor reports
spans and never builds/collects diagnostics; the lexer author turns an error span
into a `diag_lang::Diagnostic` (one-liner, shown in `tests/lexer.rs`). `diag-lang`
stays a **dev-dependency only**, not a runtime dep — a deliberate divergence from the
strategy's "deps: …, diag, …" line, on the reasoning that scanning stays total and
allocation-free and the front-end (not the scanner) decides what is wrong. If you
ever want the cursor to own diagnostics, that is a `2.0` change now.

## Deferred (non-breaking, post-1.0 minor)

- **Token-stream `Iterator` adapter** over a user `FnMut(&mut Cursor) ->
  Option<Token<K>>`, for `for tok in lexer { … }`. Additive; left out of the frozen
  core (YAGNI).

## Local linker workaround (machine-specific — do NOT commit)

Same as the other crates — this machine has no working MSVC `link.exe`; tests/benches/
doctests are linked with the rust-bundled LLD + ScopeCppSDK import libs, set per
PowerShell invocation (never in a committed `.cargo/config`):

```powershell
$lld   = "$env:USERPROFILE\.rustup\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\x86_64-pc-windows-msvc\bin\rust-lld.exe"
$scope = "C:\Program Files\Microsoft Visual Studio\18\Community\SDK\ScopeCppSDK\vc15"
$env:LIB = "$scope\VC\lib;$scope\SDK\lib"
$env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER    = $lld
$env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS = "-Clinker-flavor=lld-link"
$env:RUSTDOCFLAGS = "-Clinker-flavor=lld-link -Clinker=$lld"
```

CI runs on real runners with a complete MSVC, so none of this affects the pipeline.
