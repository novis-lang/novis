# Handoff

## State

**Stage 7's serialiser question is closed: there is one, and it is `nvs_render::json`.** ADR 0092
§ 3's JSON rendering sits beside `nvs_render::plain::render` rather than in `nvs-runtime` beside the
ladder, because § 1 already puts the model *and all three renderings* in the one crate both the
runtime and the front end depend on — a JSON writer in `nvs-runtime` for the floor plus this one for
everything else is two writers that agree today. That module's own doc comment is the home of the
decision, of the `$`-tagged spelling for a kind JSON has no value for, and of why `msg` is § 6's
`message` spelled short.

**`Core\Log::write` is now a thin binding over it**: `log.rs`'s `record` fills an
`nvs_render::Record` and `nvs_render::json::line` renders it. Three second copies went with it — the
private `Level` enum and its `tag()` table (now `nvs_render::Level::name` and the new
`from_syslog_severity`, which *searches* `Level::ALL` rather than matching the five severities
again), and the hand-written `Serialize` impl. `fields` is walked by `crate::debug::node`, the one
walk `Core\Debug::dump` uses, so ADR 0092 § 5's substitution, redaction and elision now reach a log
record for free.

**A `bytes` field renders rather than throwing.** § 1's model carries the scalar and a reporting
path that throws while reporting has nothing left to report with, so `Core\Log::write` has no
`errors` row at all now. The old refusal case is replaced by
`log-write-renders-a-field-json-has-no-value-for.nvst`; the other two are unchanged.

**`examples/logging.nvs`'s third line is blocked on the floor, not on a config line.** `[log]
handler` exists as `nvs_config::tree::Log::handler` and **nothing reads it** — the only `handler`
readers in `nvs-cli` are the router's. So the acceptance check needs the tier-3/tier-4 rungs, not an
`nvs.toml` key, and the previous handoff understated it.

## Next group

**The floor — ADR 0020 § 6's second caller — over `crates/nvs-runtime/src/deferred.rs`,
`crates/nvs-runtime/src/throwable.rs`, `crates/nvs-config/src/tree.rs` and `nvs.toml`.**

- [ ] **The tier-4 floor writes a record through `nvs_render::json`** — ADR 0020 §§ 5-6. An uncaught
      throw becomes an `nvs_render::Record` at `Level::Error` and is rendered by the same call
      `Core\Log::write` makes, on the diagnostic channel rather than the program's output.
      `crates/nvs-runtime/src/deferred.rs:129` already names the ladder in a comment and
      `crates/nvs-runtime/src/throwable.rs:362` is what `nvs run` prints today. This adds the
      `nvs-runtime` → `nvs-render` edge, whose price — moving `nvs_syntax::bidi` down — is
      `crates/nvs-render/src/lib.rs:56`'s § *Where this sits*, and it is paid now rather than
      deferred.
- [ ] **Tier 3 reads `[log] handler` and spawns it** — ADR 0020 § 3. The key is declared at
      `crates/nvs-config/src/tree.rs:332` with its two reserve settings beside it and has no reader;
      the handler is a `spawn script` isolate taking one `ErrorReport`, charged to the engine's own
      reserve. `crates/nvs-config/src/tree.rs:335`'s two reserve settings are what sizes it, and
      the repository's own entry goes in the root `nvs.toml` beside its `[[app]]` block.
- [ ] **`examples/logging.nvs` green on its three frozen lines** — `docs/agent/loop-goal.toml:2455`.
      The third line is `handler ran`, printed by `examples/logging/handler.nvs` when the child
      `examples/logging/throws.nvs` escalates past every `catch`. Both files are on disk already.

## Backlog

- **`Core\Log::write` refuses a `secret` argument and accepts `tainted` freely** — ADR 0033 § 4, its
  own file set: `crates/nvs-types/src/expr/quals.rs:447` is the positional-argument refusal and
  `:573` the serialiser sink beside it. Small and unblocked.
- `msg` versus § 6's `message` is one word in one of two places — ADR 0020 § 6's prose or the
  fixtures. `crates/nvs-render/src/json.rs`'s module doc names the choice.
- The HTML rendering is ADR 0092 § 3's third and has no milestone open on it.
- `Core\Debug::render` selects a rendering by the sink in force (ADR 0092 § 3's table); it always
  answers plaintext today.
