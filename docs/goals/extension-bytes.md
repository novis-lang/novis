---
looper: 1
title: A third-party extension moves bytes in one copy
summary: "A bytes argument or result of a third-party extension crosses as the host-held bytes resource, the bench shows a 16 MiB echo copies each byte once each way, the Rust and C templates and the guide use it, and the checks below pass."
context: [docs/rules/packaging/a-value-crosses-as-its-wit-type.md, docs/rules/packaging/the-boundary-is-the-cost.md, docs/rules/packaging/values-cross-as-handles.md, crates/nvs-ext/src/convert.rs]
stages:
  - id: 1
    title: "the dynamic path's cost is measured"
    status: done
    context: [crates/nvs-ext/benches/image_guest.rs, crates/nvs-ext/Cargo.toml, crates/nvs-ext/src/call.rs, tests/conformance/ext/fixtures/ledger/manifest.json]
  - id: 2
    title: "a third-party export's bytes cross as the host's bytes resource"
    status: done
    context: [wit/nvs-ext/types.wit, crates/nvs-ext/src/handle.rs, crates/nvs-ext/src/load.rs, crates/nvs-ext/src/types.rs, crates/nvs-ext/src/call.rs]
    extra:
      context:
        rules: [packaging/a-value-crosses-as-its-wit-type, packaging/values-cross-as-handles, packaging/a-guest-runs-under-the-requests-budget]
  - id: 3
    title: "the ledger fixture and the conformance cases cross bytes as the resource"
    status: open
    context: [crates/nvs-ext/tests/fixtures.rs, tests/conformance/ext/fixtures/ledger/api.wit, tests/conformance/ext/fixtures/ledger/module.wat, tests/conformance/ext/fixtures/ledger/manifest.json, tests/conformance/ext/an-extension-call-crosses-every-row-of-the-value-table.nvst]
  - id: 4
    title: "the templates, the guide and the figures use the resource"
    status: open
    context: [crates/nvs-cli/src/ext/new.rs, crates/nvs-cli/templates/ext/rust/src/lib.rs, crates/nvs-cli/templates/ext/c/greeting.c, website/src/content/docs/guides/extensions/rust.mdx, website/src/content/docs/guides/extensions/index.mdx]
    extra:
      context:
        rules: [packaging/the-boundary-is-the-cost, packaging/nvs-ext-is-the-authoring-tool]
  - id: 5
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: bytes-bench-recorded
    stage: 1
    name: the bytes crossing bench has written its results
    file: benches/results/ext-bytes.json
    min_bytes: 100
  - id: bytes-resource-tests
    stage: 2
    name: a third-party export's bytes cross as the host's resource
    run: [cargo, test, --test, bytes]
    tests:
      - a_bytes_argument_crosses_as_a_borrowed_resource
      - a_bytes_result_is_built_in_parts
      - a_bytes_inside_a_list_or_shape_crosses_as_a_resource
      - a_core_record_keeps_its_byte_list
      - a_result_past_the_budget_stops_at_the_memory_limit
      - a_16_mib_echo_copies_each_byte_once_each_way
      - a_list_of_u8_in_a_third_party_signature_does_not_load
  - id: owners-closes
    stage: 4
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, extension-bytes]
    reach: true
    floor: false
---

# A third-party extension moves bytes in one copy

## Why

A third-party extension's exports are known only at run time, so the host calls them through
wasmtime's dynamic `Val`. wasmtime 49 has no byte-list `Val`: a `bytes` argument becomes one
`Val::U8` per byte, and a result comes back the same way (`crates/nvs-ext/src/convert.rs:196`,
`crates/nvs-ext/src/convert.rs:349`). A `Val` is about 32 bytes, so a 16 MiB image or document builds
about 512 MiB of `Val`s each way. `rule:packaging/the-boundary-is-the-cost` already says this costs
more than the compute. It defeats the coarse API that rule asks authors to write, and it does so on
exactly the work extensions exist for: image, video, document and archive processing.

The built-in worlds do not have the problem. They are called through generated bindings
(`crates/nvs-ext/src/typed.rs:368`), so their `list<u8>` crosses in one copy. The input side has a
fast path already, too: a `mixed` argument crosses as the `value` handle, whose `as-bytes` and
`range` are typed host functions (`crates/nvs-ext/src/handle.rs:194`). A third-party author who wants
speed today must type a byte parameter `mixed`, and has no fast way to return bytes at all.

File access is not the gap. A guest with a `read` or `write` grant reads and writes files through
`wasi:filesystem` (`rule:security/extension-grants-are-an-intersection`), 64 KiB at a time.

## Standing decisions

**A `bytes` in a third-party export's signature crosses as a host-held `bytes` resource.** It is
declared once in `nvs:ext/types` (`wit/nvs-ext/types.wit:12`):

