# Next session prompt

Continue MWL. M1 (front end) is done and closed out: the lexer, the full recursive-descent parser
including the `tainted` grammar ([ADR 0024](docs/adr/0024-taint-tracking-for-injection-sinks.md) § 1),
and this milestone's own verification are all clean — `cargo fuzz run lex`/`run parse` (5 minutes each,
via WSL per CLAUDE.md's "Fuzzing on Windows: use WSL" section) found zero panics, and
`crates/mwl-syntax/tests/corpus_parse.rs` parses the full local `php-src` checkout without crashing.
`cargo test`, `cargo clippy --all-targets -- -D warnings`, and `cargo fmt --check` are all clean.

Since then, [ADR 0025](docs/adr/0025-wasm-browser-target.md) landed — a design-only decision, no code:
an optional future `wasm32` browser target (M14, contingent, same status as M13's FastCGI transport) is a
second codegen backend behind the same IR, not a language change. It doesn't move M2's start line, but
two things below now have a target-shaped edge to keep in mind while implementing them: `require`'s
static-resolution-with-dynamic-fallback (item 4 below) should keep "does this path resolve statically"
structurally separate from "fall back to a dynamic lookup," since the browser target forbids the fallback
outright rather than merely deprioritising it; and whatever shape `mwl-hir`/`mwl-types` gives the six
existing `Core` accessor domains ([ADR 0012](docs/adr/0012-no-superglobals.md)) is the shape a seventh
(`Core\Browser`, undesigned) will need to fit later — don't bake in "exactly six, enumerated" anywhere it
would need unpicking.

Also since then, [ADR 0026](docs/adr/0026-performance-measurement-methodology.md) landed — tooling/process,
no language surface, doesn't touch M2. It settles how MWL's *own* performance gets tracked across
contributor machines and OSes: `benches/abi-probe/tests/perf_guards.rs`'s existing self-relative wall-clock
ratios stay the CI regression guard, unchanged, on every push/every platform. On top of that, a new
historical dashboard records the **aggregate callgrind instruction count** (`Ir`) from a fixed workload run
under `valgrind --tool=callgrind` on a dedicated, non-shared Linux/WSL runner on every merge to `main` —
chosen because a spike (`benches/abi-probe/examples/callgrind_spike.rs`, three runs) proved it gives a
bit-for-bit identical count through Cranelift-JIT-compiled code, which wall-clock never can across
different hardware. `valgrind` is now part of the WSL one-time dev setup in CLAUDE.md, alongside
`cargo-fuzz`. **Not yet built:** the `docs/perf/history.ndjson` writer, the dedicated-runner CI wiring, and
the actual workload roster beyond the one spike benchmark — all deferred per the ADR's *Revisiting* section
to whoever picks up that infra work; it has no milestone number of its own and doesn't block M2/M3.

**M2 — HIR, types, IR — starts now.** Read `CLAUDE.md` first (it routes to the one file you need per
topic), then run `sh .claude/brief.sh` for the live status slice, then read the plan's M2 paragraph in
`docs/implementation-plan.md` in full — it's dense and every clause maps to an ADR you'll need open
while implementing.

Start with **name resolution**, the first of M2's three crates per the plan's Architecture diagram:

1. Create `crates/mwl-hir` (not started yet — M2 is the milestone that creates it, per CLAUDE.md's
   "crates for later milestones are created when their milestone starts" rule). It takes the `mwl-syntax`
   AST and produces namespaces resolved, `use` resolved, and a class/interface/trait/enum graph with
   trait flattening — conflicts resolved by `insteadof` alone, no rename/visibility-change path
   ([ADR 0015](docs/adr/0015-no-name-aliasing.md)).
2. Every callable and constant must resolve as a class member with no bare-name fallback, and a
   declaration reusing the reserved `Core` namespace is a diagnostic at that site
   ([ADR 0011](docs/adr/0011-functions-and-constants-are-class-members.md)).
3. A `type` alias resolves and is substituted away before anything downstream sees it, unless its
   expression is a single bare class/interface/enum atom, which is a diagnostic
   ([ADR 0015](docs/adr/0015-no-name-aliasing.md)).
4. `require` gets statically resolved where possible with a dynamic fallback
   ([ADR 0021](docs/adr/0021-single-file-inclusion-construct.md)).
5. The property-access resolution rule ([ADR 0014](docs/adr/0014-property-observer.md)) — refusing
   access to anything not declared on the class or an ancestor/trait — belongs here too, since it's a
   resolver-level check with no `__get`/`__set` fallback to fall into.

`mwl-types` (the type checker: ADR 0007's table, definite assignment including ADR 0022's constructor
rule, `Comparable` per ADR 0013, tainted propagation/laundering per ADR 0024 §§ 2-3) and `mwl-ir`
(CFG/SSA lowering with safepoints, refcount ops, and the stable per-statement/edge ids ADR 0018's probes
need) come after — build on top of `mwl-hir`'s resolved names rather than starting them in parallel.

M2's *Verify* line (in the plan, right after its paragraph) names the exact corpus this milestone needs
before it can be called done: one file per diagnostic across ADR 0007, ADR 0022, and ADR 0024, plus IR
snapshot tests, plus the `Comparable` refusal cases from ADR 0013. Set that corpus up as you go rather
than retrofitting it at the end.
