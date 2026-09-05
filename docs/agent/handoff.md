# Handoff

## State

**Goal 21, item 19 is closed.** `Core\Uri::decodeComponent` and `decodeFormValue` answer `bytes`, and
`Core\Uri::parseQuery` answers a value as `bytes`; spec § 12 carries the amendment the goal's
§ *Standing decisions* authorized. `crates/nvs-stdlib/src/uri.rs`'s known gap 2 is gone and the
module's *A decoder answers `bytes`* section is its home now. Conformance 1553, differential 275,
both green.

**A name is still a `string`, and that is deliberate.** A name is the array key the pair is filed
under; putting octets outside UTF-8 in a key would break ADR 0009 § 1's by-construction guarantee
the moment a program iterated the array, so `text_from` keeps that one refusal and says so at the
site. The frozen check's "a name and a value decode as octets too" is honoured as *the same
decoder on both halves*, which is what `parse_query_answers_octets_for_a_name_and_for_a_value` pins.

**`Core\Request::query` and `::post` are unchanged**, and `crate::uri::Values` is the whole of the
difference: they share `parse_query`'s bracket walk (spec § 9 requires the same code) and not its
element type, because § 15's row is not this goal's to amend and a served request's parameters are
read as text at the door. `crates/nvs-stdlib/src/request.rs`'s module doc owns that sentence.

**`buildQuery` takes a `bytes` value straight**, which is what keeps it `parseQuery`'s inverse —
`scalar_text` gained one arm rather than an `as string` that would have refused the octets that
survived the wire.

Nothing is blocked on a decision.

## Next group

**Item 20 — a route capture is decoded where it crosses into the program — and then the example.**
One file set: `crates/nvs-stdlib/src/router.rs`, `crates/nvs-server/src/route.rs`,
`crates/nvs-runtime/src/routes.rs`, `examples/octets.nvs`.

- [ ] **The decode at the crossing** — `crates/nvs-stdlib/src/router.rs:863` is the one arm where a
      capture's raw substring becomes a `Value` (`Param::Text` → `Value::str`), reached from
      `crates/nvs-stdlib/src/router.rs:777`'s `match_value`. `crates/nvs-runtime/src/routes.rs:386`
      (`Route::convert`) must stay undecoded — decoding there would run before `Uint`/`Uuid` parsing,
      so `%34` would become `4` — and `crates/nvs-runtime/src/routes.rs:66` is the module doc saying
      the capture arrives still percent-encoded, which this slice rewrites. Decide what a capture
      whose octets are not UTF-8 becomes: the handler binding declares `tainted string`, so it is a
      refusal, and `docs/adr/0102` § 1 is where the rule belongs.
- [ ] **The two named tests** — `docs/agent/loop-goal.toml:4235` files
      `a_route_capture_is_percent_decoded_once_where_it_crosses` and
      `a_uint_capture_is_unchanged_by_the_decode` under `-p nvs-server`, and
      `crates/nvs-server/Cargo.toml` names neither `nvs-stdlib` nor `nvs-types` — so the crate cannot
      reach the decoder the slice above adds. Read that manifest first: this is the playbook's
      *a check filed in a crate that may not host it*, and the repair is to move both to
      `-p nvs-stdlib` in `docs/agent/loop-goal.toml` **and** its byte-identical
      `docs/agent/goals/21-carried-gaps.toml`. `crates/nvs-server/src/route.rs:178` is the shape a
      capture test takes today.
- [ ] **`examples/octets.nvs`** — the stand-in at `examples/octets.nvs:30` is replaced by the program
      its own comment describes, against the frozen `exact` check at `docs/agent/loop-goal.toml:4248`.
      Lines 1 and 2 are landed surface now (`decodeComponent("%ff%fe%fd")` and a `parseQuery` name and
      value); line 3 needs the slice above.

## Backlog

- `crates/nvs-stdlib/src/router.rs`'s `Core\Router\Match::params()` declares
  `array<tainted string|…>` at `:898` — check it still reads true once a capture is decoded.
- `mixed as string` reaching no `bytes` is a real ergonomic edge with no diagnostic; ADR 0009 § 3
  owns whether it should have one.
- The eight spec §§ 16–17 classes still have no owner on the chain — `docs/agent/carried-gaps.md`.
