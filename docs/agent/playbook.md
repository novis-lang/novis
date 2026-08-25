# Playbook — the traps this repository has actually sprung

Hard-won specifics: things that cost a real session real time, written down so the next one does not
pay again. **This file is append-mostly.** Add a bullet when something bites you; edit one when it stops
being true; delete one when the underlying gap is closed. Never rewrite it wholesale, and never reword a
bullet to say the same thing differently — the churn is the cost this file exists to avoid.

It lived inside `handoff.md` until it was two thirds of that file, which meant every session regenerated
about 2.4k tokens of stable lore as if it were state, and reworded it a little each time. `handoff.md` is
*state* and is overwritten every session; this is *knowledge* and outlives all of them.

**Scope.** A rule that binds every agent goes in [AGENTS.md](../../AGENTS.md). A decision with reasoning
goes in an ADR. How a subsystem works goes in that crate's own module doc comment. The *shape* of
something you are about to write — a commit message, a `.mwlt` case, a `Core` member, an ADR — is
[conventions.md](conventions.md). What is left — "this looks like it should work and does not, and here
is why" — is this file.

## Tooling

- **`orient.py` prints less than `brief.py` on purpose, and the gap is a bug in the goal, not in the
  tool.** If it did not print a module, an ADR section, a convention shape or a playbook section you turned
  out to need, do not conclude the orientation is broken and re-run `brief.py` for everything — fetch the
  one thing, and say in the handoff which `[context]` field in `loop-goal.toml` was missing its selector.
  A manifest that nobody corrects becomes a manifest every session works around, which costs more than the
  wide orientation it replaced.
- **A whole ADR is about 7,000 tokens; one of its `###` sections is about 1,000.** `sed -n` between the
  heading and the next one, not `cat`. The same goes for a 1,100-line module: `grep -n` for the anchor
  first. This is the largest single line item in `loop-stats.py --attribute` every time it is measured.
- **Another session may be writing this tree right now, and `ls` will not tell you.** Two sessions once
  reached for the same ADR number on the same day: an ADR referenced `0084` and `0085` by name before those
  files existed, and the second author only noticed because `git status` showed them untracked. **Claim a
  number with `git status --short docs/adr/` and not with `ls` or `brief.py` alone**, immediately before
  creating the file. The same applies to committing: `git commit -a` sweeps in whatever the other session
  has in flight, which is not wrong — the tree is only internally consistent with all of it — but the
  commit message then describes half of what it contains, so say so in the message rather than letting
  `git log` imply one author.
- **`D:` fills up.** `target/debug` reached 33 GB and `cargo test` failed as a wall of `link.exe` 1180/1318
  errors — the real message (`no space on device`) only appears without a `Select-String` filter. `cargo
  clean` frees it in seconds; the rebuild is a few minutes. Check `Get-PSDrive D` before diagnosing a
  linker failure.
