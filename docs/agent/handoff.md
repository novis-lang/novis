# Handoff

## State

**ADR 0058 § 3 has both halves.** The denied table is unchanged and the operator's exception beside
it is `net.internal` — `nvs_config::tree::CapNet::internal` for the key and
`Capabilities::address_refused` for the rule, which is the table less whatever this deployment
excepted. `nvs_runtime::capability::pin_host` is the only caller, and a context with no snapshot
still gets the bare table. The three refusals to widen — literal addresses only, no ranges, and
`true` grants nothing — are argued in that method's doc and in ADR 0058 § 3, which is their home.
An exception reaches nothing on its own: `net.connect` still has to grant the host, asked first.

A denial's message now ends `which `net.internal` does not except`, so the two conformance cases
that quote it moved with it.

**Root `nvs.toml` carries `examples/http.nvs`'s block**: `127.0.0.1` and `169.254.169.254` granted
by name, only the first excepted — so the program's third line is refused on its *address* while its
first is reachable, which is what that fixture claims.

**`examples/http.nvs` is still red, and none of the three reasons is the address policy.** Checked
again this session: `Core\Env` exists nowhere; `$configured ?? "…"` types as
`string|tainted string` and `Qual::Launder` refuses the union; nothing in this tree serves
`127.0.0.1:8099`.

## Next group

**The three halves `examples/http.nvs` still needs.** They share that file and the stage 5 `exact`
check it feeds — `docs/agent/loop-goal.toml:2357` — and each is a different owner's, so the file
sets differ; take them in this order.

- [ ] **`Core\Env::get` and `::all`.** Placement is already decided — do not re-open it:
      `docs/adr/0012-no-superglobals.md:85` gives the class and `docs/spec/02-php-migration.md:589`
      gives both members, answering `tainted` values, so this is the five-edit `Core` member shape
      in a new `crates/nvs-stdlib/src/env.rs`, registered beside `crate::secret::CLASS` at
      `crates/nvs-stdlib/src/registry.rs:1167`. A new class owes three conformance cases —
      `crates/nvs-stdlib/tests/conformance_coverage.rs:24` — and the spec roster in
      `docs/spec/01-core-library.md` gains it.
- [ ] **Whether a `Qual::Launder` parameter admits a union carrying the tainted arm.**
      `crates/nvs-types/src/expr/quals.rs:229` is the accept set that refuses it today, and
      `examples/http.nvs:45` is the caller: `?tainted string` fed through `??` is the ordinary
      shape of a configured URL, so the answer decides whether ADR 0024 § 3's launderers are
      reachable from one.
- [ ] **An origin on `127.0.0.1:8099` for the acceptance check** — the driver's half. The goal
      file's `[docker]` block is read at `tools/loop.py:2243` (a compose file and its services,
      brought up once per run), and the check at `docs/agent/loop-goal.toml:2357` wants
      `status=200` and `body=ok` from `/ok`.

## Backlog

- A request body, and `Core\Http\Response`'s header map — `crate::http`'s "what is not here yet".
- `https` stays refused until a trust anchor set has an owner — `crate::http::transport`'s own doc.
- `docs/reference/tools/20-config.md`'s "nothing asks for them yet" line still names `db.*` and
  `debug.*`; it moves when their members land.
- Stage 6, the two stores — `docs/agent/loop-goal.toml` stage 6.
- `orient.py` did not print `nvs-config/src/tree.rs`, whose `CapNet` this item edits: the
  `[context] modules` manifest selects only `nvs-config/src/capability.rs`.
