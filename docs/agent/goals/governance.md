---
milestone: M6
---
# Loop goal 3 — config, capabilities, limits, and the disk cache

Finish **M6** — [docs/plan/m6.md](../../plan/m6.md) is the scope and this file does not restate it. An
operator can **say what a program may do and how much of it**, and the engine enforces both: a capability
at every syscall-touching entry point, a limit at every safepoint, and a compiled artifact that is
verified before a single page of it becomes executable.

This is goal `governance` of the parity program ([goals/README.md](README.md)), and it is here rather than after
goal `core-part-ii` for one reason: **every capability-bearing `Core` member in goal `core-part-ii` is gated on what this goal
builds.** A member written before its gate exists is a member whose gate gets retrofitted, and a
retrofitted gate is exactly the kind that has a hole in it. Goal `concurrency`'s isolates already run under
compiled-in defaults and say so at each site; this goal is where those sites get their real answer.

## What "done" means here

Two of the three halves are checkable the ordinary way — a config file that must be refused is a fixture,
and a cache artifact that must be rejected is a test. The third is not, and it is the one that matters:
**"every syscall-touching entry point is gated" is a claim about a set, not about a case.** So it is
checked the way M4 checked its refusal sites — a test that reads `nvs-stdlib`'s own registry and fails
naming any member that touches the filesystem, the network, the clock-with-side-effects or a process and
carries no capability. That test is Stage 6's, it is this goal's real acceptance, and its allowlist may
never grow.

## Stage 0 — the catch-up

Nothing. Goal `concurrency` landed under compiled-in defaults deliberately, and picking those up is Stage 4's item 12
rather than a catch-up: it is the *work*, not a debt.

## Stage 1 — the floor

M4's, goal `core-depth`'s and goal `concurrency`'s whole acceptance lists, inserted mechanically by `goal-switch.py`, **never
traded.**

## Stage 2 — the registry and the tree

The one file set the next four items share: a directive's declaration, and how a file becomes one.

1. **The directive registry, with three fields per directive.** The changeability class `rule:config/three-changeability-classes` already
   defines, plus `rule:config/reloadability-is-its-own-field`'s **`Reload`/`Boot`
   field, orthogonal to it** — and orthogonal is the item: reloadability is now the *only* thing that
   makes a directive boot-only, and conflating the two is what that ADR exists to stop.
2. **`nvs.toml` parses, and a duplicate or unknown key is refused.**
   `rule:config/the-file-is-nvs-toml-and-it-is-toml` and `rule:config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one`. TOML via `serde`. § 2a is the block
   list and names the ADR that argues each block's directives — including the four this milestone adds,
   `[deferred]`, `[[schedule]]`, `[http.*]`, `[metrics]` and `[trace]`.
3. **The configuration is a tree.** `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults` is the
   only copy of the resolution order and the merge rules, and every one of them is a case: a root named by
   repeatable `--config` else `./nvs.toml` else the shipped defaults (§ 1); `[[include]]` by `path` and by
   `dir` (§ 2); one ordered stream where later wins (§ 3); a value array **replaces** where a `[[table]]`
   **appends** (§ 4); a relative path resolves against the file it is written in (§ 5).
4. **Ownership is the trust boundary.** § 6: any file in the tree that another account can write refuses
   the boot. That is what makes an `optional` include safe and what makes every file in the tree equally
   trusted — and `optional` covers *absence*, never unreadability, which is the distinction a naive
   implementation loses.
5. **`password_file` yields the file's content with one trailing newline stripped** (§ 7), the CLI flag
   list is closed at the global layer (§ 8), and `nvs config check`/`nvs config dump` exist (§ 9).
   `nvs ctl config` waits for goal `server`'s socket.
6. **`[[app]]`, keyed on a canonicalized entry-file path.**
   `rule:config/an-application-is-its-entry-file-path`: every matching block applies,
   least-specific first (§ 2); a block may widen, bounded by the global ceiling (§ 3). An entry path
   reaching an `[[app]]` root through `..` or a symlink **does not match it**, which is the same
   canonicalise-then-compare rule item 10 needs and is written once.

## Stage 3 — the snapshot

