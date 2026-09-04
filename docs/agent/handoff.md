# Handoff

## State

**Goal 6, Stage 4: spec § 15's four shaping members are complete.** `Core\Response` now has
`setStatus`, `setHeader`, `redirect` and `addCookie` beside ADR 0088 § 4's `text`, `json` and
`bytes`. `html` and `sendFile` remain this module's own known gaps, stated in its `//!`.

**`addCookie` appends where `setHeader` overrides**, and that split is the module doc's opening
section now — `nvs_runtime::Ctx::declare_header` and `::append_header` own the rule, the class is
where a program meets it. A name written twice is sent twice.

**Where a cookie's defaults live: `nvs_config::http::Cookies`, not the member.** ADR 0074 § 3's
four values are resolved in one place off the typed tree, so a bare `addCookie($name, $value)` is
`Secure; HttpOnly; SameSite=Lax; Path=/` and a deployment changes that in one block. Every option
in the row defaults to `Const::Null` — "the call site said nothing" — which is `Core\Queue::push`'s
`maxAttempts` arrangement over `[queue]` and the only spelling that leaves the block anything to
answer. `Core\Response\SameSite` is the ADR 0063 R11 enum; `nvs_config::http::SameSite` is its
config twin, converted across rather than shared.

**Two new boot refusals, both in `nvs_config::http::validate`.** `E0624` for a `same_site` that is
none of the three spellings — which is what lets `Cookies::of` resolve with no fourth arm, so it
never repairs one. `E0625` for an `[http.headers]` value a header line cannot carry: those three
policies reach every response verbatim, and `nvs_server::secure` already drops an unspellable one
in favour of the shipped default, which is right for a request in flight and is exactly why the
boot must refuse it — otherwise the deployment's policy is silently not the one in force.

**The failing acceptance check is Stage 5's, and it is not a regression — this is now verified
rather than assumed.** `examples/upload.nvs`'s own header says it pins "the half of it that already
runs", and it does: `Core\IO` bounds and `Core\IO::within`. The check's wanted stdout
(`parts=2 / field=title / …`) is drafted against `Core\Request::files()`, and **`Core\Request` does
not exist in any form** — no `crates/nvs-stdlib/src/request.rs`, no row in
`registry::CLASSES`. Twenty sessions; it needs the group below, not a fix.

**`[context]` gaps.** `adrs` printed `0074 § 5` alone; this group needed **`§§ 1-3`** and read them
by hand. `0095 § 3` was needed and not printed. `0105 §§ 1-4` is what the failing check needs and
is still absent. `spec` has no selector, so § 15 costs a read a session. `modules` names no
`nvs-diagnostics` pattern, which this group edited.

## Next group

**`Core\Request`, from nothing — the class the standing acceptance failure is waiting on.** One
file set: a new `crates/nvs-stdlib/src/request.rs`, `crates/nvs-stdlib/src/registry.rs`,
`crates/nvs-runtime/src/ctx.rs`. Model it on `crate::server::CLASS`, the other request-scoped class.

- [ ] **The class exists and answers the request line** — `method`, `path`, `query`, spec § 15,
      ADR 0012. `crates/nvs-stdlib/src/server.rs:35` is `CLASS`, the sibling shape to copy;
      `crates/nvs-stdlib/src/registry.rs:1373` is where `crate::response::CLASS` is listed and
      where the new row goes. Nothing crosses from the server yet — assert the members exist and
      answer, as `server.rs`'s own cases do.
- [ ] **`Core\Request::cookie(name)` reads byte for byte, and enforces the prefixes on read** —
      ADR 0095 § 3. This is the exact other half of what landed this session: `__Host-` and
      `__Secure-` are refused on write at
      `crates/nvs-stdlib/src/response.rs:1138` (`nvs_core_response_add_cookie`), and a
      non-conforming cookie carrying either prefix must be **not visible** here. The two halves
      belong in one reading, so take this second.
- [ ] **The inbound carrier on the context** — whatever `cookie`/`header` read from,
      beside the outbound one at `crates/nvs-runtime/src/ctx.rs:4264` (`append_header`) and
      `crates/nvs-runtime/src/ctx.rs:1573` (`config`). Decide and record whether this is a parsed
      map or the raw lines; ADR 0095 § 3 wants one place that parses the header.

## Backlog

- `Core\Response::html` and `sendFile` — the two body members still open, `crates/nvs-stdlib/src/response.rs`'s `//!`.
- ADR 0105's `files()`/`saveTo`/`readAll` proper, once `Core\Request` exists — `docs/plan/m7.md`'s bounded-memory case is the load-bearing one.
- `frame_ancestors` and `hsts` are `Setting`s and get no `E0625` scan — `crates/nvs-config/src/http.rs`.
- `http.cookies.*` has no row in `nvs_config::directive`, so `Inbound::assign`'s two cookie arms are unreachable from `Core\Config::set` — `crates/nvs-config/src/directive.rs`.
- ADR 0074 § 4 says every directive is `Runtime`; the cookie block is read per call, so a reload is picked up — untested.