- **The Bash tool eats a backslash inside a heredoc**, and an em dash or apostrophe in one can defeat an
  exact-match splice — a `\\` written in a `python - <<'PY'` heredoc arrives as `\`, and a `"\n"` arrives
  as a real newline, so a block containing either will silently fail to match. Use the Write/Edit tools, or
  `python tools/splice.py <target> --patch <file>` with the patch **written by the Write tool**
  ([conventions.md](conventions.md) has the format). This is the rule sessions break most: reaching for
  `cat > f <<'EOF'` to save a call is how the mangling gets in. `splice.py` matches the anchor **exactly**,
  trailing newline included — the Write tool ends a file with one, so strip it when splicing
  mid-paragraph, and use `--dry-run` if you are unsure the anchor is still current. A Rust string holding
  a `Core\Name` label needs `r"..."`, or the backslash is an unknown escape.
- **`cargo test` does not always relink `target/debug/mwl.exe`** — `cargo build -p mwl-cli` before running
  a fixture or a `.mwlt` case by hand, or a stale binary reports a member you just registered as `mixed`.
- **`wsl.exe` needs PowerShell** and a **script file**; an inline `bash -lc "…"` mangles, and WSL's
  default shell has no `grep`/`sed` on `PATH` from a bare `bash -c`. Whole suite:
  `wsl.exe -- bash /mnt/<drive>/<repo>/tools/wsl-acceptance.sh` (background it; minutes). One fixture:
  `tools/leak-check.sh <paths>` — it takes `.mwl` files only, so a `.mwlt` passed to it reports a failure
  that is not a leak.
- **Another agent may be editing this repo at the same time.** Check the ADR directory for the next free
  number immediately before writing one, stage your own paths explicitly, check `git show --stat` after
  committing, and re-read a shared doc immediately before rewriting it. `tools/brief.py` prints a loud
  banner when `.loop/running` exists — if it does, stop and tell the user rather than editing alongside it.
- **A second writer can replace a file you created *this session*, and the only notice is a "changed on
  disk" line.** A `Core` slice run twice concurrently produced two whole implementations of the same new
  module and two conformance cases for it; the second Write silently won. Do not revert — the tree is only
  internally consistent with the newer one — but *re-read the file before every later edit*, delete the
  duplicate case rather than shipping both (a padded conformance count is worse than a missing one), and
  say in the commit message that it carries two authors. The `git status --short` you run before staging is
  where a duplicate shows up, and it is the only place.
- **After touching either spec file, `python tools/check-migration.py`**; after moving or renaming any doc,
  `python tools/check-links.py` — broken *and* mis-cased relative links.
- **A moved module takes its `insta` snapshots with it** — they resolve relative to the module's own file.
- **A `Core\Name` inside a `python - <<'PY'` heredoc is a Python escape error, not just a Bash one.**
  The heredoc bullet above is about the Bash tool eating a backslash; this is the second half of the
  same trap and it fails differently — `"""… Core\Uri …"""` reaches Python intact and *Python* then
  rejects `\U` as a truncated `\UXXXXXXXX`, so a patch script that quotes any `Core\U…`/`Core\N…`
  path dies at parse time with nothing about the real edit in the message. Use the Edit tool for a
  targeted replacement, or `splice.py` with a Write-tool patch file.
- **A test failing in a file your slice never touched is probably inherited, not caused.** A red
  `-p mwl-stdlib --lib` arrived this session from the previous one's `validate.rs`, and the first
  instinct — "my registry rows broke something" — costs a bisect. `git status --short` showing that
  file unmodified is the whole diagnosis. Fix it, but in **its own commit**, so `git log` does not
  read as though the feature slice touched it.
- **Never `plan.py --get "Open now"` just to check an edit landed.** That field is one logical line of
  about 10,000 tokens, and `--get` unwraps and prints the whole of it — five per cent of a session's
  ceiling to confirm a sentence you already wrote. `grep -n` the phrase in `docs/implementation-plan.md`
  instead; the surrounding `> `-prefixed lines are the same fact for a hundredth of the cost.
- **A plan status field is *one* logical line, and three renderings of it disagree.** `plan.py --get`
  hands back the unwrapped line, `orient.py` re-wraps it again, and on disk it is a `> `-prefixed
  blockquote wrapped at ~100 columns. So an anchor copied out of either *rendering* never matches the
  file: to change one sentence with the Edit tool, `grep -o` the phrase in `docs/implementation-plan.md`
  and copy the `> `-prefixed lines around it. `plan.py --set FIELD --from <file>` rewrites the whole
  field from a file written with the Write tool, which is the other way and the only one for a field-wide
  change.
- **A `**Bold phrase:**` inside a field body silently becomes an eighth status field.** `plan.py` finds
  fields with `^> \*\*([^*:]+):\*\*`, and it re-wraps what you `--set`, so a bolded lead that ends in a
  colon and happens to land at the start of a wrapped line is parsed as a new field name on the next read —
  which AGENTS.md's fixed field set forbids. Write `**Bold phrase** —` instead; the colon outside the
  asterisks is invisible to the regex. The symptom is `python tools/plan.py` listing eight fields, and the
  fix is `git checkout -- docs/implementation-plan.md` followed by the `--set` again.
- **Making a type `pub` owes it a `#[derive(Debug)]`.** The workspace denies `missing_debug_implementations`,
  so promoting a private struct to the public API compiles and then fails at clippy — step 3 of four, after
  the tests have already run. Add the derive in the same edit as the `pub`, not after `verify.py` says so.
- **`/tmp` is not the same directory to Bash and to Python here.** A file written by `>` in the Bash tool is
  invisible to a `python -` heredoc in the same call, which resolves `/tmp` to `%TEMP%`. Stage a scratch
  file under `.agent-tmp/` — both halves agree on a repo-relative path.
