# Handoff

## State

**Stage 0's catch-up list is closed.** Items 20 and 21 landed together this session, and neither needed
its implementation written: `StrHeader`'s capacity, `mwl_str_append`, the n-ary `InstKind::Concat`, the
data-section string literal, and `map`/`filter`/`reduce`/`sort` reading `mwl_runtime::closure_arity` once
before the walk were all already on disk. What was missing was two measurements and four *names*.

The measurements are `a_one_parameter_callback_synthesizes_no_key` and
`a_sort_that_renumbers_builds_no_keys`, both new in `crates/mwl-stdlib/tests/allocation_policy.rs:436`
and `:498`, which already carried its own debug-only `#[global_allocator]`. That file's `closure_of`
(`:321`) is new and is the reusable half: a closure value built from a `ClassTable` and a native
`extern "C"` callback, which is how a `-p mwl-stdlib` test reaches any member taking a `callable`.

**One premise of item 21 was wrong and the test says so**: preserving keys is not what cost anything on a
list. `MwlArray::slot_key` (`crates/mwl-runtime/src/array.rs:443`) answers a `SlotKey::Index` while the
array is packed and renders nothing, so `preserveKeys: true` was five allocations dearer than `false` over
64 entries, not 64. The one place a sort *renders* a key is a two-parameter `by`, and that is the gap the
test asserts.

The four names are item 20's three in `mwl-runtime` and one in `mwl-codegen`.
`docs/agent/loop-goal.toml` now names the tests the tree actually holds, with a comment per check saying
why each is the truer claim; `loop-goal.md` items 20 and 21 are struck. Verify is green: **1595** tests,
74 suites, clippy and fmt clean.

**The next red acceptance check is Stage 4's own count** — conformance 437 of 600, differential 90 of 150.
Both of Stage 4's *named* guards already pass, so every registered member has a case; what is short is
behavioural depth per member. That is the next group.

## Next group

Three conformance-case slices over the two thinnest `bytes`-facing sections, in this order. Shared file
set: `crates/mwl-stdlib/src/encoding.rs`, `crates/mwl-stdlib/src/hash.rs`,
`tests/conformance/core/encoding-*.mwlt` and `hash-*.mwlt`, and `docs/spec/01-core-library.md` §§ 7 and
11. All three live inside the playbook's `bytes` trap — `"…" as bytes` and `Core\Encoding::fromHex(…)`
are the only two spellings, and `toHex` is the only assertion, because `echo` has no `bytes` row.

- [ ] **base64 and base64url edge rows** — `encoding.rs:821` `toBase64`, `:837` `fromBase64`, `:859`
      `toBase64Url`, `:869` `fromBase64Url`. Four cases today (`encoding-base64-writes-each-variant…`);
      what is unpinned is the empty input, each padding length, an octet neither alphabet admits, and
      the round trip through a buffer `as bytes` cannot produce.
- [ ] **base32 and hex edge rows** — `encoding.rs:890` `toBase32`, `:907` `fromBase32`, `:934` `toHex`,
      `:956` `fromHex`. Same shape one alphabet over, plus the case-folding rule
      `encoding-base32-folds-case-and-padding-but-nothing-else.mwlt` states but does not exhaust.
- [ ] **`Core\Hash` per algorithm** — `hash.rs:452` `of`, `:468` `hmac`, `:501` `equals`, `:549`
      `stream`. Three cases today; one row per algorithm the registry admits, `equals` over a
      length-mismatched pair, and a `Stream` fed in chunks that straddle the block size.

## Backlog

- `Core\Json::decodeAs<T>`'s decoder — `mwl_stdlib::json` gap 2, and ADR 0071's non-scalar fields.
- ADR 0088's registry-wide qualifier classification — `docs/implementation-plan.md` `Open now`.
- `do`/`while` is the one M4 control-flow statement that does not lower — `mwl-ir`'s module doc.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
- The differential suite is 90 of 150 and needs PHP on the native leg — `tests/differential/`.
- `orient.py` gaps this session: `[context]` printed no `mwl-stdlib` map line for `arr.rs`, no
  `mwl-runtime` line for `array.rs`/`closure.rs`, and nothing at all from `loop-goal.toml`'s check
  bodies — which is where the item's real acceptance names live. Add `modules` patterns for those two
  and a selector for the current stage's `[[check]]` block.
