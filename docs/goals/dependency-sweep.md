---
looper: 1
title: "every dependency is at its latest release, or held with a reason that is still true"
summary: "bun nv deps --check exits 0: every pin is current or held with a dated, true reason, and the tree builds and verifies on the new versions."
context: [docs/agent/dependency-update.md, docs/rules/packaging/the-sweep-is-fired-by-a-human.md, tools/nv/cmd/deps.ts]
stages:
  - id: 1
    title: "the JIT and the sandbox run on cranelift 0.137 and wasmtime 50"
    status: done
    context: [Cargo.toml, crates/nvs-ext/src/call.rs, crates/nvs-codegen/src/lib.rs, docs/rules/packaging/an-extension-is-a-sandboxed-wasm-component.md, docs/rules/packaging/a-dependency-move-proves-two-things.md]
  - id: 2
    title: "the host's leaf crates are current, and every tool lockfile is at its latest in range"
    status: done
    context: [Cargo.toml, crates/nvs-stdlib/src/regex.rs, crates/nvs-db/src/sqlite.rs, deny.toml]
  - id: 3
    title: "the four built-in components are current, and their advisories are clear"
    status: done
    context: [extensions/spreadsheet/Cargo.toml, extensions/image/Cargo.toml, extensions/intl/Cargo.toml, extensions/pdf/Cargo.toml, crates/nvs-ext/build.rs]
  - id: 4
    title: "Bun, the npm packages and the editor client are current"
    status: done
    context: [package.json, editors/vscode/package.json, editors/vscode/src/extension.ts, viewer/package.json, tools/nv/test/deps.test.ts]
  - id: 5
    title: "CI actions and container images are current, and CI judges every lockfile's advisories"
    status: done
    context: [.github/workflows/ci.yml, tests/db/compose.yaml, tools/nv/cmd/ci-changes.ts, tools/nv/cmd/ci.ts, docs/setup.md]
  - id: 6
    title: "the goal is verified once, and every failure is fixed"
    status: done
    gate: true
checks:
  - id: engine-guards
    stage: 1
    name: the premise guards and the wasm boundary guards hold on the new engine
    run: [cargo, test, --release, -p, nvs-abi-probe, --features, wasm-probe]
    tests: [a_jit_frame_has_no_unwind_entry]
    timeout: 1800
  - id: components-load
    stage: 1
    name: every built-in component loads and runs under the engine's proposal set
    run: [bun, nv, verify, -p, nvs-ext]
    timeout: 1800
  - id: deny-root
    stage: 2
    name: "advisories, licences, bans and sources pass on the root lock"
    run: [cargo, deny, check]
  - id: c-deps
    stage: 2
    name: every C dependency of the binary has its ledger entry
    run: [bun, nv, gen-attribution, --check-c-deps]
  - id: fuzz-advisories
    stage: 2
    name: the fuzz lockfile has no advisory and no yanked crate
    run: [cargo, deny, --manifest-path, fuzz/Cargo.toml, --config, deny.toml, check, advisories]
  - id: guide-advisories
    stage: 2
    name: the markdown extension guide's lockfile has no advisory and no yanked crate
    run: [cargo, deny, --manifest-path, website/guides/markdown-extension/Cargo.toml, --config, deny.toml, check, advisories]
  - id: spreadsheet-advisories
    stage: 3
    name: the spreadsheet component's lockfile has no advisory and no yanked crate
    run: [cargo, deny, --manifest-path, extensions/spreadsheet/Cargo.toml, --config, deny.toml, check, advisories]
  - id: pdf-advisories
    stage: 3
    name: the pdf component's lockfile has no advisory deny.toml does not record
    run: [cargo, deny, --manifest-path, extensions/pdf/Cargo.toml, --config, deny.toml, check, advisories]
  - id: image-advisories
    stage: 3
    name: the image component's lockfile has no advisory deny.toml does not record
    run: [cargo, deny, --manifest-path, extensions/image/Cargo.toml, --config, deny.toml, check, advisories]
  - id: intl-advisories
    stage: 3
    name: the intl component's lockfile has no advisory and no yanked crate
    run: [cargo, deny, --manifest-path, extensions/intl/Cargo.toml, --config, deny.toml, check, advisories]
  - id: attribution
    stage: 3
    name: THIRD-PARTY-LICENSES.txt is what the lockfiles generate
    run: [bun, nv, gen-attribution, --check]
  - id: deps-unit
    stage: 4
    name: bun nv deps reads no pin under vendor/ or the synced looper kit
    run: [bun, test, tools/nv/test/deps.test.ts]
    tests: ["a file under vendor/ or the synced looper kit gives no pin"]
  - id: editor-headless
    stage: 4
    name: the editor extension compiles on vscode-languageclient 11 and its headless suite passes
    run: [npm, run, "test:headless"]
    cwd: editors/vscode
    timeout: 900
  - id: editor-lint
    stage: 4
    name: the editor extension lints clean
    run: [npm, run, lint]
    cwd: editors/vscode
  - id: deps-check
    stage: 5
    name: every pin is current or held with a dated reason, and none is expired
    run: [bun, nv, deps, --check]
    reach: true
    floor: false
  - id: owners-closes
    stage: 5
    name: no gap record or designed rule names this goal
    run: [bun, nv, owners, --closes, dependency-sweep]
    reach: true
    floor: false