- **`mwl-ir` cannot name `mwl_hir::QName`** — `mwl-hir` is a *dev*-dependency there, on purpose, so a
  lowering helper that wants one in its signature does not compile even though `mwl_types::Ty::Enum`
  hands it a `&QName` to pattern-match. Destructure it at the call site and pass what the callee actually
  needs (an `&EnumInfo`, or the name already rendered with `to_string`); `mwl_types` re-exports the enum
  and layout tables but not `QName`, and adding the dependency to get one is the wrong direction.
- `python`, not `python3`. `gen` is reserved in Rust 2024. `cargo insta test --accept -p <crate>`; a renamed
  test needs its old `.snap` deleted. `cargo test --release -p mwl-abi-probe` takes over two minutes.

## Running things

- **Verification is one call:** `python tools/verify.py` — build, test, clippy and fmt in order, stopping at
  the first failure, ~10 lines when green. `-p <crate>` scopes it, `--fast` drops clippy and fmt for a
  mid-work check, and every step's full output lands in `.agent-tmp/verify-<step>.log` either way.
- **The whole acceptance test in one command:** `python tools/loop.py --goal-only` (both legs plus the
  valgrind sweep, naming the first failure), or `--list` to see it without running it.
- **One case, quickly:** `mwl test tests/conformance/core/str-case-members.mwlt`, or
  `mwl test tests/ --filter str-` over the tree.
- **A scratch `.mwl` under `.agent-tmp/` run with `mwl run` is the fastest way to find out whether a shape
  lowers**, and is worth doing before writing a batch of cases around it. A scratch file is top-level
  statements, like `examples/*.mwl` — there is no `Main::main` entry point, and a `for` header takes
  *expressions* only, so the loop variable is declared on the line above it.
- **`mwl run` printing the right output and exiting **127** is a heap corruption at teardown**, not a
  missing command: Windows reports a double release that way, with nothing on stderr. So check `$?` on a
  scratch run rather than reading the output and moving on — a refcount bug is otherwise completely silent
  until the WSL valgrind leg catches it. `try { … } catch (Throwable $e) { … $e->message … }` **does**
  lower at file scope now, which is what `tests/conformance/core/time-datetime-is-a-civil-time-in-a-zone.mwlt`
  and both `Date`/`TimeOfDay` cases write; the class-method shape
  `tests/conformance/lang/a-lossy-conversion-throws.mwlt` uses is no longer needed for that.
- **A leak whose "definitely lost" size is `16 + strlen(a literal in the probe)` is a temporary abandoned
  on a throwing edge, and the allocating stack carrying no `mwl_` frame at all is the confirmation.** That
  is the whole recognition test, and it is worth knowing because bisecting to it costs an hour. Most of
  those are now closed: a call's arguments and receiver, and the operands of `.`, an interpolation and an
  `echo`, all go on `mwl_ir::lower::Lowering`'s owned-temporaries stack and are released on both edges. A
  probe that `catch`es a throw from a `Core` member taking a `string` is therefore *green* now and is a
  fair leak check. What is still open is narrower and named in that field's own doc comment (an argument
  being **transferred** when a later one throws) plus the producers that still release inline — a
  normalized subscript key, a `match` subject.

## Adding a `Core` member

- **A new `Core` member owes four things**, and the third is the one that bites: the registry row, the
  `mwl_helper!` body, an arm in that module's own `address()` (a miss is a *runtime* panic naming the
  symbol, not a link error), and a `.mwlt` case that calls it — `tests/conformance_coverage.rs` fails
  `cargo test -p mwl-stdlib` without one. An instance member is covered by a case writing `->name(`.
  [conventions.md](conventions.md) writes all four out; `python tools/brief.py`'s *anchors* block
  resolves each spelling to a file and line.
- **A spec §§ 1-12 member owes a *fifth* thing: striking its line from
  `crates/mwl-stdlib/tests/spec-members-outstanding.txt`.** That file is the outstanding-member ratchet
  `tests/spec_registry_coverage.rs` reads, and the test fails on a **stale** line — one naming a member
  that is registered now — exactly as loudly as on an unregistered member the file does not list. So the
  failure you see after landing a member is not a regression; it is the list telling you it did not
  shrink. Its keys are `§<section> <the spec's own Member-cell spelling>`, which is why `§1 chunk` and
  `§2 chunk` are two different lines.
