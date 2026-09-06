# Handoff

## State

**Goal 25 — `Core\Compress`, `Core\Mime` and `Core\Zip` — has just started; nothing of it has landed
yet.** Goal 24's whole list is this goal's Stage 1 floor.

**They are M8's, not M9's**, and that correction is why this entry exists.
`crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt`'s header said "the four in § 17 are the
document and archive formats M9 carries"; [m9.md](../../plan/m9.md) is the extension system — `.nvsx`
loading, the WIT world, the capability bridge — and carries none of them, while
`rule:core-api/tier-roster` puts all three at Tier 0.

All three are Tier 0 for one reason: what a compressed or archived input can do to a server is
**policy**, and policy must be non-optional. `Core\Zip` is Core "despite passing test 5" for exactly
this — a sandboxed decoder gets the memory cap for free and the traversal rules not at all.

## Next group

**Stage 2: `Core\Compress`, and the bound** — one file set: the new `crates/nvs-stdlib/src/compress.rs`,
`crates/nvs-stdlib/src/registry.rs`, `crates/nvs-config/src/tree.rs`.

- [ ] **Four codecs, one API** — gzip, deflate, brotli, zstd. The codec is an **enum**, never a string;
      there is no `compress($data, "gzip")` for the same reason there is no cipher-name-as-string.
- [ ] **The bound is a parameter with a default, not an option that can be `null`** — a ratio and an
      absolute output ceiling, both. Exceeding either throws; a truncated decompression that looks like
      success is the bug the class exists to prevent. `[limits]` gives the default; a call may lower it
      and never raise it past the configured ceiling.
- [ ] **Dependencies picked under ADR 0051 § 4** — pure Rust for all four, no audited-C exception, and
      each owes `python tools/gen-attribution.py`.
- [ ] **Decide streaming-or-whole-buffer in the module doc.** The `Core\Xml` precedent is that a tree
      and a stream are different jobs stated as such; that sentence is either written here or
      explicitly does not apply.

## Backlog

- **Stage 3 (`Core\Mime`)** is a magic-byte table this crate carries — never libmagic's rule language
  and never a file extension, because a type detected from an extension is a type an attacker chose.
  The answer is a closed enum plus "unknown". **Detection is not a laundering** and that sentence goes
  in the member's own doc card, because it is the thing a caller will get wrong.
- **Stage 4 (`Core\Zip`)** is where the policy lives: traversing, absolute and symlink entries refused
  at *read* time so a program cannot opt out by extracting entries itself; the bomb is stage 2's bound
  applied per entry and across the archive. The proofs are the four attacks, each refused by a
  diagnostic that names the rule rather than by a failed file operation.
- **The server still compresses nothing** (ADR 0097 § 1). This goal gives a *program* a compressor and
  puts none in the response path.
