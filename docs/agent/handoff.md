# Handoff

## State

**Goal 4 is running and ADR 0086 § 6 is closed.** `Core\Command` now has all three of its members:
`help`, `run` and `completions`, the last generating a script for each of `Core\Cli\Shell`'s four
cases from the same table `help` renders. What each script completes — the command word and the
selected command's option spellings, and no *value*, because the table carries no types — is
decided in `crates/nvs-stdlib/src/command.rs`'s module doc, which is its one home along with why
each shell gets its own idiom rather than one shape bent four ways.

**A program now carries the name the shell knows it by**, which is what the four scripts register
against. `nvs_runtime::Ctx::program_name` is the accessor and its doc comment is the one home for
which name that is: the *script's* own stem for `nvs run script.nvs` — never `nvs`, or the run
would generate completions for the toolchain — and the *executable's* for ADR 0048's single-file
bundle, whose entry file is a synthetic path no shell has seen. It is filled at
`crates/nvs-cli/src/main.rs:843` from the same path the configuration tree already keys on, so the
bundle case is right for free. Empty everywhere else, and `completions` throws `LogicError` there
rather than inventing a name: a served request is not something a shell completes.

**The driver's acceptance failure is unchanged, and closing it is the next group.**
`examples/crypto.nvs` fails on `Core\Password::hash` — stage 4's fixture waiting on stage 4 — and a
program leg runs before every cargo-named check, so it masks the rest of the list until stage 4
lands. That is why the group below is stage 4 rather than more of § 6. Three fixtures still owe
configuration their own stage must write: `examples/http.nvs` needs stage 5's
`http://127.0.0.1:8099` origin, `examples/logging.nvs` an `[[app]]` block naming
`examples/logging/handler.nvs`, and every remaining fixture that reaches the world its
`net.connect` grant. `nvs_runtime::commands`' gap 1 — four `ArgConv` variants still `Unconverted` —
is unchanged, and `command.rs`'s remaining gap is renumbered 1 (a page names no types).

**`orient.py`'s manifest is unchanged and still wrong in the same ways**: two dead
`[context] modules` selectors — `crates/nvs-host/src/pool.rs` and `crates/nvs-host/src/stream.rs` —
and it wants `crates/nvs-runtime/src/commands.rs`, `crates/nvs-types/src/defaults.rs`,
`crates/nvs-test/src/case.rs`, `crates/nvs-cli/src/main.rs` and `crates/nvs-runtime/src/ctx.rs`
added. Nothing is blocked.

## Next group

**Stage 4 — the crypto half, which is the acceptance check the driver keeps failing.** The file set
is `crates/nvs-stdlib/src/registry.rs`, `crates/nvs-stdlib/src/lib.rs`, two new modules beside them,
and `crates/nvs-types/src/expr/quals.rs`. `examples/crypto.nvs` is the fixture all three answer to,
and its six frozen lines are in `docs/agent/loop-goal.toml`'s stage 4.

- [ ] **`Core\Password::hash`, `::verify` and `::needsRehash`** — a new
      `crates/nvs-stdlib/src/password.rs`, its class registered in
      `crates/nvs-stdlib/src/registry.rs:984` and its symbols in
      `crates/nvs-stdlib/src/lib.rs:312`. **No algorithm argument and no cost argument**: the
      parameters are the library's to choose and `needsRehash` is how a stored hash learns it has
      fallen behind (ADR 0051 § 3, and `examples/crypto.nvs`'s own header). Picking the Argon2id
      crate is pre-authorized under ADR 0051 § 4 and owes the `[workspace.dependencies]` comment,
      `cargo deny check` and `python tools/gen-attribution.py`.
- [ ] **`Core\Crypto::generateKey`, `::seal` and `::open`, AEAD only** — a new
      `crates/nvs-stdlib/src/crypto.rs` over the same two anchors
      (`crates/nvs-stdlib/src/registry.rs:984`, `crates/nvs-stdlib/src/lib.rs:312`). No ECB, no
      unauthenticated CBC, no cipher-name-as-string; `open` on a truncated message throws rather
      than returning what is left. The goal names the test `every_registered_cipher_is_an_aead`.
- [ ] **`hash` and `verify` are the second launderer of `secret`, and the only other one** —
      `crates/nvs-types/src/expr/quals.rs:141` states today that `Core\Secret::reveal` is the whole
      list. ADR 0033 § 3, and the goal names the test
      `reveal_and_the_password_helpers_are_the_only_launderers_of_secret` in `-p nvs-types`.

## Backlog

- Stage 4's four protocol tests — signed cookie, CSRF, TOTP, JWT — `docs/adr/0060`, loop-goal stage 4.
- `examples/http.nvs`'s origin and `examples/logging.nvs`'s `[[app]]` block — their own stages.
- `nvs_runtime::commands` gap 1: the four `ArgConv` variants that are `Unconverted`.
- `crates/nvs-stdlib/src/command.rs` gap 1: a usage page names no parameter types.
- `docs/agent/loop-goal.toml`'s `[context]`: two dead module selectors, five missing.
