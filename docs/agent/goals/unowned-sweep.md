---
milestone: post-parity
---
# Loop goal 31 — the gaps a past milestone left and no goal claimed

Five gaps that goals `core-depth` through `unix-sockets` shipped around, each recorded in the module doc that owns it, each real, and
none of them owned by any goal on the chain. Four of the five are the *same* gap wearing
different clothes: **a member that needs an options bag the registry could not spell**, which is
precisely the spelling [goal `input-shapes`](input-shapes.md) lands. This goal is that follow-through, plus the
two decisions the user took when the unowned list was drawn up.

## Why here

**It sits last of the goals added after `unix-sockets`** because every one of its items waits on
something an earlier goal delivers — goal `input-shapes`'s optional shape field for four of them,
and goal `gap-owners`'s attribution pass for the confidence that these five are the whole list
rather than the five somebody remembered.

## Stage 0 — the catch-up

1. **Four module docs state the blocker as open** and are rewritten when it closes, not amended:
   `crates/nvs-stdlib/src/uri.rs` gap 1, `crates/nvs-stdlib/src/queue.rs` gaps 1–2,
   `crates/nvs-stdlib/src/lib.rs` gap 4, `crates/nvs-runtime/src/lib.rs` gap 4.
2. **`carried-gaps.md` § *Unowned* loses the bullets this goal closes**, per that file's contract: an
   entry leaves exactly one way, which is the gap being closed.

## Stage 1 — the floor

Goal `gap-owners`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded.

## Stage 2 — the keystone: an options bag can tell omitted from written `null`

Goal `input-shapes` gave `Ty::Shape` an optional field, `{name?: T}`, and
`rule:core-api/shape-parameter` gave a `Core`
signature a shape parameter that carries its arms. This stage is what those were for, and
**`rule:core-api/omission-is-not-a-written-null` is the rule it
implements** — read it first. The design is settled; a session takes it rather than re-deriving it.

1. **`Core\Uri::with` gains a removal spelling**, `rule:core-api/a-nullable-field-omits-as-the-never-written-marker` and `rule:core-api/the-bag-abi-is-unchanged` and § 5.
   `crates/nvs-stdlib/src/uri.rs`'s `written` owns the mechanism and the gap names the fix exactly:
   today an omitted option and a written `null` arrive as the same `Tag::Null`, so the option types
   are `string` rather than `?string`. The fix is § 1's pairing — a field admitting `null` omits as
   `Tag::Unset` rather than `Const::Null` — so the two are distinguishable and `port`, `query` and
   `fragment` become nullable: a written `null` removes, an omitted key leaves alone. `path`, `host`
   and `scheme` do **not** join them, and § 5 gives each its own reason. **Not** an `""`-means-remove
   rule: `""` is already an empty query, which `?` with nothing after it produces and which `query()`
   reports as distinct from `null`. Five places restate the invariant this replaces and must read as
   `rule:core-api/omission-is-not-a-written-null` does when the stage closes: `registry.rs`'s `CoreField` and `CoreTy::Union` docs and their
   two tests, `core_lib.rs`'s `shape_fills`, `uri.rs`'s `written`, and `value.rs`'s `Tag::Unset`.
2. **`Core\Uri` gains `queryParameter` and `withQueryParameter`** — `rule:core-classes/uri-removable-components`'s second level.
   `queryParameter(string $name): mixed` and `withQueryParameter(string $name, mixed $value): Uri`,
   both composing `parseQuery`, `buildQuery` and `with` rather than adding a mechanism, so a query
   string gains no second canonicalization. A `null` value removes the pair, which `buildQuery`
   already does; a value may itself be an `array<mixed>`, so the bracket convention needs no second
   spelling; and removing the last parameter leaves **no** query rather than a bare `?`. This is the
   one item in the goal that *adds* a spec row rather than editing one, so note what that costs:
   `crates/nvs-stdlib/tests/spec_registry_coverage.rs` treats a spec row with no registry entry as the
   regression it exists to catch, so § 12's two new rows, the registry entries, the conformance cases
   and the `docs/reference/core/Uri.md` example are **one edit**. `spec-members-outstanding.txt` gains
   nothing — that file's exception is for a member blocked on something unbuilt, and these two need
   only members that already ship.
3. **`Core\Queue`'s `limits` and `grants` are settled** — § 1 sketches each as a `{…}`, and the answer
   is that neither of them is one: an option's type is never a shape, and
   `rule:concurrency/queue-four-members` puts both inside the one trailing bag. What they wait on is
   the isolate that would apply the narrowing rather than a spelling the registry lacks, so the item
   closes by recording that in the module doc and asserting the absence the refusal is read off.
   The gap notes this is the same blocker `Core\Db::open` waits on and that
   "the two lift together"; goal `carried-gaps` owns the `Core\Db` half, so this stage takes the other and the two
   are checked against each other.
