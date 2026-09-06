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

## B17 — packaging

- crates/nvs-cli/src/cache.rs:49 — the read path `mmap`s the cache file read-only, which 0106 §11 (`rule:packaging/the-artifact-cache-is-read-not-mapped`) forbids because a file replaced under the mapping raises `SIGBUS`; the doc comment argues the mapping from 0042's *Investigation* and never mentions 0106.
- crates/nvs-cli/src/cache.rs:70 — "Cost of a hit: one `open`, one `mmap`" restates the mapped read path 0106 §11 overturned; the hit cost after the rule is one `open`, one `read` into a heap buffer, one BLAKE3 pass.
- docs/adr/0042-on-disk-artifact-cache-format.md:164 — § 3's read path still says "`mmap` the file read-only", and 0042's *Investigation* (line 76) still weighs the mapping as the winner, while 0106 §11 decides the opposite and 0106's *Amends* list does not name 0042; one of the two records is wrong and the citing rule follows 0106.
- crates/nvs-stdlib/src/random.rs:60 — says `nvs service` "is unbuilt"; `nvs service unit`, the `E0630`–`E0634` refusals and the `ImagePath` encoder are on disk in `crates/nvs-cli/src/service.rs`, and only registration (`install`/`run`/`start`/`stop`/`status`) is not, so the sentence should say the service manager entry point is unbuilt.
- docs/spec/02-php-migration.md:1355 — the `php_strip_whitespace` row says "Novis ships a compiled artifact (ADR 0048)"; a bundle ships source (0048 § 2), so the row's reason is wrong and will cite `packaging/nvs-build-compile-appends-the-program-to-a-copy-of-the-host` while saying the opposite.
- docs/novis.md:21973 — the same row, generated from the line above; regenerates once the spec row is fixed.
- crates/nvs-cli/src/service.rs:119 — "an [ADR 0048] bundle (§ 6)" reads as 0048 § 6 (the `.nvsx` section) but means 0093 § 6, which is `programs/bundle-trust-domain`; a section-aware rewrite would name the wrong rule.
- crates/nvs-cli/src/bundle.rs:46 — the module doc records that `codesign --sign -` failing is a warning, where 0048 § 5 has the build signing by default; once `packaging/a-macos-bundle-is-ad-hoc-signed-at-build` is the citation the gap should be stated against the rule rather than the section.
- docs/plan/m15.md:24 — says `E0604`-`E0607` are 0112's codes and land in M15, but `nvs_diagnostics::code` has since issued E0604–E0607 to configuration diagnostics (`E_DUPLICATE_DIRECTIVE` … `E_UNTRUSTED_CONFIG`) and 0112's own *Diagnostics* section says the four numbers are claims the band has moved past; the plan should say "the band's next free numbers".
- docs/adr/0068-dependency-currency-and-the-version-contract.md:57 — "Sections 3–8 below do not apply" during prototyping, yet `tools/release.py:192` implements § 3's scheme for the 0.0.x releases today and `docs/agent/dependency-update.md:49,107` apply § 7 and § 8 now; the record should say §§ 2, 4, 5 and 6 are contract-only (which is what the tree practises) rather than 3–8.
- crates/nvs-config/src/tree.rs:89 — cites `ADR 0003 § 3` for the `[[extension]]` pin; ADR 0003 has no numbered sections, and the pin is 0003's own rule (not one of records 0061/0068/0081/0112), so whoever owns 0003 in this chapter should give the site a rule id.
- crates/nvs-config/src/tree.rs:332 — same `ADR 0003 § 3` citation for the `[[extension]]` entry struct; same fix as :89.

## B18 — observability

