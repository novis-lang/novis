---
looper: 1
title: A JSON document within maxDepth decodes into objects on every build
summary: "Building objects from a decoded document takes no native frame per level, so maxDepth is the only depth bound and the jsonAs attack prints the same on the proof and covws builds."
context: [crates/nvs-stdlib/src/json.rs, docs/rules/errors/stack-depth.md]
stages:
  - id: 1
    title: the iterative build walk
    status: done
    context: [crates/nvs-host/src/stack.rs, tests/conformance/core/a-json-chain-hydrated-near-the-stack-floor-throws-a-recursion-error.nvst]
    extra:
      context:
        rules: [errors/stack-depth, errors/on-limit, programs/memory-priority]
        shapes: ["A `.nvst` test case"]
  - id: 2
    title: "the cards, the rule and the attack say what is now true"
    status: done
    context: [tests/hostile/core/Test-Response/jsonAs/01-a-class-that-contains-itself-and-wrong-bodies.nvs, docs/examples/core/Json/decodeAs/about.md]
    extra:
      context:
        rules: [errors/stack-depth, core-api/reference-card]
        shapes: [A rule fragment, "A `Core` member"]
checks:
  - id: within-max-depth
    stage: 1
    name: "a chain 1000 levels deep decodes under `maxDepth: 1024`"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/a-json-chain-within-max-depth-decodes.nvst]
  - id: near-the-floor
    stage: 1
    name: "a decode entered with little stack left still throws a catchable `RecursionError`"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/a-json-chain-hydrated-near-the-stack-floor-throws-a-recursion-error.nvst]
  - id: attack-proof-build
    stage: 1
    name: the jsonAs attack reads the 1000-level body on the proof build
    run: [bun, nv, proofs, --run, --show, --id, "Core\\Test\\Response::jsonAs"]
    stdout_contains: ["deep: read f"]
  - id: attack-covws-build
    stage: 1
    name: the jsonAs attack reads the 1000-level body on the covws build
    run: [bun, nv, proofs, --run, --show, --id, "Core\\Test\\Response::jsonAs", --nvs, target/covws/x86_64-pc-windows-msvc/debug/nvs.exe]
    stdout_contains: ["deep: read f"]
  - id: rules
    stage: 2
    name: the rulebook is consistent
    run: [bun, nv, rules, --check]
---

# A JSON document within maxDepth decodes into objects on every build

## Why

`tests/hostile/core/Test-Response/jsonAs/01-a-class-that-contains-itself-and-wrong-bodies.nvs` step 2
decodes 1000 nested `Folder` objects with `maxDepth: 1024`. The proof build prints `deep: read f`; the
covws debug build prints `deep: the stack is full`. Both are a catchable `RecursionError`, so this is
not a security issue, but whether a document inside the documented `maxDepth` decodes depends on Rust
frame sizes, and the recording run is marked diverged on every sweep.

The cause: the parse runs on the 8 MiB spare stack (`crates/nvs-stdlib/src/json.rs:1828`), and building
the objects does not. `decode_fields` → `decode_field` → `convert_field` → `decode_nested` →
`decode_fields` (`json.rs:2561`, `:2768`, `:2838`, `:2956`, `:3021`), and for a list of objects
`convert_field` → `decode_list` → `decode_element` → `decode_fields` (`json.rs:3214`, `:3385`, `:3451`),
recurses natively on the 1 MiB task stack
(`crates/nvs-host/src/stack.rs:88`), and stops at the soft limit `STACK_RESERVE` (256 KiB) above the
floor (`crates/nvs-runtime/src/ctx/mod.rs:1740`; the compare is `json.rs:2574`). The walk cannot move
to the spare stack, because it runs compiled constructors on the way back up, and the spare stack takes
only code that calls nothing compiled (`json.rs:1827`).

`rule:errors/stack-depth` already says engine frames that request data drives are bounded by an
explicit depth counter (`docs/rules/errors/stack-depth.md:18`). This walk is the one that is not.

## Standing decisions

The user chose this fix. A session does not reopen it.

**Build the objects iteratively, with frames on the heap, the way the encoder does**
(`json.rs:224`). One loop descends into each nested object and list element, pushing a frame,
and runs each constructor from that loop on the way back up. Then `maxDepth` is the only depth bound on
every build, and the attack prints `deep: read f` on both. If a door turns out to need a native frame
per level after all, it keeps the stack compare it has today and the handoff names it.

