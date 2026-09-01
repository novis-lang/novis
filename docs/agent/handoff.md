# Handoff

## State

**`Core\Storage::list` is on disk, and stage 9's third owed piece with it.** The class is four
rows now: `list($disk, {prefix?: string}): array<string>` answers the keys of the disk's objects,
sorted byte-ascending, and an entry is a key only when it is a **regular file** whose name the key
grammar accepts — a subdirectory, a dot-file and a symlink are omitted rather than refused, since a
disk's own contents are not the caller's input. `crates/nvs-stdlib/src/storage.rs`'s module doc is
the home of that reading, of the sort, and of why `prefix` is an option.

**The door is new and it is the tenth filesystem one.** `nvs_runtime::capability::read_dir` sits
behind `fs.read` over **the directory**, not over its entries: reading a directory is one read of
one path, and a name it hands back is a second path every other door still asks about. That
function's own doc argues it; ADR 0118 § 2 needed no amendment, since its roster there is
illustrative rather than closed. The class still declares no capability of its own — the new
`CAPABILITIES` row is `FsRead`, and `storage_over_local_disk_is_gated_on_the_same_fs_capability`
pins all four.

**What stage 9 still owes is TLS, and `AUTH` behind it.** Both are `Core\Mail`'s, and its module
doc already says the dependency runs the other way: `STARTTLS` and `AUTH PLAIN` are one function
each **on top of** a TLS-capable stream, and the stream is what `Core\Http\Client` is refusing
`https` for. So the next group is the stack, not the two members.

**The acceptance check still names `every_part_two_spec_member_is_registered`** — stage 10's gate
over a *complete* Part II, which needs §§ 15-19. Those are `Core\Request` and its neighbours and
are goal 6's, so this check cannot pass inside this goal and is not a regression.

**Read the plan's stage 7 clause before taking it.** *"`write`'s `fields` refuses a `secret`"*
scans as owed, but `reject_secret_logged_argument` and `is_fields_argument` are already on disk
(`crates/nvs-types/src/expr/quals.rs:658`, `:708`) under `E_SECRET_LOGGED`. What is owed there is
reading `[log] target`.

**Two pack gaps, both still open.** `[context] modules` does not select
`nvs-runtime/src/terminal.rs`. And `[context] playbook` filters to the *item's* anchor paths, so a
group that also writes `.nvst` cases never sees the case-authoring bullets — this session wrote
three and paid one run for the rule at `docs/agent/playbook.md:1785` (a local needs its type, and a
`foreach` binding declares one).

## Next group

**The TLS stack, which is one dependency and then two members that were waiting on it — over
`crates/nvs-host/src/net.rs`, `crates/nvs-stdlib/src/http/transport.rs` and
`crates/nvs-stdlib/src/mail.rs`. ADR 0051 § 3 names `rustls`, and picking it is pre-authorized
under § 4.**

- [ ] **A TLS-capable parking stream.** `rustls` in `[workspace.dependencies]` with the comment
      saying why, `cargo deny check`, `python tools/gen-attribution.py`, and a wrapper over the
      plain stream at `crates/nvs-host/src/net.rs:223` that keeps the hand-the-core-back contract.
      The trust anchors are the decision to record — `crates/nvs-stdlib/src/http/transport.rs:229`
      is the sentence that names them as missing.
- [ ] **`https` in the outbound transport**, replacing the refusal — the scheme is already read at
      `crates/nvs-stdlib/src/http/transport.rs:355` and the module doc's gap section is at
      `crates/nvs-stdlib/src/http/transport.rs:36`. ADR 0058 § 3's pinned address is unchanged: the
      name is resolved once and the TLS name is the one that was pinned.
- [ ] **`STARTTLS` and `AUTH PLAIN` on the SMTP session** — `crates/nvs-stdlib/src/mail.rs:604` is
      `Session`, `crates/nvs-stdlib/src/mail.rs:314` is the credential refusal that comes out, and
      `crates/nvs-stdlib/src/mail.rs:55` is the module doc gap and `:98` the port 25 default that
      becomes 587.

## Backlog

- Reading `[log] target` — stage 7, `docs/implementation-plan.md`'s *Open now*.
- `[context] modules` does not select `nvs-runtime/src/terminal.rs` — `docs/agent/loop-goal.toml`.
- `[context] playbook` filters to the item's anchor paths, hiding the case-authoring bullets from a
  group that writes cases — `docs/agent/loop-goal.toml`.
- `every_part_two_spec_member_is_registered` needs spec §§ 15-19, which are goal 6's.
- Redis `AUTH` and TLS, if the shared store ever needs them — `crates/nvs-stdlib/src/cache/redis.rs`.
