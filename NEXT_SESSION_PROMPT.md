# Next session prompt

Continue MWL. M1 (front end) is done. M2 (HIR/types/IR) is in progress — run `python .claude/brief.py` first,
then read `docs/implementation-plan.md`'s M2 paragraph for exactly what landed and how (this file only
points at what's next; the plan is the one home for status detail, per CLAUDE.md's "state a fact once").

**Last session closed ADR 0038 (`lateinit`) end to end for M2's scope**, mirroring ADR 0022 § 2's own
landing shape:

- **Grammar** (`mwl-syntax`): a new `Keyword::Lateinit` and `Modifier::Lateinit`, parsed in
  `parse_modifiers` alongside `readonly`. Lexer/parser round-trip tests added.
- **Diagnostics** (`mwl-diagnostics`): five new `E04xx` codes, `E0424`-`E0428` —
  `E_LATEINIT_NOT_OBJECT_TYPE`, `E_LATEINIT_NULLABLE`, `E_LATEINIT_PROMOTED_PARAM`,
  `E_LATEINIT_READONLY_CONFLICT`, `E_LATEINIT_READ_BEFORE_WRITE_LOCAL`.
- **§ 1 placement checks** (`mwl-types::signatures`): `collect_members`'s `Property` arm validates a
  `lateinit` property's type (refusing a scalar/enum, refusing `?T`) and its `readonly` combination; the
  `Method` arm refuses `lateinit` on any parameter (promoted or not). All four diagnosed at
  signature-collection time, the same point `E_ARRAY_TYPE_TOO_DEEP` already fires from.
- **§ 2's exemption**: a `lateinit` property is excluded from `ClassSignature::required_properties`, so
  `ctor_init.rs`'s existing ADR 0022 § 2 pass never flags it — whether or not the class has a constructor.
- **§ 3's intraprocedural check** (`mwl-types::lateinit`, new module): a sibling flow-analysis pass to
  `ctor_init.rs`, run over *every* method a class declares (not only its constructor). Tracks each of the
  class's own `lateinit` properties (`signatures::own_lateinit_properties`, trait-flattened like
  `own_required_properties`) as "written" or not, joining `if`/`else`/`switch`/`try` branches by
  intersection exactly like `ctor_init::InitState` does. A `$this->prop` read with no proven write on some
  path is `E_LATEINIT_READ_BEFORE_WRITE_LOCAL`; any call (method/static/free-function) conservatively marks
  every tracked property written, per the ADR's explicit "never false positive" mandate. Wired into
  `check.rs` right after `check_class_init`.
- **Known gap, matching the ADR's own scoping**: only a class's *own* (+ trait-flattened) `lateinit`
  properties are tracked by the § 3 pass — one declared on a parent class and read via `$this` in a
  *subclass* method relies entirely on the § 2 runtime throw (M4, not yet built). See `lateinit.rs`'s
  module docs for the full list.
- Also fixed, in passing: a stale known-gap note in `mwl-types/src/lib.rs` claiming the `parent` type atom
  (`parent $x`) was unresolved — it has been handled by `lower::resolve_parent` since an earlier session;
  only the module-doc summary hadn't caught up.

`cargo build`/`test`/`clippy --all-targets -- -D warnings`/`fmt --check` all clean (154 tests in
`mwl-types`, up from 148). Landed in three commits: grammar+diagnostics, § 1/§ 2 checker work,
§ 3's new module.

**ADR 0038 is now fully done for everything M2 can verify** — its *Verification* section's M2 bullet
(refusing the three rejected shapes, exempting `lateinit` from ADR 0022 § 2, and the § 3 call-free
read-before-write check) is satisfied. Its M4 half (the actual runtime throw) has no backend to attach to
yet, same residual as ADR 0022 § 3's own case.

**With ADR 0038 closed, M2's name-resolution and type-checking work is essentially done** — every
checker-side ADR M2 names (0007, 0010, 0013, 0014, 0015, 0021, 0022, 0024 §§ 2-3, 0027, 0028, 0029/0030/0032,
0033 §§ 2-4 M2-reachable entries, 0036, 0037, 0038) is implemented and tested. **The one item left in M2
is the milestone's own last sentence: lowering to a CFG/SSA IR** — a new `mwl-ir` crate (doesn't exist yet
— confirm with `find crates -maxdepth 1 -iname 'mwl-ir*'`), carrying explicit safepoints, refcount
operations and runtime-helper calls, with every lowered statement and conditional CFG edge carrying the
stable id [ADR 0018](docs/adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md) needs for
its coverage/branch probes — cheap to reserve now, expensive to retrofit once M3 builds on top of the IR
without it.

This is a milestone-sized task on its own — plan the session's actual scope down to a first slice rather
than attempting the whole thing in one sitting:

1. Read [ADR 0018](docs/adr/0018-coverage-tracing-and-profiling-as-safepoint-shaped-probes.md) first for
   the stable-id shape every lowered statement/edge needs to carry, since that's "cheap now, expensive
   later" per the milestone text above — get the id scheme right before lowering anything.
2. Stand up the `mwl-ir` crate (workspace member, following the same bring-up shape `mwl-hir`/`mwl-types`
   used when their milestones started — lint/deny/fmt policy inherited from the workspace `Cargo.toml`).
3. Design the IR's value/instruction representation (CFG of basic blocks, SSA form) — this is a real design
   decision with memory/complexity tradeoffs (SSA construction cost/complexity vs. a simpler
   non-SSA CFG deferred to codegen); per CLAUDE.md's last "Ground rules" bullet, if this looks like a
   contested tradeoff rather than a mechanical follow-through of what the plan already committed to, stop
   and ask rather than picking silently.
4. Lower a first, narrow slice (e.g. a single method with only straight-line arithmetic/`return`) end to
   end with a snapshot test, before widening to the rest of the checked AST's shapes (control flow, calls,
   `new`, etc.).

Also still open from before (independent, low priority, pick up only if there's time left over after the
IR work above): a `set`-hooked property is exempted from ADR 0022's constructor check entirely rather than
verified against the hook's body; the identical question now also applies to whether a `lateinit` +
hooked property should discharge on the hook's first commit (ADR 0038's own *Revisiting* names this,
deferred to `docs/spec/`).