---

# every dependency is at its latest release, or held with a reason that is still true

## Why

On 2026-10-30 `bun nv deps` read 335 pins: 51 behind, 15 held and 4 floating. The engine is one
major behind, and the hold that keeps it there says wasmtime 50 is only a release candidate, which
is no longer true. Four lockfiles fail `cargo deny check advisories`, and CI judges none of them.
The procedure is `docs/agent/dependency-update.md`. This goal is its work list for this tree, with
each pin's anchor, what moves with it, and what each hold's reason is today.

## Standing decisions

**The user fired this sweep.** `rule:packaging/the-sweep-is-fired-by-a-human` and
`docs/agent/loop-authoring.md` § 8 keep a sweep out of the loop unless a person starts it. The user
started this one on 2026-10-30, in the pre-release review. That is the reason the goal exists. No
session widens it into a second sweep, and nothing in it schedules the next one.

**Prototyping regime.** The workspace version is `0.0.1` (`Cargo.toml:37`), so
`rule:packaging/the-version-contract-starts-at-0-1-0`'s first regime applies, and
`docs/agent/dependency-update.md` § *Before you start* says to do its steps 1, 2, 4 and 7. There is
no classification, no ladder and no deprecation cycle. A change a program author can see goes in as
it is, and its commit says so. Of the § 3 battery, the goal keeps two legs, because a rule makes
them binding in both regimes (`rule:packaging/a-dependency-move-proves-two-things`): leg B, the
premise guards, as the `engine-guards` check, and the regenerated `THIRD-PARTY-LICENSES.txt` in the
same commit as every graph change. Leg C is CI's at the goal-end push. Leg D has nothing to run: no
lexer or parser crate moves. Legs E and F are skipped: callgrind is a contract-regime leg, and the
PHP oracle does not change (the `php` images in `benches/` are bench peers, not the oracle).

**Latest stable means a final release** at the time the stage is worked. The versions below are
the ones `bun nv deps` named on 2026-10-30, and they do not cap a bump. A release that appears while
the goal runs is moved or held in the stage of its kind, or in the gate stage if that stage has
passed.

**One bump, one commit** (`rule:packaging/one-bump-one-commit`), with these groups as one commit
each, because their crates pin each other:
- the engine: `cranelift`, `cranelift-jit`, `cranelift-module`, `cranelift-native`,
  `cranelift-object`, `wasmtime`, `wasmtime-wasi-io`, `wasmtime-internal-jit-icache-coherence`,
  `wit-parser`, `wasmparser` and `wit-component`. wasmtime N pins cranelift 0.(N+87) exactly, and
  cranelift-jit 0.137.1 pins `wasmtime-jit-icache-coherence` `=50.0.1`.
- `icu_experimental` with the `icu_*` floors of the intl component.
- the in-range `cargo update` of one lockfile.
- one action's SHA across every workflow that names it, and one image across every file that pulls it.

A commit that changes a lockfile also carries the regenerated `THIRD-PARTY-LICENSES.txt`
(`bun nv gen-attribution`), whenever the notice moves. The message is `build(deps): <what is now
true>`, or `chore(deps):` for a hold, an ignore or a record.

