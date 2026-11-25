# Playbook — the traps this repository has actually sprung

Hard-won specifics: things that cost a real session real time, written down so the next one does not
pay again. A bullet is a *trap* — "this looks like it should work and does not, and here is what to do
instead" — in three sentences, ending with the `until: <kind> <arg>` trailer that says what retires it
([conventions.md](conventions.md) § *A playbook bullet*; the kinds are looper's `docs/goal-format.md`
§ *The playbook*, in looper's repository, and the kit's `tools/looper/src/goal/playbook.ts` parses
them). **It is not a changelog.** The session that hit the trap, the stage it was on and what it tried first
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

- **A `changes.json` entry for a changed feature needs a test with a `// covers:` line for it.**
  A feature's behaviour hash reads only the tests that cover it. In `static-in-callables`, three new
  conformance cases had no `// covers:` line, so `site --check changes` said the feature "has not
  moved", and a DONE was refused. Give each new case the `// covers:` line of the feature it pins.
  See `tools/nv/lib/texts.ts` `behaviour()`.
- **A change to a test makes every member its `// covers:` line names stale on the website.** In
  `extend-blocks` stage 8, a Miri ignore on nine `Core\Time` tests made seven members' `about.md`
  and help stale, and `close` listed only some of them, so it took two close runs. After a test-only
  change, run `bun nv site --check stale` alone. Then stamp every key it names with
  `bun nv site --stamp`, once the texts are still right. See `tools/nv/cmd/site.ts`.
- **`cargo test --lib <name>` never runs a unit test of `nvs-cli`, because it is a binary crate.**
  In `extend-blocks` stage 6, the check `agent` reported "did not run" for a test that passed. Every
  test filtered out, and the result still said `ok`. A goal check over a test in `crates/nvs-cli/src/`
  runs `cargo test --bin nvs <name>`. See `crates/nvs-cli/Cargo.toml`'s `[[bin]]`.
- **A reread that adds a goal's facts to an `about.md` can push it past 200 words.** In
  `route-values` stage 10, nine files went over after a stale-stamp pass, and only
  `bun nv proofs --gate` caught it, after they were stamped. Tell a writing subagent the bound
  (at most 190 by `wc -w`), and run the gate before you stamp. Cut a key from `site --check stale`
  at its first `: `, because a feature id holds `::`. See `tools/nv/cmd/proofs.ts`.
- **A `bun nv splice` patch written through a shell heredoc can lose a backslash.** In
  `route-values` stage 5, `"#[Core\\Query]"` in a Rust string reached the patch file as `Core\Query`,
  though the heredoc was quoted, and the build then failed on an unknown escape. Write a patch
  that holds a `\\` with the Write tool, or edit that line with Edit. See `tools/nv/cmd/splice.ts`.
- **`site --review --stamp help:<id>` hashes the help text of the built binary, not the source.**
  In `time-values` stage 7, three subagents rewrote help texts in `time.rs` and the intl
  `manifest.json` and stamped them with a stale `target/debug/nvs.exe`. Each stamp recorded the old
  text, and the review went red again. Edit every help text first, run `cargo build` once, which
  also rebuilds a built-in component's manifest, and then stamp. A writing subagent that may not
  build lists its help keys for the session to stamp. See `tools/nv/lib/texts.ts` `card()`.
- **A goal check over a crate under `extensions/` needs `cwd: extensions/<name>`.** The workspace
  excludes `extensions`, so `cargo test --lib <filter>` at the root runs green with none of the
  check's tests, and `looper check` finds none of the names it needs. ICU4X's baked `Baked` providers do
  not list their locales either: a test that walks every locale invokes the data crate's
  `impl_*!(Probe, ITER)` macro on a probe type of its own. See `extensions/intl/src/dates.rs`
  `every_dates_locale` and `tools/looper/src/verify/accept.ts` `cwdOf`.
- **`site --check questions` owes a new member's page only once `render --website` has written
  `core.json`.** In `reflect-once`'s close, the render and the questions were red together, and the
  questions named only the stale class page. After the render, two owed member pages appeared and
  cost a second round. Run `NVS_BIN=target/debug/nvs.exe bun nv render --website` before reading
  the questions list. See `tools/nv/cmd/site.ts` `questionSources`.
- **A rename of a common member name finds its calls with `nvs check`, not with a grep, and the
  unknown-method error's span covers the receiver, not the name.** `api-rename` listed 4,234
  `->add(`/`->format(` calls for stage 6, most on other classes. Renaming the declaration and then
  fixing each `E0405 \`<Class>\` has no method named \`<old>\`` found every real call, including those
  hidden behind an earlier error. A script that compared the span's text with the old name fixed
  nothing, because the span is `$book`. Match `->old` right after the span's end instead. See
  `nvs check --json`'s `labels[].span` and `tools/nv/cmd/api-rename.ts`.
- **`bun nv close` prints only the first lines of a red check's list, and a rename makes every
  page's search questions stale.** `close` showed 17 stale questions, and `site --check questions`
  alone showed 538: the renames moved the text hash of every card, `about.md` and example they
  touched. Run the red part alone for the whole list. Then rewrite only the questions that name an
  old member, and stamp the rest with `MSYS_NO_PATHCONV=1`, because Git Bash turns a `/reference/…`
  URL into a Windows path. See `tools/nv/cmd/site.ts` `questionSources`.
- **An example app's ignored files grow across local runs, and `site --check apps` once read them.**
  `apps/registry/registry.sqlite` reached 113 MB, so the backup tests' dump passed the memory
  ceiling and `close` went red where CI was green. `inApp` now moves every untracked file under the
  app to `.agent-tmp/` for the run and puts it back. See `tools/nv/cmd/site.ts` `inApp`.
- **`cargo test --lib -- <filter>` never runs `nvs-cli`'s unit tests, because they live in the
  `nvs` binary.** A run filtered on `reload_cmd` printed only other crates' results. Run
  `cargo test --bin nvs -- <filter>` for a test under `crates/nvs-cli/src/`. See
  `crates/nvs-cli/Cargo.toml`'s `[[bin]]`.
- **`bun nv splice` edits a file another live session claims, and only `git add` refuses it.** The
  edit guard checks Edit and Write, not a shell tool. A loop session spliced its goal file while an
  interactive session still claimed it, and then could not commit its handoff. Run `looper claims`
  before a splice that touches a goal file. See `tools/looper/src/claims/claims.ts`.
