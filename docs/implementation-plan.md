# Novis — The Web-Native Programming Language: Implementation Plan

<!-- This block has a fixed field set: Status, Done, On disk, Toolchain, ADR slices landed, Open now,
     Blocking. Overwrite a field in place; never add a paragraph or a new field name. That is what
     keeps it bounded as milestones accumulate. Aim for ~400 bytes a field — guidance, and nothing
     trims to it. What IS checked is growth: `python tools/session.py --wrap` refuses an edit that
     leaves a field over 5x that and bigger than it was, so past the ceiling a sentence is replaced
     and never added; a shrink is always taken. This comment is the one home of both numbers.
     History lives in `git log`, per-crate gaps in each crate's module doc — see AGENTS.md § *Keep
     each slice small, commit every one of them*. -->

> **Status:** 2026-08-28. **M4's loop goal is reached** — every check in its acceptance list passes, so
> the language surface is closed and nothing a CLI program reaches for panics below the front end. The
> next target is **the parity program**, orders 1–5 of the milestone table: PHP core feature parity, all
> five SQL drivers, concurrency, governance and the server, run as the six-goal chain in
> [docs/agent/goals/](agent/goals/README.md). Dependencies: `regex` + `fancy-regex` and `jiff` are named
> by the user; the rest the loop picks under ADR 0051 § 4.
>
> **Done:** M0 (setup) and M1 (front end) whole, M2 (HIR, types, IR) and M3 (baseline Cranelift
> backend) whole, **M4 (language completeness) to its loop goal's acceptance list** — its own 1000-case
> corpus figure is the one thing left and it is met through orders 1–4. M4S Part I registered —
> `crates/nvs-stdlib/tests/spec-members-outstanding.txt` holds no keys, which is this project's
> definition of *registered*. M1's own section lists the one grammar addition still owed — the pipeline
> operator, ADR 0098 — which blocks nothing and is scheduled after the current loop goal. Each milestone
> file under [docs/plan/](plan/) states its own acceptance.
>
> **On disk:** the workspace and its CI (three platforms, with miri, asan and fuzz legs), and the
> nine crates — `nvs-diagnostics`, `nvs-syntax`, `nvs-hir`, `nvs-types`, `nvs-ir`, `nvs-runtime`,
> `nvs-stdlib`, `nvs-codegen`, `nvs-cli` — plus `nvs-test`, `fuzz/`, `tools/`, `benches/abi-probe`,
> and the two case trees `tests/conformance` and `tests/differential`. **Each crate's own module doc
> is the authority on what it holds and what it still owes**; `python tools/brief.py` prints one map
> line each, and `python tools/disk.py` the live counts.
>
> **Toolchain:** Rust 1.97.1 stable (pinned), Cranelift 0.135.0, wasmtime 48, MSVC 14.44 + Windows
> SDK 10.0.26100 for linking, PHP 8.5.9 as the differential oracle — on the Windows `PATH` and
> inside the WSL distro alike, at the same version — `cargo-fuzz` 0.13.2 and `valgrind` under a WSL
> nightly toolchain (docs/setup.md is what a machine installs, and why).
>
> **ADR slices landed:** **each ADR's own *Verification* section is the authority on what its slice
> covers, and this field never restates one** — `python tools/brief.py --where <keyword>` routes to
> the ADR that owns a topic, and `python tools/adr.py --stats` shapes the whole set. What a crate
> still owes is its own module doc's `# Known gaps`. What landed in which session is in `git log`.
>
> **Open now:** **Goal 5 of the parity program — `Core\Db`, M8 — is running.** **Stage 0 has
> landed**: ADR 0071's derive pass, and § 9's type map refusing a field no column reads back as
> (`E0756`). **`crates/nvs-db` exists**: § 5's `Connection` enum, § 4's busy field and the
> `NVS_DB_MATRIX_*` reader, over an `NvsTls` generic in its transport. Its wire I/O is PostgreSQL's:
> § 3's `SSLRequest` upgrade, SCRAM-SHA-256, § 4's extended-query state machine and § 13's reset.  §
> 9's whole type map decodes over it, `PgScalar` carrying the five rows only `nvs-stdlib` can build
> into a `Core\Time` or `Core\Uuid`; § 4's count, `lastId` and `executeMany` too. § 7's
> `BEGIN`/`COMMIT`/`ROLLBACK` is on it, a nested one a `SAVEPOINT`, with § 8's kind inside every
> refusal. **`Core\Db` has opened in `nvs-stdlib`**: § 18's `connect` over a request-held
> connection, `inList`, `quoteIdentifier` and § 4's `query`, `execute` and `executeMany` — bound in
> § 5's rewriter's order — and § 18's readers: five of `Rows`' six, all fourteen of `Row`, `Write`'s
> three, with `columns()` owed on a `ColumnType` enum. `open` waits on a shape-parameter type. **§
> 7's `transaction` is on both classes** and `Core\Db\Isolation` is registered; `nvs.toml` now opens
> `[db.main]` and grants `db.connect` to the two fixtures, which verify the compose CA through the
> new `tls_ca_file` and stop inside the protocol. The pool is Stages 3 to 7, and **ADR 0132 has
> claimed the goal's one ADR slot**: a driver is a sans-IO codec plus a state machine over the
> parking stream, the five an enum not a trait. **Goals 1-4 and M4 are closed and are this goal's
> Stage 1 floor** — `Core`'s capability-bearing half in full, with `ring` the one C dependency ADR
> 0051 § 4 records. Conformance 1408, differential 250, its gate met, migration 90%/74%.
>
> **Blocking:** Nothing waiting on a decision — every design call orders 1–5 reach is pre-authorized
> in the goal's own § *Standing decisions*, and each goal names the numbered ADRs it may open and no
> others. One external dependency, two goals wide: **goals 4 and 5 need a reachable Docker daemon**,
> and on this machine it is up — `examples/transaction.nvs` got an answer from
> `tests/db/compose.yaml`'s PostgreSQL. That wall is down: a `[db.<name>] tls_ca_file` names a PEM
> bundle, `nvs_config::db` resolves and trust-checks it at boot, and the handshake verifies against
> it alone. What stops `examples/transaction.nvs` now is inside the extended-query protocol. Picking
> every dependency but the two the user named is pre-authorized under ADR 0051 § 4.

