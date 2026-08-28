# Handoff

## State

**Goal 3 of the parity program has just started; nothing of it has landed yet.** Goals 1 and 2 and M4
reached their whole acceptance lists and all three are now this goal's Stage 1 floor.

**`crates/nvs-config` does not exist yet.** Goal 2's isolates run under compiled-in defaults and say so at
each site; this goal is where those sites get their real answer, and picking them up is Stage 4's item 12
rather than a debt to pay off first.

**The real acceptance is a set claim, not a case.** "Every syscall-touching entry point is gated" cannot
be proven by fixtures, so it is proven the way M4 proved its refusal sites: a test over `nvs-stdlib`'s own
registry that fails naming any member touching the filesystem, the network or a process with no
capability declared. That is Stage 6's `every_capability_bearing_member_declares_its_capability`, and
**its allowlist may never grow.**

## Next group

**The directive registry and the config tree** — Stage 2, items 1–3. One file set, and everything above
reads what it produces.

- [ ] **A directive carries three fields, and reloadability is one of them.** The changeability class ADR
      0005 defines, plus ADR 0078 § 2's `Reload`/`Boot` field **orthogonal to it** — orthogonal is the
      item, because reloadability is now the only thing that makes a directive boot-only and conflating
      the two is exactly what that ADR exists to stop.
- [ ] **`nvs.toml` parses and a duplicate or unknown key is refused.** ADR 0064 §§ 1, 3, TOML via `serde`.
      § 2a is the block list and names the ADR that argues each block's directives; this goal adds five —
      `[deferred]`, `[[schedule]]`, `[http.*]`, `[metrics]`, `[trace]` — and their *validation* is Stage
      6, not this item.
- [ ] **The tree resolves.** ADR 0103 §§ 1–5: a root named by repeatable `--config`, else `./nvs.toml`,
      else the shipped defaults; `[[include]]` by `path` and by `dir`; one ordered stream where later
      wins, with **both origins reported**; a value array replaces where a `[[table]]` appends; a relative
      path resolves against the file it is written in. § 6's ownership check is the next group, with the
      `[[app]]` block — it is the trust boundary and deserves its own slice rather than a clause of this
      one.

## Backlog

- Item 10 carries this goal's one pre-authorized ADR slot — the capability enforcement points. ADRs 0051
  and 0024 name capabilities constantly and none says *where* the check sits; that gap is the slot.
- The canonicalise-then-prefix path comparison is needed by items 6, 10 and 12. Write it once; writing it
  three times is how one of them ends up accepting a symlink.
- ADR 0017's freeing of executable memory is carried by m6.md and has no consumer until there is a
  long-running process to free it in. Off path.