- **A `Core` instance's slots hold only values MWL already holds, so a member wanting native mutable
  state has to accumulate instead** — there is no destructor to free a `sha2::Sha256` context with, and
  `digest 0.10` cannot serialize one into a slot. The COW-correct read/write of a slot that holds an
  array is `identity_store::borrow`/`edit`/`replace`, which are generic over `(receiver, index, class,
  member)` despite that module being named for § 9's store; `instance::set_slot` is the raw write and
  `instance::slot` the borrowed read. `Core\Hash\Stream` is the worked example.
- **A new domain module is `mod`, not `pub mod`, so its `CLASS`/`NAME` are `pub(crate)`.** The
  workspace warns `unreachable_pub`, and half of `mwl-stdlib`'s modules are `pub mod` while the newer
  half is not — copying `uuid.rs`'s `pub const NAME` into a privately-declared module is a warning at
  build time, before `verify.py` says anything. `objmap.rs:36` is the shape to copy.
- **A `Core` symbol that is not a member breaks `every_registered_member_has_an_implementation_address`.**
  `mwl_stdlib::symbols()` used to be exactly one entry per `CLASSES` member, and that test asserts the
  count — so a constructor symbol from `registry::CONSTRUCTORS`, or anything else chained in beside the
  members, has to be added to the sum on the test's right-hand side in the same edit. The failure is a
  bare `left: 213, right: 211` in `-p mwl-stdlib --lib`, with nothing naming the symbol.
- **A `CoreTy::Array(&CoreTy::Uint)` parameter receives `Tag::Int` elements**, so a helper that reads
  each one through `as_uint` alone answers the member's most obvious call site with a fatal. A written
  `[97, 98]` type-checks against `array<uint>` and stays int-tagged all the way into the helper — a
  scalar `uint` parameter does not have this problem, because the call site materializes the literal at
  the declared type. Read both tags (`str.rs`'s `code_point`), and probe the literal spelling in a
  scratch `.mwl` before writing the case.
- **A registry row's arity and its helper's `args: [N]` are two numbers that must agree**, and an
  options bag flattens to one argument per option — so `round(float, {precision, mode})` is
  `args: [3]`. A **variadic tail is one argument**, whatever the call writes. **An instance member's
  receiver is argument slot 0 and is not in `params`**, so `plus(Duration)` is `args: [2]`. A mismatch is
  an index-out-of-bounds panic at the first call.
- **A member on `registry::WRITTEN_CLASS_MEMBERS` takes one argument its row does not declare** — the
  class its call site wrote, in slot 0 — so its helper's `args: [N]` is `params` + 1 (+ the options bag's
  flattening). `tests/conformance_coverage.rs` looks for such a member spelled `Class::name<`, not
  `Class::name(`, because that is what every call site writes.
- **Registering a `Core` class narrows `Core`'s blanket trust for that name.** An unregistered
  `Core\X::y()` is waved through by `mwl_hir::members`; once `X` is in `registry::CLASSES`, an unknown
  member on it is a diagnostic. So adding a class can turn a fixture that "compiled" into one that
  reports — which is the point, but check the fixtures that name it.
- **A new dependency owes three things**: a `[workspace.dependencies]` line with a comment saying why that
  crate, `cargo deny check`, and `python tools/gen-attribution.py` (ADR 0065 — the notice is committed). A
  license identifier new to the tree must be added to **both** `deny.toml`'s allow list and
  `tools/gen-attribution.py`'s `PREFERENCE`, in the same commit: that script fails if the two disagree.
  The dependency *sweep* is a pass the user fires by hand (ADR 0068); never start it as a side effect.
- **A registry rule quoted by test name may not be that rule, or may not exist.** A handoff opened
  ADR 0069's slice with "`registry`'s own `a_union_is_only_ever_a_parameter` says a union cannot be a
  return type", which would have forced `array<mixed>` on four members. There is no test by that name —
  the real one is `a_union_option_excludes_null`, which restricts an **option's** type and
  nothing else, and `CoreTy::Union`'s doc says "legal in **either** direction" outright. One
  `grep -n 'fn [a-z_]*(' registry.rs` over the test names costs one call and settles it; designing around
  a constraint that is not there costs a member's whole surface.

## Writing a test case

- **`var` and a written type are two different declarations, and `var T $x = …` is neither.**
  `var $x = …` infers; a declared type is `T $x = …;` with no `var` at all — so
  `var Core\Regex\Pattern $p = …` is four diagnostics (`E0101` three times and then `E0301` for a name
  that was never declared), none of which says "drop the `var`". The trap is that the *inferring*
  spelling is the one every case reaches for, so the typed one looks like it should take a keyword too.