- crates/nvs-runtime/src/trace_context.rs:22 — cites "[§ 1]'s exporter is unbuilt", but the exporter is § 6/§ 8's; § 1 is the default series. Should cite observability/the-exporters-are-crates
- tests/conformance/core/a-topic-name-is-a-sink-and-refuses-one-that-came-from-outside.nvst:9 — "0076 § 1 applies to a metric label" names the wrong section; the label refusal is § 4, i.e. security/metric-label-refuses-tainted
- crates/nvs-stdlib/src/topic.rs:152 — bare 0076 cited for the tainted-label rule will resolve to the chapter's framing rule; it wants security/metric-label-refuses-tainted
- docs/adr/0083-persistent-connections-are-isolates.md:189 — bare 0076 cited for the tainted-label rule; wants security/metric-label-refuses-tainted
- docs/adr/0080-the-audience-nvs-is-built-for.md:188 — "per-tenant observability labels (0076)" reaches for the label rule, security/metric-label-refuses-tainted, not the framing rule
- crates/nvs-render/src/lib.rs:428 — "§ 6's rule for trace_id/span_id" will resolve to observability/metrics-and-trace-blocks-are-system; it wants observability/a-log-record-carries-trace-ids-when-a-trace-is-active
- crates/nvs-runtime/src/ctx/output.rs:275 — same: § 6's omitted-not-empty rule wants observability/a-log-record-carries-trace-ids-when-a-trace-is-active
- crates/nvs-server/Cargo.toml:52 — "ADR 0076 § 6 has a log record carry the trace id" wants observability/a-log-record-carries-trace-ids-when-a-trace-is-active
- crates/nvs-server/src/trace.rs:234 — "§§ 2 and 6" about the log record's ids wants observability/a-log-record-carries-trace-ids-when-a-trace-is-active beside the trace-id rule
- docs/adr/0020-error-escalation-ladder.md:232 — 0076 § 6 cited for trace_id/span_id on the record wants observability/a-log-record-carries-trace-ids-when-a-trace-is-active
- docs/adr/0092-one-diagnostic-record-three-renderings.md:119 — 0076 § 6 cited for trace_id/span_id wants observability/a-log-record-carries-trace-ids-when-a-trace-is-active
- docs/agent/carried-gaps.md:7 — "ADR 0076 § 6 names six" log-record fields wants observability/a-log-record-carries-trace-ids-when-a-trace-is-active
- docs/agent/goals/21-carried-gaps.md:140 — 0076 § 6's four log-record fields wants observability/a-log-record-carries-trace-ids-when-a-trace-is-active
- docs/agent/goals/21-carried-gaps.md:242 — "omitted, per ADR 0076 § 6" wants observability/a-log-record-carries-trace-ids-when-a-trace-is-active
- docs/agent/goals/9-schema.toml:4152 — "Stage 6 -- a log line can be jumped to from a trace. 0076 § 6" wants observability/a-log-record-carries-trace-ids-when-a-trace-is-active
- docs/agent/loop-goal.toml:4157 — same line as 9-schema.toml:4152, same target
- docs/agent/goals/chain.toml:66 — "ADR 0076 § 6's four log-record fields" wants observability/a-log-record-carries-trace-ids-when-a-trace-is-active
- docs/agent/goals/README.md:70 — "0076 § 6's four missing log-record fields" wants observability/a-log-record-carries-trace-ids-when-a-trace-is-active
- crates/nvs-stdlib/src/http.rs:1218 — "§§ 2 and 6: [trace] propagate" wants observability/an-outbound-call-propagates-traceparent
- crates/nvs-stdlib/src/http/transport.rs:351 — "ADR 0076 § 2" on outbound propagation wants observability/an-outbound-call-propagates-traceparent, not the trace-id rule
- crates/nvs-stdlib/src/http/transport.rs:688 — same, wants observability/an-outbound-call-propagates-traceparent
- crates/nvs-stdlib/src/http.rs:928 — "§ 2 — propagating" wants observability/an-outbound-call-propagates-traceparent
- docs/agent/goals/4-core-part-ii.md:126 — "Outbound traceparent propagation — ADR 0076 § 2" wants observability/an-outbound-call-propagates-traceparent
- docs/plan/m8.md:65 — "(ADR 0076 § 2), which is the point at which a trace crosses a service" wants observability/an-outbound-call-propagates-traceparent
- docs/agent/goals/9-schema.toml:2469 — "ADR 0076 § 2 -- where a trace crosses a service boundary" wants observability/an-outbound-call-propagates-traceparent
- docs/agent/loop-goal.toml:2474 — same line as 9-schema.toml:2469, same target
- crates/nvs-cli/src/serve.rs:403 — "§ 2's trace ... continued" wants observability/an-inbound-traceparent-is-continued
- crates/nvs-server/src/trace.rs:1 — module doc on continuation wants observability/an-inbound-traceparent-is-continued beside the trace-id rule
- crates/nvs-server/src/trace.rs:45 — same, observability/an-inbound-traceparent-is-continued
- crates/nvs-server/src/trace.rs:114 — same, observability/an-inbound-traceparent-is-continued
- crates/nvs-runtime/src/ctx/inbound.rs:57 — "§ 2's trace is the door's decision" wants observability/an-inbound-traceparent-is-continued
- crates/nvs-runtime/src/ctx/inbound.rs:325 — same, observability/an-inbound-traceparent-is-continued
- crates/nvs-runtime/src/ctx/inbound.rs:514 — same, observability/an-inbound-traceparent-is-continued
- crates/nvs-server/src/route.rs:8 — "ADR 0076 § 1's route label" will resolve to observability/default-series; it wants observability/route-label-is-the-declared-name
- crates/nvs-server/src/route.rs:24 — same, observability/route-label-is-the-declared-name
- crates/nvs-server/src/route.rs:41 — same, observability/route-label-is-the-declared-name
- crates/nvs-server/src/route.rs:119 — same, observability/route-label-is-the-declared-name
- crates/nvs-server/src/route.rs:226 — same, observability/route-label-is-the-declared-name
- crates/nvs-server/src/route.rs:263 — same, observability/route-label-is-the-declared-name
- crates/nvs-runtime/src/routes.rs:459 — "§ 1's route label" wants observability/route-label-is-the-declared-name
- crates/nvs-stdlib/src/router.rs:1038 — "§ 1's route label reads" wants observability/route-label-is-the-declared-name
- crates/nvs-server/src/metrics.rs:924 — the test on the declared name cites § 1 by proximity; wants observability/route-label-is-the-declared-name
- docs/adr/0077-compile-time-routing.md:174 — "0076 § 1's route label" wants observability/route-label-is-the-declared-name
- docs/adr/0077-compile-time-routing.md:198 — same, observability/route-label-is-the-declared-name
- docs/adr/0077-compile-time-routing.md:415 — same, observability/route-label-is-the-declared-name
- docs/adr/0102-a-request-is-matched-once-and-the-route-table-completes-without-dispatching.md:42 — "0076 § 1's route label" wants observability/route-label-is-the-declared-name
- docs/adr/0102-a-request-is-matched-once-and-the-route-table-completes-without-dispatching.md:88 — same, observability/route-label-is-the-declared-name
- docs/adr/0102-a-request-is-matched-once-and-the-route-table-completes-without-dispatching.md:208 — same, observability/route-label-is-the-declared-name
- docs/adr/0102-a-request-is-matched-once-and-the-route-table-completes-without-dispatching.md:491 — same, observability/route-label-is-the-declared-name
- docs/adr/0110-one-methods-repeated-routes-share-a-name-when-they-share-a-path.md:29 — "0076 § 1's route label" wants observability/route-label-is-the-declared-name
- docs/adr/0110-one-methods-repeated-routes-share-a-name-when-they-share-a-path.md:58 — same, observability/route-label-is-the-declared-name
- docs/adr/0110-one-methods-repeated-routes-share-a-name-when-they-share-a-path.md:131 — same, observability/route-label-is-the-declared-name
- docs/adr/0110-one-methods-repeated-routes-share-a-name-when-they-share-a-path.md:234 — same, observability/route-label-is-the-declared-name
- crates/nvs-stdlib/src/db/mod.rs:113 — "ADR 0067 § 11's slow_query" will resolve to observability/a-query-is-a-trace-event; it wants observability/a-slow-query-is-logged-past-a-threshold
- crates/nvs-stdlib/src/db/span.rs:11 — "§ 11's slow_query line" wants observability/a-slow-query-is-logged-past-a-threshold
- crates/nvs-stdlib/src/db/span.rs:86 — "§ 11's threshold" wants observability/a-slow-query-is-logged-past-a-threshold
- crates/nvs-config/src/db.rs:388 — "§ 11's slow_query threshold" wants observability/a-slow-query-is-logged-past-a-threshold
- crates/nvs-config/src/tree.rs:641 — "(ADR 0067 § 11), as 200ms or 1s" wants observability/a-slow-query-is-logged-past-a-threshold
- crates/nvs-config/tests/db.rs:198 — "§ 11's threshold" wants observability/a-slow-query-is-logged-past-a-threshold
- crates/nvs-server/src/metrics.rs:52 — "Nothing scrapes or pushes this" and "Core\Metrics's three members have no row yet" is a known-gap paragraph the chapter now records as two `designed` rules; re-point it at observability/the-exporters-are-crates and observability/metrics-three-members when they ship
- crates/nvs-stdlib/src/server.rs:12 — `Core\Server::traceId()` is recorded as a known gap; observability/a-trace-id-exists-for-every-request names it as reading the id, and the module should cite that rule when it lands
- crates/nvs-runtime/src/ctx/trace.rs:56 — `TraceKind::Call`'s doc says it is "the only kind anything in the tree records today", but `Ctx::record_query` at line 147 files a `Query` and the test at line 420 says "the two kinds anything in the tree records"; the `Call` comment is stale.
- crates/nvs-config/src/tree.rs:648 — "ADR 0041's `query` event carries, which is what § 11 means" cites a bare 0041 beside a `§ 11` that belongs to 0067; once the bare record resolves to a rule the `§ 11` reads as a section of it, so the sentence should name `rule:...` for 0067 § 11 explicitly.
- docs/adr/0041-timeline-export-and-gc-spawn-trace-events.md:13 — `Amended by: 0067, 0076` while the body already folds both; harmless until C1 strips it, listed so the freeze does not miss that §§ 1 and 5 are the folded text.

