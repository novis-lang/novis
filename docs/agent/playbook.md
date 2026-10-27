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
- **A `bun nv linux-test` piped into `grep` is killed as a hang before it builds.** The mirror and the
  WSL build run inside the VM, where the watcher sees no CPU, and a filter that drops the `Compiling`
  lines leaves it no output either, so it kills the run at 90 s. Run it with `run_in_background`,
  unfiltered, and grep its output file once it ends. `until: gone tools/nv/cmd/linux-test.ts:linux-test`

- **A `docs/reference/lang/` chapter is generated, so a wording change goes in the feature's
  `about.md` or `details.md`, then `bun nv render`.** An edit to the chapter itself is overwritten by
  the next render. Each render that changes the chapter makes its `lang:` features' perf stale again,
  so settle the words (`bun nv proofs --id` checks `about.md`'s 200-word bound) before
  `bun nv proofs --record-perf`. `until: gone tools/nv/proofs/chapters.ts:generated form`

- **`bun nv splice` writes past the edit-claim guard, so check `looper status` for a side loop before
  splicing a file it may have touched.** A side session's claim on `serve.rs` outlived its commit,
  because the claim is released only when a listing sees the file clean, and the splice dirtied it
  first. The Edit tool was then refused, and when the side session ended its driver committed the
  splice's hunks as that session's `wip(loop)` commit, which did not build alone. Splice a file
  only once Edit would take it. `until: gone tools/looper/src/claims/claims.ts:releaseAsked`
- **The installed `looper` binary may lack a subcommand the prompt names, such as `decision`.** It
  answers `unrecognized subcommand` with exit 2. The project's copy, `tools/looper/main.ts`, has every command, so run
  `bun tools/looper/main.ts decision close <id>` instead (AGENTS.md § *Novis works without looper*).
  `until: gone tools/looper/main.ts:decision`
- **A helper script written through a Bash heredoc into `.agent-tmp/` can lose its backslashes,
  even under a quoted `'EOF'`.** `Core\\Xml` in the script arrived as `Core\Xml`, and as `CoreXml`
  inside a template literal, so a replacement either missed its anchor or wrote a broken `.nvs`.
  Write the script with the Write tool, or make a backslash-heavy edit with Edit.
  `until: gone tools/looper/main.ts:guard`
- **A goal check with a `tests:` list cannot pass when its `run` is `bun nv verify`.** looper's
  check needs a `test <path> ... ok` line per name (`tools/looper/src/verify/accept.ts:568`), and
  verify prints only its summary, so the check says "did not run and pass" while the test is green.
  Its log is `.looper/checks/<id>.log`, not the sweep's `.looper/loops/main/checks/`. Write such a
  check's `run` as `cargo test -- <names>` when you write the goal. The permission classifier refused
  a session's later edit of a check as a CI bypass.
  `until: gone tools/looper/src/verify/accept.ts:did not run and pass`
- **The installed `looper` binary can be older than the kit's copy, and then `looper verify` and
  `looper check` fail with "unrecognized subcommand".** In the background, `looper verify 2>&1 | tail`
  still exits 0, so the error looks like a finished run. Run the copy instead:
  `bun tools/looper/main.ts verify` and `bun tools/looper/main.ts check --name <text>`
  (AGENTS.md § *Novis works without looper*). See `tools/looper/main.ts`. `until: gone tools/looper/main.ts:verify`
- **A formatter rule that starts a new line has to register it with `Indent`, or the lines below it
  indent from the author's old line.** `Indent::opening_of` measures a body from the line its opener
  was written on, so `{ public function f()` split onto two lines still left the method's `{` and
  body at column 0. `crates/nvs-fmt/src/statement.rs` computes its breaks before any indentation, and
  `Indent::new` keeps them. A later rule that inserts a line break adds its offsets there.
  `until: gone crates/nvs-fmt/src/indent.rs:breaks`

## Running things
- **A frame's size on Windows is measured, never guessed from the source.** No frame-size tool
  works on an MSVC build, and a guess cost a 5-minute release run: an 8 KiB buffer in a function
  with one caller was not in its caller's frame. A temporary probe that prints the distance from
  the task stack's base to a local, and the resident pages, at each step gives every frame's size
  in one run, as the reading in `crates/nvs-host/src/stack.rs` does. A by-value `Ctx` was the big
  frame each time. `until: gone crates/nvs-host/src/stack.rs:What a waiting request holds`
- **A recursive walk over the repository root overflows the stack in `target/`, and takes half a
  minute there.** A proof run leaves a folder thousands of levels deep under `target/proof/.nvsdata`,
  and `target/` has links back up the tree. `nvs test --list .` died with "has overflowed its
  stack" until both walks used a stack of their own and skipped linked and `CACHEDIR.TAG` folders
  (`nvs_test::is_cache_dir`). A new walk of a user's folder does the same.
  `until: gone crates/nvs-test/src/lib.rs:is_cache_dir`
- **`bun nv site --check` and `bun nv proofs --run` pick `target/release/nvs` before the debug
  build, and the release binary is rebuilt only at a goal's end.** Mid-goal they judge the code of
  the last goal, so a red names a bug already fixed and a green proves nothing about the change.
  Pass `--nvs target/debug/nvs.exe` to `site`, and run a single hostile case with
  `target/debug/nvs run <file>`. `until: gone tools/nv/cmd/site.ts:target/release/${exe}`
- **`bun nv proofs --record-perf --group lang:concurrency` run in the background is killed as a
  `HANG:` at `no-colouring-i-o-just-waits`, and it appends nothing for the features it had already
  measured.** That bench's callgrind leg runs in WSL and prints nothing for over 90 s, and the hang
  watch counts only output for a tree with `wsl.exe` in it. Alone, in the foreground, it ends in about
  265 s. Record that feature alone with `--only`, then the others.
  `until: gone benches/members/lang/concurrency/no-colouring-i-o-just-waits.nvs`
- **`bun nv bench-compare` run with `run_in_background` is killed as a `HANG:` while Docker builds
  the `nvs` image.** looper's watcher (`tools/looper/src/jobs/hang.ts`) has no `OUTSIDE` rule, so a
  compile in Docker's VM looks idle. Run a row in the foreground with a 600 s timeout. A full
  record survives in the background, unpiped, once the image is built. `--profile wait` piped into a
  file dies the same way: `docker logs -f <id>` recovers it.
  `until: gone tools/looper/src/jobs/hang.ts:WAIT_LINES`
- **A new `benches/compare.json` changes nothing on the built comparison page until `bun nv render`
  copies it to `website/src/data/compare.json`.** `bun nv site --build` reads only the copy, so a
  chart new to the record shows its empty text. Run `bun nv render`, then build, and commit the copy
  beside the record. `until: gone website/src/lib/compare.ts:../data/compare.json`
- **The `looper` on `PATH` may be an older build with no `verify` subcommand, and run in the
  background it exits 0 with only a usage error.** Run `bun tools/looper/main.ts verify`, the
  project's own copy, and read the log for `✓ 1:` before you trust the exit status. See
  `tools/looper/main.ts`. `until: gone tools/looper/main.ts:verify`
- **`bun nv soundness --probe` runs the `covws` build, not the debug build you just changed.** After a
  lowering change, the probe reports the old binary's panic for every new cell. Run it as
  `NVS_BIN=E:/mwl/target/debug/nvs.exe bun nv soundness --probe`, with an absolute path, because a
  relative one is not found. See `tools/nv/cmd/soundness.ts`.
  `until: gone tools/nv/cmd/soundness.ts:covws`
- **`bun nv proofs --id` reads the roster from `target/release/nvs`, which no build of yours refreshes.**
  A `Core` member added since the last release build prints `no feature '…'`, which reads as a quoting
  mistake. Pass `--nvs target/debug/nvs.exe`, or run `bun nv proofs --record-perf --group '<class>'`,
  which rebuilds the release binary first. See `tools/nv/cmd/proofs.ts`.
  `until: gone tools/nv/cmd/proofs.ts:const nvs = bin?.path ?? binary(named);`
- **A balance that grows by a fixed amount per spawn is a buffer freed on the scheduler's stack.**
  A task's turn charges its tree and the scheduler's stack charges none, so a `Vec` a task fills and
  the scheduler `mem::take`s is never credited back (`drain_pending`, 1.8 KB per connection). A debug
  test that spawns many children and reads `budget::live_bytes()` finds it in seconds, as the one
  below in `crates/nvs-host/src/scheduler.rs` does.
  `until: gone crates/nvs-host/src/scheduler.rs:a_parent_that_spawns_and_outlives_many_children_stays_flat`

## Writing a test case

- **`bun nv proofs --bless` rewrites an example's `.out` and not a `.nvst`'s `--EXPECT--`.** Handed a
  case, it runs the case as a program and fails with `'--events <FILE>' cannot be used with '--peer
  <FILE>'`, which reads like a CLI bug. When a message changes, bless the examples with it, edit each
  case's `--EXPECT--` with Edit, and run the cases once with `target/debug/nvs.exe test <paths>`.
  `until: gone tools/nv/cmd/proofs.ts:--bless`
- **`--bless` on a `// proof: exit 1` example accepts a compile error.** A program that does not
  compile also exits `1`, so `--bless` writes an empty `.out` and the proof passes. Run such an
  example once with `target/debug/nvs.exe run <file>` before trusting what `--bless` wrote.
  `until: gone tools/nv/cmd/proofs.ts:--bless`
- **A rewrite of `Class::method(...)` by pattern also hits a call written with elided arguments.**
  A diagnostic or a `--TEST--` line that says `parent::constructor(...)`, `Core\Html::escape(...)` or
  `Bench::run(...)` means the call, and dropping the `(...)` there changes what it says. Rewrite from
  `git show HEAD:<file>` with a list of the calls to keep, then run the touched cases once. See
  `crates/nvs-syntax/src/parser/expr.rs`. `until: gone crates/nvs-syntax/src/parser/expr.rs:names_a_method`

## Splitting a file

## Writing Novis
- **Every byte added to a `Core\Test` ledger entry is paid once per assertion for the whole of an
  `nvs run`, because nothing empties the ledger outside a test.** Stage 2 of `diagnostic-records`
  added two record nodes per failed comparison, and a hostile proof that catches a million failures
  hit the memory ceiling; only the goal-end `floor-debt` ran it. Keep anything a runner alone reads
  behind `Ctx::owned_by_test`. See `crates/nvs-runtime/src/ctx/error.rs`.
  `until: gone data/gaps/nvs-runtime/an-assertion-outside-a-test-grows-the-ledger.json:goal`
- **`nvs test apps/<app>` from the repository root reads the root's `nvs.toml`, not the app's**, so
  every `Core\Db::connect` fails with "`db.connect` ... is not granted" and the app's tests look
  broken (`apps/notes` fails the same way). Pass `--config apps/<app>/nvs.toml`, as the
  `telemetry-service` goal's `app-tests` check does. And `Core\Regex` takes no `tainted` subject, so
  validate request text with `Core\Str::trim($text, {characters: ...}) == ""` and `Str::split`.
  `until: gone apps/telemetry/src/Report.nvs:only`
- **An app's tests pass while every request under `nvs serve` is refused a read.** With no
  `[capabilities.fs] read`, `nvs test` reads any file and a request reads only the entry's folder,
  so `IO::read("../known.toml")` in `apps/telemetry` gave `500` on every report, and a
  `catch (RuntimeError)` around a read hides the refusal completely. Name each file a request reads
  outside `public/` in `read`, and run the app once under `nvs serve`, as
  `crates/nvs-cli/tests/telemetry_e2e.rs` does. A relative `read` entry is read from the folder of
  the `nvs.toml` that has it, so a copy elsewhere needs full paths.
  `until: gone crates/nvs-config/src/capability.rs:fn default_of`

## Divergences