- **An array literal written *directly* as a `Core` argument infers `array<mixed>` and is refused.**
  `Core\Arr::replaceRange($a, 1, 2, ["X"])` is `E0401: expected array<string>, found array<mixed>` — the
  parameter's type is not pushed into the literal, whether the parameter is `array<T>` or concrete. Declare
  a typed local one line above and pass it, which is what every existing case already does; a scratch probe
  written the obvious way fails at the checker before it ever reaches the member you are testing.
- **A case cannot index into an `array<mixed>`'s elements, and `Core\Json::encode` is the way round
  it.** `$q["b"] as array<string>` panics `mwl-ir` outright — *"got `Tagged as Array`"*, ADR 0007 § 2's
  `array<T> as array<U>` row being the one still missing — so a member answering a nested shape has no
  spelling that reaches past the first level. `Core\Json::encode($q)` renders the whole structure in one
  line and it is byte-identical to PHP's `json_encode` over `parse_str`'s array, which makes it the
  strongest assertion available as well as the only one. `$q["a"] as string` on a top-level scalar does
  lower.
- **A `?array<T>` cannot be indexed even after a `!= null` guard** — `mwl-ir` panics outright at
  `crates/mwl-ir/src/lower/expr.rs:2952`, *"has no resolved element type recorded … its base erased to
  `mixed`"*. The narrowing itself works for a `?string`, so the hole is specifically that a narrowed
  nullable **array** loses its element type, and `Core\Arr::first`/`last` over an `array<array<string>>`
  is where a case meets it. Three spellings do lower and are the way round it: `$rows["0"]["name"]`
  (nested indexing, no nullable in the path), `foreach ($rows as array<string> $row)` — the binding's
  declared type is what re-supplies the element type — and binding `var $row = $rows["0"];` first.
- **Registering a `Core` member and writing its conformance case are one slice, not two.**
  `mwl-stdlib`'s `tests/conformance_coverage.rs` fails the moment a registry row has no `.mwlt` case
  calling it, so a plan that lands the rows in one session and the cases in another leaves the tree red
  in between — and `verify.py` reports it as a `-p mwl-stdlib` test failure with nothing about the
  member in the message. A class constant counts too: `Core\Path::SEPARATOR` needs a case that writes it.