**A stage checks with a build.** Each bump's narrow check is `cargo build` and `cargo clippy
--all-targets -- -D warnings`, not `bun nv verify -p` as the procedure's step 2 says. AGENTS.md's
rule that a goal is verified once, in its gate stage, wins. An item that names a run is the
exception, and it runs that run once.

**The engine follows wasmtime, and `wit-*` follows the engine.** `wit-parser`, `wasmparser` and
`wit-component` sit at the version wasmtime parses with, so they add no second copy to the host's
graph (`Cargo.toml:383`). wasmtime 50.0.1 needs `^0.261.0` of all three. So they move to `0.261`,
each held with `# HOLD (revisit 2026-11-30): 0.262 is out, and wasmtime 50.0.1 parses with 0.261.
It moves with wasmtime.` Safe fallback: if a wasmtime 50.0.x that needs 0.262 is out when stage 1 is
worked, take it and drop the three holds.

**`region` stays on 3.** It is a dev-dependency that adds no crate only because `cranelift-jit`
already depends on it (`Cargo.toml:545`). cranelift-jit 0.137.1 needs `^3.0.2`, so `region` 4 would
build a second copy. The row gets `# HOLD (revisit 2026-11-30): 4 is out, and cranelift-jit 0.137.1,
the latest, needs ^3.0.2, so 4 would build a second copy.`

**`fancy-regex` stays on 0.19.** The dev-dependency `jsonschema` 0.60.2, the latest, needs `^0.19`,
so 0.20 would build a second copy, and stage 2's `cargo tree --duplicates` item allows none. 0.20
builds with no source change, so lifting the hold is a one-line move. The row gets `# HOLD (revisit
2026-11-30): 0.20 is out, and jsonschema 0.60.2, the latest, needs ^0.19, so 0.20 would build a second
copy.` Declined: taking 0.20 with a test-only second copy, which costs every test build one more
regex engine for nothing a program sees.

**A failed guard never moves a threshold** (`rule:packaging/a-dependency-move-proves-two-things`). If
`engine-guards` fails on the new engine, the session reads the rule the failing test's comment
names. If the premise really changed, the engine group goes back to wasmtime 49.0.4 and cranelift
0.136.4 in one revert commit. The wasmtime row gets a hold naming the failing guard and the rule, a
gap record under `data/gaps/` names the rule's revisit, and the goal goes on with stage 2. That
records the decision, and nothing blocks.

**Every hold is checked again, not re-dated blindly.** A hold whose reason is still true gets
`revisit 2026-11-30` and a reason that names the latest upstream version of today. A hold whose
reason is false is lifted, and its pin moves. On 2026-10-30 the crates.io index and the npm registry
gave these answers:

| Pin | Anchor | Today |
|---|---|---|
| `wasmtime` | `Cargo.toml:346` | false: 50.0.1 is final. Lift and move. |
| `wit-parser`, `wasmparser`, `wit-component` | `Cargo.toml:388`, `Cargo.toml:395`, `Cargo.toml:405` | true at 0.261, as the decision above says |
| `fallible-iterator` | `Cargo.toml:1101` | true: `postgres-protocol` 0.6.12 needs `^0.2` |
| `base64`, `hayro-interpret` | `extensions/pdf/Cargo.toml:16`, `extensions/pdf/Cargo.toml:19` | true: `krilla` 0.8.2 is still the latest |
| `usvg` (pdf) | `extensions/pdf/Cargo.toml:32` | true: `krilla-svg` 0.8.1 is still the latest, and needs `^0.47` |
| `color_quant`, `moxcms` | `extensions/image/Cargo.toml:18`, `extensions/image/Cargo.toml:32` | true: `image` 0.25.10 is still the latest |
| `harfrust` | `extensions/image/Cargo.toml:22` | true, with a new reason: `usvg` 0.49.0 still needs `^0.12` |
| `mariadb`, `mysql` | `tests/db/compose.yaml:184`, `tests/db/compose.yaml:131` | true: the held lines are each vendor's LTS line |
| `@types/node` | `editors/vscode/package.json:577` | true: `engines.vscode` `^1.97` runs Node 20 |
| `typescript` (editor) | `editors/vscode/package.json:585` | true, with a new reason: `typescript-eslint` 8.72.1 allows `<6.1.0` |

New holds, each for the reason given: `region` (above), the `wit-*` three (above), and `@babel/core`
and `@babel/preset-typescript` in `viewer/package.json`, because `babel-preset-solid` 1.9.16, the
latest, has the peer `@babel/core` `^7.0.0`. Safe fallback for any hold: if the session cannot tell
whether the reason is true, it tries the bump, and a build that fails for the stated reason makes it
true.

**The synced looper kit is not this sweep's.** `tools/looper/package.json:12` pins `typescript`
`^5.9.3`, and 7.0.2 is out. `tools/looper/` is written by `looper sync` and never edited by hand, so
neither a bump nor a `holds` entry can go there. `bun nv deps` skips `tools/looper/` as it skips
`vendor/` (`tools/nv/cmd/deps.ts:506`), and its head comment says why. looper's own sweep moves the
kit's pins, and the next `looper sync` brings them.

**A live advisory is cleared by an update wherever one exists.** On 2026-10-30, with the root
`deny.toml`:
- `extensions/spreadsheet`: RUSTSEC-2026-0367, -0368 and -0369, and a yanked `zip` 8.6.0, all
  through the dev-dependency `calamine` 0.36. `calamine` 0.37 needs `zip` `^9.0`, which fixes all four.
- `fuzz`: RUSTSEC-2026-0381 (`webpki-roots` 1.0.9) and -0382 (`rustls-webpki` 0.103.15), and a yanked
  `inout` 0.2.3. An in-range `cargo update` fixes all three.
- `website/guides/markdown-extension`: RUSTSEC-2026-0414 (`pulldown-cmark` 0.13.4). An in-range
  `cargo update` fixes it.
- `extensions/pdf`: RUSTSEC-2026-0206, `rustybuzz` is unmaintained, through `krilla` 0.8.2 and `usvg`
  0.47. No release fixes it. It is unmaintained, not vulnerable, and it shapes text inside the
  sandboxed component. So `deny.toml`'s `ignore` gets an entry in the form of the `ttf-parser` one
  (`deny.toml:26`): the reason, and "Revisit 2027-04-01".

A vulnerability that applies and has no fixed release is never ignored. The session files
`looper decision ask --built-on <n>` with its recommendation (replace the crate or drop the feature
it serves), builds on the recommendation, and goes on.

**CI judges every lockfile's advisories.** Today CI's `supply-chain` job and `bun nv ci --check`'s
`advisories` leg read the root lock only, which is how four lockfiles went red unseen. Stage 5 adds
the eight others: `extensions/{image,intl,pdf,spreadsheet}`, `fuzz`, `tools/covwrap`,
`tools/nv-scan` and `website/guides/markdown-extension`, each as `cargo deny --manifest-path
<manifest> --config deny.toml check advisories`. Their manifests and lockfiles join the `deps`
change set that gates the job. Licences and bans stay root-only: what the binary ships is
`gen-attribution`'s to judge. The home is `rule:testing/ci-lanes` and
`rule:packaging/a-dependency-move-proves-two-things`, whose fragment names the eight. Cost: one
advisory pass per lockfile, a few seconds each after the database is fetched once.

**`icu_experimental` stays an exact pin**, at `=0.7.0`. It is ICU4X's unstable crate, and its exact
pin keeps a patch release from changing the intl component's output. 0.7 needs the `icu_*` crates at
`~2.4`, so the intl component's `icu_*` floors become `2.4`.

**`vscode-languageclient` 11 is taken, not held.** It needs `engines.vscode` `^1.91`, and the
extension already needs `^1.97`, so no user loses an editor. The `.vsix` is rebuilt by the goal-end
hook (`bun nv vsix --hook`), because `editors/vscode/` changed.

**Bun moves in the tree.** `engines.bun` becomes `1.4.4`, and CI reads it through
`bun-version-file`. The machine already runs Bun 1.4.4 and Node 26.13 (the user updated both on
2026-10-30), so the tree catches up with the machine and nothing waits on the user.

**Recorded measurements keep their versions.** `benches/compare.json`,
`website/src/data/compare.json` and `benches/compare/README.md:139` say which images a recorded run
used. They change with the next recording, never with a sweep.

**An action keeps its major comment.** A SHA moves to the release `bun nv deps` names, and the
`# v2` or `# v4` comment beside it stays.

