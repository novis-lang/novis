---
milestone: post-parity
---
# Loop goal 8 — The on-disk artifact cache has a producer

`rule:packaging/an-artifact-is-one-immutable-content-addressed-file` is built and unreachable. Goal `governance` item 14
landed it "exactly as specified" — `crates/nvs-cli/src/cache.rs` has the content-addressed layout, the
`fsync`/`rename` write, the mmap-verify-then-execute read, the probabilistic eviction sweep and its own
test suite — and **nothing calls it.** `grep -rn "crate::cache" crates/nvs-cli/src/` returns nothing:
the only caching on the run path is `crates/nvs-cli/src/script.rs:76`'s in-process `HashMap`, which
dies with the process. Every `nvs run` and every `nvs serve` boot compiles from source, every time.

When this goal is green a second run of the same program on the same toolchain does not compile it,
and the number that says so is measured rather than asserted.

It sits after goal `carried-gaps` because the reason the cache has no producer was recorded in a handoff backlog
that a goal switch then deleted, and goal `carried-gaps` is what builds the file such a note now goes in. It sits
before goal `temp-sweep` because a warm start is the largest single latency item on the chain and everything
after it only adds source to compile.

Goal `carried-gaps`'s whole acceptance list is this goal's floor, and it is never traded.

## Why here

`crates/nvs-cli/src/cache.rs` is the whole store — layout, fsync/rename write, mmap-verify read,
eviction sweep, its own tests — and nothing in `nvs-cli` calls it, so every run compiles from source
and the only cache on the path is one in-process HashMap that dies with the process. The obstacle is
real and is `nvs-codegen`'s: `emit.rs` bakes host addresses in as `iconst` immediates with no
relocation record, so a page dump is wrong in the next process however it is serialized. That makes
this a subsystem rather than a gap, which is why it is its own entry and not a stage of goal `carried-gaps`.

## Stage 0 — the catch-up

Nothing. `crates/nvs-cli/src/cache.rs` is written against `rule:packaging/an-artifact-is-one-immutable-content-addressed-file` as specified and stays; what
changes is § 3's *letter*, and stage 4 is where that amendment is written rather than left as a
surprise for a reader of the ADR.

## Stage 1 — the floor

Goal `carried-gaps`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`.

## Stage 2 — the keystone: `nvs-codegen` can emit the same IR a second way

The whole obstacle, and `crates/nvs-cli/src/cache.rs`'s own known gap is the diagnosis. A warm hit
cannot map the JIT's finished pages, for two independent reasons: `cranelift_jit::JITModule` has no
serialization at all, and — the one that would survive such an API — `crates/nvs-codegen/src/emit.rs`
bakes **host addresses in as `iconst` immediates carrying no relocation record**. Three sites do it: a
class descriptor's address in `class_desc`, the same address again in the `instanceof` lowering, and a
statically resolved target's code address through `method_address`. Those are valid only for the
process that allocated the descriptors and compiled the callee.

1. **A second `Module` implementation** — `cranelift-object`'s, producing an `ObjectProduct` — behind
   the same `nvs_ir::Program` walk, so there is one lowering and two backends rather than two
   lowerings. `crates/nvs-codegen/src/lib.rs:449`'s `compile` is the entry point that gains a sibling.
2. **A named symbol for every address the JIT bakes in.** Each of the three sites above emits a
   relocation against a symbol the loader can resolve, instead of an immediate. Under `JITModule` the
   symbol resolves to the same address it bakes today, so the hot path is unchanged and the two
   backends stay one walk.
3. **A guard test that the two agree**: the same program through both modules produces the same
   observable answers, which is what stops the object path drifting into a second semantics.

Files: `crates/nvs-codegen/src/lib.rs`, `crates/nvs-codegen/src/emit.rs`.

## Stage 3 — the relocating read path, and the wiring

4. **A warm hit maps private-writable, verifies, relocates, then makes the pages executable.** This
   **amends `rule:packaging/an-artifact-is-verified-whole-before-a-page-is-executable`'s letter**, which maps
   `PROT_READ` and `mprotect`s the very same mapping; a relocated image needs a private writable one
   first. The checksum discipline is untouched — the hash still covers the file's bytes and the
   patching happens after it.
5. **`nvs run` and `nvs serve` consult the cache.** `crates/nvs-cli/src/runner.rs:384`'s `compile` and
   `crates/nvs-cli/src/script.rs:84`'s resolver are the two seams, and the in-process `HashMap` stays
   in front of the on-disk store rather than being replaced by it: they answer different questions
   (this process again, versus this machine again).

## Stage 4 — the amendment and the measurement

6. **`rule:packaging/an-artifact-is-verified-whole-before-a-page-is-executable`'s read path is rewritten to what landed**, in the ADR's own body — never as an
   overlay, per AGENTS.md — and its *Investigation* paragraph about mapping "the bytes directly as the
   pages the JIT would otherwise have produced" is corrected to what stage 2 proves is reachable.
7. **The warm-start figure is measured and recorded**, not asserted: `benches/` gains a cold-versus-
   warm run of a real program, and the number goes in the ADR beside the decision. A cache that does
   not measurably beat compiling is a cache to delete, and this is the check that would say so.

## Standing decisions

- **This goal opens no new ADR number.** Its one record's `changes:` block names
  `rule:packaging/an-artifact-is-a-relocatable-object-behind-a-self-describing-header` and `rule:packaging/an-artifact-is-verified-whole-before-a-page-is-executable` and nothing else.
- **The object backend is a second `Module`, never a second lowering.** If the two cannot share the
  `nvs_ir::Program` walk, the goal stops and says so rather than forking `emit.rs` — a second
  lowering is a second semantics, and this repository has one execution tier on purpose.
- **A warm hit that fails verification is a cold compile, silently.** A corrupt or stale payload is
  never an error a user sees; it is a miss. `rule:packaging/an-artifact-is-one-immutable-content-addressed-file` already says so and this restates the
  consequence rather than the rule: nothing in this goal may make a cache problem into a program
  failure.
- **`nvs run` keeps working with the cache directory absent, unwritable or full.** Every one of those
  is a miss, and the eviction sweep already decided what a full cache does.
- **If stage 2 proves unreachable**, the safe fallback is recorded and taken: `cache.rs` is
  **deleted** rather than left compiled-in with no caller, and `rule:packaging/an-artifact-is-one-immutable-content-addressed-file` is retired with the reason.
  Dead code that looks like a feature is worse than an absent feature, which is the whole finding
  this goal came out of.
