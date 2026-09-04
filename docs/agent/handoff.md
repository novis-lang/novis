# Handoff

## State

**Goal 6, Stage 4: `Core\Response::redirect` is the fourth of spec § 15's non-body members.**
`crates/nvs-stdlib/src/response.rs` owns every decision below in its module doc; nothing else
restates them.

**It declares a status and a `Location` at once, and the status is a closed enum.**
`Core\Response\Redirect` — `SeeOther` 303, `Temporary` 307, `Permanent` 308 — is the first `Core`
enum whose case values are not numbered from zero: the ordinal *is* the status code. `301` and
`302` are left out because RFC 9110 still lets a client rewrite a `POST` into a `GET` under them,
and nothing is thereby unspellable — `setStatus(301)` beside `setHeader("Location", …)` is the
general pair this member is the shorthand for, and a conformance case pins that it still works.

**The URL is a sink and is checked as a header value**, `carriable` plus non-empty, reusing
`setHeader`'s byte rule rather than growing a third near-identical predicate. Both checks run
before either declaration, so a refusal leaves neither the status nor the header behind.

**`addCookie` is a four-crate slice, not a five-edit one.** Nothing in this tree can write one
header name twice: `Ctx::declare_header` overwrites in place and `nvs_server::serve`'s answer
calls `hyper`'s replacing `insert`. A repeated `Set-Cookie` is dropped silently at both layers, so
an appending path is owed at each before the member exists. The playbook's *Writing Novis itself*
bullet owns it. That is why this session stopped at one slice: the group's declared file set does
not contain `crates/nvs-server/src/serve.rs`.

**The failing acceptance check is still Stage 5's, not a regression.** `native examples/upload.nvs`
wants ADR 0105's whole surface and `crates/nvs-stdlib` has no `request.rs`; eighteen sessions now,
and nothing in Stage 4 can close it. The playbook's *Tooling* bullet owns it.

**`[context]` gaps, reported again.** `adrs` prints `0074 § 5` but not **`§§ 1 and 4`** — § 4 is
the override rule `setHeader` and now `redirect` both write through. `0105 §§ 1-4` is not printed
either and is what the failing check needs. `spec` still has no selector, so § 15 costs a read a
session. `modules` names no `nvs-cli` pattern, and now also no `nvs-config/src/tree.rs` — the
`[http.cookies]` block the next slice defaults from lives there.

## Next group

**A response that can write one header name twice, and then the cookie that needs it** — spec
§ 15, ADR 0074 §§ 3-4. One file set: `crates/nvs-runtime/src/ctx.rs`,
`crates/nvs-server/src/serve.rs`, `crates/nvs-stdlib/src/response.rs`,
`crates/nvs-config/src/tree.rs`.

- [ ] **An appending header path, so a name a program means twice survives both layers** — the
      trap above. `crates/nvs-runtime/src/ctx.rs:4219` is `declare_header`, which overwrites in
      place, and `crates/nvs-server/src/serve.rs:543` is the `headers_mut().insert(name, value)`
      that would collapse the pair again even if the context carried it. `hyper`'s `append` is the
      second half; the first is a sibling of `declare_header` that pushes without searching, since
      the replacing one is what `setHeader` is named for and must stay.
- [ ] **`addCookie`, whose options shape defaults every field from `[http.cookies]`** — spec § 15,
      ADR 0074 § 3. `crates/nvs-config/src/tree.rs:429` is `HttpCookies` and it has four fields
      (`secure`, `http_only`, `same_site`, `path`), so the bag's own defaults are `Const::Null`
      —  "not given" — and the helper reads the snapshot through `Ctx::config()`, which
      `crates/nvs-stdlib/src/config.rs` already does. `crates/nvs-stdlib/src/response.rs:288` is
      where the row goes, after `redirect`'s; `crates/nvs-stdlib/src/cli.rs:446` is the worked
      `CoreOption` bag. `SameSite` is an enum and never a string, beside
      `crates/nvs-stdlib/src/response.rs:459`.
- [ ] **A boot refusal for an `[http.headers]` value the wire cannot carry** — today a value
      holding a `\r\n` is read as if it had not been written, which
      `crates/nvs-server/src/secure.rs` owns. `crates/nvs-config/src/tree.rs:388` is the
      `HttpHeaders` block whose fields it would check, `crates/nvs-config/src/http.rs:109` is the
      worked boot refusal to copy, and `crates/nvs-diagnostics/src/lib.rs:1501` is the last
      configuration code, so `E0624` is the next free one. The refusal belongs in `nvs-config`,
      which has the `nvs-diagnostics` dependency `nvs-server` deliberately does not.

## Backlog

- ADR 0097 § 6's forwarded walk, the one line `crates/nvs-server/src/serve.rs`'s
  `let scheme = Scheme::Http;` changes — ADR 0097 § 6.
- `Core\Response::html` and `sendFile`, the two body members still owed — response.rs's module doc.
- Raw/unparsed body access for an arbitrary content-type — ADR 0024's *Revisiting*, m7.md.
- `examples/upload.nvs` needs ADR 0105's whole surface; there is no `nvs-stdlib/src/request.rs`.