**Unused workspace rows are removed, not bumped** (`docs/agent/dependency-update.md` § 1). Stage 2
lists every `[workspace.dependencies]` row that no member manifest names.

**Lanes.** Stages 1, 2 and 3 share `Cargo.lock`, `deny.toml` or `THIRD-PARTY-LICENSES.txt`, and run in
order. Stages 4 and 5 touch files no other stage touches, and could run as side goals. The goal runs
them in order, because one sweep moves one tree. The goal depends on no other chain goal.

**The hostcall fuel is set at each call's start, and no longer at each tick.** wasmtime 50's epoch
callback gets a `StoreHookState`, which cannot set the store's hostcall fuel. The tick refresh never
bounded the worst case: a guest can grow its memory by any amount between two ticks. The bound is
still the charge each lifted value makes after its crossing. Cost: one crossing's bytes, held until
that charge. The home is `crates/nvs-ext/src/call.rs`'s module doc. Declined: refreshing the fuel
in every host import, which is about twenty sites in `handle.rs` and `wasi.rs` and still misses the
guest's own growth.

**`embed_component_metadata` passes `canonical_names: false`.** wit-component 0.261 added the
argument. `false` encodes the full interface names, as 0.258 did, so a packed `.nvsx` is the same.

**Trade-offs.** Security: four lockfiles lose their live advisories, and CI judges every lockfile
from now on. Latency: the JIT and the sandbox move one cranelift and wasmtime release, so the
callgrind figures may move. CI's cost guards judge that on the push. Memory: nothing changes per
request. Simplicity: one engine stack with no second copy of `wasmparser`, `region` or `wit-parser`.
Usability: the editor client needs no newer VS Code, and a program author sees whatever the
`rusqlite`, `fancy-regex` and `icu_experimental` releases change, as their commits say.