**Every door that reaches the walk changes with it.** They all reach `decode_fields` through `hydrate`
(`json.rs:2340`) or `decode_as` (`json.rs:1960`): `Core\Json::decodeAs` (`json.rs:1930`),
`Core\Test\Response::jsonAs` (`crates/nvs-stdlib/src/test.rs:3302`), `Core\Request::jsonAs`, `queryAs`
and `postAs` (`crates/nvs-stdlib/src/request.rs:3100`, `:2457`, `:2583`), `Core\Http\Response::jsonAs`
(`crates/nvs-stdlib/src/http.rs:4751`), the JWT claims (`crates/nvs-stdlib/src/jwt.rs:2036`) and
`Core\Arr::shapeAs` (`crates/nvs-stdlib/src/arr.rs:6233`). A hand-written decoder member (`json.rs:2288`)
calls compiled code in the middle of the walk; how the loop hands it a frame is not checked yet, and
stage 1 starts there.

**The constructor call keeps a stack compare.** A constructor is compiled code and runs on the task
stack, so a decode entered with little stack left still throws a catchable `RecursionError` before it
calls one. The near-floor case keeps its expected output; its description and comment say which
compare throws now.

**Trade-offs, stated once:**

- Behaviour: a `RecursionError` from `jsonAs` or `decodeAs` then comes only from constructor code, or
  from a decode entered with little stack left, never from the depth of a document within `maxDepth`.
- Memory: O(depth) small heap frames per call, freed when the call ends. Estimated at tens of KB at
  `DEPTH_CEILING` (`json.rs:547`); not measured. Say the measured figure in the module doc.
- Latency: about neutral; not measured.
- Simplicity: more complex code in a module that uses `unsafe`. Each new `unsafe` block carries its
  reason, as the existing ones do.

## Stages

### 1. the iterative build walk

**Does:** building the objects takes a heap frame per level, and a 1000-level document under
`maxDepth: 1024` decodes on the covws build as it does on the proof build.

- [x] The frame type and the loop, beside the encoder's (`crates/nvs-stdlib/src/json.rs:224`)
- [x] `decode_fields`, `decode_field`, `convert_field` and `decode_nested` as steps of the loop, constructors run on the way up (`crates/nvs-stdlib/src/json.rs:2561`, `crates/nvs-stdlib/src/json.rs:2838`, `crates/nvs-stdlib/src/json.rs:2956`)
- [x] A list of objects the same, and a shape built on the way up as a class is (`crates/nvs-stdlib/src/json.rs:3214`, `crates/nvs-stdlib/src/json.rs:3385`, `crates/nvs-stdlib/src/json.rs:2693`)
- [x] A hand-written decoder member inside the loop (`crates/nvs-stdlib/src/json.rs:2288`)
- [x] The new case at 1000 levels with `maxDepth: 1024` (`tests/conformance/core/a-json-chain-within-max-depth-decodes.nvst:1`)
- [x] The near-floor case's description and comment (`tests/conformance/core/a-json-chain-hydrated-near-the-stack-floor-throws-a-recursion-error.nvst:1`)
- [x] The module doc says what the walk spends, measured (`crates/nvs-stdlib/src/json.rs:233`)

### 2. the cards, the rule and the attack say what is now true

**Does:** no reference card says a deep document fills the stack.

- [x] The `RecursionError` text on the three cards that have one (`crates/nvs-stdlib/src/json.rs:467`, `crates/nvs-stdlib/src/test.rs:1346`, `crates/nvs-stdlib/src/request.rs:767`)
- [x] The rule names the build walk among the engine walks with a depth counter (`docs/rules/errors/stack-depth.md:18`)
- [x] The attack's step 2 comment says the result is an object (`tests/hostile/core/Test-Response/jsonAs/01-a-class-that-contains-itself-and-wrong-bodies.nvs:47`)

## Handoff

### State

Both stages are done and every check is green. `Core\Json`'s decoder builds objects on heap frames
(`crates/nvs-stdlib/src/json.rs:2640`), so `maxDepth` is the only depth bound and the jsonAs attack
prints `deep: read f` on the proof and covws builds. The three `RecursionError` cards, the rule's
engine-walk paragraph (`docs/rules/errors/stack-depth.md:18`) and the attack's step 2 comment say
so. `bun nv verify --doc` is green and `bun nv owners --closes json-decode-depth` names no gap.

The goal is reached. The branch lands on `main` by hand, and this file is deleted as it lands.

### Next

Nothing on this goal. Land `side/json-decode-depth` on `main`, delete this file, then remove the
worktree and the branch.

### Backlog

- `rule:errors/stack-depth` states an 8 MB ceiling per coroutine, and a task stack is 1 MiB: not checked which is meant (`docs/rules/errors/stack-depth.md:11`, `crates/nvs-host/src/stack.rs:88`).
- The `Core\Json` member benches are stale after the change to `json.rs`: `bun nv proofs --record-perf --group 'Core\Json'` (`docs/agent/playbook.md`).
- The issue-path prefix cost is a gap record now: `data/gaps/nvs-stdlib/a-decoded-objects-issue-prefixes-grow-with-depth-squared.json`.