## B19 — http-server

- docs/adr/0091-run-mode-is-two-values-a-ceiling-and-a-list-of-defaults.md:256 — "Four directives, printed by `nvs info --config`" contradicts § 3's own table, which has five rows since 0097 § 9 added `[log] access`.
- docs/adr/0091-run-mode-is-two-values-a-ceiling-and-a-list-of-defaults.md:314 — the verification bullet says "the four rows of § 3's table"; the table has five.
- docs/adr/0097-development-server-and-proxied-origin.md:216 — the `[server]` example's `dispatch` and `static` comments say "§ 3a of `rule:config/two-modes-and-the-default-is-production`", but 0091 § 3a landed as `config/a-startup-default-is-never-flipped`; the folded token names the wrong rule.
- crates/nvs-server/src/cors.rs:16 — cites "[ADR 0097] § 4's two headers" for `Origin` + `Access-Control-Request-Method`; the preflight definition is 0097 § 7, so the citation will resolve to the five-step resolution rule instead of `http-server/a-preflight-is-answered-before-any-code-runs`. Same at cors.rs:230 and cors.rs:708.
- crates/nvs-stdlib/src/request.rs:1429 — "ADR 0097 § 2's server refuses a token outside the roster with a `501`" attributes a verb-roster rule to § 2, which states only that a path is never derived from a URL; the citation will resolve to `http-server/a-path-is-never-derived-from-a-url` and name the wrong thing. Same at crates/nvs-stdlib/src/lib.rs:251.
- crates/nvs-config/src/snapshot.rs:74 — the doc comment spells the fallback as "`[app] origin`", but there is no `[app]` table (`config/origin-is-a-block-key-and-there-is-no-app-table`); it is the `[[app]]` block's `origin` key.
- crates/nvs-stdlib/src/server.rs:12 — the module doc records `Core\Server::traceId()` as a known gap; when it lands, `http-server/the-trace-id-is-the-request-identifier` can move to `shipped` if the response header is emitted by then too.
- docs/agent/goals/16-request-json.md:70 — "ADR 0139" here names a body-read rule ("a body is read once; buffering readers share, streaming readers consume") that goal 16 planned to open under that number; 0139 became the session record, so after remap this cites the session store rule. Find the record that actually holds the body rule and re-point it.
- docs/agent/goals/16-request-json.md:93 — same stale "ADR 0139" as the body rule; will resolve to the session store rule.
- docs/agent/goals/16-request-json.md:128 — "[ADR 0139]" as the number this goal may open; the number is taken by the session record.
- docs/agent/goals/16-request-json.toml:111 — "Re-pointed at ADR 0139" for the body rule; wrong record after remap.
- docs/agent/goals/16-request-json.toml:157 — stage-3 comment cites ADR 0139 for the buffering/streaming reader rule; wrong record.
- docs/agent/goals/16-request-json.toml:163 — a stage named "nvs-stdlib (ADR 0139 -- who may read a body)"; wrong record.
- docs/agent/goals/16-request-json.handoff.md:44 — "Stage 3 (ADR 0139, hold_body, claim_body …)" cites the body rule under the session record's number.
- docs/agent/goals/18-input-shapes.md:106 — "`postAs` is a buffering reader under ADR 0139" cites the body rule under the session record's number.
- docs/agent/goals/18-input-shapes.toml:178 — "ADR 0139, unchanged: this is a buffering reader" cites the body rule under the session record's number.
- docs/agent/goals/chain.toml:208 — "ADR 0139's body rule and the two JSON body readers" cites the body rule under the session record's number.
- docs/adr/0074-http-defaults-safe-and-finite.md:355 — the Verification bullet spells `retry: {attempts: 3}`, the nested shape § 5's own body refuses; the option is `retryAttempts: 3`.
- docs/adr/0074-http-defaults-safe-and-finite.md:360 — "`Client::post` with `retry` and no `retryIdempotencyKey`" spells the nested `retry` key § 5 refuses; it is `retryAttempts`.
- docs/adr/0074-http-defaults-safe-and-finite.md:210 — "Expiry throws `TimeoutError` (docs/spec/01-core-library.md § 10)" — a link into `docs/spec/`, which C2 retires; the class is `TimeoutError` in `crates/nvs-runtime/src/throwable.rs`.
- crates/nvs-config/src/session.rs:105 — the plainer-note branch says "ADR 0139 § 3 admits …" as running diagnostic text; after remap this should name the rule, and the assembler's rewriter may or may not reach a format-string literal.
- crates/nvs-stdlib/src/session.rs:620 — a thrown message for `backend = "db"` spells "a store ADR 0139 § 3 admits" inside a string literal; same concern as the config note.
- crates/nvs-cli/src/cache.rs:49 — the artifact cache is `mmap`ed read-only on the read path, which ADR 0106 § 11 forbids outright ("read into memory rather than mapped", because a file replaced under a mapping raises `SIGBUS`, tier A's failure); the module argues the opposite trade under ADR 0042 § 3, so one of the two records is wrong and neither says so.
- crates/nvs-host/src/watchdog.rs:32 — "admission control, which does not exist in this crate yet" is stale in half: `nvs_server::admit` exists now, but it does not subscribe to the watchdog's report, so ADR 0106 § 7's shed half (the core stops accepting, `max_in_flight` counts its share as unavailable) is still unbuilt and the comment should say which.
- crates/nvs-server/src/body.rs:55 — `UPLOAD_TOTAL` is a constant standing in for the `[limits] upload_total` row ADR 0105 § 5 specifies; the comment says the configuration slice will replace it, and no `nvs-config` row exists yet.
- crates/nvs-stdlib/src/request.rs:1948 — `REQUEST_BODY` is a constant standing in for the `[limits] request_body` row ADR 0105 § 5 (and 0097 § 8 before it) specify; there is no `request_body` directive anywhere in `crates/nvs-config`.
- docs/adr/0106-nothing-a-request-sends-terminates-or-wedges-a-worker.md:433 — "None of these guards exists yet" is stale: the § 2 and § 3 guards are in `benches/abi-probe/tests/invariants.rs`, the § 5 perf guard in `benches/abi-probe/tests/perf_guards.rs`, the § 8 accept-loop test and § 7 watchdog tests are in their modules, and the § 13 admission test is in `crates/nvs-server/src/admit.rs`.
- docs/adr/0106-nothing-a-request-sends-terminates-or-wedges-a-worker.md:448 — names `tests/invariants.rs`, which does not exist; the file that holds the § 2 claim is `benches/abi-probe/tests/invariants.rs`.
- docs/adr/0105-an-uploaded-file-is-a-stream-and-there-is-one-way-to-receive-it.md:126 — the worked example writes `readAll(max: "200M")` and `saveTo($dest, max: "50M")`, but the shipped bag takes `max` as a `uint` count of bytes (`tests/conformance/core/a-file-parts-read-all-bound-is-a-count-of-bytes-in-a-closed-bag.nvst` refuses the text spelling); the fragment for § 3 writes the numeric form.

## B20 — tooling

- docs/adr/0086-core-cli-terminal-is-a-sink.md:379 — § 8 says every `Core\Cli` member throws in a request and in a spawned isolate, but no member in `crates/nvs-stdlib/src/cli.rs` checks its context, and the landed `docs/rules/core-classes/cli-arguments.md:14` records that the shipped `arguments()` answers empty inside a request rather than throwing; the record and the landed rule disagree about the same surface.
- crates/nvs-runtime/src/terminal.rs:775 — the region does not repaint on resize because `Core\Signal` is not built, while 0086 § 5 lists repaint-on-resize among the runtime's obligations; the comment records the gap but nothing tracks it.
- crates/nvs-cli/src/main.rs:3 — the module doc's "Ten subcommands so far" roster omits `serve`, `queue`, `service` and `worker`, all of which exist as `Command` variants and modules in this crate.
- crates/nvs-cli/src/main.rs:57 — "`serve`, `fmt` and the rest of the architecture diagram arrive with the milestones that need them" is stale for `serve`, which is a `Command::Serve` variant in this file.
- docs/adr/0089-convert-is-one-rule-table-with-two-modes.md:5 — the record's `Amended by: 0090` line names no section and the body's § 2 already carries 0090's `==` reading; when the record freezes at C1 this line is an overlay with nothing left to apply.
- crates/nvs-cli/Cargo.toml:48 — a `# …` manifest comment cites "ADR 0117 § 2"; if the rewriter does not walk `.toml` under `crates/`, this site must be re-pointed by hand to `rule:tooling/meta-json`.
- docs/adr/0137-a-doc-comment-is-three-slashes-and-two-tags.md:68 — states in the present tense that `Lexer::skip_trivia` at `lexer.rs:357` classifies `///` as `TriviaKind::DocComment`, but `crates/nvs-syntax/src` has no `TriviaKind` and no trivia layer at all; the record describes a design as landed.
- docs/adr/0137-a-doc-comment-is-three-slashes-and-two-tags.md:131 — anchors `nvs meta --json` to `crates/nvs-cli/src/main.rs:509`; the `Meta` subcommand is at `main.rs:367` today, so the line reference is stale.
- docs/adr/0099-the-resilient-tree-is-the-ast-plus-trivia.md:197 — cites `0137 §§ 1-2` as a section list; the rewriter is expected to expand it to `tooling/doc-comment-is-three-slashes` and `tooling/doc-comment-tags-are-see-and-example`, and the sentence ("defines — prose plus two tags") reads correctly only against the second, so check the expansion by eye.
- docs/adr/ground-rules.md:337 — cites `0039 §§ 9-11` as a section list; after expansion it should name `tooling/fmt-is-never-a-diagnostic`, `tooling/fmt-normalizes-only-reserved-spellings` and `tooling/fmt-never-reorders-members`, which is what the sentence ("what `nvs fmt` refuses to rewrite") reaches for.

## B21 — ide

- docs/adr/0040-vscode-deep-tooling-and-resilient-parsing.md:145 — cites "ADR 0099 § 6's latency guard", which will resolve to ide/contributions-are-frozen-and-only-ever-added; the guard is ide/a-full-reanalysis-stays-under-a-bound
- docs/adr/0040-vscode-deep-tooling-and-resilient-parsing.md:294 — same citation of "ADR 0099 § 6's latency guard"; should be ide/a-full-reanalysis-stays-under-a-bound
- docs/adr/0040-vscode-deep-tooling-and-resilient-parsing.md:341 — "Closed by ADR 0099 § 3: exactly two" is about the code actions and will resolve to the request-set rule; should be ide/a-code-action-ships-only-a-fix-a-diagnostic-already-knows
- docs/adr/0101-secret-is-redacted-in-the-editor-and-the-range-comes-from-the-server.md:213 — "the allowlist test ADR 0099 § 6 already runs" will resolve to the frozen-roster rule; should be ide/dependencies-are-allowlisted
- docs/agent/goals/14-lsp-server.md:73 — "ADR 0099 § 3 has the worked example" is the phase-gating example and will resolve to the request-set rule; should be ide/diagnostics-are-phase-gated
- docs/adr/0137-a-doc-comment-is-three-slashes-and-two-tags.md:12 — "0099 § 5's hover row": the hover row is in 0099 § 3's table, not § 5, so the citation resolves to the .lspt rule; should be ide/the-request-set-is-closed (same defect at lines 40 and 196)
- docs/agent/goals/11-doc-comments.md:10 — "ADR 0099 § 5's textDocument/hover row" has the same wrong section number as 0137; should be ide/the-request-set-is-closed (same at line 26 and in 11-doc-comments.toml:34's comment)
- docs/agent/goals/2-concurrency.md:16 — cites bare 0099 for "tokio appears in neither Cargo.toml nor Cargo.lock"; the rule's home is concurrency/one-scheduler, and the claim is stale since hyper 1.11 pulls tokio with `sync` alone (crates/nvs-runtime/tests/manifest_policy.rs:171 says what is actually asserted)
- docs/plan/design.md:245 — same bare-0099 citation for the tokio claim; should cite concurrency/one-scheduler with the narrower claim
- docs/plan/m5.md:10 — same bare-0099 citation for the no-async-runtime claim; should cite concurrency/one-scheduler
- docs/adr/0106-nothing-a-request-sends-terminates-or-wedges-a-worker.md:398 — cites bare 0099 for the narrow runtime dependency surface, which is concurrency/one-scheduler's, not ide/one-grammar-one-tree's
- docs/adr/0099-the-resilient-tree-is-the-ast-plus-trivia.md:611 — the Verification bullet "tokio appears in neither Cargo.toml nor Cargo.lock" is no longer what manifest_policy.rs asserts (tokio is in the lockfile via hyper, `sync` only); record defect, and the same sentence is at line 59
- docs/adr/0099-the-resilient-tree-is-the-ast-plus-trivia.md:208 — the casing fix is cited as `rule:core-api/identifier-casing` glued by a slash to a markdown link on `0030`; the record link is redundant once the token resolves
- docs/adr/0101-secret-is-redacted-in-the-editor-and-the-range-comes-from-the-server.md:107 — cites "ADR 0040's folding ranges" but 0040 names no folding-range request anywhere; after the bare remap the sentence will point at `ide/every-feature-is-staged-behind-its-dependency` and name a thing that rule does not say.
- tools/verify.py:373 — cites the record by path, "`docs/adr/0016-ide-integration.md` § 5", a spelling no `ADR 0016 § 5` remap key matches; should become `rule:ide/editor-clients-live-under-editors` by hand.
- docs/agent/goals/12-resilient-tree.toml:33 — the manifest's `adrs` entry "0040 §3" is there for the AST panel and `nvs ast --json`; after remap it resolves to the catalog rule, and the goal wants `ide/the-ast-panel-shells-out-to-the-cli` (or 0099 §7's rule) instead.
- docs/agent/goals/12-resilient-tree.md:82 — "ADR 0040 § 3 assumed this already existed" is about `nvs ast --json`; same re-point as the toml line above.
- docs/plan/m10.md:60 — "reversing ADR 0016 § 4 for VS Code specifically" will read "reversing rule:ide/the-debug-adapter-does-not-wait-for-an-editor", but that rule already carries the reversal, so the sentence should say the wiring is what that rule commits to, not a reversal of it.
- docs/adr/0099-the-resilient-tree-is-the-ast-plus-trivia.md:560 — "per ADR 0040 § 1's exclusion" refers to a historical exclusion of semantic tokens; `ide/the-first-server-answers-a-closed-list` includes `semanticTokens/full`, so the remapped citation names the opposite of what it did.
- docs/adr/0039-canonical-code-formatting.md:174 — "ADR 0040 § 3's workspace-wide rename" will resolve to the catalog rule; `ide/no-refactoring-introduces-an-alias` is the rule that actually names rename, if the assembler prefers the finer target.

## B22 — php-migration

- docs/adr/0007-explicit-type-system.md:417 — row 16 lists "a `try` whose `finally` returns" as an exit shape, which 0124 § 1 refuses outright; `crates/nvs-types/src/returns.rs:26` and `:172` implement the same escaping-`finally` case and will need the arm removed or turned into the § 1 refusal when goal 13 lands it.
- docs/adr/0007-explicit-type-system.md:420 — "failures in these fifteen classes" but the table above it holds sixteen rows.
- crates/nvs-diagnostics/src/lib.rs:1671 — "ADR 0007 § 7's fifteenth deliberate divergence" will resolve to the framing rule; it should cite `rule:php-migration/an-element-write-needs-storage-to-write-back-into`.
- tests/conformance/lang/a-discarded-expression-statement-still-runs.nvst:2 — cites bare 0007 § 7 for a claim (an expression statement is evaluated and its value discarded) that no row of the divergence table states; after the rewrite it names the framing rule for nothing, and wants a citation of whatever rule owns expression statements or none.