```wit
/// A byte string the host holds. Every method copies in one call.
resource bytes {
    /// An empty byte string, which the guest fills and returns.
    constructor();
    size: func() -> u64;
    /// The whole byte string, refused past the request's memory.
    read: func() -> list<u8>;
    /// At most `length` bytes from `offset`, at most 1 MiB per call.
    range: func(offset: u64, length: u64) -> list<u8>;
    /// Adds bytes to the end, charged to the request as it grows.
    append: func(data: list<u8>);
}
```

A parameter is a `borrow<bytes>`, and a result is an `own<bytes>`. This holds at every position the
manifest names: a parameter, the `ok` of a result, an element of a list, a field of a shape, an
`option` and a `variant` case. Every method is a typed host function (`func_wrap`), so each crosses in
one copy. Why: wasmtime's public API cannot lower or lift a byte list without one `Val` per byte, so
the bytes must not cross as a value on the dynamic path at all. A resource costs a typed host call of
about 9 ns, against one `Val` per byte. If WIT refuses the name `bytes`, it is written `%bytes`.

**The host lends the program's own copy.** A `bytes` argument is lent by number as `Value::Lent`
already is (`crates/nvs-ext/src/handle.rs:172`), and nothing is copied until the guest reads. `read`
and `range` ask `Budget::left` first, as `as-bytes` does
(`rule:packaging/a-guest-runs-under-the-requests-budget`). A borrowed handle ends with its call
(`crates/nvs-ext/src/handle.rs:83`).

**A result is built in host memory.** The guest makes an empty `bytes`, calls `append` with each
part, and returns it. Each `append` is charged to the request before it copies, and a part past the
budget ends the call at the memory limit. The host takes the returned handle's buffer as the
program's `bytes` value without copying it again. Why: a guest that writes a large output, such as a
transcoded video, never holds the whole output in its own linear memory.

**A built-in world keeps `list<u8>`.** Its exports are called typed, so its byte lists already cross
in one copy. The table in `rule:packaging/a-value-crosses-as-its-wit-type` gets a second WIT column:
the built-in worlds, then a third-party export. The loader holds a built-in's `bytes` to `list<u8>`
and a third-party's to the resource (`crates/nvs-ext/src/load.rs:951`). A third-party manifest whose
export has `list<u8>` where it names `bytes` does not load, and the error names the resource.

**A `Core` value class record keeps its byte list.** `big-int` and the records with an `spki` field
are defined once in `nvs:ext/types` and shared by both kinds of extension
(`crates/nvs-ext/src/types.rs:91`, `crates/nvs-ext/src/types.rs:107`). Their byte fields are small by
construction, so they stay `list<u8>`, and the rule says why.

**`mixed` is unchanged.** It stays the one way to pass a value whose type is not known. A `bytes`
value passed as `mixed` still reads through `value`'s `as-bytes` and `range`.

**The bench measures work and time, before and after.** A new `crates/nvs-ext/benches/ext_bytes.rs`
calls the ledger fixture's `echoBytes` at 1 KiB, 1 MiB and 16 MiB through
`nvs_ext::call::Request::call_values` (`crates/nvs-ext/src/call.rs:1038`), so the same bench runs on
both sides of stage 2. It records the time per call and the host bytes allocated per call into
`benches/results/ext-bytes.json`, beside the image figures. Stage 1 writes the before row, and
stage 4 the after row. Stage 1 also records whether the `Val` vectors are charged to the request's
budget. If they are not, the handoff says so, because that is host memory a request can grow past
its cap, and stage 2 removes it.

**Not done:** a patched wasmtime with a byte-list `Val`. wasmtime is the sandbox. A fork of it would
be carried through every dependency update, against the priority ordering's first item. The rule's
Declined list records this, together with a closed set of typed call shapes for third-party exports
(they multiply with each parameter and cover only some signatures), and with changing the built-in
worlds to the resource (they are already one copy).

**The rules this goal rewrites.** `rule:packaging/a-value-crosses-as-its-wit-type` gets the two
columns and the Declined entries above. `rule:packaging/values-cross-as-handles` stops saying `mixed`
is the one value that crosses as a handle. `rule:packaging/the-boundary-is-the-cost` replaces "costs
more than the compute" with stage 4's figures. Each fragment says what the change spends: one handle
table entry per `bytes` in flight, and the result buffer, charged to its request.

**Lanes.** Stages 1 to 3 run in order: each needs the one before. Stage 4's templates and guide do not
touch `nvs-ext`, so its writing subagents may start once stage 2's WIT is committed. Its bench row
needs stage 3. The goal depends on no other chain goal.

