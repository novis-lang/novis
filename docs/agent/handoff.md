# Handoff

## State

**Goal `gap-owners`, stage 3 — the attribution pass — has finished `crates/nvs-cli/`.** The roster is
48 tags, every one resolving to a live goal, a milestone that is not `done`, or an `unowned` with its
reason written (`python tools/owners.py --check --reasons` is green). 108 items elsewhere still name
nobody, and they sit in six files' worth of crates: `nvs-stdlib` 77, `nvs-db` 14, `nvs-runtime` 8,
`nvs-syntax` 6, and one each in `nvs-test` and `nvs-hir`.

**Three of the five blocks read this session were decisions rather than gaps.** `bundle.rs`'s macOS
signing item and its `./nvs.toml` item both state settled, argued behaviour the code already has:
`crates/nvs-cli/src/bundle.rs:281`'s `codesign` block does what
`rule:packaging/a-macos-bundle-is-ad-hoc-signed-at-build` says, and `bundle.rs:122` passes no
`--config`, so the working-directory search is the ordinary one at `crates/nvs-cli/src/main.rs:169`
— which the packaging rule itself requires, having everything past the footer reader be the `nvs run`
pipeline. `crates/nvs-ir/src/ty.rs:15` is the same case whole: every paragraph states what the IR
lattice erases and why, closing with "it does not replace the erase-checker-qualifiers design
itself", so the heading was wrong rather than the content and it reads `# What this lattice does not
carry` now. `cache.rs` held two of each kind.

**A block that enumerates nothing is one item to `owners.py`, whatever its paragraph count** — the
new playbook bullet under *Tooling*. That is why `crates/nvs-cli/src/cache.rs:153`'s bold paragraphs
are a `*` list now: they needed `M6`, `unowned` and `unowned`, and one block carries one tag.

**[carried-gaps.md](carried-gaps.md) § *Unowned* is thirteen entries.** The new one is that the
compiled-unit cache loads on no `aarch64` host although `docs/plan/design.md:94` names macOS
`aarch64` a supported platform; what has to be decided is where instruction-cache maintenance lives —
this loader or `nvs-codegen`'s mapper — not whether it is wanted.

**The rustdoc gate's unresolved link was never a session's.** It is
`benches/serve-probe/src/main.rs:31`, in the bench crate the user landed as `39decf358` while this
session ran, and it links a `#[cfg(test)]` function rustdoc cannot resolve. It is a code span now, in
a commit of its own — the file is tracked, so leaving the fix in the tree was not an option.

**`python tools/verify.py`: 9 of 9 green** — 3838 tests, both `.nvst` trees, the reference and
clippy. The previous handoff's clippy failure is gone because `benches/serve-probe/` has a `src/`
now, so cargo can load that workspace member; nothing about it was a regression.

## Next group

**Stage 3: the attribution pass, module by module** — one file set: `crates/nvs-syntax/src/lib.rs`'s
six items, then the two single-item blocks left outside the three big crates. The kinds are the
goal's § *Standing decisions*, the three owner kinds are `python tools/owners.py --help`, and
`--untagged` is the worklist.

- [ ] **Tag `crates/nvs-syntax/src/lib.rs:69`'s six items** — `goto` labels, a bare inline shape type
      on a local declaration, grouped `use`, `var` in a class body, a keyword-spelled member name, and
      `Core\Static`-style segment collisions. Four are already claimed one level up by
      `docs/agent/carried-gaps.md:58`'s row, which names goal `doc-comments` — **retired**, so each is
      the judgement `owners.py --help` describes for a retired owner: closed, or left behind and then
      `M1`'s, whose own *Verify* is a `php-src` corpus parse and which is not `done`.
- [ ] **Tag `crates/nvs-hir/src/lib.rs:48`.** It is that crate's only gap record — no other `nvs-hir`
      module has a `# Known gaps` heading — and it rolls four edges into one item, two of which say
      they wait on `nvs-types`' static types, which has since landed. Enumerate it, check those two
      against `nvs-types` before tagging, and note that `M2` is `done`, so a milestone tag is not
      available to whatever is still open.
- [ ] **Tag `crates/nvs-test/src/lib.rs:188`** — one item, no `--EXPECTREGEX--`, which reads as a
      decision (`--EXPECTF--` covers what the corpus needs) and so probably leaves the block rather
      than taking an owner.

## Backlog
- `crates/nvs-runtime`'s eight: `array.rs:214`, `commands.rs:41`, `decimal.rs:45` and `:52`,
  `graph.rs:61` and `:66`, `routes.rs:74` and `:79`.
- `crates/nvs-db`'s fourteen, then `crates/nvs-stdlib`'s seventy-seven — the bulk of stage 3.
- Stage 4 is untouched: `verify.py` still does not run the owners gate, so
  `docs/agent/loop-goal.toml`'s acceptance check is its only reader.
- `benches/serve/echo.nvs` and `tools/bench-load.py` are the user's untracked work, and `Cargo.lock`
  is modified by nothing this loop did.