7. **The registry becomes an immutable `Arc<Config>` a request clones at start and reads for its whole
   life.** `rule:config/the-config-is-an-immutable-snapshot`. A request that started
   before a swap reads the old value to completion; one started after reads the new. A malformed file
   leaves the previous snapshot serving and **names the offending line**.
8. **`Core\Config::set` is `ini_set`'s replacement, and its three outcomes are one rule.**
   `rule:config/ini-set-is-core-config-set` and m6.md's *Verify*: above the `[limits]`
   default succeeds and takes effect; above the `[limits.hard]` ceiling returns `false` with the previous
   value intact; and either way it is invisible to the next request on the same core. The third clause is
   the one an implementation on a shared mutable registry gets wrong.
9. **`env_hash` lands, carried by both compiled-unit cache keys** — § 4. It is what stops an artifact
   compiled against one extension set from ever being reused against another, and it is cheap now and a
   cache-invalidation pass later.

## Stage 4 — capabilities and limits

10. **Capability enforcement at every syscall-touching stdlib entry point.** The mechanism is this stage's
    ADR slot (§ *Standing decisions*); the *rule* is that a member either declares the capability it needs
    or is proven not to need one, and Stage 6's test is what proves the set is closed. Path-bearing
    capabilities resolve **canonicalise-then-prefix**, so a path reaching a granted root through `..` or a
    symlink does not match — item 6 wrote that comparison once.
11. **Safepoint-driven limit enforcement.** Memory and CPU caps terminate a runaway script as a `FATAL`,
    reported to `Core\Fatal::onLimit` if registered and **never to an ordinary `catch`** —
    `rule:errors/escalation-ladder`. Safepoints have been emitted since the first
    backend commit and goal `concurrency`'s cancellation is their first consumer; this is the second.
12. **The isolate's governance, which is goal `concurrency`'s deferred half.** `script.spawn` with
    canonicalise-then-prefix path resolution, `max_script_depth`, per-tree accounting of every `[limits]`
    value, spawn-site sub-caps, and derivation of a child's overlay from its parent's *effective* config.
    Two failures have their own names and both are easy to report as something else: a recursive spawn is
    stopped by `max_script_depth` and reported **as that** rather than as an out-of-memory, and N
    concurrent isolates cannot *together* exceed the tree's budget.
13. **`fatal_reserve_memory`/`fatal_reserve_time` and `Core\Fatal::onLimit` registration.** ADR 0020: the
    reserved slice a resource-limit `FATAL`'s handler runs with is carved out of the request's own budget
    **at the same point these limits are set up**, which is why it is this item and not goal `core-part-ii`'s.

## Stage 5 — the artifact cache

14. **`rule:packaging/an-artifact-is-one-immutable-content-addressed-file`, exactly as specified.** That ADR is a
    finished design, not a starting point: § 1's fan-out directory of immutable content-addressed files,
    § 2's file shape, § 3's **verify fully before a single page becomes executable**, § 4's one atomic
    rename and **no lock file, ever**, § 6's piggybacked probabilistic eviction off the request path, and
    § 7's `System`-class directives. A world-writable cache directory is refused.
15. **A tampered artifact is rejected**, and § 5 is the one home for what the checksum defends against and
    what it explicitly does not. Do not widen that claim in a doc comment.

## Stage 6 — the closure test, and the boot-time validations

16. **`every_capability_bearing_member_declares_its_capability`.** The set claim, checked over
    `nvs-stdlib`'s own registry. **Its allowlist may never grow**; every entry is a bullet in
    § *Standing decisions* with its reason, and adding one to make a run go green is the single move this
    goal forbids outright. `crates/nvs-stdlib/src/registry.rs:490` is what a member's row may say and is
    where the declaration goes.
17. **The four new blocks refuse a bad boot, each per its own ADR's *Verification*.**
    `rule:config/scheduled-work-is-a-config-block`: a `[[schedule]]` entry with no `scope`, a
    malformed `cron`, a `script` outside `script.spawn`'s roots, or `scope = "fleet"` with no shared
    store. `rule:http-server/an-unsafe-or-unbounded-default-is-a-defect`: `origins = ["*"]` with
    `credentials = true`, and `same_site = "None"` with `secure = false` — refused at boot **and by
    `Core\Config::set` alike**, which is the clause that needs one implementation rather than two.