**Trade-offs.** Security: unchanged, and stronger if stage 1 finds the `Val` vectors uncharged.
Every read and every append is charged to the request. Latency: a large `bytes` crossing goes from
one allocation per byte to one copy. A small one, such as a 32-byte hash, pays two or three typed host
calls, about 30 ns. Memory: one handle per `bytes` in flight, and a result is built once in host
memory instead of twice. Simplicity: an author calls `read()` on a parameter and `append()` on a
result in place of a plain vector. The templates hide that behind one helper each.

## Stages

### 1. the dynamic path's cost is measured

**Does:** a bench records what a third-party `bytes` crossing costs today, in time and in host bytes.

- [x] The bench, calling `echoBytes` at three sizes through `call_values`, and its Cargo entry. `crates/nvs-ext/benches/ext_bytes.rs:1`, `crates/nvs-ext/Cargo.toml:98`
- [x] The before row in `benches/results/ext-bytes.json`, with time and host bytes per call. `benches/results/ext-bytes.json:1`
- [x] Whether the `Val` vectors are charged to the request's budget, read from the code and recorded in the handoff. `crates/nvs-ext/src/convert.rs:196`, `crates/nvs-ext/src/call.rs:1376`

### 2. a third-party export's bytes cross as the host's bytes resource

**Does:** the `bytes` resource exists, the host implements it, and the dynamic path lends and takes it.

- [x] The resource in `nvs:ext/types`, as § *Standing decisions* gives it. `wit/nvs-ext/types.wit:12`
- [x] The host's five methods as typed functions, each asking the budget first. `crates/nvs-ext/src/handle.rs:194`
- [x] `to_wit_with` lends a `bytes` at every position, and `from_wit_with` takes an owned one without a copy. `crates/nvs-ext/src/convert.rs:196`, `crates/nvs-ext/src/convert.rs:349`
- [x] The loader holds a third-party `bytes` to the resource and a built-in's to `list<u8>`, and the refusal names the resource. `crates/nvs-ext/src/load.rs:951`
- [x] The convert module doc says what crosses where, and drops the one-`Val`-per-byte paragraph. `crates/nvs-ext/src/convert.rs:53`
- [x] The three rule fragments rewritten, as § *Standing decisions* says. `docs/rules/packaging/a-value-crosses-as-its-wit-type.md:1`, `docs/rules/packaging/values-cross-as-handles.md:1`
- [x] The tests the `bytes-resource-tests` check names. `crates/nvs-ext/tests/bytes.rs:1`, `crates/nvs-ext/Cargo.toml:103`

### 3. the ledger fixture and the conformance cases cross bytes as the resource

**Does:** the ledger fixture's byte exports use the resource, and the row-table case passes.

- [ ] `echoBytes` and every other `bytes` export of the fixture take and return the resource. `tests/conformance/ext/fixtures/ledger/api.wit:1`, `tests/conformance/ext/fixtures/ledger/module.wat:1`
- [ ] The fixture packed again with `NVS_WRITE_FIXTURES=1 cargo test --test fixtures`, and the diff read. `tests/conformance/ext/fixtures/ledger.nvsx:1`, `crates/nvs-ext/tests/fixtures.rs:17`
- [ ] The row-table case passes unchanged, run once by name. `tests/conformance/ext/an-extension-call-crosses-every-row-of-the-value-table.nvst:1`
- [ ] A refused fixture: a third-party export with `list<u8>` for `bytes` does not load. `tests/conformance/ext/fixtures/ledger/manifest.json:17`, `crates/nvs-ext/tests/fixtures.rs:29`

### 4. the templates, the guide and the figures use the resource

**Does:** an author meets the resource in the templates and the guide, and the rule carries
measured figures.

**Split:** the Rust template, the C template and the two guide pages are disjoint files, so three
writing subagents may write them side by side once stage 2's WIT is committed. None of them builds.
The session writes the bench row and the rule's figures itself.

- [ ] The Rust template: a helper that reads a `bytes` parameter into a `Vec<u8>`, and one that returns a `Vec<u8>` in 1 MiB parts. `crates/nvs-cli/templates/ext/rust/src/lib.rs:1`
- [ ] The C template: the same two helpers. `crates/nvs-cli/templates/ext/c/greeting.c:1`
- [ ] `nvs ext new` still writes a project that builds and passes its own test, in both languages. `crates/nvs-cli/src/ext/new.rs:1`
- [ ] The Rust guide's type table and Step 4 use the resource, and *Files and the network* says a large file on disk is passed as a path with a `read` grant. `website/src/content/docs/guides/extensions/rust.mdx:86`, `website/src/content/docs/guides/extensions/rust.mdx:250`
- [ ] *What a call costs* says a byte string crosses in one copy. `website/src/content/docs/guides/extensions/index.mdx:34`
- [ ] The after row in the bench results, and the figures in the cost rule. `benches/results/ext-bytes.json:1`, `docs/rules/packaging/the-boundary-is-the-cost.md:9`
- [ ] `bun nv proofs --id` for the extension feature owes nothing this goal changed, and `bun nv owners --closes extension-bytes` passes. `tools/nv/cmd/proofs.ts:1`

