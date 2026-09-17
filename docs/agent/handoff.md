# Handoff

## State

**Goal `decided-closures` is reached.** The register is empty from every side: `python
tools/owners.py --closes decided-closures` and `python tools/playbook.py --closes decided-closures`
both answer "owns no …", and `python tools/owners.py --check --past-is-an-error` reports
`past-milestone: 0` over 24 milestone-owned gaps. `python tools/verify.py` is 11 of 11 green and
`python tools/verify.py --doc` resolves every link.

The goal's last item is built rather than struck: a bundle now carries the file set a
*re-compilation* reads — the `require` graph **and** everything the `autoload` roots declare — and a
bundled process resolves and enumerates through the payload instead of the filesystem. The rule that
owns the language fact is `rule:programs/no-runtime-autoload`; how the closed world answers a path,
a listing and a directory test is `crates/nvs-diagnostics/src/embedded.rs`'s module doc.

One unrelated breakage was fixed on the way, because the rustdoc gate only runs at a goal's end: the
intra-doc link in `crates/nvs-cli/src/serve.rs:1407` named a private `worker::open`.

## Next group

**Goal `one-type-test`, stage 2: the record and the rulebook** — the driver installs
`docs/agent/goals/67-one-type-test.handoff.md` over this file at the switch, and that file is
authoritative for the stage; these are its first two items, unchanged.

- [ ] **The record** — one new record at the next free number, to the shape
      `docs/agent/goals/67-one-type-test.md:67` spells out: `changes.creates` is
      `php-migration/one-type-test`, `changes.modifies` is the six fragments the goal names, §
      *Context* freezes the one reading of PHP's RFC, § *Revisiting* names one Novis trigger and no
      PHP one.
- [ ] **The new fragment and its JSON entry** — `docs/rules/php-migration/one-type-test.md` at
      `status: designed`, placed beside the `let-and-is-are-reserved` entry at
      `docs/rules/php-migration.json:176`, its `divergesFromPhp` the one sentence `divergences.md`
      prints.
- [ ] **The six fragments rewritten** to the language that goal ships —
      `docs/rules/types/type-test.md:1`, `docs/rules/types/narrowing.md:1`,
      `docs/rules/types/class-reference-sites.md:1` and the three under `docs/rules/php-migration/`
      the goal names — each `because` gaining the record's number, then `python tools/rules.py
      --render`.

## Backlog

- `crates/nvs-cli/src/bundle.rs`'s remaining known gap: § 6's `.nvsx` entries are not embedded, owned
  by M9 and blocked on Tier 1 extensions loading at all.
- A bundle grows by every `.nvs` file under a declared root, stated in `bundle::build`'s doc comment;
  nothing measures it and no cost guard names a bundle's size.
- Goal `gap-zero` follows `one-type-test`; its gate declares the tree clean of owed work, so a
  construct deleted after it would reopen sites it passed over (`docs/agent/goals/68-gap-zero.md`).
