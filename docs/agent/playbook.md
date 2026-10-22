# Playbook — the traps this repository has actually sprung

Hard-won specifics: things that cost a real session real time, written down so the next one does not
pay again. A bullet is a *trap* — "this looks like it should work and does not, and here is what to do
instead" — in three sentences, ending with the `until: <kind> <arg>` trailer that says what retires it
([conventions.md](conventions.md) § *A playbook bullet*; the kinds are looper's `docs/goal-format.md`
§ *The playbook*). **It is not a changelog.** The session that hit the trap, the stage it was on and what it tried first
are `git log`'s to keep, and a bullet that carries them is charged to every session after it.

**One file per bullet.** The bullets are under [playbook/](playbook/), one directory per section and
one file per bullet, named for the words of its bold lead-in. Grep the directory for a bullet's
words to find it. A new trap is a new file and a retired one is a deleted file, so two sessions that
each add a bullet never edit the same file.

**Append, expire, never reword.** Add a bullet when a trap costs you time — `looper wrap`'s
`## playbook: <section>` is the one way in, and it refuses a bullet past 700 bytes or with no path in
backticks. Edit one when it stops being true. `looper playbook retire` deletes every bullet whose
condition holds, so a trap that dies with a flag, a file or a test declares that and leaves on its own. Never reword a
bullet to say the same thing differently: the churn is the cost this playbook exists to avoid.

**Scope.** A rule that binds every agent goes in [AGENTS.md](../../AGENTS.md). A settled rule lives in
the rulebook under [docs/rules/](../rules/), with its reasons and its Declined list. How a subsystem
works goes in that crate's own module doc comment. The *shape* of something you are about to write is
[conventions.md](conventions.md). What is left — the trap — is the playbook, and `looper orient` hands a
session the bullets it ranks highest for the live goal, and names how many more there are.

## Tooling

## Running things
- **`bun nv proofs --record-perf --group lang:concurrency` run in the background is killed as a
  `HANG:` at `no-colouring-i-o-just-waits`, and it appends nothing for the features it had already
  measured.** That bench's callgrind leg runs in WSL and prints nothing for over 90 s, and the hang
  watch counts only output for a tree with `wsl.exe` in it. Alone, in the foreground, it ends in about
  265 s. Record that feature alone with `--only`, then the others.
  `until: gone benches/members/lang/concurrency/no-colouring-i-o-just-waits.nvs`
- **The `looper` on `PATH` may be an older build with no `verify` subcommand, and run in the
  background it exits 0 with only a usage error.** Run `bun tools/looper/main.ts verify`, the
  project's own copy, and read the log for `✓ 1:` before you trust the exit status. See
  `tools/looper/main.ts`. `until: gone tools/looper/main.ts:verify`
- **`bun nv soundness --probe` runs the `covws` build, not the debug build you just changed.** After a
  lowering change, the probe reports the old binary's panic for every new cell. Run it as
  `NVS_BIN=E:/mwl/target/debug/nvs.exe bun nv soundness --probe`, with an absolute path, because a
  relative one is not found. See `tools/nv/cmd/soundness.ts`.
  `until: gone tools/nv/cmd/soundness.ts:covws`

## Writing a test case

## Splitting a file

## Writing Novis
- **Every byte added to a `Core\Test` ledger entry is paid once per assertion for the whole of an
  `nvs run`, because nothing empties the ledger outside a test.** Stage 2 of `diagnostic-records`
  added two record nodes per failed comparison, and a hostile proof that catches a million failures
  hit the memory ceiling; only the goal-end `floor-debt` ran it. Keep anything a runner alone reads
  behind `Ctx::owned_by_test`. See `crates/nvs-runtime/src/ctx/error.rs`.
  `until: gone data/gaps/nvs-runtime/an-assertion-outside-a-test-grows-the-ledger.json:goal`

## Divergences