### 5. the goal is verified once, and every failure is fixed

**Does:** the stages before this one checked with a build alone, so this stage verifies the
whole goal once and fixes every failure in one pass.

**Split:** start `bun nv affected --run` with `run_in_background` and end the turn to wait. Group
the failures by cause. A group whose fix touches files no other group touches may go to a writing
subagent. Such subagents run side by side, and none of them builds.

- [ ] `bun nv affected --run` has run once over the whole goal, and its failures are grouped by cause. `tools/nv/cmd/verify.ts:1`
- [ ] Every failure is fixed in one pass, and no test is deleted, renamed or loosened to pass. `tools/nv/cmd/verify.ts:1`
- [ ] The second `bun nv affected --run` is green. `tools/nv/cmd/verify.ts:1`

## Handoff

### State

Stages 1 and 2 are done (`216cacdfd`). `crates/nvs-ext/tests/bytes.rs`'s seven tests pass, run
once by name, and build and clippy are green.

**How it is built.** `nvs_ext::handle` has a second table, `Handles::blobs`, with its own reps, and
`link_bytes` defines the resource. `lend_bytes` moves a `Value::Bytes` into it, or holds a
`Value::Lent`, and `take_bytes` moves a returned buffer out. `append` takes a `WasmList<u8>` and
copies from guest memory once. Its charge goes through `crate::call::grow`, and `Live::end_call`
gives it back after the result has converted. `Handles::end_call` clears every `bytes` handle, so
one a guest keeps past its call traps. `Crossing` has two default methods, `lend_bytes` and
`take_bytes`, that give the `list<u8>`. `Call::lends_bytes` is `manifest.builtin.is_none()` on
the dynamic path, so a built-in's method with no typed binding keeps the list. A `Core` record's
byte field always goes through `convert::bytes_to_list`. The loader reads the imported resource
from `nvs:ext/types` (`Loader::imported_bytes`), and `BytesAs` in `Side` says which form each
position takes. `nvs run` lends a third-party `bytes` argument by number
(`crates/nvs-cli/src/extensions.rs:840`).

**The 16 MiB test uses the budget, not an allocator.** It runs the echo under a 40 MiB limit, which
one `Val` per byte (over 512 MiB) cannot fit. Stage 4's bench row gives the heap figures.

**`the-boundary-is-the-cost` has the before figures only.** Stage 4 adds the after row's.

The sweep's reds this session were the same stale ones: `env-secret-proofs`'s log exits 0, and
nightly 40847802941 ran on 8d39c49bb, before `ef7e629e2`, `df208a541` and `dafe6c98d` fixed its
three causes.

### Next

**Stage 3, one group**, on the fixtures and the test guests that still declare `list<u8>`. No gate.

- [ ] The ledger fixture's `bytes` exports take and return the resource. `tests/conformance/ext/fixtures/ledger/api.wit:1`, `tests/conformance/ext/fixtures/ledger/module.wat:1`
- [ ] Pack it again with `NVS_WRITE_FIXTURES=1 cargo test --test fixtures`, and replace each old pin across the tree. `crates/nvs-ext/tests/fixtures.rs:17`
- [ ] `crates/nvs-ext/tests/convert.rs`'s `echo-bytes` guest takes the resource. Its `echo` calls `Request::call` with `to_wit`, which has no call to lend through, so the bytes row goes through `call_values`. `crates/nvs-ext/tests/convert.rs:108`, `crates/nvs-ext/tests/convert.rs:240`
- [ ] `crates/nvs-ext/tests/call.rs`'s guest declares `size(data: list<u8>)` under a `bytes` manifest, so it no longer loads. Its `size` takes a `borrow<bytes>`. `crates/nvs-ext/tests/call.rs:69`, `crates/nvs-ext/tests/call.rs:89`
- [ ] The row-table case passes, run once by name. `tests/conformance/ext/an-extension-call-crosses-every-row-of-the-value-table.nvst:1`
- [ ] The refused fixture item is covered by `bytes.rs`'s `a_list_of_u8_in_a_third_party_signature_does_not_load`, so stage 3 ticks it after it checks that the fixture side needs nothing more. `crates/nvs-ext/tests/bytes.rs:369`

Expected red until stage 3 lands: every test of `crates/nvs-ext/tests/convert.rs` and
`crates/nvs-ext/tests/call.rs` whose fixture no longer loads, `crates/nvs-ext/tests/fixtures.rs`,
the row-table conformance case, and the `ext_bytes` bench, which loads the ledger fixture.

### Backlog