**The body wait ends with the body.** Once `hyper` has read a request body to its end, the
connection leaves `Phase::Body` for a phase with no socket idle wait. The request is then bounded
by its own `[limits] wall_time` and by a client EOF, which `hyper`'s read probe still sees. Today
the probe re-arms `body_idle_timeout` (30 s) on every pass while the handler runs, so a handler
that reads its body and then works for more than 30 s is cut with no reply at all. The gate run
found it as `registry_build`'s two publish tests under load. It is a product bug the sweep
surfaced, and the gate stage fixes it, because the stage owes every failure it names. Declined:
raising the timeout or the test's bound, which loosens a check and still cuts any slow handler.
Cost: one more phase value per connection, nothing per request. The home is
`crates/nvs-server/src/io.rs`'s module doc and `rule:http-server/four-idle-waits-all-finite`, which
owns the waits. The new phase is `Phase::Run`.

## Stages

### 1. the JIT and the sandbox run on cranelift 0.137 and wasmtime 50

**Does:** the engine group moves as one commit, its stale hold is lifted, and the premise guards
pass on it.

- [x] Lift the wasmtime hold and move `wasmtime` to `50.0.1`. Rewrite the comment above it as a whole: the minimum is the release this commit takes, and the `cache` and `zstd` paragraph is checked again against 50's features. Adapt the feature list to what 50 names (`rule:packaging/an-extension-is-a-sandboxed-wasm-component`). `Cargo.toml:346`, `Cargo.toml:356`
- [x] Move the five `cranelift-*` rows to `0.137.1`. `Cargo.toml:98`, `Cargo.toml:99`, `Cargo.toml:100`, `Cargo.toml:101`, `Cargo.toml:107`
- [x] Move `wasmtime-wasi-io` and `wasmtime-internal-jit-icache-coherence` to `50.0.1`. `Cargo.toml:414`, `Cargo.toml:544`
- [x] Move `wit-parser`, `wasmparser` and `wit-component` to the version wasmtime 50.0.1 parses with, and write their holds as the standing decision says. Check the `sha2` and `generic-array` line again against 50, and rewrite or delete it. `Cargo.toml:388`, `Cargo.toml:395`, `Cargo.toml:405`, `Cargo.toml:406`
- [x] Hold `region` on 3 with the reason the standing decision gives. `Cargo.toml:549`
- [x] Adapt the call sites the moved APIs break. The engine config keeps naming every wasm proposal through `PROPOSALS`, so a new default in 50 stays off. `crates/nvs-ext/src/call.rs:872`, `crates/nvs-ext/src/call.rs:895`, `crates/nvs-ext/src/section.rs:1`, `crates/nvs-ext/src/pack.rs:1`, `crates/nvs-ext/src/wasi.rs:1`, `crates/nvs-codegen/src/lib.rs:513`, `crates/nvs-codegen/src/emit.rs:1`, `crates/nvs-runtime/src/helpers.rs:1`
- [x] `cargo build` and `cargo clippy --all-targets -- -D warnings` are clean. `bun nv gen-attribution` regenerates the notice. One commit holds the group, its lock and the notice (`rule:packaging/one-bump-one-commit`). `Cargo.lock:1`, `THIRD-PARTY-LICENSES.txt:1`
- [x] Run the `engine-guards` check's command once, with `run_in_background`, then the `components-load` check's. A guard that fails is handled as the standing decision says, never by its threshold (`rule:packaging/a-dependency-move-proves-two-things`). `benches/abi-probe/tests/unwind_unavailable.rs:43`, `crates/nvs-ext/tests/load.rs:93`

