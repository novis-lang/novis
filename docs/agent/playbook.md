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
- **A changed parameter list on a built-in component's export needs three edits outside the
  component, and the build passes without any of them.** `wit/<name>.wit`, the component's
  `manifest.json` and `crates/nvs-ext/src/typed/<name>.rs`'s signature list must agree. A stale
  typed list fails only at call time, as "the method takes 2 arguments, and the call passed 2",
  where the first number is the typed list's. Edit all three in one slice, then run
  `cargo test --test builtin`. See `crates/nvs-ext/src/typed/pdf.rs`.
- **A package's proofs group is `package:<namespace>`, such as `--group 'package:Novis\Rest'`.**
  `--group 'Novis\Rest'` and `--group 'Novis\Rest\Client'` both answer `no group`, while
  `--id 'Novis\Rest\Client'` takes the class. A bench declares `// bench: complexity constant` when
  each round does the same work; `linear` needs a `.scale.nvs` beside it. See
  `tools/nv/proofs/roster.ts`. `until: gone tools/nv/proofs/roster.ts:package:${name}`
- **`bun nv site --questions --stamp` stamps an entry whatever its shape, and only `bun nv site
  --check questions` judges it.** An entry needs four to six questions of at most 15 words, so a
  page stamped with three passes `--status` and turns `bun nv ci --check`'s website stale guard red.
  Write four or more, and run `bun nv site --check questions` once after stamping. See
  `tools/nv/cmd/site.ts`. `until: gone tools/nv/cmd/site.ts:four to six`
- **`bun nv proofs --gate --group 'Core\X'` does not reach a value class inside it, `Core\X\Y`.**
  Each class is its own group, so a goal that gated only the outer class learns at `bun nv ci
  --check` that every member of the inner one owes all its proofs. Gate both groups from the start,
  or run `bun nv proofs --gate --limit 0` once before the gate stage. See `tools/nv/cmd/proofs.ts`. `until: gone tools/nv/cmd/proofs.ts:--group`
- **`gix`'s `blocking-network-client` feature does not bring `gix_transport`'s `Http` trait.** The
  `blocking_io::http` module sits behind gix-transport's own `http-client` feature, which gix turns
  on only with curl or reqwest. Name `gix-transport` with `http-client` alone, as the workspace row
  does, and implement the trait over `nvs_stdlib::tool_stream`. See `crates/nvs-cli/src/package/git.rs`. `until: gone crates/nvs-cli/src/package/git.rs:impl http::Http for Checked`
- **A `bun nv linux-test` piped into `grep` is killed as a hang before it builds.** The mirror and the
  WSL build run inside the VM, where the watcher sees no CPU, and a filter that drops the `Compiling`
  lines leaves it no output either, so it kills the run at 90 s. Run it with `run_in_background`,
  unfiltered, and grep its output file once it ends. See `tools/nv/cmd/linux-test.ts`. `until: gone tools/nv/cmd/linux-test.ts:linux-test`
- **An acceptance check's `must run` finds a passed test only by a `✓`, `(pass)` or `... ok` line.**
  Mocha's spec reporter prints `√` on Windows, so a green headless run read as "did not run and
  pass". `editors/vscode/scripts/headless.mjs` sets the mark to `✓`; a new mocha runner does the
  same. See `tools/looper/src/verify/accept.ts`. `until: gone editors/vscode/scripts/headless.mjs:symbols.ok`

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
- **An attack's memory `FATAL` is red past 1.5x its ceiling plus 8 MiB, and `bun nv proofs --overshoot`
  lists every reading, worst first.** See `tools/nv/proofs/run.ts`. `until: gone tools/nv/proofs/run.ts:OVERSHOOT_FACTOR`
- **A green `bun nv affected --run` leaves the proofs a change reaches owed, and `bun nv debt` pays them
  only once DONE is claimed.** A `Core` member change that turned an attack red passed the gate and
  then refused DONE. Before DONE, run `bun nv proofs --run --only '<member>'` for each member the goal
  changed. See `tools/nv/cmd/affected.ts`. `until: gone tools/nv/cmd/affected.ts:stay owed`
- **Never end a shell call inside a scratch git repository under `.agent-tmp/`.** The guard hook
  finds `tools/looper/main.ts` from the working directory's git root, so once the shell's directory
  stays in a nested `git init` folder, every later Bash call fails with *Module not found*, and
  removing that folder is refused while it holds the working directory. Run git in it with
  `git -C <dir>` or `cd <dir> && ... && cd <repo>` in one call. `until: gone tools/looper/main.ts:run(process.argv`
- **`hyper` reads the socket while a request's service is pending, once the body is done.** That
  read is its EOF probe (`mid_message_detect_eof`), and any socket wait armed on it closes a slow
  program with no reply. A served test that goes red only under load with an empty answer, or a
  client error like `no header section ended it`, is that shape. `Phase::Run` in
  `crates/nvs-server/src/io.rs` is the phase with no wait; a new phase change keeps the probe out of
  every idle wait. `until: gone crates/nvs-server/src/io.rs:Run,`
