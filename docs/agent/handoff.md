# Handoff

## State

**Stage 7's `[log] target` is read.** `nvs_runtime::Ctx::write_log_record` is its only reader and
both of ADR 0092 § 6's writers call it — `Core\Log::write` with `LogChannel::Output` and
`nvs_runtime::floor::report` with `LogChannel::Diagnostic` — so a deployment that names a
destination gets one destination and each caller keeps its own channel only while the directive
names none. The grammar is `nvs_config::log::Target`, read twice on purpose: the boot check refuses
what it cannot spell (`E0613`, new), and the runtime resolves what it can.

- **A named target displaces both default channels, and is not captured.** The record leaves through
  the sink rather than through `Ctx::write_output`, so ADR 0088 § 5's capture stack does not see it
  and `[limits] max_output` is not charged; `write_log_record`'s doc comment is the home of why.
  With nothing configured, both rules apply exactly as before and no fixture's output moved.
- **`syslog` is spelled and not transported.** It resolves, and then routes nowhere new — a syslog
  sink is a datagram carrying ADR 0092 § 2's severity in a priority field, which the byte-oriented
  sinks do not take. Routing it to `stderr` would be a destination this build claims and does not
  reach. `crates/nvs-config/src/log.rs:37` is where that is written down.
- **`log.target` is a `System` directive now** (`crates/nvs-config/src/directive.rs`), which ADR 0020
  § 4 states in as many words and the row did not: a request that could move it could send the record
  of its own failure somewhere nobody reads.
- **The sink is held per context.** `OutputSink::File` counts ADR 0106 § 10's bound against a
  handle, so two contexts writing one configured path hold two handles and each sizes itself from the
  file's length at open. Fine for a CLI run; the served case is in the backlog.

**The acceptance check still names `every_part_two_spec_member_is_registered`** — stage 10's gate
over a *complete* Part II, which needs spec §§ 15-19. Those are goal 6's, so it cannot pass inside
this goal and is not a regression.

## Next group

**The rest of `[log]`: the two directives beside `target` that nothing reads yet, and the case that
pins what landed. All three converge on one call — `write_log_record` takes *bytes* today and both
remaining slices want the record's level, so take them in this order and grow the signature once.
The file set is `crates/nvs-runtime/src/ctx.rs`, `crates/nvs-stdlib/src/log.rs`,
`crates/nvs-runtime/src/floor.rs` and `crates/nvs-render/src/lib.rs`.**

- [ ] **A `.nvst` case that both writers land in the target the deployment named** — ADR 0092 § 6's
      sameness asked of the destination. `crates/nvs-stdlib/src/log.rs:210` and
      `crates/nvs-runtime/src/floor.rs:173` are the two writers, and the Rust half of this claim is
      already `both_writers_land_in_the_target_the_deployment_named` in that module. A case *can*
      configure one: `crates/nvs-test/src/lib.rs:31` lists `--INI--`, and a `--FILE nvs.toml--`
      section is a tree resolved out of the case's own directory — write a relative `file:` target
      and read it back with `Core\IO`, granting `fs.read` in the same block.
- [ ] **`[log] level` is the minimum level written** — ADR 0092 § 2's last paragraph, a `<=` over the
      syslog severities and not a `>=`. It belongs at `crates/nvs-runtime/src/ctx.rs:3566`, the one
      place both writers pass through, which means the level crosses with the line;
      `crates/nvs-render/src/lib.rs:150` is `Level::syslog_severity` and
      `crates/nvs-config/src/mode.rs:84` is the per-mode default the directive already has.
- [ ] **`[log] format = "text"` selects ADR 0092 § 3's second rendering** at that same call, so the
      choice is made once for both writers rather than at each `nvs_render::json::line`
      (`crates/nvs-stdlib/src/log.rs:204`, `crates/nvs-runtime/src/floor.rs:177`);
      `crates/nvs-config/src/mode.rs:79` is the directive.

## Backlog

- The syslog transport, and the severity argument a sink needs — `crates/nvs-config/src/log.rs:37`.
- One configured file, many contexts: the rotation bound is counted per handle, which is right for a
  CLI run and undecided for a served host — `crates/nvs-runtime/src/logfile.rs`'s module doc.
- Stage 10's `every_part_two_spec_member_is_registered` waits on spec §§ 15-19 (goal 6).