18. **An adversarial suite.** m6.md's *Verify* is the list: a script attempting to widen a capability or
    set a `System` directive fails; `spawn script` without `script.spawn` fails; a path outside the
    granted roots fails including one reaching it through `..` or a symlink; a child cannot widen a
    capability its parent narrowed.

## Stage 7 — the bundler

19. **`nvs build --compile`.** `rule:packaging/nvs-build-compile-appends-the-program-to-a-copy-of-the-host` is the only
    copy of the scope, the source-not-precompiled-artifacts trade, and why bundling a web-serving
    deployment is explicitly out of scope. It appends an entry file's statically-resolved `require` graph
    to the host `nvs` binary as **plain source**, read back through Stage 5's cache with no new mechanism —
    which is why it is this goal's last stage rather than its own goal.
20. **A bundled executable runs identically to `nvs run` against the same source, on all three platforms.**
    `rule:packaging/nvs-build-compile-appends-the-program-to-a-copy-of-the-host`'s own verification list.

## The harness this goal owes

Two acceptance checks name a tool flag that does not exist yet, and writing it is part of the item
rather than a follow-up to it. Neither is a new tool:

- **`python tools/bench.py --warm-start --max-work-ms 6`** — m6.md's *Verify* names "warm-cache CLI
  startup under 10 ms" and nothing measures it. Item 14's own number, and it belongs beside the cache it
  measures. The budget is on the total less `nvs --version`, because most of the total is the OS creating
  a process rather than anything Novis does; `bench.py`'s `warm_start` owns that. The total is still
  printed, and m6.md's figure stays the criterion the cache has to meet once it has a caller.
- **`python tools/try.py --bundle <file> --expect <line>`** — item 20's "runs identically to `nvs run`",
  which is a comparison rather than an assertion about one output.

## Acceptance

**This goal is retired: its checks are the floor stage of the live goal**, carried
there by the switch that left it and folded forward at every switch since.

## Standing decisions — pre-authorized, do not stop the loop for these

- **Decide and record; never `BLOCKED` for a design call.**
- **One ADR slot: the capability enforcement points** (Stage 4, item 10), and it is the first slice of
  that stage. What a capability *is* at the point of a call, where the check sits so that no member can
  route around it, what it costs on a hot path, and how the closure test in item 16 knows a member needs
  one. ADRs 0051 and 0024 name capabilities constantly and none of them says where the check is; that gap
  is why this slot exists. Anything else is decided-and-recorded.
- **A path comparison is canonicalise-then-prefix, in one implementation.** Items 6, 10 and 12 all need
  it. Writing it three times is how one of them ends up accepting a symlink.
- **`Core\Config::set` above the hard ceiling returns `false`; it does not throw.** m6.md's *Verify* says
  so and an implementation that throws is a different API.
- **A limit breach is a `FATAL` and never reaches a `catch`.** `rule:errors/escalation-ladder` decided it. A fixture that wants
  to catch one has found the rule, not a bug.
- **The artifact cache is `rule:packaging/an-artifact-is-one-immutable-content-addressed-file` as written.** If the implementation forces a different shape, record
  *that* in the crate's module doc with the reason and put the redesign in `## Backlog` — do not start one
  mid-run.
- **No socket.** `nvs ctl` needs a long-running server and arrives in goal `server`. `nvs config check` and
  `nvs config dump` are this goal's and read the tree directly.
- **Picking every dependency but the two the user named** stays pre-authorized under `rule:packaging/a-c-dependency-answers-two-questions`.

## What this goal does not touch

Every capability-bearing `Core` **member** — that is goal `core-part-ii`, and this goal builds the gate rather than
the thing behind it. The listener, the control socket and `[http.*]`'s *runtime* behaviour (goal `server`; only
its boot-time validation is here). `rule:config/an-edit-reaches-the-next-request-without-a-restart`'s freeing of executable memory, which m6.md carries and which
has no consumer until there is a long-running process to free it in — it goes in `## Backlog` if a
session reaches it.
