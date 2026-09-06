# Sweep items — the C8 hand-fix list

One bullet per item, `file:line — what is wrong there`, grouped by the unit whose transaction found
it. `--sweep` (unit C8) works this list down to nothing; a bullet leaves when its site is fixed.
`.migration/assemble.py` appends a unit's `## sweep:` lines here at assembly, so a hand-fix is
recorded once, in one file, instead of in the unit's commit body. The items below B16 were lifted
from the commit bodies of c7a9afca6 through 44ba60ce9, re-anchored to the lines they sit on today.

## B4 — enums

- docs/adr/0019-reflection-and-ast-parsing-are-core-features.md §4 — asserts enums already have `::cases()`; they do not, and `enums/reflection` states the currently true rule. Record defect, frozen as is at C1.

## B6 — attributes

- docs/adr/0046-*.md §6 — "exactly two call sites at launch" is stale; `parse_call_type_args` also serves `Core\Program::implementing<T>()`. Record defect.
- .migration/topic-map.json — `0089 §5` is assigned to attributes but is `nvs convert`'s `TODO(convert:<id>)` rule; B6 declined it and B20 (tooling) claims it. Correct the row.

## B7 — testing

- docs/adr/0079-*.md §24 — the milestone table; deliberately not remapped, its one site is a sentence about a schedule. Record-level completeness claims the anchor; confirm the sweep does not count it as lost.

## B9 — types

- crates/nvs-types/src/generics.rs (module doc) — names the ADR 0136 gap: `Ty::CallableTo` / `Ty::CallableShapeTo` still exist, the `callable` atom takes no parameter list, `check_param_tags` runs at every call. Five `types/` rules are `designed` until this closes; a conformance case still asserts the restriction 0136 §6 retires.

## B10 — classes

- crates/nvs-diagnostics/src/lib.rs — `E_INTERFACE_MEMBER_CONFLICT` (ADR 0043 §5) has no `Code` and nothing reports it; `crates/nvs-types/src/conformance.rs` never checks that two sources answer one name. `classes/member-conflict-is-an-error` is `designed`.
- docs/adr/0014-*.md §6 — contradicts itself on `__call` (compiles as ordinary method vs. refused by the casing checker); the tree refuses, `classes/no-call-magic` says so. Record defect.

## B11 — core-api