- **Git Bash rewrites a leading `/` argument into a Windows path.** `bun nv site --questions --stamp
  /install/server/` arrived as `C:/Program Files/Git/install/server/` and stamped nothing. Prefix the
  call with `MSYS_NO_PATHCONV=1`. `until: gone tools/nv/cmd/site.ts:--stamp`
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
- **`bun nv proofs --run --only` with several ids crashes in `suiteLines` (`r.verdict` of
  undefined), because the pass has no result for one of the files.** Run one `--id` per call
  until it is fixed. `until: gone tools/nv/proofs/run.ts:pass.results.get(\`${what}:${f}\`)!`
- **A `--group` for `--record-perf` is one class, not a namespace.** `--group 'Core\Http'` measures
  only `Core\Http`'s own members, and `Core\Http\Client`'s stay stale. Take the classes from
  `bun nv proofs --gate --limit 0`'s list and give each its own `--group`
  (`tools/nv/cmd/proofs.ts`).
  `until: gone tools/nv/cmd/proofs.ts:--group`
- **A "comments only" change still makes a feature's perf figure stale when it touches a string.**
  The figure's hash drops comments but keeps every string literal of the implementing file, and a
  reference chapter is hashed whole. A sweep that rewrites a message or a panic text there turns
  the proofs gate and the page stamp red at the goal end's push. In the same commit, run
  `bun nv proofs --record-perf --group <class>` for each class the gate names, and
  `bun nv site --stamp <page>` once the page still agrees (`tools/nv/proofs/collect.ts`).
  `until: gone tools/nv/proofs/collect.ts:without comments, layout, test code`
- **A bench's closing `Bench::run` passes the same literal as its `// bench: iterations` line.**
  An edit to one alone makes the ramp skip the bench, and `--record-perf` counts against the
  wrong N, so a bench with no allocation fails `allocations 0` (`tools/nv/cmd/scaling.ts`).
  `until: gone tools/nv/cmd/scaling.ts:does not pass \`iterations\` as one literal`
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
  `NVS_BIN=<repo>/target/debug/nvs.exe bun nv soundness --probe`, with an absolute path, because a
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
- **A test that passes alone and fails in the suite is shared state, and the first suspect is a
  pool.** A `UNIQUE` failure on a row a test wrote into its own `:memory:` database was another
  test's database, handed back by the per-core connection pool. Run the failures alone with
  `--rerun-failed` first; a pass there means two tests meet in a pool, a file or a shared table.
  See `crates/nvs-stdlib/src/db/pool.rs`. `until: gone crates/nvs-stdlib/src/db/pool.rs:is_private`

## Splitting a file

## Writing Novis
- **In a `jsonAs<{...}>` shape, `key: ?T` means the key must be there and may be `null`; a key a
  server may leave out is `key?: T`, read with `??`.** `expires_in: ?int` type-checks and then
  throws "2 value(s) of `$shape{...}` did not match" on a token reply without `refresh_token`.
  Comparing a `key?: T` field with `null` is E0466, so read it as `$reply->key ?? ''`. Namespaced
  code imports `RuntimeError` with `use RuntimeError;`, since `\RuntimeError` is E0240.
  See `packages/rest/src/Token.nvs`. `until: gone packages/rest/src/Token.nvs:refresh_token?: string`
- **A script under an app's `jobs/` runs on every `Core\Test::request` of `nvs test <app folder>`,
  so each request answers `500` with no body.** The folder's generated entry requires every file no
  other file requires, and the request runs that entry. It shows as "`Response::json()`: the program
  wrote no body" in every test that signs up; `apps/registry` was red from its first `jobs/` file
  until the runner ran `public/index.nvs` alone. Prove it by moving `jobs/` aside for one run.
  `until: gone docs/goals/registry-service.md:A test request runs the request entry`
- **A scheduled script's `[[schedule]]` must be `scope = "host"` in an app with no shared store,
  and `nvs check <app folder>` reads every entry file at once.** `fleet` is E0611 at boot. A
  `jobs/` script with the same `autoload` line as `public/index.nvs` was E0315 until the checker
  took same-root lines as one home, and the per-file `nvs check` a session ran never showed it.
  Check an app with a new entry file by its folder: `nvs check apps/<app> --config apps/<app>/nvs.toml`.
  `until: gone crates/nvs-hir/src/autoload.rs:the same roots are one place`
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
- **A route that calls `Core\Session::start()` after its gate did throws, and the route's
  `catch (RuntimeError)` turns it into an ordinary 400.** `apps/registry`'s sign-up failed as "the
  passkey could not be checked" until `Gate::allows` became the one `start()`; there is no
  `isStarted`. Two more there: rows a `Core\Test::request` writes outlive `#[Test(db:)]`'s rollback,
  because the request is its own run, and an arrow closure appending to an outer array appends to a
  copy. See `apps/registry/src/Gate.nvs`. `until: gone apps/registry/src/Gate.nvs:Core\Session::start`

## Divergences
