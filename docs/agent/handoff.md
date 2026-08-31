# Handoff

## State

**Stage 7 has opened.** `crates/nvs-stdlib/src/log.rs` is `Core\Log::write` and ADR 0092 § 2's
`Core\Log\Level` — rows, cards, a body and an `address()` arm — plus the class row beside its
ADR 0020 sibling `Core\Fatal` and a `None` row in `registry::CAPABILITIES` saying what a log write
reaches instead of a door. The enum's integers **are** the syslog severities ADR 0092 § 2 fixes,
so the mapping is the roster rather than a second table; that module's doc comment is its home.

**A record is `{"level":…,"msg":…}` plus a `fields` object when the bag is not empty**, written to
the program's own output stream (`Ctx::write_output`, not `write_diagnostic` — the module doc owns
the split). `fields` goes through `crate::json::Encodable`, now `pub(crate)` with a `document`
constructor, so the value encoder is shared with `Core\Json::encode`. Three conformance cases pin
the renderings, the empty-bag omission and the `bytes` refusal.

**The record itself is still written here, which is the thing ADR 0020 § 6 exists to prevent** —
`nvs_render::Record` (`crates/nvs-render/src/lib.rs:387`) is ADR 0092 § 1's model and nothing in
`nvs-runtime` emits a record at all yet. That is the next slice, and `log.rs`'s module doc says so
under *Known gap*. `examples/logging.nvs` is still red: the acceptance check's third line needs
`nvs.toml` to name `examples/logging/handler.nvs` for the child, which nothing configures yet.

## Next group

**Stage 7's remaining three, over `crates/nvs-stdlib/src/log.rs`, `crates/nvs-render/src/lib.rs`,
`crates/nvs-types/src/expr/quals.rs` and `nvs.toml`.**

- [ ] **`write` reaches one serialiser rather than a second one** — ADR 0020 § 6's whole claim.
      `crates/nvs-stdlib/src/log.rs:322`'s `Record` and `crates/nvs-stdlib/src/log.rs:289`'s
      `render` are what move; `crates/nvs-render/src/lib.rs:387` is ADR 0092 § 1's model they
      should become, and the floor that calls it directly is what does not exist yet. Decide there
      whether the JSON Lines rendering belongs beside `nvs_render::plain::render` or in
      `nvs-runtime` beside the ladder, and say which in the ADR-owning module doc.
- [ ] **`Core\Log::write` refuses a `secret` argument and accepts `tainted` freely** — ADR 0033 § 4
      and ADR 0024. The sibling to copy is `crates/nvs-types/src/expr/quals.rs:511`'s
      `reject_secret_debug_argument`, which asks the same question of `Core\Debug::dump`'s open
      `mixed`; `crates/nvs-types/src/expr/quals.rs:557`'s `contains_secret` is what reaches inside
      the `array<string, mixed>` bag. The goal check names the test:
      `a_secret_operand_at_log_write_fields_is_refused_despite_the_open_type`, `-p nvs-types`.
- [ ] **`examples/logging.nvs` green on its three frozen lines** — `docs/agent/loop-goal.toml:2455`.
      The first two lines land already; the third is the child's handler script, so `nvs.toml`
      needs an `[[app]]` block for `examples/logging/throws.nvs` naming
      `examples/logging/handler.nvs` — the reader is `crates/nvs-cli/src/main.rs`'s
      `configured_origin` neighbourhood, and ADR 0020 § 3 is the contract the block spells.

## Backlog

- ADR 0020 § 6's `ts`, `request_id`, `trace_id`, `span_id`: no source for any of them yet — `log.rs`'s module doc.
- `[log] target`/`format`/`level` are unimplemented directives — ADR 0020 § 4, ADR 0092 §§ 2-3.
- ADR 0106 § 10's floor rotation and identical-record coalescing — the goal check names the test.
- Stage 8's `Core\Reflect`/`Core\Ast`/`Core\Decimal` — `docs/agent/loop-goal.toml:2465`.
- `Core\Debug::dump` writes to the diagnostic channel while a log record writes to output; ADR 0092 § 4 says dump goes to the log, so one of the two is wrong once the floor lands.