- **A Python edit script under `.agent-tmp/` that writes with `open(p, 'w')` turns every line
  ending into CRLF on Windows.** Git then warns "CRLF will be replaced by LF" for each file, and the
  working copy no longer matches rustfmt's output. Open the file with `newline=''`, or write bytes.
  Pass `encoding='utf-8'` too: the default is cp1252, so an `in` test on a string with a `—` fails
  on a file that holds it. See `.gitattributes`.
- **A check's `tests:` name for a bun test inside a `describe` is written `describe > test`.** Bun
  prints `(pass) the plan > counts ...`, and looper looks for the name as a whole substring of that
  line. `the plan counts ...` never matched, so a green run was refused as "did not run and pass".
  Try a new check with `looper check --name <id>`. See `tools/looper/src/verify/accept.ts` `testPassed`.
- **`site --check changes` counts a feature as moved only through tests that carry `// covers:
  <feature>`.** A new `.nvst` case for `Core\Html::later` had no marker, so its `changes.json`
  entry was refused as "has not moved". Add the marker under `<?nvs` in the case, not a
  `highlight`. See `tools/nv/lib/texts.ts` `behaviour`. Only `.nvst` cases, Rust tests and
  package `.nvs` tests are read: an `.lspt` case or an extension TypeScript test never moves an
  editor feature, so put the marker on a Rust test that pins the change (`MARKER_ROOTS`,
  `tools/nv/proofs/markers.ts`).
- **A new `#` heading in a reference chapter adds a feature to the roster, and that feature owes
  every proof.** `# nvs test --since` in `docs/reference/tools/10-cli.md` made
  `tools:cli/nvs-test-since`, and `site --check changes` and `--check stale` went red. Write a
  section of an existing feature as `##`. See `bun nv proofs --id <feature>`'s *implemented at*.
- **A `cd` into a folder under `target/` that ends a successful call moves the session's shell, and
  then the guard hook cannot find `tools/looper/main.ts`, so every Bash call fails, `cd` included.**
  Inspecting a test's scratch checkout did this. Never `cd` there: pass `-C <dir>` to git, or give
  the folder as a path argument. See `.claude/settings.json`'s guard hook.
- **Git Bash rewrites a leading `/` argument into a Windows path.** `bun nv site --questions --stamp
  /syntax/values/enums/` reached the tool as `C:/Program Files/Git/syntax/...` and stamped nothing.
  Prefix the call with `MSYS_NO_PATHCONV=1`. An app's tests run as `nvs test .` from the app's own
  folder with `NOVIS_NO_INIT=1`, as `site apps` runs them. See `tools/nv/cmd/site.ts`.
- **`bun nv suite --case <file>` also reruns every case that is still red, and its line names only
  three.** A check over three new cases reported 21 red, none of them the three. Run the named
  files with `target/debug/nvs.exe test <files>` to read their own verdict and diff. See
  `tools/nv/cmd/suite.ts`.
- **A database error's `to_string()` never contains the server's own sentence.** `ServerError`'s
  `Display` prints only the backend, severity and kind, so a check for "no such table" in it is
  always false. A probe in the session store's open failed every fresh SQLite file that way. Read
  `nvs_db::conn::ServerError::of(&err)` and its `message`. See `crates/nvs-stdlib/src/session/sqlite.rs`
  `opened`.
- **A new `Core` member's symbol needs a row in its module's address `match`, or every program
  panics at startup.** The build, clippy and the member's Rust test are green without it, and
  `bun nv suite` then reports thousands of red cases. The panic reads `registers ... with no
  implementation address`. Add the `"nvs_core_..." => (fn as *const ()).cast()` row beside the
  member's neighbour. See `crates/nvs-stdlib/src/lib.rs` `entry_of`.
- **A new syntax node kind needs its `Core\Ast` class, at the same index.** `AttributeDecl` went
  into `nvs_syntax::walk::KINDS` alone, and every `Core\Ast` class after it named the next
  production: `File` came back as `Float`. The build and the parser tests were green, and two
  conformance cases went red a stage later. Add the name to `productions!` in
  `crates/nvs-stdlib/src/ast.rs`, in `KINDS`'s order.
- **A bigger string header can push a hostile proof's early step past the memory ceiling, and only
  `floor-debt` runs every proof.** The start word made each one-letter string 49 bytes, and the
  `chunk` and `graphemes` attacks hit the FATAL at step 1, not their last step. The gate and `ci
  --check` were green, and the retire was refused. After a layout change, run `nvs run` on each
  attack that makes millions of small values. Shrinking its input is refused as loosening a test.
  See `tests/hostile/core/Str/chunk/`.
- **A cut in what a loop or a value allocates can stop a hostile proof's `ends-early` step from
  reaching its FATAL, and only `floor-debt` runs it.** Once a collection loop allocated nothing at its
  start, 256 nested loops over a set held no copies, and the attack printed its depth. The retire was
  refused. After an allocation cut, run `nvs run` on the attacks that cover the feature. Make the step
  force the cost it names, never drop the step. See
  `tests/hostile/lang/iteration/core-collections-are-iterable/`.
- **A suite that is red only through `bun nv` and green from the shell has `LLVM Profile Error`
  lines in each case's stderr.** The covws `nvs` writes its counters into a shared discard pool, and
  a profile an older build left there fails to merge. Every case that expects an error then fails,
  811 of them once. Print one failing case's whole report before searching the compiler. The sweep
  now empties the pool after its build. See `tools/nv/lib/covws.ts` `emptyDiscardPool`.
- **`looper check --name <text>` matches by substring, and does not count a `should panic ... ok`
  line as a pass.** `--name table` ran `table`, `differential` and `members`. A test that must panic
  catches it with `catch_unwind` and asserts the message. See
  `crates/nvs-runtime/src/callable.rs`.
