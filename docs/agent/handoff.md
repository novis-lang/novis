# Handoff

## State

**Goal 5's Stage 0 has landed and the goal's three fixtures now exist, red.** `examples/db.nvs`,
`examples/transaction.nvs` and `examples/queue.nvs` (plus the two job scripts the last one pushes,
under `examples/queue/`) are written as the programs ADR 0067 and ADR 0084 specify, and each fails at
`E0405` naming the member it needs. That is the point: `LoopGoal.begin` walks `loop-goal.toml`'s
`files` and returns on the first path not on disk, so until this commit **not one of the 174 stage-1
floor checks had run** — the goal had no regression coverage at all. The frozen output each fixture
owes is `loop-goal.toml`'s (stages 9, 5 and 8); its *source* is not frozen and the stage that lands
the members rewrites it freely.

**`crates/nvs-db` still does not exist**, and neither does `tools/db-matrix.py`. `tests/db/compose.yaml`
does, and the driver preflights the Docker daemon before a session starts. **The goal's one ADR slot is
unclaimed** — Stage 2 item 3, the driver crate's shape and its wire I/O. The next free number was 0132
at this commit; re-check `git status --short docs/adr/` immediately before creating the file.

**`nvs.toml` deliberately gained nothing.** The fixtures read a `[db.main]` block and a `[queue]` block
that cannot be written yet — see the playbook bullet added this session — so they belong to the slices
that add the config structs behind them.

## Next group

**The keystone's two prerequisites, then the seam itself** — Stage 2 in `docs/agent/loop-goal.md`, whose
own § *The harness this goal owes* says both harness files come before the first driver. One file set:
the ADR, `tools/db-matrix.py` and the new `crates/nvs-db`, over the two host types the driver is written
against. The three fixtures above are the contract each slice is closing.

- [ ] **ADR 0132 — the driver crate's shape and its wire I/O**, the goal's one pre-authorized slot:
      which protocol crate backs which driver, how TLS layers on the parking stream
      (`crates/nvs-host/src/tls.rs:112`), how a connection's busy state is tracked over the parking
      stream (`crates/nvs-host/src/net.rs:130`), and how the five drivers share code without a trait
      that flattens their differences. ADR 0067 specifies behaviour and deliberately not this. Add the
      row to `docs/adr/README.md`'s two tables and the bullet to `docs/adr/ground-rules.md`.
- [ ] **`python tools/db-matrix.py`** — runs ADR 0067's per-driver list against `tests/db/compose.yaml:1`'s
      five services and prints one `<driver>: ok` line each; the check that consumes it is
      `docs/agent/loop-goal.toml:2848` and the sentence that specifies it is
      `docs/agent/loop-goal.md:165`. A harness, not a test: the assertions stay `nvs-db`'s.
- [ ] **`crates/nvs-db` exists, and a PostgreSQL connection is opened, TLS-wrapped and authenticated**
      over goal 2's `NvsTcp` with `rustls` on it (`crates/nvs-host/src/tls.rs:112`). The signature it
      is answering is `docs/spec/01-core-library.md:1144`, and the first caller is
      `examples/db.nvs:48`.

## Backlog

- `nvs.toml` owes a `[db.main]` block and a `[queue]` block, and `crates/nvs-config/src/tree.rs` owes
  the structs that let it parse them — the slice that adds each struct writes the block.
- `orient.py` warned that `[context] modules` pattern `crates/nvs-host/src/stream.rs` matches nothing;
  that file has moved or the glob is wrong, and the entry is dead weight in `loop-goal.toml`.
  (`crates/nvs-db/src/*.rs` warning is expected until Stage 2 lands.)
- The three fixtures exit non-zero while red, so the valgrind sweep would grade them as leaks if it were
  ever reached — it is not, because their own `[[check]]`s fail first. No `[valgrind] skip` was added,
  and none should be: they are red temporarily, not by design (`docs/agent/playbook.md:1113`).
- Stage 2 items 4-7 — settings as five types, `db.connect` vs `db.open`, the `LOCAL INFILE` refusal,
  the forced UTF-8 charset — `docs/agent/loop-goal.md:62`.
- `derive.rs`'s own gap 2: nothing generates `fromRow` from `ExprTypeTable::db_codec` yet.
