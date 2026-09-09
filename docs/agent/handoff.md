# Handoff

## State

**Goal 25's Stage 4 is complete: `Core\Zip` is three members, and all six of the `4 zip` acceptance
tests pass.** `entries`, `read` and `extract(bytes $archive, string $destination, uint $maxBytes =
67108864, uint $maxRatio = 1000): uint` are on disk, all three registered and all three carrying a
reference card; `crates/nvs-stdlib/src/zip.rs`'s module doc is the home of why each refusal happens
where it does.

**The bound is one `Budget` for a call, and `contents` is gone.** `crates/nvs-stdlib/src/zip.rs:@Budget`
hands each entry a `Bound` whose `bytes` is what the archive has left, so
`rule:core-classes/decompression-bound`'s two halves are one piece of arithmetic and one message;
`Budget::read` is the only route to an entry's octets, so nothing decompresses without being charged.

**`extract` resolves before it creates.** The archive is judged whole first, so a hostile entry
refuses with no destination created at all; then each level is created, resolved and compared against
the destination before anything is made inside it, and the file itself is created exclusively. Two
consequences are stated rather than worked around, both in the module doc and both pinned by a case: a
refusal part way leaves what it had already written, and a second extraction over the same names is an
`IOError`.

**`Core\Zip` is now a capability-bearing class**: three rows in `crates/nvs-stdlib/src/registry.rs`'s
`CAPABILITIES`, `entries` and `read` declaring `None`. `extract` declares `fs.write` and also shows
`fs.read` at the door, because resolving a name reads the directories above it.

The symlink half of `extraction_cannot_escape_its_destination_when_a_symlink_appears_during_it` runs
only where the host will create a link; it was run under WSL this session and passes there, and prints
why it did not run on the Windows leg. The playbook bullet under *Writing a test case* is the whole of
that trap.

## Next group

**Stage 5: the three keys are struck, and the server still compresses nothing** — one file set:
`crates/nvs-stdlib/tests/spec_registry_coverage.rs` and `docs/spec/02-php-migration.md`, with the last
item alone in `crates/nvs-server`.

- [ ] **`every_migration_member_is_registered`**, so every member spelling the migration table names
      resolves to a registered `Core` row — the machinery that reads the table is already there and
      already counts, at `crates/nvs-stdlib/tests/spec_registry_coverage.rs:1253`, so this is the
      assertion over it rather than a new reader. `rule:core-api/tier-roster`.
- [ ] **`every_part_two_spec_class_is_registered` with the three new classes in it**, at
      `crates/nvs-stdlib/tests/spec_registry_coverage.rs:1064` — it is named by the same acceptance
      check and may already be green, in which case the slice is the case that says so out loud.
      `rule:core-api/tier-roster`.
- [ ] **`the_server_still_sets_no_content_encoding_of_its_own`** in `crates/nvs-server`, beside the
      response assertions at `crates/nvs-server/src/serve.rs:4986` —
      `rule:http-server/two-deployments-and-nothing-a-proxy-owns`, whose reasoning is
      `docs/decisions/0097.md` § 1's compression row: a program may compress its own body and nothing
      in the response path does it implicitly.

## Backlog

- Zip64 and the entry CRC are `crates/nvs-stdlib/src/zip.rs`'s two known gaps, both stated there.
- An all-or-nothing extraction is a caller's own empty directory, not a member — module doc, § *An
  extraction that refuses part way*.
- `[context] modules` for this goal has no `nvs-server` pattern, and Stage 5's last item is there.