### 2. the host's leaf crates are current, and every tool lockfile is at its latest in range

**Does:** the rest of the root's behind rows move, every hold in the root manifest is true, and
each Rust lockfile outside `extensions/` takes its in-range updates.

**Split:** an `Explore` subagent lists every `[workspace.dependencies]` row in `Cargo.toml` that no
member manifest names (`<name>.workspace = true` or `workspace = true` under that name), as
`Cargo.toml:NN` and one line each.

- [x] Move `fancy-regex` to `0.20` and adapt `Core\Regex`'s backtracking engine to its API. Held on 0.19 instead, as the standing decision says. `Cargo.toml:613`, `crates/nvs-stdlib/src/regex.rs:922`, `crates/nvs-stdlib/src/regex.rs:1261`
- [x] Move `rusqlite` to `0.41` and adapt its callers. `bun nv gen-attribution --check-c-deps` passes, since `libsqlite3-sys` 0.39 carries a new SQLite amalgamation (`rule:packaging/a-c-dependency-answers-two-questions`). `Cargo.toml:1131`, `crates/nvs-db/src/sqlite.rs:1`, `crates/nvs-db/src/catalog.rs:1`, `crates/nvs-cli/src/debug/index.rs:1`, `crates/nvs-lsp/src/recorded.rs:1`
- [x] Re-date the `fallible-iterator` hold to 2026-11-30, naming `postgres-protocol` 0.6.12. `Cargo.toml:1101`
- [x] Remove each workspace row the `Explore` list names. The list is empty: all 117 rows are named by a member. `Cargo.toml:1`
- [x] `cargo update` on the root lock takes every in-range release, in one commit with the notice. `cargo tree --duplicates` names no second copy the moves above added (`docs/agent/dependency-update.md` § 1). `Cargo.lock:1`
- [x] `cargo update --manifest-path` on `fuzz/`, `tools/covwrap/`, `tools/nv-scan/` and `website/guides/markdown-extension/`, one commit each. The `fuzz-advisories` and `guide-advisories` checks pass. `cargo build --manifest-path` builds `tools/covwrap/` and `tools/nv-scan/`. `fuzz/Cargo.lock:1`, `tools/covwrap/Cargo.lock:1`, `tools/nv-scan/Cargo.lock:1`, `website/guides/markdown-extension/Cargo.lock:1`
- [x] `cargo build`, `cargo clippy --all-targets -- -D warnings` and `cargo deny check` are clean on the tip. `deny.toml:1`

### 3. the four built-in components are current, and their advisories are clear

**Does:** each component crate takes its behind rows and its in-range updates, every hold under
`extensions/` is true, and every component lockfile passes `cargo deny check advisories`.

Each component is its own workspace, built by `crates/nvs-ext/build.rs` with `--locked`, so a
component's lockfile moves with `cargo update --manifest-path extensions/<name>/Cargo.toml`, and the
root `cargo build` rebuilds it (`rule:packaging/the-first-party-components-are-built-in`).

- [x] spreadsheet: move the dev-dependency `calamine` to `0.37`, which clears RUSTSEC-2026-0367, -0368, -0369 and the yanked `zip` 8.6.0. Adapt `tests/excel.rs`. `cargo check --all-targets --manifest-path extensions/spreadsheet/Cargo.toml` is clean. `extensions/spreadsheet/Cargo.toml:30`, `extensions/spreadsheet/tests/excel.rs:1`
- [x] image: move `resvg` to `0.49`. Re-date the `harfrust` hold with its new reason, and the `color_quant` and `moxcms` holds naming `image` 0.25.10. `extensions/image/Cargo.toml:37`, `extensions/image/Cargo.toml:22`, `extensions/image/Cargo.toml:18`, `extensions/image/Cargo.toml:32`
- [x] intl: move `icu_experimental` to `=0.7.0` and the `icu_*` floors to `2.4`, and adapt the component. `extensions/intl/Cargo.toml:17`, `extensions/intl/Cargo.toml:27`, `extensions/intl/src/lib.rs:1`
- [x] pdf: re-date the `base64`, `hayro-interpret` and `usvg` holds, naming `krilla` 0.8.2 and `krilla-svg` 0.8.1. Add the `rustybuzz` entry to `deny.toml`'s `ignore`, as the standing decision says. `extensions/pdf/Cargo.toml:16`, `extensions/pdf/Cargo.toml:19`, `extensions/pdf/Cargo.toml:32`, `deny.toml:22`
- [x] `cargo update --manifest-path` on each of the four lockfiles, one commit per component with its bumps and the notice. `extensions/image/Cargo.lock:1`, `extensions/intl/Cargo.lock:1`, `extensions/pdf/Cargo.lock:1`, `extensions/spreadsheet/Cargo.lock:1`
- [x] The four `*-advisories` checks pass, `bun nv gen-attribution --check` passes, and the root `cargo build` and `cargo clippy --all-targets -- -D warnings` are clean (`rule:testing/attribution-is-diffed-in-ci`). `THIRD-PARTY-LICENSES.txt:1`

