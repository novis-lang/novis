# Handoff

## State

**`Core\Http\Response` is two slots and the two members that read them back.** `status(): int` and
`text(): tainted string` over slots `status` and `body`, at `crates/nvs-stdlib/src/http.rs:452`.
That a reader is a **member and never a property** is `crate::http`'s module doc's to own — a
`Core` instance has no property a program can reach, so `examples/http.nvs` was corrected to
`->status()` rather than the rule being bent — and the body is `tainted` because pinning settles
where bytes came from and nothing about what is in them, which ADR 0024 § 1's roster now says.

**The transport is still the whole of what is missing behind the five rows.** Every client member
ends at `crates/nvs-stdlib/src/http.rs:700`'s refusal. The `body` slot holds an already-decoded
`string`, so the decode is the transport's one question and no reader asks it twice.

**`examples/http.nvs` now fails on its last two lines and nothing else** — checked with
`nvs check`, not assumed. `Core\Env` exists nowhere but in that example (a grep over `nvs-stdlib`,
`nvs-types` and `docs/spec/01-core-library.md` finds nothing), so item 2 below is a placement
decision. The third error is a separate finding and the interesting one: `$configured ??
"http://elsewhere.invalid/"` types as `string|tainted string`, and `Core\Http::allowUrl`'s
`Qual::Launder` parameter refuses a *union* carrying the tainted arm rather than laundering it.

## Next group

**The transport, and the two errors left in the example.** The file set is
`crates/nvs-stdlib/src/http.rs`, `crates/nvs-host/src/net.rs`, `crates/nvs-stdlib/src/registry.rs`
and `tests/conformance/core/`.

- [ ] **The transport behind the five rows** — ADR 0074 § 6, ADR 0058 § 4. It deletes
      `crates/nvs-stdlib/src/http.rs:700`'s refusal and fills
      `crates/nvs-stdlib/src/http.rs:452`'s two slots through `crate::instance::build` — `status`
      an `int`, `body` a decoded `string` — over `crates/nvs-host/src/net.rs:1`'s parking stream.
      Every retry attempt reuses the pinned `Target` and re-resolves nothing; a redirect hop
      re-pins.
- [ ] **A spelling for `examples/http.nvs:43`'s environment read** — placement first: an ADR 0051
      § 3 roster line, a `docs/spec/01-core-library.md` row, and whether reading the environment is
      one of ADR 0118's doors. ADR 0024 § 1 already says every `Core\Env` member answers the
      tainted form, and the class joins `crates/nvs-stdlib/src/registry.rs:1054`'s roster once it
      exists.
- [ ] **Whether a `Qual::Launder` parameter admits a union that carries the tainted arm** —
      `examples/http.nvs:45` is the whole case, and it is `E0401` today. The rule lives at
      `crates/nvs-types/src/expr/quals.rs:222`'s `admits_tainted_argument`; whichever way it goes,
      ADR 0024 § 3 is where a launderer's argument shape is recorded.

## Backlog
- `Core\Http\Response::header()` and the header-map slot — arrives with the transport that fills
  it, per `crates/nvs-stdlib/src/http.rs:452`'s own doc.
- A reader for a non-text body needs a `CoreTy::TaintedBytes`; `TaintedStr` is the only tainted
  return spelling `crates/nvs-stdlib/src/registry.rs` has.
- `Core\Http\Client::send(Request)` and `stream` — `docs/spec/01-core-library.md` § 16's row names
  both, and neither is a row yet.
- ADR 0074 § 7's dynamic half — a verb chosen at run time throws before the first attempt — waits
  on the `send(Request)` row.