- **A multi-file `.mwlt` case works now — but only the entry file's statements run.**
  `--FILE <relative/path>--` repeats and writes another file into the case's working directory
  (`crates/mwl-test`'s module doc), and `mwl-cli`'s `front_end` resolves, checks and lowers the whole
  `require`/`autoload` graph, so a class declared in a second file is reachable from `mwl run`. What a
  second file contributes is its *declarations*: a bare `echo` at its file scope compiles and prints
  nothing (`mwl-ir` gap 22). So a case pins the second file by *using* what it declares, never by what it
  echoes on its own.
- **A `--EXPECTF-ERROR--` case must not also *use* what the broken declaration would have provided.**
  Diagnostics are ordered by phase, not by file, so an `E0303` from the entry point's reference is printed
  *before* the resolution error the case exists to pin, and the block no longer matches at its first line.
  A compile-error case's entry file should do the least that reaches the diagnostic — often a bare
  `require` and nothing else. Between two diagnostics `%A` covers the span, notes included.
- **A rule added to `mwl_syntax::check_declarations` reaches far less of the corpus than a grep
  suggests.** Only `mwl-cli` and `mwl_hir::requires` call that walk, so every `mwl-types` fixture, every
  parser test and every `mwl-codegen` fixture goes straight past it — ADR 0094's estimated "sixty inline
  snippets to rewrite" turned out to be eleven, all in `casing.rs`'s own tests. Grep for the *callers*
  before budgeting a corpus rewrite; a `<?mwl` snippet in a Rust string is not automatically subject to
  everything the compiler enforces.
- **A row the checker accepts is not a row that runs.** `mwl-codegen` refuses a binary operator over two
  representations with *"does not lower a binary operator over mismatched representations"*. Equality is
  out of that hole — `$n == $f` is `Helper::NumericEq` now — but `$n + $f` and `$n < $f` still type-check
  and still fail there, so a conformance case written straight off an ADR's compiling rows can fail at run
  time. Run the rows in a scratch `.agent-tmp/*.mwl` before writing the case; if one does not lower, pin it
  in the crate's own `tests/` and say in the case comment why it is not here.
- **`"…" as bytes` is how a case writes a `bytes` it can read, and `Core\Encoding::fromHex("…")` is how
  it writes one it cannot.** There is no `bytes` literal at all (`00-overview` § 5), so those are the two
  spellings; `as bytes` is total and free and only reaches octets that are valid UTF-8, which is why an
  arbitrary buffer — a lone `ff`, a truncated sequence — still has to come from `fromHex`. Assert the
  result with `toHex` either way, since `echo` has no `bytes` row: ADR 0009 § 3 makes `bytes as string`
  *checked*, and an implicit render is not that check.
- **`emit_binop`'s ordering rows are `Int | Uint | Bool` only, so `<`/`<=`/`>`/`>=` over two `string`s
  does not lower** — and the checker does not stop you, because `operators.rs`'s result table models only
  `int`/`uint`/`float` operands and falls back to `mixed` for everything else. A `.mwlt` case that
  compares two strings for order therefore compiles and then fails at run time. Assert a fixed slice with
  `==`, or a shape with `Core\Regex::matches`, and pin the ordering in the crate's own `#[test]`, which
  can also sleep. Watch the neighbouring trap too: a `Core` member answering `uint`
  (`Core\Str::length`) in `$int + …` is `E0407`, not a widening.
- **`mwl-codegen` has no `BinOp` row for `Ty::Enum` at all**, matched pair or not: `emit_binop`'s
  `integral` set is `Int | Uint | Bool`, so an `Eq` a lowering emits over two enum values fails with
  *"a `Eq` over representation Enum(Int)"*. Compare one representation down — `InstKind::Reinterpret` to
  the backing integer is free, and it is the row `$m as int` already uses.
- **Never put `--ORACLE--` in a `tests/conformance/` case** — CI runs that suite on all three hosted
  runners and none of them has PHP, so an oracle section makes the runner *skip the whole case* there,
  subtracting from the very count Stage 4 measures. Verify against PHP while authoring — `php -r '…'` is
  enough to settle a semantics question, and it is on `PATH` under Windows *and* inside the WSL distro
  ([docs/setup.md](../setup.md)) — then drop the section or put the case in
  `tests/differential/`, which is where an oracle belongs. The `.mwlt` format
  is `crates/mwl-test`'s module doc; a `--EXPECTF-ERROR--` block must reproduce the diagnostic's own
  indentation, which widens with the line number. A trailing space before a `\n` is unreliable in an
  `--EXPECT--` block — echo a sentinel character after it.
- **The `php` on Windows `PATH` has no `mbstring`**, so every `mb_*` oracle a `Core\Str` slice reaches
  for — `mb_convert_case`, `mb_strtolower`, `mb_str_split` — dies with *"Call to undefined function"*
  rather than answering. The byte-wise half (`strcmp`, `strnatcmp`, `substr_count`, `str_replace`) is
  all there, so a member replacing both spellings can still be checked on its ASCII rows. For the
  Unicode rows, cite the UCD table the member implements (folding is UAX #44's `C`+`F` mappings) and
  say so in the case's comment; do not silently assert whatever the implementation printed. WSL's
  `php` may have the extension — worth one `php -m | grep mbstring` before writing the rows off.
- **A `mwl-types` test that asserts an interned type's `describe` string is fragile.** A union orders its
  members by type id, so registering a member anywhere can flip `T|null` to `null|T`. Compare against
  `interner.make_union([...])` instead.
- **`MwlStr::from_raw`/`MwlArray::from_raw` return an *owning* handle.** Reading a refcount through one in
  a unit test releases a reference when it drops — wrap it in `std::mem::ManuallyDrop`, or the test ends in
  a heap corruption rather than an assertion failure. `crate::arr::borrowed` is that wrapper for an
  argument, and `crate::instance::slot` is the borrowed read of an object's slot.
- **`Core\Path` emits a platform separator**, so a fixture or case asserting a built path must normalize it
  (`Core\Str::replace($p, Core\Path::SEPARATOR, "/")`) — otherwise it passes one leg and fails the other.
  A `Core\Time` case has the same hazard in a different place: never assert `Zone::system()`'s answer, and
  never assert a wall-clock value.
- **Clippy refuses a float literal that approximates π or e**, and refuses `assert!` over two constants —
  a compile-time invariant belongs in `const _: () = assert!(…);`, not a `#[test]`.

## Splitting a file that got too big

- **The mechanism is two lines, and it is a pure move.** Rust lets one inherent `impl` and one set of free
  functions live in several modules of the same crate: each child starts with `use super::*;` (which
  reaches the parent's private imports *and* its siblings' names once `mod.rs` globs them back), and every
  item that crosses a seam becomes `pub(super)` — the reach it had as a private item of one file. A child
  can also see the parent's private items, so plumbing stays private in `mod.rs`.
- **A `pub(crate)` item needs an explicit `pub(crate) use` in `mod.rs`** or `crate::thing::name` stops
  resolving for the rest of the crate. A glob `use self::child::*;` covers the in-directory names; the
  re-export list covers the crate-facing ones, and the two coexist.
- **Cut by *entity*, never by line number, and check the count afterwards.** A range that starts one line
  late leaves a `#[test]` attached to the previous item — which is a *silent* lost test, not an error,
  unless the function happens to take arguments. `grep -c '#\[test\]'` before and after, and the test count
  in `verify.py`'s output, are the two checks that catch it.
- **A moved test module also *renames* its snapshots**, on top of the *Tooling* bullet above: the file
  name is the test's module path, so `parser::tests::foo` becoming `parser::tests::stmt::foo` needs the
  `.snap` moved *and* its `source:` line updated. Do that by hand instead of accepting the `.new`, and the
  diff stays a rename rather than a delete plus an unreviewable add.
- **A big file hides doc comments attached to the wrong item.** Two of `mwl-types`' were 120 lines from the
  function they described, invisible in a 3.5k-line file and obvious the moment it became eight. When a
  carve leaves a doc block stranded above an unrelated item, that is a bug the split found, not one it
  made.
- **Header prose splits with the code.** A module doc that grew a paragraph per ADR slice *is* the split
  plan: each paragraph already names the rule it belongs to. What is left in `mod.rs` afterwards is its
  charter — see AGENTS.md's length-target table for why the charter is the part that matters.

## Writing MWL itself

**The traps that cost the most time are not gaps.** A `"%1$s"` template must be written in **single**
quotes or the `$s` interpolates; `as` binds tighter than every binary operator *and* than unary minus,
so write `($a > $b) as string` and `(0 - 3) as ?uint`; a duration literal used as a receiver needs
parentheses (`(30s)->toSeconds()`); a `foreach` binding declares a type (`as int $i`);
`Core\Str::length` answers `uint`, so a running total it feeds must be one too, and `?? 0` against a
`?uint` needs `?? 0 as uint` to stay one; `bool as string` is PHP's `""`/`"1"`; a bare array literal in
a `foreach` head types as `mixed`, and `var` refuses one outright; a `foreach` key binding must be
declared `string` even over a list; there is no int-to-float widening, so `Math::sqrt(2)` is a
diagnostic; a `catch` binding is function-scoped **until this loop re-scopes it** (pre-authorized), so
two clauses on one `try` need two different variable names; `Exception` is spelled `Core\Error` in the
spec's own prose but the tree's root is `Throwable`, a caught value's text is `$e->message` and not a
getter, and a typed `catch` on a `Core`-owned class does not lower yet — a `catch` on a spec § 10 class
(`ParseError`, `LogicError`, …) now does. **Inside a `namespace X;` every name resolves relative to it**,
PHP's rule exactly, so `Core\Str::upper` in a namespaced file is `X\Core\Str` and takes `E0303` plus a
knock-on `E0403` on the method's declared return; write `\Core\Str`, which does resolve, and reach a
sibling in the same namespace unqualified.
- **Adding a dependency costs a whole `target/` generation, and cargo never collects the old one.** A
  crate's artifacts are named `<name>-<metadata-hash>` and that hash covers the dependency graph, so a
  `Cargo.toml` or `Cargo.lock` edit orphans the previous set for every crate downstream — permanently, on
  stable. Editing *source* is free: a source-only rebuild reuses every hash and adds nothing. On a
  milestone that adds a crate most sessions, that is a generation most sessions, and it is how this tree
  reached 20 GB and zero free disk on 2026-08-25. You do not need to do anything about it in a session —
  the driver refuses to *start* a run under 10 GB free and says what to run — but if you are the one who
  hits it, `python tools/disk.py --clean` is the answer, not `cargo clean`.