### 4. Bun, the npm packages and the editor client are current

**Does:** every `package.json` pin is current or held, the editor runs on `vscode-languageclient`
11, and `bun nv deps` no longer reads the synced looper kit.

- [x] `engines.bun` is `1.4.4`, and `bun install` writes `bun.lock`. `package.json:10`, `bun.lock:1`
- [x] `vscode-languageclient` `^11.0.0`, with `npm install` in `editors/vscode/`, and the client adapted to 11's API (`rule:ide/editor-clients-live-under-editors`). `editors/vscode/package.json:572`, `editors/vscode/src/extension.ts:52`, `editors/vscode/package-lock.json:1`
- [x] Re-date the `@types/node` and `typescript` holds, the second naming `typescript-eslint` 8.72.1. `editors/vscode/package.json:590`
- [x] `viewer/package.json` gets a `holds` object holding `@babel/core` and `@babel/preset-typescript` on 7, naming `babel-preset-solid` 1.9.16's peer range. `viewer/package.json:9`
- [x] `bun nv deps` skips `tools/looper/` as it skips `vendor/`, the head comment says why, and the test the `deps-unit` check names covers both prefixes. `tools/nv/cmd/deps.ts:9`, `tools/nv/cmd/deps.ts:506`, `tools/nv/test/deps.test.ts:1`
- [x] Run the `editor-headless` and `editor-lint` checks' commands once each in `editors/vscode/`. `editors/vscode/package.json:561`

### 5. CI actions and container images are current, and CI judges every lockfile's advisories

**Does:** every action SHA and image tag is current or held, CI and `bun nv ci --check` run
`cargo deny check advisories` over all nine lockfiles, and the versions no manifest pins are checked.

- [x] `Swatinem/rust-cache` to v2.9.3's SHA `d1527c8f…` (the full SHA from `bun nv deps`), keeping `# v2`, at all 19 sites: `.github/workflows/build.yml:137`, `.github/workflows/ci.yml:197`, `.github/workflows/ci.yml:351`, `.github/workflows/ci.yml:449`, `.github/workflows/ci.yml:552`, `.github/workflows/ci.yml:639`, `.github/workflows/ci.yml:687`, `.github/workflows/ci.yml:782`, `.github/workflows/ci.yml:849`, `.github/workflows/ci.yml:901`, `.github/workflows/ci.yml:941`, `.github/workflows/ci.yml:1061`, `.github/workflows/ci.yml:1152`, `.github/workflows/ci.yml:1228`, `.github/workflows/ci.yml:1306`, `.github/workflows/ci.yml:1427`, `.github/workflows/ci.yml:1458`, `.github/workflows/ci.yml:1509`, `.github/workflows/ci.yml:1590`
- [x] `docker/login-action` to v4.7.0's SHA `81c6f93d…`, keeping `# v4`. `.github/workflows/image.yml:141`, `.github/workflows/release-promote.yml:87`
- [x] The bench images: `php:8.5.12-fpm`, `nginx:1.31.8-alpine`, `oven/bun:1.4.4-slim`, `golang:1.27.3-alpine` and `alpine:3.24.3`. `benches/compare/compose.php.yaml:10`, `benches/proxied/compose.php.yaml:16`, `benches/compare/compose.common.yaml:15`, `benches/proxied/compose.nvs.yaml:58`, `benches/proxied/compose.php.yaml:56`, `benches/compare/compose.bun.yaml:10`, `benches/compare/go/Dockerfile:7`, `benches/compare/go/Dockerfile:26`
- [x] The test database images: `mssql/server:2025-CU10-ubuntu-24.04`, `redis:8.10.3`, `axllent/mailpit:v1.32.0` and the `smblds` digest `sha256:b99c19f3…`. Re-date the `mysql` and `mariadb` LTS holds. `tests/db/compose.yaml:265`, `tests/db/compose.yaml:299`, `tests/db/compose.yaml:477`, `tests/db/compose.yaml:396`, `tests/db/compose.yaml:130`, `tests/db/compose.yaml:183`
- [x] CI's `supply-chain` job runs `cargo deny check advisories` over the eight other manifests, as the standing decision says, with the action's inputs read from the pinned release's README. `bun nv ci --check` gets the same eight legs, and the `deps` change set names their manifests and lockfiles (`rule:testing/ci-lanes`). `.github/workflows/ci.yml:986`, `tools/nv/cmd/ci.ts:110`, `tools/nv/cmd/ci-changes.ts:75`
- [x] `rule:packaging/a-dependency-move-proves-two-things` says `cargo deny` reads every lockfile, and names the nine. Then `bun nv rules --render`. `docs/rules/packaging/a-dependency-move-proves-two-things.md:12`
- [x] `docs/setup.md` § *Versions no manifest pins*: the `cargo-fuzz` row names the latest release, and the other rows are checked. `docs/setup.md:51`
- [x] `bun nv deps --check` exits 0, and `bun nv owners --closes dependency-sweep` passes. `tools/nv/cmd/deps.ts:1`

