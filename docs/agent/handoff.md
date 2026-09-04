# Handoff

## State

**Goal 6, Stage 4: the response path can now write one header name twice, at both layers.**
`Ctx::declare_header`'s doc comment and `DeclaredHeader`'s own — `crates/nvs-runtime/src/ctx.rs` —
own every decision below; nothing else restates them.

**A declared header is a row, not a pair.** `nvs_runtime::DeclaredHeader { name, value, append }`
crosses `Completion::headers`, and `nvs_server::answer` writes each row with `insert` or with
`hyper`'s `append` as that flag says. Which of the two a row is cannot be read back off a pair — a
repeated name looks identical either way — so the row carries it and no layer below guesses.

**`Ctx::append_header` pushes without searching, and `declare_header` now replaces *totally*:** every
further value already under that name is dropped, so a set leaves the name with exactly one value.
That is what makes the server's row-at-a-time write safe — a replacing row always precedes every
appending row for its name, so an `insert` never wipes a value that was meant to survive.

**`addCookie` is a two-file slice now, not a four-crate one.** Both layers append; what is left is
the member, its options bag and the `[http.cookies]` block it defaults from.

**The failing acceptance check is still Stage 5's, not a regression.** `native examples/upload.nvs`
wants ADR 0105's whole surface and `crates/nvs-stdlib` has no `request.rs`; nineteen sessions now,
and nothing in Stage 4 can close it. The playbook's *Tooling* bullet owns it.

**`[context]` gaps, reported again.** `adrs` prints `0074 § 5` and none of **`§§ 1-4`**, which is
every section the group below names — § 2 is the `[http.headers]` defaults and §§ 3-4 the cookie and
override rules. `0105 §§ 1-4` is not printed either and is what the failing check needs. `spec` still
has no selector, so § 15 costs a read a session. `modules` names no `nvs-cli` pattern.

## Next group

**The two members that read a `[http.*]` block, and the boot refusal beside them** — spec § 15,
ADR 0074 §§ 2-4. One file set: `crates/nvs-stdlib/src/response.rs`, `crates/nvs-config/src/tree.rs`,
`crates/nvs-server/src/secure.rs`.

- [ ] **`addCookie`, whose options shape defaults every field from `[http.cookies]`** — spec § 15,
      ADR 0074 § 3. The runtime half is landed and is the one to call:
      `crates/nvs-runtime/src/ctx.rs:4264` is `append_header`, which takes the rendered line and
      pushes it. `crates/nvs-stdlib/src/response.rs:706` is `setHeader`'s body — the sink and
      `carriable` checks to reuse — and `crates/nvs-stdlib/src/response.rs:749` is `redirect`, the
      newest row of the five edits to copy. The block it defaults from does not exist yet:
      `crates/nvs-config/src/tree.rs:366` is `Http`, which has no `cookies` field.
- [ ] **A boot refusal for an `[http.headers]` value the wire cannot carry** — ADR 0074 § 2.
      `crates/nvs-server/src/secure.rs:177` is `spell`, which today falls back to the shipped default
      and says nothing, so an operator's typo is a header they think they set;
      `crates/nvs-config/src/tree.rs:391` is `HttpHeaders`, the block whose fields it reads. Next
      free in the boot band is `E0624`.

## Backlog

- `native examples/upload.nvs` — Stage 5's `Core\Request::files()`, ADR 0105 §§ 1-4;
  `docs/agent/loop-goal.toml` is the check.
- The `[context]` gaps above — `docs/agent/loop-goal.toml`.
- Nothing declares a header on the CLI path, so `Ctx::append_header` has one caller-to-be —
  `crates/nvs-runtime/src/ctx.rs`'s own doc.
