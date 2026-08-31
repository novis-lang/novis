# Handoff

## State

**`Core\Env` is on disk**, in `crates/nvs-stdlib/src/env.rs`: `get(string $name): ?tainted string`
and `all(): array<string, tainted string>`, registered beside `crate::config::CLASS`. Two decisions
that module's own doc is the home of — it is **not** a capability door (ADR 0012 § 7 makes the
environment a process-wide fact the operator already chose, and `nvs_runtime::environment` is the
door-that-asks-nothing beside `terminal`), and the **name is a `Qual::Sink`**, so a `tainted` name
is `E0401` on `Core\IO`'s reasoning about a path. Every value is `tainted` unconditionally. A value
that is not UTF-8 makes `get` throw and `all` omit; the throw is declared unreachable from source at
its site, because no program can write the environment.

**`--ENV--` is honoured.** `crates/nvs-test` no longer refuses a case that uses it: pairs are one
`NAME=value` per line, split at the first `=`, added to the runner's own environment rather than
replacing it, and given to a `--ORACLE--`'s PHP half too. `--INI--` is the only section left on
`case.rs`'s `NOT_YET`.

**`examples/http.nvs` has two blockers left, both below.** `Core\Env` is no longer one of them:
the file now stops at line 45's `string|tainted string`, and nothing serves `127.0.0.1:8099`.

The spec needed no edit — `docs/spec/01-core-library.md` § 15 already carried `Core\Env`'s row,
including `mode()` and the `EOL`/`OS`/`VERSION` constants this slice did not write.

## Next group

**The two halves `examples/http.nvs` still needs**, in this order. They share that file and the
stage 5 `exact` check it feeds — `docs/agent/loop-goal.toml:2357` — and nothing else, so the file
sets differ.

- [ ] **Whether a `Qual::Launder` parameter admits a union carrying the tainted arm.**
      `examples/http.nvs:45` is the whole case and is `E0401` today, now that `Core\Env::get` gives
      it a real `?tainted string` to write `?? "…"` over — which is how every environment read
      will arrive, so this is the shape and not a corner. The rule is
      `crates/nvs-types/src/expr/quals.rs:222`'s `admits_tainted_argument`; ADR 0024 § 3 is where a
      launderer's argument shape is recorded, and whichever way it goes the answer belongs there.
      A refusal that stands owes the example a spelling that is not `??`.
- [ ] **An origin on `127.0.0.1:8099` for the acceptance check** — the driver's half.
      `docs/agent/loop-goal.toml:2357`'s stage 5 check runs `examples/http.nvs` with nothing
      listening, so its first two lines cannot print what the fixture freezes.
      `crates/nvs-stdlib/src/http/transport.rs`'s own `#[cfg(test)]` listener is the shape a
      harness would reuse; root `nvs.toml` already grants `127.0.0.1` by name and excepts it under
      `net.internal`.

## Backlog
- `Core\Env::mode()` and the `EOL`, `OS` and `VERSION` constants —
  `crates/nvs-stdlib/src/env.rs`'s gap 1 owns them, and `EOL` carries a real decision: a
  `CoreConst` is inlined by the machine that compiles, `PHP_EOL` by the machine that runs.
- `Core\Http\Response::header()` and the header-map slot — `crates/nvs-stdlib/src/http.rs:452`.
- A reader for a non-text body needs a `CoreTy::TaintedBytes`; `TaintedStr` is the only tainted
  return spelling `crates/nvs-stdlib/src/registry.rs` has.
- `Core\Http\Client::send(Request)` and `stream` — `docs/spec/01-core-library.md` § 16 names both.
- ADR 0074 § 7's dynamic half — a verb chosen at run time throws before the first attempt.
- `--INI--` is the last section `crates/nvs-test/src/case.rs`'s `NOT_YET` refuses; `nvs.toml` has
  been read since M6, so the entry may be older than the tree.