- crates/nvs-config/src/capability.rs:31 — "a grant reachable two ways is what R20 forbids": that is R17, not R20 (0063's own *Verification* files `E0458` under the wrong row).
- crates/nvs-config/tests/capability.rs:429 — same slip, "what R20 asks for" is R17/R18.
- crates/nvs-config/src/schedule.rs:459 — cites `ADR 0063 § 4` for "no ambient timezone"; §4 is deliberately unremapped, re-cite by hand to the no-ambient-state rule B11's note named.
- crates/nvs-config/tests/schedule.rs:148 — same, `ADR 0063 § 4` for the ambient-zone claim.
- crates/nvs-stdlib/src/objmap.rs:461 — `ADR 0063 § 4` for `int|string` keys; core-classes material no chapter owns.
- docs/spec/01-core-library.md:607 — `ADR 0063 § 4` for inline-shape participation; retired with docs/spec at C2 or re-cited.
- docs/spec/01-core-library.md:704 — same.
- tests/conformance/core/json-encode-decode-and-validate.nvst:54 — `ADR 0063 § 4` for written wire-format participation; re-cite to the written-participation rule.
- tests/conformance/core/json-encodes-a-shape-as-an-object.nvst:36 — same.

## B12 — core-classes

- crates/nvs-stdlib/src/regex.rs:30 — module doc says literal folding "is not built"; `crates/nvs-types/src/intrinsics.rs` validates `Grammar::Regex` and records the tier at check time. Stale.
- .migration/topic-map.json — `0121 §6` says `classes`; B12 claimed it as core-classes (`core-classes/pdf-one-engine`). Correct the row.
- docs/adr/0012-*.md §5, 0071 §5, 0067 §4, 0056 §§2-3, 0019 §1 — each contradicted by the tree as B12's commit lists (`Core\Cli::args()`/`argc()` vs `arguments()`; no `issues` slot on `DbError`; `stream` PostgreSQL-only and no `streamAs`; no `[regex]` block; `ClassInfo::of` not the shipped shape). Record defects.

## B13 — security

- crates/nvs-config/src/capability.rs:63 — cites `ADR 0006 § 5`; ADR 0006 has no numbered sections (legacy dangling).
- crates/nvs-config/src/tree.rs:245 — same, `ADR 0006 § 5`.
- crates/nvs-config/src/tree.rs:266 — same, `ADR 0006 § 5`.
- crates/nvs-config/tests/capability.rs:159 — same, `ADR 0006 § 5`.
- docs/adr/0112-*.md §8 — calls itself the roster's home and lists nine capabilities; `crates/nvs-config/src/capability.rs` carries eleven (`db.schema`, `mail.send` missing from the table). Record defect.

## B14 — concurrency

- crates/nvs-host/src/group.rs:308 — a second throw goes to the diagnostic channel where 0072 §4's table sends it to `Core\Log`; the rule says "reported, never swallowed". Reconcile tree and record.
- crates/nvs-runtime/src/deferred.rs:36 — says in bold an isolate has a deferred queue; 0072 §6 and `task.rs:611`'s user-facing message say it has none. The tree disagrees with itself.
- crates/nvs-config/src/tree.rs — `[limits] max_tasks` is parsed, rostered, named to users and covered by a config test, and enforced nowhere outside `nvs-config`.
- (twelve sites) — of the 36 citing `rule:concurrency/all-answers-a-typed-shape`, twelve mean `rule:concurrency/a-child-belongs-to-the-calling-task` (0072 §1's second claim). B14's note listed them and was not preserved; re-derive by reading each site.

## B15 — routing

- crates/nvs-stdlib/src/links.rs:230, :375 — cite `routing/matching-is-not-dispatching` for the link half; want `routing/link-name-and-params-are-checked`.
- crates/nvs-diagnostics/src/lib.rs:2474, :2485 — same (0077 §4 link half).
- crates/nvs-stdlib/src/router.rs:186, :503, :770 — same.
- crates/nvs-stdlib/src/registry.rs:1313 — same.
- crates/nvs-stdlib/src/lib.rs:320, :462 — same.
- crates/nvs-types/src/expr_table.rs:248, crates/nvs-types/src/lib.rs:428, crates/nvs-types/src/expr/calls.rs:487 — same.
- tests/conformance/*/router-url-*.nvst (headers), goal 29's manifest and handoff, docs/agent/playbook.md:6867, :6888 — same.
- crates/nvs-diagnostics/src/lib.rs:2528 — cites `routing/an-absolute-link-takes-a-configured-origin` when it means `routing/a-leftover-link-key-is-a-query-string` (0102 §6 query half).
- crates/nvs-stdlib/src/links.rs:207, :263, :316 — same.
- crates/nvs-stdlib/src/uri.rs:1820, crates/nvs-stdlib/src/router.rs:607, crates/nvs-types/tests/routes.rs:765, tests/conformance/*/router-url-turns-a-leftover-params-key-into-a-query-string.nvst — same.
- crates/nvs-stdlib/src/router.rs:777 — throws "the compile-time route table is not built yet", citing `rule:routing/table-is-opt-in` in a user-visible string; the table is built and the message is stale.
- crates/nvs-diagnostics/src/lib.rs:2405 — cites `security/route-capture-is-laundered-by-its-type` for "its duplicate-route error", which that rule does not state; the claim is `routing/routes-are-compiled-not-registered`.
- crates/nvs-types/src/routes.rs:535 — `RouteTable::named`'s doc comment states the pre-0110 unconditional duplicate-name rule.
- docs/adr/0085-*.md §3 — spells `nvs build --openapi <path>` as an output path; the command takes the source and writes to stdout. Record defect.
- docs/adr/0077-*.md §4, 0102 §8 — give `Core\Router\Match` `method` and `access` members; the shipped class has neither. Record defects.
- tools/adr.py:89 — spells `ADR 0102 SS 9`, which no pass reads.
- (no file) — 0110 §3 is shipped and unguarded: no fixture declares a shared-name route.

## B16 — config

- crates/nvs-config/src/mode.rs:24 — 0091 §3's five defaults are not applied at boot, while `crates/nvs-runtime/src/ctx/output.rs:343` claims the tree carries them and falls back to `Debug`.
- crates/nvs-diagnostics/src/lib.rs:1551 — ADR 0142 §4 assigns `E0627`, already `E_UNSPELLED_EXPORTER`; 0142's Validated-by names two `cache.rs` tests that do not exist.
- crates/nvs-config/src/tree.rs:120 — cites the file cache as `ADR 0042 § 9`; the record ends at §8 and the directives are §7.
- crates/nvs-config/src/tree.rs:978 — same, `ADR 0042 § 9` beside `0017 § 2` (both legacy dangling).
- crates/nvs-cli/src/cache.rs:189 — still describes the store refusal in `net.connect` terms.
- docs/agent/carried-gaps.md:52 — says the 0042 cache has no caller; `crates/nvs-cli/src/main.rs:1200` and `runner.rs:265` read units off disk.
- docs/adr/0064-*.md §2a — block table lacks `[io]`, `[mail.<name>]`, `[storage.<name>]`, `[session]`, `[app.log]`. Record defect.
- docs/adr/0103-*.md §7 — says `W1xxx` where the tree uses `W1005` and `W1007`. Record defect.
- docs/adr/0005-*.md — names `E0602` for a refused `System` set; the tree's is `E_CAPABILITY_DENIED`, and the setter raises no code and emits no `W`. Record defect.
- docs/adr/0103-*.md — `nvs info --config` does not exist; the answer is `nvs config dump`. Record defect.
- (goals 23, 24; crates/nvs-config/src/tree.rs:978; docs/agent/playbook.md) — `0017 §2`, `§3a`, `§5` name sections the record never had; legacy dangling.