**How the plan relates to the ADRs.** The plan is the record of *what* gets built, in what order, and how
each milestone is verified. It states decisions but does not argue them. The reasoning lives in
[docs/adr/](adr/README.md), and where a decision has its own ADR the plan links to it instead of restating
it — follow the link rather than expecting the argument here. For decisions with no ADR of their own, the
*why* is in [adr/README.md](adr/README.md) under *Decisions taken at project start* and the *mechanics*
are [docs/plan/design.md](plan/design.md) § *Architecture*.

## Milestones

Each milestone ends with something runnable and its own tests. Do not start the next until the current
one's verification passes.

**A milestone's number is its identity, not its position.** The **Order** column is the schedule and it is
the only thing that says what comes next; M15 has carried that distinction since it was written, and as of
2026-08-28 it applies to the whole table. Nothing is renumbered when the order changes, because a number
that moves invalidates ~650 cross-references in `docs/adr/` and every one of them is a link somebody has
already followed.

| Order | Milestone | What it builds | Loop-days |
|---|---|---|---|
| done | [M0](plan/m0.md) | Project setup (~3 days) | 0.3 |
| done | [M1](plan/m1.md) | Front end (~3 weeks) | 0.7 |
| done | [M2](plan/m2.md) | HIR, types, IR (~4 weeks) | 1.5 |
| done | [M3](plan/m3.md) | Baseline Cranelift backend → **Hello World** (~3 weeks) | 0.5 |
| done\* | [M4](plan/m4.md) | Language completeness — a usable CLI language (~10 weeks) | ~3 |
| **1** | [M4S](plan/m4s.md) | The `Core` API contract and its pure half (~5 weeks) | ~1.5 |
| **2** | [M5](plan/m5.md) | Concurrency and script isolates (~5 weeks) | ~3.5 |
| **3** | [M6](plan/m6.md) | Config, limits, capabilities, disk cache (~3 weeks) | ~1 |
| **4** | [M8](plan/m8.md) | Stdlib and databases (~16 weeks) | ~6.5 |
| **5** | [M7](plan/m7.md) | Built-in HTTP server (~4 weeks) | ~2 |
| **6** | [M4B](plan/m4b.md) | Minimal `nvs-lsp`, syntax highlighting and the VS Code extension (~3 weeks) | ~1.5 |
| **7** | [M9](plan/m9.md) | Extension system, and the `nvs:ext@1.0.0` world it freezes (~6 weeks) | ~2.5 |
| **8** | [M10](plan/m10.md) | Developer tooling and IDE integration (~14 weeks; scope shifted by ADR 0040, net change undetermined) | ~8 |
| **9** | [M11](plan/m11.md) | PHP transpiler (~10 weeks) | ~3 |
| **10** | [M15](plan/m15.md) | Packages, the registry and the supply chain (~8 weeks) | ~3 + a calendar floor |
| **11** | [M16](plan/m16.md) | `nvs/web`, `nvs new`, and the framework (~12 weeks) | ~4 |
| ongoing | [M12](plan/m12.md) | Optimising JIT tier (ongoing) | measurement-bound |