### 6. the goal is verified once, and every failure is fixed

**Does:** the stages before this one checked with a build alone, so this stage verifies the
whole goal once and fixes every failure in one pass.

**Split:** start `bun nv affected --run` with `run_in_background` and end the turn to wait. Group
the failures by cause. A group whose fix touches files no other group touches may go to a writing
subagent. Such subagents run side by side, and none of them builds.

- [x] Run `looper verify` over the whole goal.
- [x] Fix every failure it names in one pass, never deleting or loosening a test.
  - [x] `Core\Cldr`'s plural rules are CLDR 49's, so `plural_rules_agree_with_core_cldr_on_its_whole_roster` passes.
  - [x] The body wait ends with the body, as the standing decision says, with a server test for the reproduced shape.
  - [x] `the_publishing_commands_work_against_the_registry_service` and `the_website_shows_a_published_package_and_search_finds_it_once_released` pass, run once by name.
- [x] Run `looper verify` once more, and it is green.

## Handoff

### State

Every stage is done. The second verify is green (23 of 23 steps), and so are the goal-end gates:
`bun nv verify --doc`, `bun nv owners --closes dependency-sweep` and `bun nv ci --check`.
`bun nv deps --check` exits 0: 319 pins current, 18 held, 4 floating.

Two DONEs were refused on `floor-debt`, each for one red attack, and both are fixed at the cause.
`ed76b4102` makes the Serialize encoder ask `budget::affords` before its buffer grows
(`crates/nvs-runtime/src/graph.rs:661`). That turned the cache-memory attack
(`tests/hostile/config/cache-memory/01-storing-more-than-the-limit-allows.nvs`) red at step 2,
alone too: the walk copied each string before the carrier read it, so a 128 MB put held it twice.
`a926faef0` makes the walk read the bytes in place (`crates/nvs-runtime/src/graph.rs:265`), and the
memory cache encodes with `nvs_runtime::encode_within` up to its shard cap
(`crates/nvs-stdlib/src/cache.rs:2665`). The proofs of `directive:cache.memory` and
`Core\Serialize::encode`, and both unit tests, are green. No test is expected to go red.

The sweep finding `env-secret-proofs` is stale: its log shows exit 0.

### Next

The goal is reached. The driver runs the acceptance checks, `floor-debt` among them, and retires
it. The runtime change makes the debt owe its atoms again, so that check takes about 30 minutes.
`editors/vscode/` changed, so the goal-end hook rebuilds the `.vsix`, and
`bun nv relink --build --hook` rebuilds the release binary.

- [x] Read the second verify's report: green. `tools/nv/cmd/ci.ts:103`
- [x] `bun nv verify --doc`, `bun nv owners --closes dependency-sweep` and `bun nv ci --check`, each red fixed. `tools/nv/cmd/ci.ts:104`
- [x] `floor-debt`'s red atom fixed at its cause. `crates/nvs-runtime/src/graph.rs:661`
- [x] `floor-debt`'s second red atom, the cache-memory attack, fixed at its cause. `crates/nvs-runtime/src/graph.rs:265`

### Backlog

- wasmtime 50's `Engine::release_idle_pool_memory` could free an idle pool's memory. It is not this goal's (`crates/nvs-ext/src/call.rs`).
- `tools/looper/package.json`'s `typescript` 7 goes to looper's kit sweep (`tools/looper/`).
