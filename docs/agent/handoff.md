# Handoff

## State

**Goal 6, Stage 4: ADR 0074 § 1's secure header set is on every response this server writes.**
`crates/nvs-server/src/secure.rs` is the new module and the home for every decision below; nothing
else restates them.

**It fills, it does not overwrite, and that is what makes `setHeader` an override with no ordering
rule.** `Secure::fill` writes a name the response does not already carry and leaves one it does, so
the set is applied at a *single* point — `serve_connection`'s tail — covering a program's response,
a mount table's `404`, a static file's and § 5's `503` alike. `Content-Type` is never in the set:
a body member owns its own media type (ADR 0088 § 4).

**`Serving` is why the two signatures changed.** `serve_on_this_core` and `serve_connection` take
one `Serving` — an `Arc<Admission>` and an `Arc<Secure>` — where they took an `Admission`: those two
are the shared half of a connection's context, `waits` stays a `Copy` value beside them, and the
argument count did not grow.

**HSTS is a `Scheme` parameter, and every request is `Scheme::Http` today.** ADR 0097 § 6's
forwarded walk has not landed and answers `http` while `trusted_proxies` is empty anyway, so § 1's
rule is implemented and asserted on both schemes while nothing yet passes `Https`.
`crates/nvs-server/src/serve.rs`'s `let scheme = Scheme::Http;` is the one line § 6's slice changes.

**A written value the wire cannot carry is read as if it had not been written** — one rule, every
field, so a `\r\n` in a `referrer_policy` answers with the shipped default rather than silently
removing the header. A boot refusal naming the line is better and is a backlog item: it belongs in
`nvs-config`, which has the `nvs-diagnostics` dependency `nvs-server` deliberately does not.

**The failing acceptance check is Stage 5's, not a regression.** `native examples/upload.nvs` wants
ADR 0105's whole surface and `crates/nvs-stdlib` has no `request.rs`; seventeen sessions now, and
nothing in Stage 4 can close it. The playbook's *Tooling* bullet owns it.

**`[context]` gaps, reported again.** `adrs` prints `0074 § 5` but not **`§§ 1 and 4`** — § 1 is the
section this entire slice implements, and slicing it by hand was the session's first call. `0105
§§ 1-4` is not printed either and is what the failing check needs. `spec` still has no selector, so
§ 15 costs two reads a session. `modules` still names no `nvs-cli` pattern, and
`crates/nvs-cli/src/serve.rs` is the boot every server slice has to edit.

## Next group

**What a response says beside its body, continued** — spec § 15, ADR 0074 §§ 1 and 3. One file set:
`crates/nvs-stdlib/src/response.rs`, `crates/nvs-runtime/src/ctx.rs`, `crates/nvs-config/src/tree.rs`.

- [ ] **`redirect`, which is a status and a `Location` in one member** — spec § 15. The five edits
      ride `setHeader`'s: `crates/nvs-stdlib/src/response.rs:246` is the row,
      `crates/nvs-stdlib/src/response.rs:335` the card, `crates/nvs-stdlib/src/response.rs:365` the
      `address()` arm and `crates/nvs-stdlib/src/response.rs:501` the body;
      `crates/nvs-runtime/src/ctx.rs:4193` and `crates/nvs-runtime/src/ctx.rs:4219` are the two
      declarations it makes at once. The URL is a sink (ADR 0088), and the status is a closed set
      rather than any `3xx`.
- [ ] **`addCookie`, whose options shape defaults every field from `[http.cookies]`** — spec § 15
      and ADR 0074 § 3, whose `secure`/`same_site` pair `crates/nvs-config/src/http.rs:105` already
      refuses. `crates/nvs-config/src/tree.rs:426` is the block,
      `crates/nvs-stdlib/src/response.rs:246` the row it joins and
      `crates/nvs-runtime/src/ctx.rs:4219` the channel it crosses on — where `Set-Cookie` is the one
      header that legitimately repeats, so `declare_header`'s set-not-add rule is what to decide
      first.
- [ ] **A boot refusal for an `[http.headers]` value the wire cannot carry** — the fallback above,
      made loud. `crates/nvs-config/src/http.rs:188` is the pass that already refuses §§ 2-3's pairs
      and is where this belongs; `crates/nvs-server/src/secure.rs:177` is `spell`, which stays as
      the layer below rather than being replaced by it.

## Backlog
- ADR 0105's upload surface — `Core\Request::files()` — is what the one failing acceptance check
  needs; `docs/plan/m7.md`'s *Verify* owns its bounded-memory case.
- ADR 0097 § 6's forwarded-header walk: the trusted-proxy scheme, and the only thing that can make
  `Secure::fill` emit HSTS.
- `[http.headers]` is `Runtime`-class in ADR 0074's table and `Secure` is boot-fixed; a reload that
  moves it needs `serve_on_this_core` to hold a snapshot rather than a value.
- ADR 0074 § 2's CORS response headers — refused at boot today, never emitted.
- ADR 0092 § 3's HTML rendering of a `Throwable`, which is what `failed()`'s empty body waits on.