- **A goal that changes a `Core` member's source file makes that member's perf record stale, and
  `bun nv ci --check` shows only the first 29.** Recording those 29 brought the next batch to
  light. List every one with `bun nv proofs --gate --limit 0 --nvs target/debug/nvs.exe`, grepped
  for `perf: stale`. Then build the release binary with `bun nv relink --build`, and record them in
  one run with `xargs -d '\n' -a <list> bun nv proofs --record-perf --nvs target/release/nvs.exe
  --only`. Any edit of a source, even `bun nv render` of a chapter, makes it stale again, so edit
  and render first and measure last. See `tools/nv/proofs/collect.ts` `perfAny`.
- **A `.scale.nvs` ramp whose last three sizes start below where buffers settle reads a linear
  cost as faster than linear.** Three `Session` scale benches failed at `max 4096` with increments
  2.0, 7.9 and 5.0 bytes per unit. The increments were 5.0 at every size from 2048 up. A hand run
  of `nvs run --count` shows the tool's figures only with `--request <bench>.nvsr --config
  nvs.toml` and `NOVIS_NO_FILE_CACHE=1`, as `spawnProof` passes them. Raise `max` until the last
  three increments agree, which makes the ramp longer and never looser. See
  `benches/members/core/Session/setShortLived.scale.nvs`.
- **A chapter example's expected output is its `.out` under `docs/examples/`, not the chapter.**
  A member that allocates less changes a bench example's `count:` line, and `bun nv reference`
  fails on the chapter. Editing `docs/reference/lang/*.md` makes `bun nv render --check` fail
  next. Edit the `.out`, bless the matching `.term` snippet with `bun nv site --bless`, and run
  `bun nv render` and `bun nv render --website`. The chapter's feature perf records then go stale,
  and the pages that cover the feature need `bun nv site --stamp`. See `tools/nv/cmd/reference.ts`.
- **Three readers parse `.github/workflows/build.yml`'s matrix: `bun nv ci`, the supply-chain test
  and `extension_release`.** A workflow edit that changed the matrix's shape made 33 tests of
  `tools/nv/test/ci.test.ts` throw, and a later one broke the other two, and no session saw it until
  the gate. After a change to `build.yml`'s matrix, run `bun test tools/nv/test/ci.test.ts
  tools/nv/test/supply-chain.test.ts` and `cargo test --test extension_release` by name. See
  `tools/nv/cmd/ci.ts` `parseLegs`.
- **`cargo test --lib` runs none of `nvs-cli`'s tests.** The crate has only the `nvs` binary, so a
  test in `crates/nvs-cli/src/` runs under `cargo test --bin nvs`. A goal check that names such a
  test with `--lib` finds zero tests and fails. See `crates/nvs-cli/Cargo.toml`.
- **The Edit tool refuses a whole file for one byte that is not UTF-8.** A goal file carried one
  Latin-1 `§` (byte `a7`), and no edit of its handoff was possible until it was fixed. Find the line
  with a Python `decode('utf-8')` per line, and replace the bytes with a script under `.agent-tmp/`
  that reads and writes bytes. See `docs/goals/`.
- **The Edit tool drops a trailing space at the end of `new_string`.** A `replace_all` of
  `natural(&mut ` to `open_natural(&mut ` wrote `open_natural(&mutpermutation`, and the build failed
  at five sites. End both strings on a character that is not a space, such as the argument after
  it. See `crates/nvs-stdlib/src/sort.rs`.
- **The Write and Edit tools turn a backslash, `u` and four hex digits into that character.** A JSON
  test text with an escape, written in a Rust raw string, came out holding the character itself, and
  a lone surrogate's escape became a different code point. Write the escape in a plain string with a
  doubled backslash, which arrives as written, and grep the file for the escape after the write.
  See `crates/nvs-stdlib/src/json/reader/tests.rs`.
- **A Python script under `.agent-tmp/` writes CRLF on Windows when it opens a file in text mode.**
  A replace script turned two whole Rust files to CRLF, and its `\n` patterns stopped matching, so
  half its edits were silently skipped. Open both ways with `newline=''`, or read and write bytes,
  and check `git diff --stat` for a whole-file change after the script. See `.gitattributes`.
- **`bun nv splice --patch` writes a `\\` in a patch's NEW block as one `\`.** A Rust string such as
  `"Core\\Env::get"` lands as `"Core\Env::get"`, and the build fails with `unknown character escape`.
  Write a `Core` name in a patch as a raw string, `r"Core\Env::get"`, or make that edit with Edit.
  See `tools/nv/cmd/splice.ts`. `until: gone tools/nv/cmd/splice.ts`
- **A scratch script's `String.replace` or `replaceAll` with a string replacement reads `` $` ``,
  `$'` and `$&` in it as patterns.** An anchor such as "without the `` `$` ``," copied into the
  replacement inserts the whole text before the match, and one edit of
  `crates/nvs-stdlib/src/reflect.rs` grew it by 1700 lines. Write `src.split(anchor).join(text)`,
  which is literal, and check `git diff --stat` after the script.
- **`looper lint` reads a `### Next` item's anchor on its first line alone.** An item whose
  `path:line` sits on a continuation line, or behind a bare directory path, fails as "an open
  `### Next` item has no repo-rooted path:line anchor". Start the item with its anchor, as in
  `docs/goals/calls-by-name-are-checked.md`'s `### Next`.
- **A shell heredoc in this harness writes `\\` as one backslash, even under `<<'EOF'`.** A patch
  for `bun nv splice` or a scratch script that holds `"\\\n"` arrives as `"\n"`, so a JS string
  loses its escaped backslash and a test fails for a reason the code does not have. Write any text
  with a doubled backslash with the Edit or Write tool. See `tools/nv/test/site.test.ts`.
- **The Edit tool turns a single backslash, `u` and four hex digits into the character they name.**
  A Rust raw string `r#"..."#` that should hold the six-byte escape for `<` gets a literal `<`, so
  the test compares the output against itself and fails. Write such an expected value as an ordinary
  string with a doubled backslash, such as `"\\u003c"`, which the tool leaves alone, and grep the
  line after the edit. See `crates/nvs-stdlib/src/json/pieces.rs`.
- **On Windows a pipe is a file to `Metadata::is_file`, and a seek on it succeeds and moves
  nothing.** A "can this seek" check written as `meta.is_file()` passes on Linux and reads a pipe's
  later bytes as its start on Windows. Ask `nvs_runtime::os::can_seek`, which reads `GetFileType`.
  `until: gone crates/nvs-runtime/src/os.rs:pub fn can_seek`
- **A new `CoreTy` variant breaks exhaustive matches that only test builds compile.** `cargo build` is
  green, and then `cargo test --lib` fails in `mentions` (`crates/nvs-stdlib/src/ast.rs`,
  `crates/nvs-stdlib/src/xml.rs`) and `every_text_position_is_tainted` (`xml.rs`). Check a new
  variant with `cargo clippy --all-targets` before the first test run. See `crates/nvs-stdlib/src/registry.rs`.
  `until: gone crates/nvs-stdlib/src/xml.rs:fn every_text_position_is_tainted`
- **`git mv` of a whole folder fails with "Permission denied" on Windows while an editor watches it.**
  The user's VS Code language server can hold the directory. Move each tracked file with
  `git mv <file> <new path>` into folders you create, then delete the old folder once it is empty.
  See `tools/nv/cmd/ext-templates.ts`.
- **Git Bash on Windows rewrites a leading-slash argument into a path under its own install.**
  `bun nv site --questions --stamp /reference/core/json/` arrives as `C:/Program Files/Git/reference/...`
  and answers "no built page has this URL". Prefix the call with `MSYS_NO_PATHCONV=1`. See `tools/nv/cmd/site.ts`.
- **A union in a diagnostic prints its members in intern order**, so a new `Core` registry type can
  reorder a message such as `tainted bytes|bytes` and turn a conformance case that pins it red. Update
  the case to the new text. See `crates/nvs-types/src/ty.rs`. `until: gone crates/nvs-types/src/ty.rs:ids.sort_by_key(|id| id.0)`
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
- **Any edit to a hand-written page makes its questions stale, even one that only adds fence meta
  such as `skip="..."`.** The entry's hash is the whole file, so a sweep over many pages turns
  `bun nv ci --check`'s website stale guard red at the goal's end. Run `bun nv site --check questions`
  after such a sweep. Where the reader sees no change, stamp every page it names. See
  `tools/nv/cmd/site.ts`. `until: gone tools/nv/cmd/site.ts:a handwritten page its key and its file`
- **An edit to a `docs/examples/` file makes its reference page's questions stale too.** A fix to
  `Core\Storage::delete`'s third example turned the website stale guard red, and the goal's DONE
  was refused. After any example edit, run `bun nv site --check questions`, and stamp the page it
  names with `MSYS_NO_PATHCONV=1`. See `tools/nv/cmd/site.ts`.
- **`bun nv proofs --gate --group 'Core\X'` does not reach a value class inside it, `Core\X\Y`.**
  Each class is its own group, so a goal that gated only the outer class learns at `bun nv ci
  --check` that every member of the inner one owes all its proofs. A goal's check is `bun nv proofs
  --gate --goal <slug>`, which reaches both. See `tools/nv/proofs/since.ts`. `until: gone tools/nv/cmd/proofs.ts:--group`
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
- **A member made cheaper can fail its old `.scale.nvs`, because the sibling counts its setup per
  op.** `Core\Session::get` became constant, and `get.scale.nvs` still built a growing value inside
  `run`, so the ramp read slope 1.39 and refused the record. Declare `complexity constant` and give the
  sibling `// bench: rounds N` with `Bench::run(start, N)`, so half and full rounds cancel the setup.
  See `benches/members/README.md` and `benches/members/core/Session/get.scale.nvs`.

## Running things
- **An `nvs-suite` check runs every case the change reaches, not only its `--case` list.** In
  `enum-tables`' DONE, `cases` went red on three `image-*` cases, which passed when run alone.
  Run with `nvs test tests/conformance/novis/`, one printed os error 1455: each process's
  WebAssembly engine committed a 1.25 GiB table pool, and the page file ran out. Run the red
  cases' directory with `nvs test` to read the error. See `crates/nvs-cli/src/extensions.rs`
  `Requests`.
- **`looper check --name <check>` can wait for a heavy-job slot that another project holds, and
  print nothing.** In `enum-tables` stage 4, the check `cases` started no child for ten minutes
  while another project ran a release build. The check's own `run:` line, run directly as
  `bun nv suite ...`, went green in a few minutes. When a check prints nothing for minutes, look
  for its child process before you wait longer. See `tools/looper/src/verify/accept.ts`.
- **`--ignored` and `--nocapture` go after `--` in `bun nv linux-test`, or cargo stops at
  "unexpected argument".** A goal wrote `linux-test --release -p <crate> --lib <name> --ignored --
  --nocapture`, and the first probe run failed before it built. Write `--lib <name> -- --ignored
  --nocapture`: both are the test binary's flags. See `tools/nv/cmd/linux-test.ts` `testLine`.
- **`nvs check <directory>` of loose example scripts reports none of a new warning that `nvs check
  <file>` reports in each one.** A sweep for a new `W1xxx` over `docs/examples/` or `tests/hostile/`
  checks each file on its own; only an app with an entry (`apps/registry/public/index.nvs`) is checked
  whole. From Git Bash the binary is `target/debug/nvs.exe`, because `nvs` without `.exe` is refused
  as an exec format error. See `crates/nvs-cli/src/main.rs`.
- **A goal that makes a type stricter can be closed without a gate, and `ci --check` is green while
  CI is red.** taint-fail-closed made an error's message and several `Core` answers tainted and was
  retired in a chain. `ci --check` does not run the Rust tests, the conformance tree or the hostile
  proofs, so 17 jobs went red on the next push. `bun nv ci --watch --next` stops at the first red
  job: wait for the whole run with `gh run watch <id>`, then fix every red in one pass. See
  `tools/nv/cmd/ci.ts`.
- **`bun nv verify` stops at its first red step, so a full gate that only reruns it finds one
  failure per run.** A gate spent three runs on `render --check`, `rules --render --check` and
  `layout --check` in turn. After a red doc step, run the rest of `DOC_CHECKS` by hand before the
  next run: `bun nv render --check`, `rules --render --check`, `links`, `layout --check`,
  `release --check`. See `tools/nv/cmd/verify.ts` `DOC_CHECKS`.
- **Miri does not run on the Windows host.** `cargo +nightly miri test` stops at `cargo uses an
  argfile to invoke rustc, which is not supported by cargo-miri`, after minutes of building into
  `target/miri`. A Miri timeout cannot be timed here: cut the `cfg!(miri)` input by a wide margin,
  delete `target/miri`, and let the next nightly judge it. See `156f11a7ca`.
- **`git add <file> && git commit` commits everything already staged, and a writing subagent's
  `git mv` stages its moves.** A one-file Miri fix took 46 renames and deletions two subagents had
  staged in the same tree. While writing subagents run, commit a slice with `git commit -F <msg>
  <paths>`, which commits only those paths. See `9478fd2f66` and `crates/nvs-stdlib/src/granularity.rs`.
- **A whole-suite run grepped five ways is five whole-suite runs.** A close's gate stage spent
  about 20 minutes rerunning the conformance suite through a different `grep` each time, only to
  see other lines of one red. Write a run of minutes once to `.agent-tmp/<name>.txt` and grep the
  file. A red `bun nv suite --no-*-too` run already saves its output under `.agent-tmp/suite/` and
  names the failing cases, so read that log and run one named case alone.
- **An owning member's `apply` cannot throw, so a `let _ =` there drops the allocator's refusal.**
  `Core\Arr::padStart`/`padEnd` returned a short array for a run the process could not hold, and
  only two `count-shaped-producers-*` conformance cases saw it, at the gate. Build or reserve what
  can fail in `check`, and let `apply` only move it. See `crates/nvs-stdlib/src/arr.rs` `Padding`.
- **A test binary that `bun nv verify` reports as `exit 116` with no panic hit heap corruption.**
  116 is the low byte of Windows' `0xC0000374`, `STATUS_HEAP_CORRUPTION`, and it shows only when
  the tests run in parallel. Bisect the test list with `cargo test -q --lib -- --exact <names>`, a
  few runs per half. Miri cannot run on the Windows host, and the test binary run directly exits 127
  without cargo's `PATH`. The cause was a member that walked a borrowed value with no reference of
  its own. See `crates/nvs-runtime/src/owned.rs` `run_borrowed`.
- **A thread's `thread_live_bytes` does not show what a compile keeps.** The JIT compile added
  5.9 MB on the compile thread, but the artifact's object payload, built on the scoped `nvs
  artifact` thread, is freed there too, so the thread read +143 KB per compile. The units' bytes
  then showed as negative balances on whichever core freed them. Measure the step itself, around
  `nvs_codegen::compile` or a `Drop`, and not a thread's running balance. See
  `crates/nvs-cli/src/cache.rs`'s `compile_beside_object`.
- **A buffer filled by `resize` then `read` zeroes a whole block per fill, and a scale bench sees
  it as growth.** `readLine`'s read-ahead resized to 8 KiB before every read, so a short line paid
  for 8 KiB of zeroing and the slope read 1.26. Keep the buffer's length and track where the read
  bytes end. See `crates/nvs-runtime/src/ctx/held.rs`'s `ReadAhead`.
- **A test verify calls "failed alone as well" can pass under `cargo test`.** Verify reruns it
  with its own recording environment, `NVS_FOOTPRINT_LOG` and `LLVM_PROFILE_FILE` among them, and
  `cargo test` sets neither. Reproduce with the `covws` test executable and those variables set
  before reading the code for a cause. See `tools/nv/select/record.ts`'s `env`.
- **`nvs check --count` run from the repository root counts the repository's own `nvs.toml`.**
  Its configuration phase showed about 34,000 allocations, where a start with no `nvs.toml` of its
  own makes about 400. Run a start count from `.agent-tmp/`, with `target/debug/nvs.exe` on
  Windows (`target/debug/nvs` there is a Linux build). `run --count` prints its `phases:` line
  before the TLS client and the run, so a mark placed after the compile shows only through its own
  `eprintln!` of `phases::line()`. See `crates/nvs-cli/src/phases.rs`.
- **`cargo test --lib` runs none of `nvs-cli`'s tests, and says `0 passed` for every crate.**
  `nvs-cli` is a binary crate with no library target, so a test in `crates/nvs-cli/src/` runs under
  `cargo test --bin nvs -- <name>`. A goal check whose test lives there runs
  `cargo test --lib --bin nvs -- <name>`, or its `must run` fails with the test green.
  `until: gone crates/nvs-cli/Cargo.toml:[[bin]]`
- **A value `nvs run` reads off the context before `ctx.set_config` is the empty configuration.**
  The `--events` publisher took its configuration there, so its topics had no application and the
  `Core\Sse\Message` examples printed nothing, which only `floor-debt` caught. Read anything
  configured after `set_config`. See `crates/nvs-cli/src/main.rs`.
- **`bun nv try` runs the coverage build's `nvs`, not `target/debug/nvs`, so a runtime change is not
  in what it prints.** A refusal added to the decoder still showed the old property-count message.
  Run `cargo build --bin nvs` and then `NVS_BIN=target/debug/nvs.exe bun nv try <file>`, or
  `target/debug/nvs.exe test <case.nvst>`. See `tools/nv/cmd/try.ts`.
  `until: gone tools/nv/cmd/try.ts:covwsNvs`
- **A Docker Desktop restart in the middle of a test run makes the server examples fail with
  "deadline passed" or "failed to fill whole buffer".** Its port proxy still accepts on 15432 and
  16379 while the compose containers are down, so `examples.rs` thinks the servers are up. If
  `docker ps -a` shows `novis-db-*` with `Exited (255)`, run `docker compose -f
  tests/db/compose.yaml up -d --wait redis postgres` and run the test again. See
  `crates/nvs-cli/tests/examples.rs`.
- **Format with `cargo fmt --all`, never `rustfmt <file>`.** Plain `rustfmt` does not read the
  workspace's settings, so it rewrites lines CI's `cargo fmt --all --check` accepts. `cargo fmt
  --all --check` is the check CI runs, and it found `fuzz.rs` and `segments.rs` unformatted after
  sessions that ran only clippy. `until: gone tools/nv/cmd/ci.ts:cargo", "fmt`
- **A source edited while `bun nv bench-compare` builds its image can be skipped by the next image
  build.** The image's `target` is a cache mount, and cargo judges a crate fresh by mtime. A build that
  started before the edit compiles the old source into an artifact newer than the edited file, so the
  next build keeps it, and a changed signature fails to compile in a crate that did rebuild. `touch`
  each file you edited since the last image build before building again. See
  `benches/proxied/Dockerfile`. `until: gone benches/proxied/Dockerfile:target=/src/target`
- **An access violation in a Windows optimized build is located without a debugger by a temporary
  vectored exception handler, not by a backtrace.** The proof profile writes no PDB, so
  `std::backtrace` names only system frames. Install one with `AddVectoredExceptionHandler` in the
  test file. Print the record's address, its read or write target, `rsp`, and the TEB's stack bounds
  (`gs:[0x8]`, `gs:[0x10]`), then return `0` to keep searching. Bracket the fault with `eprintln!`
  probes in each `Drop`. Remove all of it before the commit. See `crates/nvs-host/src/stack.rs`.
  `until: gone crates/nvs-host/src/stack.rs:fn native_frame`
- **When most proof programs judge red with `exit 5` and correct output, a native teardown crashed
  at exit on Windows.** `5` is the low byte of `0xC0000005`. Git Bash shows it as `Segmentation
  fault`, exit 139, once you run the proof binary on one example with its output in a file. The
  debug binary may not crash, so a green `cargo test` proves nothing here. Run
  `bun nv proofs --id '<feature>' --run --show` on one red feature, then compare a run with the
  root `nvs.toml`'s `[queue] workers` at `0`. See `crates/nvs-cli/src/worker.rs`. `until: gone crates/nvs-cli/src/worker.rs:CONNECT_DEADLINE`
- **A PDF layout step that measures a box's content widths, then lays it out, is exponential in
  nesting unless each measurement is kept, and a by-name run of the stage's own test never shows
  it.** Auto-width tables measured each cell twice per level, so 20 nested tables spun the
  `render` suite for 600 s while every other test passed. Keep a measured width per atom for the
  pass, as `Cx::ranges` does, and run `layout_nested_past_the_depth_bound_throws` after any
  change to measuring. See `extensions/pdf/src/layout/table.rs`. `until: gone extensions/pdf/src/layout/table.rs:self.ranges.get`
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
- **`--record-perf` writes the ledger only when its run ends, so a run that is killed keeps
  nothing.** One run over 1062 features takes about four hours, and a background command stops at
  two. Record with `xargs -n 100`, so each run of 100 writes its figures. See
  `tools/nv/proofs/perf.ts` `writeLedger`.
- **A stale perf figure refuses every goal's DONE, whoever made it stale.** The floor check
  `ci-check` runs `bun nv proofs --gate`, so a handoff that leaves another session's stale `arr.rs` figures to them gets its
  DONE refused. Before `DONE`, run `bun nv proofs --gate --limit 0` and record each class it names.
  A `calibration did not measure anything` line means the machine was busy: rerun that class alone.
  See `tools/nv/proofs/collect.ts`. `until: gone tools/nv/proofs/collect.ts:changed since it was last measured`
- **Run `bun nv render` before `--record-perf`, never after.** A `lang:` figure hashes its whole
  reference chapter, and the chapter takes in each example's `about.md`. An `about.md` edited
  before the render makes a figure measured before it stale again, so the class is measured twice.
  See `tools/nv/proofs/collect.ts`. `until: gone tools/nv/proofs/collect.ts:changed since it was last measured`
- **A "comments only" change still makes a feature's perf figure stale when it touches a string.**
  The figure's hash drops comments but keeps every string literal of the implementing file, and a
  reference chapter is hashed whole. A sweep that rewrites a message or a panic text there turns
  the proofs gate and the page stamp red at the full gate's push. In the same commit, run
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
- **`bun nv try` runs the `covws` build too, so a case can pass or crash for an old binary.** A new
  attack panicked on a hole the change on disk had already closed, and a subagent reported passes
  from the same stale build. Run it as `NVS_BIN=target/debug/nvs.exe bun nv try <files>` after
  `cargo build`. See `tools/nv/cmd/try.ts`. `until: gone tools/nv/cmd/try.ts:covwsNvs`
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

- **`allocation_policy.rs`'s `member(symbol)` allocates, because it builds the list of every
  registered symbol on each call.** A roster test that called it inside the counting window counted
  10 allocations per call, and the member under test spent none. Resolve every member before the
  window opens, or once in a `OnceLock`. See `crates/nvs-stdlib/tests/allocation_policy.rs`
  `properties_of`.
- **A directory of programs run by `nvs test <dir>` has the folder above `<dir>` as its program
  folder, because the snapshot's entry is the directory and `app::folder` takes its parent.** A
  `--since` case expected a file beside the directory to count as outside the program's folder,
  and it counted as inside. Put the directory one level down, as `lib/app/`. See
  `crates/nvs-cli/tests/since_selection.rs` `app`.
- **A new `.nvs` example, attack or bench is a formatter corpus file, and `nvs-fmt::identity`
  fails on every CI leg when it is not written as the formatter writes it.** Six `Core\Test\RunResult`
  proof files used `else if` and `{stdin: ...}`, and one push run went red on all four test legs.
  Run `bun nv nvs-fmt <paths>` on each new file before its commit. See `crates/nvs-fmt/tests/identity.rs`.
- **A test that starts `nvs serve` and reaches its reload endpoint fails only on macOS when its data
  folder's path is long.** The endpoint `run/serve-<pid>-<ms>.new` must fit 103 bytes there, and
  107 on Linux, so `nvsdata-<pid>-<n>/case/` passed on Linux and failed every macOS case. Use
  `nvs_repo::scratch_private_for_socket`, which checks the length. See
  `crates/nvs-cli/tests/reload_command.rs` `data_folder`.
- **Two helpers that build a scratch folder from a caller's short name collide when two tests pass
  the same name, and the tests run in parallel threads of one process.** `moded("plain")` and
  `linked("plain")` both made `completion-values-<pid>-plain`, and one deleted the other's folder
  partway: `create_dir_all` failed one run in a few. Put a process-wide counter in the folder name.
  See `crates/nvs-lsp/tests/completion_file_values.rs` `Workspace::new`.
- **An `.lspt` cursor in an unfinished statement inside a `{ }` body resolves to `Block`, and the
  coverage gate then owes a case for every other request at `Block`.** A completion case wrote
  `echo $tag-><|>` inside a `foreach` body, and `cargo test --test coverage` named nine empty cells.
  End the statement with `;`, so the cursor resolves to the member access. See
  `tests/lsp/completion/a-key-from-a-map-completes-its-class-methods.lspt`.
- **The cycle collector runs only near the request's memory ceiling, so a test of cycles under no
  ceiling grows with every cycle.** A test compared the peak of 200 and 2,000 map-and-node cycles and
  failed with code that was fine; a plain class grows the same way. Hold the run to a ceiling far
  below its cycles' total with `ctx.set_memory_limit` and assert it finishes. See
  `crates/nvs-codegen/tests/revocable.rs` `a_cycle_of_a_map_and_its_member_is_reclaimed_cleanly`.
- **A `--GET--` case fills `Core\Html::later` slots with no host, on the request's own context.** A
  slot there sees everything the main script sees, and under a real host it sees none of the request.
  `session-a-read-inside-later-or-after-response-compiles.nvst` passed while every session read in a
  served slot threw. Test what a slot reads with a routed case through `Core\Test::request`. See
  `crates/nvs-runtime/src/later.rs` `assemble`.
- **A `.lspt` case whose code does not compile can pass for the wrong reason.** A case wrote
  `$shop = new Shop();` with no declaration, so its calls resolved to nothing. Its `none` expectation
  passed, and the index listed no call. Run `nvs check` on a case's code before trusting a `none`.
  See `tests/lsp/actions/a-parameter-whose-callers-differ-is-offered-no-type.lspt`.
- **Examples in one folder share its `disk` and run at the same time.** An example that printed
  `Storage::list` of the whole disk also saw a file its sibling had just written, and `floor-debt`
  went red on a refused DONE. An example that writes to a shared disk reads back only its own keys.
  See `docs/examples/core/Storage/delete/03-remove-old-exports.nvs`.
- **`Unit::call_static` gives the callee the caller's reference to each argument.** A test that
  passes a string, object or array and then reads `Value::refcount` sees one fewer holder than it
  expected, and once the callee's copies are released it reads freed memory. Keep a second holder
  in compiled code, such as a static property the method that made the value also writes, as
  `compiled_code_and_the_runtime_agree_on_the_count_word` does. See
  `crates/nvs-codegen/tests/refcount_calls.rs`.
  `until: gone crates/nvs-codegen/tests/refcount_calls.rs:compiled_code_and_the_runtime_agree_on_the_count_word`
- **A test a goal's acceptance names must not be `#[cfg(unix)]`.** The loop's acceptance runs on
  the Windows host, where a Unix-gated test does not exist, and the check fails as "did not run and
  pass". A DONE was refused for that. Gate only the assertion that differs by platform, as
  `a_short_read_parks_the_next_read_on_edge_triggered_platforms` does (`crates/nvs-host/src/net.rs`).
- **An `assert!` in a serve case's client thread hangs the case.** The panic ends the thread before
  it calls `end_the_loop`, so `run_until_idle` waits on the accept loop until the run is killed, and
  the failure is never printed. Record each wrong answer, end the loop, and assert after
  `client.join()`, as `a_static_request_makes_no_file_call_on_its_core` does
  (`crates/nvs-server/src/serve.rs`).
- **A test task that waits in a loop of `suspend_current(Waiting::Yielded)` hangs `run_until_idle`.**
  A yielded task goes back on the run queue, so the queue never empties and the reactor never
  polls to wake the task it waits for. One such stream test ran for two hours with no output. Park
  the waiting task with a `RemoteWake` the other task drops, as
  `a_read_that_parks_pays_no_streak_check` does (`crates/nvs-host/src/net.rs`), and run a new test
  alone from its binary under `timeout 60` first.
  `until: gone crates/nvs-host/src/net.rs:fn a_read_that_parks_pays_no_streak_check`
- **A `#[cfg(test)]` helper above a module's `mod tests` cuts every source scan of that file
  short.** Many scans, the capability guard among them, read a file up to its first
  `#[cfg(test)]` line, so they never see the shipped code below such a helper. Put a test helper
  inside the test module, as `cache::tests::memory_held` is (`crates/nvs-stdlib/src/cache.rs`).
  `until: gone crates/nvs-stdlib/tests/capability.rs:line.trim_start() != "#[cfg(test)]"`
- **A use-after-free test on Windows needs about a thousand same-size allocations after the free.**
  The Windows heap does not hand a freed block back at once, so a fill of 16 strings left the freed
  property's bytes intact and the test passed with the fix switched off. Fill with 1024, never
  build the expected answer by concatenation (it can land in the freed block and match), and switch
  the fix off once to watch every row fail. See `crates/nvs-codegen/tests/borrowed_reads.rs`.
  `until: gone crates/nvs-codegen/tests/borrowed_reads.rs:fn statement`
- **The fill goes after the free, and the free is often when a callee returns.** A value a
  `PropertyGuard` or `toString` frees is still held by that call's own argument, so it is freed
  when the call returns. A fill inside the call runs first and the test passes with the fix off;
  fill in a later statement. A memory stop cannot reach a `new` with no constructor:
  `nvs_object_new` returns no status. See `crates/nvs-codegen/tests/borrowed_reads.rs`.
  `until: gone crates/nvs-codegen/tests/borrowed_reads.rs:fn a_property_guard_cannot_free_the_value_being_stored`
- **A `PropertyGuard` that stores to its own guarded property runs again, and the test dies of a
  stack overflow.** Give the guard a flag the program sets just before the store under test, and
  clear it first thing inside. See `crates/nvs-codegen/tests/borrowed_reads.rs`.
  `until: gone crates/nvs-codegen/tests/borrowed_reads.rs:fn a_guarded_element_store_keeps_its_key_and_array_alive`

- **A heavy `nvs serve` test answers `503` under a full nextest run, and passes alone.** Every
  server pins its workers to the first CPUs, so a test run's servers all share CPU 0. A starved
  worker misses the watchdog's five-second margin, is reported wedged (`the core is not turning`
  in its stderr), and admission sheds it. Set `[server] watchdog_margin` to the case's answer
  bound, and reproduce with copies of the test beside `yes > /dev/null` hogs in WSL. See
  `crates/nvs-cli/tests/serve_isolation.rs`. `until: gone crates/nvs-cli/tests/serve_isolation.rs:const WATCHDOG_MARGIN`

- **An lsp test that edits an open library file cannot see a stale result for that file's lender.**
  The open file's own analysis borrows the first lender's map and indexes that program's files again,
  so a stale result for that program files nothing, and the test passes with the check it guards
  turned off. Edit a method that a later program calls, and turn the check off once to see the test
  go red. See `crates/nvs-lsp/tests/incremental.rs`. `until: gone crates/nvs-lsp/tests/incremental.rs:an_edit_during_a_parallel_index`

- **A `.scale.nvs` ramp cannot guard the growth of native code.** It judges interpreted statement
  counts, so a bench that makes one native call per size reads `size flat` whatever the call costs,
  and no CI job runs callgrind. A growth property of a native path is guarded by a step count read
  in a test (`find_counted` in `crates/nvs-stdlib/src/str.rs`), or by a cost per item at two sizes
  measured in the same run. See `tools/nv/proofs/perf.ts`. `until: gone tools/nv/proofs/perf.ts:size_growth_clock`

- **A stopped `nvs serve` never force-unwinds a parked task, so a server test cannot reach a
  teardown crash in `Task::force_unwind`.** A core's serving loop turns until nothing is parked, and
  a task parked inside a script stands on a helper frame and is forgotten at the scheduler's drop.
  Such a test stays green with the fix undone. Reach the unwind through `nvs-host`'s
  `tests/forced_unwind.rs`, or a cancel of a task with no script on its stack, and see it red first
  with `native_frame` set to `inline(always)`. See `crates/nvs-cli/src/serve.rs`.
  `until: gone crates/nvs-cli/src/serve.rs:Ok(report) if report.parked > 0 => {}`
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
- **`ExprTypeTable::local_ty` finds no parameter of an arrow function.** Only a block body records
  its locals, so a lookup by the parameter's name falls back to the enclosing body and returns
  `None`. A lowering match that worked on `=> { return …; }` missed `=> expr`. Read a parameter's
  written type with `declared_ty(param.ty.span)`. See `crates/nvs-types/src/expr/calls.rs`
  `record_locals`.

## Splitting a file

## Writing Novis
- **A new `Core\Str` member needs its symbol in `str.rs`'s `address` match as well as its
  `CoreMethod` row.** The unit test calls the helper directly and passes, but every program then
  panics at registry build ("registers `…` with no implementation address"), so a suite run shows
  thousands of cases red at once. Add the `"nvs_core_str_…" => return Some(….entry())` arm beside
  its neighbour's. `until: gone crates/nvs-stdlib/src/lib.rs:with no implementation address`
- **The published corpus calls `Core\Test::answerHttp`, `answerSocket`, `tlsSession` and
  `answerCli` at the top of `nvs run` programs.** Hundreds of examples, attacks and benches answer
  their own outbound calls that way, so a goal item to make them throw "outside a test" broke them
  all. They throw only in work a server runs. Grep `docs/examples`, `tests/hostile` and `benches`
  before narrowing a `Core\Test` member. See `not_in_a_server` in `crates/nvs-stdlib/src/test.rs`.
- **A proof with a `.nvsr` beside it is a request, so `Core\Test::answerHttp` throws there.** Two
  `Novis-Rest-Session` examples went red only in `floor-debt`, after the goal's DONE. Build the value
  the call would return, or make the program `// proof: test` and fake the reply inside the request a
  `Core\Test::request` sends. See `docs/examples/package/Novis-Rest-Session/03-*.nvs`.
- **In a namespaced file, `Iterator<int>` names `<Namespace>\Iterator`, and `\Iterator<int>` is
  E0240.** A generator written into `apps/registry/src/App.nvs` failed E0441 and E0303 twice before
  it reached the compiler. Add `use Iterator;` under the `namespace` line, as
  `extensions/spreadsheet/nvs/Reader.nvs` does. See that file's head.
- **`Core\Script::arguments()`'s strings are `tainted`, and `as` cannot target a tainted type.**
  `as array<string>` fails `E0401` and `as array<tainted string>` fails `E0711`. Convert to
  `array<mixed>`, then each value `as string`, which gives a `tainted string` to declare as one.
  `docs/examples/config/schedule/02-work-that-runs-on-a-server-event.nvs` is the shape.
- **In a `jsonAs<{...}>` shape, `key: ?T` means the key must be there and may be `null`; a key a
  server may leave out is `key?: T`, read with `??`.** `expires_in: ?int` type-checks and then
  throws "2 value(s) of `$shape{...}` did not match" on a token reply without `refresh_token`.
  Comparing a `key?: T` field with `null` is E0466, so read it as `$reply->key ?? ''`. Namespaced
  code imports `RuntimeError` with `use RuntimeError;`, since `\RuntimeError` is E0240.
  See `packages/rest/src/Token.nvs`. `until: gone packages/rest/src/Token.nvs:refresh_token?: string`
- **A script under an app's `jobs/` runs on every `Core\Test::request` of `nvs test <app folder>`,
  so each request answers `500` with no body.** The folder's generated entry requires every file no
  other file requires, and the request runs that entry. It shows as "`Response::sendJson()`: the program
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
- **A route's `catch (RuntimeError)` turns a session error into an ordinary 400.** `apps/registry`'s
  sign-up failed as "the passkey could not be checked" when the session threw underneath. Two more
  there: rows a `Core\Test::request` writes outlive `#[Test(db:)]`'s rollback, because the request is
  its own run, and an arrow closure appending to an outer array appends to a copy. See
  `apps/registry/src/App.nvs`. `until: gone apps/registry/src/App.nvs:catch (RuntimeError`

## Divergences

- **For a CommonMark construct, check current cmark's source before you patch the engine to
  match comrak.** comrak copies cmark-gfm, which forked cmark before several fixes. A link title's
  `\"` (cmark issue 607) and `---` under a paragraph of definitions alone are two of them. One
  patch followed comrak there, and the next session reverted it. Read
  `gh api repos/commonmark/cmark/contents/src/<file> --jq .content | base64 -d`, and use
  cmark-gfm only for a GFM construct (decisions 17 and 19 in `docs/goals/markdown.md`).
  `until: gone fuzz/fuzz_targets/markdown_comrak.rs`
