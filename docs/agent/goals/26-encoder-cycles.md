---
milestone: M8
---
# Loop goal 26 — An encoder ends a cycle where it closes, and every walk that can meet one is audited

A program that hands a self-referencing object to `Core\Json::encode` is told **where the cycle
closed**, by the property chain that closed it, instead of being told its value nests past
`DEPTH_CEILING` levels. Shared-but-acyclic structure still encodes and still duplicates, and the
depth cap still bounds a document that is merely deep — so after this goal the two failures are two
messages rather than one. Every other walker in `nvs-stdlib` that could reach an object graph has
been read and reported on rather than assumed.

It sits immediately before goal `xml-tree` because `Core\Xml` is the next encoder that will be written and
does not exist yet: landing this rule afterwards would make it a retrofit of the very encoder it
exists to bind.

Goal `formats`'s whole acceptance list is this goal's floor, and it is never traded.

## Why here

Nothing ahead of it is a dependency — the encoder it fixes is on disk and the record that argues the
fix is already accepted — so it is placed by what comes next rather than by what it needs.

## Stage 0 — the catch-up

Nothing on disk contradicts the rule — `rule:classes/an-encoder-ends-a-cycle-by-identity` is
`designed` and no code claims to implement it. What is stale is a *claim*:
`crates/nvs-stdlib/src/json.rs`'s module doc enumerates what the encoder refuses and says nothing
about a cycle, because today it cannot tell one from a deep nest. That paragraph is rewritten whole
in stage 2 with the rest of the doc, never edited to leave the old sentence beside the new one.

## Stage 1 — the floor

Goal `formats`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded for
anything above it.

## Stage 2 — the keystone: the ancestor chain in `Core\Json::encode`

One file set — `crates/nvs-stdlib/src/json.rs` — so the whole group opens in one `peek.py` call.

1. **`crates/nvs-stdlib/src/json.rs:@Encodable`** — carry the identities of the objects on the path
   from the root beside the existing `depth`. The ancestor chain, **not** a set of everything seen:
   [0164](../../decisions/0164.md) § 2 is why, and a global set would refuse a legal document that
   merely holds one object twice.
2. **`crates/nvs-stdlib/src/json.rs:@serialize_object`** — before descending, refuse if the object is
   already on that path. The existing `DEPTH_CEILING` check stays exactly where it is and keeps its
   own message.
3. **`crates/nvs-stdlib/src/json.rs:@serialize_array`** — the same check at the array arm, since a
   cycle can close through either.
4. **The refusal names the path** — the property chain that closed the cycle, thrown as the
   `ThrownClass::Logic` this call site already throws. Detecting at the first repeat is what makes
   this possible: the encoder is standing exactly where the answer is.
5. **The module doc, rewritten whole** — its list of what the encoder refuses now has a cycle on it,
   and its depth paragraph says what the cap does and no longer implies it is the cycle answer.

## Stage 3 — the other walkers, audited rather than assumed

`crates/nvs-stdlib/src/csv.rs`, `uri.rs` and `encoding.rs` each walk a value. The question this stage
answers, per file, is whether the walk can reach an object graph at all — and the answer is written
down either way, because "we checked and it cannot" is the finding that stops the next reader
checking again.

A walker that **can** reach one takes stage 2's treatment and its own case. A walker that **cannot**
gets one sentence in its module doc saying so and nothing else. `Core\Serialize` is not in this stage:
it runs the graph copy and reaches the property through `rule:classes/graph-copy` already.

## Stage 4 — the rulebook

`rule:classes/an-encoder-ends-a-cycle-by-identity` moves from `designed` to `shipped` and its
`guardedBy` names the cases stage 2 and stage 3 landed. `python tools/rules.py --render` rewrites the
generated chapters, and `--check` and `python tools/decisions.py --check` are what say the fragment,
its topic entry and [0164](../../decisions/0164.md)'s `changes:` block still agree.

## Standing decisions

- **The set is the ancestor chain, never everything seen** ([0164](../../decisions/0164.md) § 2). A
  value reached twice by two different paths is shared, not cyclic, and encodes by being written
  twice — a format with no way to express sharing has no other answer. Do not "fix" the duplication.
- **The refusal is a throw, and no marker is invented in the document** ([0164](../../decisions/0164.md)
  § 4). `{"$cycle": N}` is right for a record rendering and wrong for a payload, and the encoder here
  produces somebody else's payload. A cyclic graph has no JSON encoding; that is the answer, not a
  gap to close.
- **The depth cap stays** ([0164](../../decisions/0164.md) § 5). It is not what was wrong, and it is
  the only reason today's failure is safe rather than a stack overflow.
- **This goal opens no ADR number.** [0164](../../decisions/0164.md) is already accepted and its
  `changes:` block names the one rule this goal ships.
- **If the stage 3 audit finds a walker that can reach an object graph**, the fix lands in this goal
  rather than becoming a new one — it is the same edit against the same rule, and the file set is
  already open. If it finds one that cannot, that is a sentence in that module's doc and nothing
  else, and neither outcome is a reason to stop.
