# Carried gaps — what a shipped feature still owes, and who owns it now

[carried-refusals.md](carried-refusals.md) does this for one kind of hole: a `nvs-ir` refusal site,
which `python tools/holes.py` can find on its own because the site is in the source. **This file is
for the other kind** — a gap that is real, written down in the module doc that owns it, and invisible
to every tool, because nothing in the tree is shaped wrong. A `Core\Log` record with two keys where
[ADR 0076](../adr/0076-observability-export.md) § 6 names six compiles, tests green and ships.

**It exists because the handoff cannot hold one.** `docs/agent/handoff.md` is *state*: `tools/loop.py`
overwrites it with the next goal's seed at every switch, and `tools/goal-switch.py` carries the
outgoing goal's `[[check]]` blocks forward and nothing else. So a `## Backlog` bullet lives exactly
until the goal that wrote it goes green — which is how ADR 0042's cache redesign came to be "in the
handoff's backlog" according to a module doc, and in no file at all according to the repository, and
how `§18 stream` came to be filed under "goal 5's" six goals after goal 5 closed. Same failure,
same fix, one file up.

## The contract

- **An entry names an owner.** A goal number that is a live `[[goal]]` in
  [goals/chain.toml](goals/chain.toml), or the word **`unowned`** with the reason it is nobody's yet.
  `unowned` is a legitimate state — it is a scheduling question for the user — but it is never the
  *absence* of an answer.
- **An entry leaves exactly one way: the gap is closed.** Not when it is rewritten, not when it stops
  being convenient. An entry whose owner went green without closing it is the failure this file
  exists to make visible; strike the owner, not the entry.
- **One line of *what*, and a pointer to the module doc that owns the detail.** Every fact in this
  repository has one home, and for a gap that home is the module. This file is an index of who, not a
  second copy of what.
- **A session that finds a gap off its path writes it here**, not in the handoff, and moves on —
  [loop-authoring.md](loop-authoring.md) § 8's rule with a durable destination.

## Owned

Each of these is claimed by an entry on the chain and will be struck when that entry goes green.

| Gap | Owner | Where the detail lives |
|---|---|---|
| `db.open`'s `"*.tenants.internal"` grant matches no host — the wildcard has no reader | 21 | `crates/nvs-config/src/capability.rs` § *Known gaps*, ADR 0067 § 3 |
| `nvs check` builds no grants, so ADR 0067 § 10's host diagnostic fires for nobody | 21 | `crates/nvs-types/src/intrinsics.rs` gap 6 |
| A cycle whose only closing edge is inside an `array<T>` survives `object::sweep` | 21 | `crates/nvs-runtime/src/object.rs` § *The five walks*, ADR 0116 § 2 |
| `Core\Db::stream`/`streamAs`, `Connection::close`, § 18's three readonly properties | 21 | `crates/nvs-stdlib/src/db/mod.rs` gap 5 |
| `[db.<name>.pool]` has no spelling, and `pool = false` cannot reach a program-opened connection | 21 | `crates/nvs-stdlib/src/db/mod.rs` gap 1 |
| `{timeout?: Duration}` is in both spec signatures and in neither registry row | 21 | `crates/nvs-stdlib/src/db/mod.rs` gap 6 |
| A `Core\Log` record carries `level` and `msg` and none of § 6's other four | 21 | `crates/nvs-stdlib/src/log.rs` § *The envelope* |
| `scope = "fleet"` parses, boots and is not armed | 21 | `crates/nvs-server/src/schedule.rs` § *What is not armed*, ADR 0073 § 3 |
| ADR 0133 § 3's computed `$reason` is not refused — was blocked on a full diagnostic band | 21 | `crates/nvs-stdlib/src/html.rs` § *Known gaps* |
| `queryAs<T>`'s three refusals are at run time — same blocker, same band | 21 | `crates/nvs-stdlib/src/db/mod.rs` gap 8 |
| Twenty CLDR plural rosters throw; ordinals absent; eight pattern letters refused | 21 | `crates/nvs-stdlib/src/cldr.rs` gaps 2–4 |
| `decodeComponent`/`parseQuery` throw on octets that are not UTF-8 | 21 | `crates/nvs-stdlib/src/uri.rs` gap 2 |
| A route capture reaches the handler still percent-encoded | 21 | `crates/nvs-runtime/src/routes.rs` gap 3 |
| ADR 0042's artifact cache is written, tested and has no caller | 22 | `crates/nvs-cli/src/cache.rs` § *Known gaps* |
| `Core\Request::clientIp`/`host`/`scheme`, `Response::html`/`sendFile` | 17 | `crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt` |
| `goto` labels, grouped `use`, `var` as a property declarator, an enum case named with a keyword | 13 | `crates/nvs-syntax/src/lib.rs` § *Known gaps* — M1's own *Verify* is a `php-src` corpus parse |

## Unowned

Nobody's, and each is a scheduling question rather than a session's.

- **Spec §§ 16–17's eight `Core` classes — `Core\Metrics`, `Core\Net`, `Core\Os`, `Core\Signal`,
  `Core\Compress`, `Core\Mime`, `Core\Xml`, `Core\Zip`.** Listed in
  `crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt`, named by no `[[goal]]` on a
  23-entry chain, and the largest unowned item in the repository. `Core\Xml` is not free-standing:
  `Core\Html::sanitize` and [ADR 0122](../adr/0122-html-parsing-is-a-whatwg-entry-on-core-html-over-core-xmls-tree.md)'s
  WHATWG parser both wait on its tree, so two more gaps sit behind it. This is a milestone's worth of
  work and wants its own entries or an M8 amendment, which is a decision for the user and not for a
  session.
- **[ADR 0116](../adr/0116-an-isolates-arena-is-an-ownership-root.md)'s optional in-flight cycle
  collector**, for a long-running CLI script that builds cycles *between* teardowns. That ADR's
  *Consequences* says outright that it "remains open"; goal 21's item 7 closes the *leak* at teardown
  and does not build the collector. This is an open decision, not an unclosed gap, and it stays here
  so that it stays visible.
- **`array<T>` is invariant, so an `array<int|string>` parameter takes only that exact spelling.**
  `crates/nvs-stdlib/src/lib.rs` gap 4 and `nvs_types::expr::is_assignable`'s own docs argue both
  sides: widening accepts strictly more programs and breaks none, and an Novis array is a
  copy-on-write value so an element-covariant read cannot be aliased into an unsound write. Narrow is
  the safe thing to hold while the question is open. It is a decision to take, not work to schedule.
- **`Core\Uri::with` replaces a component and cannot remove one**, so there is no spelling for "this
  URI without its fragment" — `crates/nvs-stdlib/src/uri.rs` gap 1. It needs an options bag that can
  tell an omitted option from a written `null`, which is a registry question rather than a `Core\Uri`
  one.
- **No custom panic hook.** [ADR 0002](../adr/0002-error-handling-and-panic-containment.md)
  § *Corollary* wants a panic message routed to the request log with its request id;
  `crates/nvs-runtime/src/lib.rs` gap 4 has it, and its blocker — there being no request log — went
  away in M5. It is presentation rather than containment, which is why it has waited, and it rides
  naturally with goal 21's item 12 if a session finds itself there.
