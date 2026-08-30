# Handoff

## State

**Stage 8's corpus count is 993 of 1000.** One case landed:
`tests/conformance/reject/bytes-pack-and-unpack-refuse-a-tainted-format.nvst` — `Core\Bytes::pack` and
`::unpack` share one format grammar and one `Qual::Sink`, so a tainted format is refused over five
routes (a local, a concatenation, a member's return type, an array element, composed with `secret`)
with the same grammar written plain accepted beside them.

**The handed group was mostly unwritable or already on disk, and one gap is why.**
`nvs_types::core_lib::lower` throws the registry's `Qual` away and nothing else in the checker reads
it, so a `Contagious` or `Neutral` parameter refuses a tainted argument exactly as an unclassified one
does. Item 1's contagion half (`Core\Bytes::at` keeping a buffer's taint) and item 2
(`Core\Validate::isEmail` accepting a tainted address without laundering it) both describe ADR 0088
§ 2's *specified* behaviour, which this tree does not do — neither can be asserted today, and pinning
what it does instead would freeze the gap. Recorded where its code is, with the shape of the fix:
`crates/nvs-types/src/core_lib.rs:317`.

**Items 3 and 4 were already on disk.** `dump`/`render` agreement is
`a-dump-is-its-arguments-rendered-and-a-capture-never-swallows-one.nvst` and
`a-dump-writes-one-record-per-argument-and-none-at-all-for-none.nvst`; ADR 0033's refusal is
`reject/a-secret-value-cannot-be-dumped.nvst`; `Core\Random`'s two bounds are
`random-float-s-unit-interval-is-bounded-at-both-ends.nvst` and
`random-int-s-closed-bound-reaches-the-ends-of-int-itself.nvst`. **Two triage methods in a row have now
handed items already covered** — `gaps.py`'s depth ranking, and the `//!`-claim sweep that replaced it —
so check each row before writing, by the playbook's new *Writing a test case* bullet.
`gaps.py --errors` has one non-fatal row left (`crates/nvs-stdlib/src/csv.rs:610`) and that helper's own
comment says it is unreachable from source; do not spend a session on it.

**Three known gaps carry forward unchanged**, each recorded where its code is: item 18's
`Core\Secret::reveal()` is not in the registry (`nvs_types::expr::quals`); `Live::admit`'s same-class
check is asked of the answer and not of the argument (`crates/nvs-runtime/src/graph.rs` § *Known
gaps*); item 22's `Core\Script` members are unwritten (`crates/nvs-stdlib/src/script.rs`).

## Next group

**Close the `Qual` gap, then write the two cases it unblocks.** One file set —
`crates/nvs-types/src/{signatures.rs,core_lib.rs,expr/calls.rs,expr/quals.rs}` and
`crates/nvs-types/tests/tainted.rs`, then `tests/conformance/`. The registry needs no change: ADR 0088
§ 2 keeps the classification on the `CoreMethod` row, and this is the consumer side of it.

- [ ] **Carry the classification into `MethodSig`** (`crates/nvs-types/src/signatures.rs:53`), filled
      where a registry row becomes one (`crates/nvs-types/src/core_lib.rs:89`) rather than dropped at
      `lower` (`:310`, whose comment now states the gap and the fix).
- [ ] **Admit a tainted argument on `Contagious` and `Neutral` at the call check**
      (`crates/nvs-types/src/expr/calls.rs:245`, beside `reject_secret_debug_argument`), with the rule
      itself in `crates/nvs-types/src/expr/quals.rs:301`'s module; `Sink` and unclassified keep
      refusing. **Scope it to `tainted`: `secret` keeps refusing everywhere it refuses today**, so
      nothing here decides whether a `Neutral` member may launder a secret — that half is ADR 0088's to
      answer before it is written. Unit cases go beside `crates/nvs-types/tests/tainted.rs:104`.
- [ ] **A `Contagious` call's result gains `tainted`** when any contagious argument carries it, while
      `Neutral`'s answer stays plain — then the two `.nvst` cases this session could not write:
      `Core\Bytes::at` keeps the buffer's taint where `length`, `indexOf` and `compare` answer plain
      (agreement over four, one tainted subject, `crates/nvs-stdlib/src/bytes.rs:179-228`), and
      `Core\Validate::isEmail` accepts a tainted address without laundering it
      (`crates/nvs-stdlib/src/validate.rs:181`), the plain-binding assignment still `--EXPECTF-ERROR--`.

## Backlog

- 7 cases to 1000, Stage 8's remaining acceptance; two of them come with the group above.
- `[context] modules` has no pattern for `crates/nvs-cli/src/`, `benches/abi-probe/`,
  `crates/nvs-types/src/expr/`, or `crates/nvs-stdlib/src/{csv,hash,uuid,math,debug,random,validate,
  bytes}.rs` — every module the last two sessions had to read.
- `[context] shapes` does not print the `.nvst` runner invocation
  (`./target/debug/nvs.exe test <path>`), which a corpus session uses on every case.
- ADR 0088 owes the `secret` half of the classification: whether `Neutral` launders one, and what a
  `Contagious` member does with one it is allowed to receive.
- `Core\Str`'s 36 `Contagious` rows are the widest surface the gap above closes; one sweep case over
  them is worth more than one per member.