\* **M4 reached its loop goal on 2026-08-28** — every check in that goal's acceptance list passes, which is
what closes the language holes. What it has not reached is its own milestone acceptance's **1000 `.nvst`
cases**; that count was deliberately left as a corpus figure to be met as the suite grows through orders
1–4, and [m4.md](plan/m4.md) still carries it unchanged.

**Orders 1–5 are one program, not five independent milestones: PHP core feature parity.** Everything a
program written in PHP reaches for without loading an extension, plus every planned SQL driver, plus the
concurrency, governance and server the capability-bearing half of `Core` cannot exist without. It is
scheduled as one continuous unattended run — see *The parity program* below, and
[docs/agent/goals/README.md](agent/goals/README.md) for the six loop goals it is cut into. PHP's optional
extensions (`gd`, `intl`, `imap`, and the rest of the list in
[02-php-migration.md](spec/02-php-migration.md)) are explicitly not part of it and stay with M9.

Each row is a file under [docs/plan/](plan/). `python tools/plan.py --show M8` prints one
without you needing to know that, and `--show M8:verify` prints only its acceptance paragraph.
The decisions those milestones sit inside, the architecture and the verification strategy are
[docs/plan/design.md](plan/design.md).

## The parity program

Orders 1–5, in that order, are the run that takes Novis from "a usable CLI language" to "everything PHP
does out of the box, and the four databases it does it against". The order inside the program is a
dependency chain rather than a preference: `Core`'s pure half is what everything else is written against;
the reactor is what a socket, a driver and a listener all need; capabilities are what every
capability-bearing member is gated on; the capability-bearing half of `Core` and the databases sit on both;
and the server sits on all of them.

| Loop goal | Milestone | Lands |
|---|---|---|
| [1 core-depth](agent/goals/1-core-depth.md) | M4S tail | `Core` §§ 1–13 depth, `autoload`, the compile-time attribute passes, OpenAPI |
| [2 concurrency](agent/goals/2-concurrency.md) | M5 | the reactor and its parking streams, the scheduler, `spawn`/`await`, `Core\Task`, isolates, `Core\Serialize` |
| [3 governance](agent/goals/3-governance.md) | M6 | the config tree, capability enforcement, limits, the artifact cache, `nvs build --compile` |
| [4 core-part-ii](agent/goals/4-core-part-ii.md) | M8, non-database | `Core\IO`, crypto, `Process`, `Cli`, `Cache`, `RateLimit`, `Log`, `Http\Client`, `Reflect` |
| [5 database](agent/goals/5-database.md) | M8, database | `Core\Db`, five drivers, the pool, the type map, `Core\Queue` |
| [6 server](agent/goals/6-server.md) | M7 | `nvs serve`, the request-facing `Core` classes, mounts, uploads, `Core\Session`, the control socket |

**The program's own stop condition is `python tools/check-migration.py` reporting 100% classified** —
every one of the oracle build's **1151 functions and 253 types** accounted for as a `member`, `language`
or `dropped` row, every `member` row's member registered, and every one of them carrying a conformance
case. It was 25% when the program was scheduled. A count of conformance cases is a proxy for parity; a
table that enumerates the source of truth is not.

**The oracle build gained `mysqli`, `pgsql` and `sqlite3` on 2026-08-29**, which is why that inventory is
1151 rather than the 925 the program was first sized against. It is a better program for it: 236 of the
new names are the three APIs [ADR 0067](adr/0067-core-db.md) exists to replace, so *"one API replaces
`PDO`, `mysqli`, `pgsql` and `sqlite3`"* stops being an assertion about four APIs and becomes an audit of
236 functions, each with a row saying what became of it. The floors in each goal were re-derived against
the new denominator in the same commit.

**The two columns are not the same unit.** The parenthesised weeks are the original estimate, written for
a human team before any code existed; **Loop-days** is what this project's unattended loop actually spends,
elapsed and continuous — M0–M3 are measured, the rest projected. The measured conversion is ~23×, it is
not applied uniformly, and it carries a rework tax and four ways it breaks:
[docs/plan/velocity.md](plan/velocity.md) is the one home for all of that. Summed, the milestones left
come to **~7 weeks** against the ~99 the original column still shows. Neither figure gates anything.