4. **`Core\Queue`'s `$args` refuses a `secret`.** § 1 asks for it and `CoreTy::Mixed` carries no
   qualifier, so the refusal needed a spelling rather than a line. A durable row is an output and
   `rule:security/secret-qualifier`'s sinks are the shape of the
   answer: a queued job's arguments are written to a database and read back by another process, which
   is a sink by every test that ADR applies.

## Stage 3 — `array<T>` accepts a covariant read

**The user decided this when the unowned list was drawn up**; it is recorded here rather than
re-argued. `crates/nvs-stdlib/src/lib.rs` gap 4 and `nvs_types::expr::is_assignable`'s own docs argue
both sides, and the widening side wins:

1. **`is_assignable` admits an element-covariant `array<T>`** — an `array<int>` satisfies an
   `array<int|string>` parameter. Today it does not, so an `array<int|string>` parameter takes only
   that exact spelling.
2. **It is sound because an Novis array is a copy-on-write value.** An element-covariant *read* cannot
   be aliased into an unsound write: the callee that widens gets its own copy the moment it writes.
   This sentence is the whole argument and it lives in `is_assignable`'s doc comment, not in three
   places.
3. **It accepts strictly more programs and breaks none**, which is why it needs no migration and no
   diagnostic — nothing that compiles today stops compiling.
4. **The proof is a case that does not compile now and does after**, plus the negative: a write
   through the widened parameter does not affect the caller's array.

## Stage 4 — the two small ones

1. **A custom panic hook**, `rule:errors/helper-abi`:
   the panic message routed to the request log with its request id. `crates/nvs-runtime/src/lib.rs`
   gap 4 says the blocker went away in M5 — the request log exists as `Ctx::write_log_record` under
   `nvs_stdlib::log` — and that `nvs_helper!` already captures the message into `Ctx`, so what is left
   is the hook. **The default hook's stderr output stays the right destination for a CLI script**; this
   is the served case only. It is presentation rather than containment, which is why it waited, and it
   is one item rather than a stage of its own.
2. **`[limits] max_output` bounds a capture.** `crates/nvs-stdlib/src/process.rs` gap 1:
   `rule:core-classes/process-run` reuses that directive rather than
   adding a cap and nothing reads it, so what bounds a child's stdout today is the request's memory
   limit. `Core\IO::read` is the same question with the same answer, and the module doc says the same
   signature closes both — so both are closed here or neither is.

## Stage 5 — the list is shorter, and says so

`carried-gaps.md` § *Unowned* is rewritten to what survives. What is expected to survive is one
entry — `rule:security/arena-is-an-ownership-root`'s optional in-flight cycle
collector — because it is an **open decision rather than an unclosed gap**, and it stays visible for
exactly that reason. Its consequence is visible in a second place and that is not a duplicate: it is
one of the two flags `nvs_safepoint` clears and ignores (`crates/nvs-runtime/src/lib.rs` gap 5), the
other being `DEBUG_BREAK`, which waits on `nvs dap` and is M10's.

## Standing decisions

- **A session on this goal opens no ADR number.** Every item is a folded edit to an ADR whose body
  already states the rule — 0002's *Corollary*, 0033's sinks, 0044 § 1,
  [0147](../../decisions/0147.md) for stage 2 — or
  a widening whose argument lives in a doc comment.
- **`array<T>` widening is decided and is not re-litigated by a session.** The user took it; a session
  that finds the invariant position more comfortable has found a decision, not a question.
- **An options bag distinguishes omitted from written `null`, everywhere, and never by a sentinel** —
  `rule:core-api/omission-is-not-a-written-null`, which owns
  this and is not re-argued by a session. No `""`-means-remove, no magic string, no second parameter
  meaning "and also clear these". Where a bag admits `null` it means *remove* and nothing else, and a
  field is made nullable only where the member has a removal to offer; if it has none, the field stays
  non-nullable and the member waits.
- **The panic hook changes presentation and never containment.** A panic still ends the request the
  way it does today; what changes is where the message is written. Anything that would let a hook
  *recover* is out of scope and stays out.
- **Ambiguity resolves toward closing the gap rather than re-scoping it**, recorded in the module doc.
  These five have each waited a milestone or more; a session that finds a sixth writes it into
  `carried-gaps.md` and moves on, per [loop-authoring.md](../loop-authoring.md) § 8.
- **What this spends**, per `rule:programs/memory-priority`: nothing per request.
  The panic hook holds one message on a path that was already ending; `max_output` *reduces* what a
  capture may hold; the widening is a compile-time judgement.
