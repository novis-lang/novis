# Handoff

## State

**Conformance is the only frontier left, at 510 of 600; the differential gate is met at 153 of the
150 it requires.** Verify is green (**1597** cargo tests, 74 suites, clippy and fmt clean) and
executes both `.mwlt` trees itself, so after a green `verify.py` there is nothing else to run
(playbook, *Running things*) — in particular no release rebuild.

**`Core\Path::basename` and `Core\Path::dirname` now have oracle cases**, so `python tools/gaps.py
--differential` is down to six members with a PHP twin and no case: `Core\Path::normalize`,
`Core\Math::gcd`/`lcm`, `Core\Json::decode`/`isValid` and `Core\Arr::flattenDeep`. Both new cases
sweep the boundaries the member is written around rather than adding rows — a trailing separator
(one and several), a bare name, the root, the empty subject, `.`/`..`, and the option that is PHP's
second argument on both sides. Neither pins a *divergence*: `dirname('')` answering `.` and
`{levels: 0}` answering the path itself are PHP-unaskable and stay pinned in
`tests/conformance/core/path-decomposes-a-path-without-touching-the-disk.mwlt` and
`path-normalize-has-a-normal-form-and-join-never-replaces-its-base.mwlt`, which the new cases point
at instead of restating. A `Core\Path` oracle case has to normalize its own answer
(`Core\Str::replace($p, Core\Path::SEPARATOR, "/")`) *and* the oracle's (`str_replace("\\", "/",
…)`), and must never spell a `\` in a subject: MWL parses both separators on every platform where
PHP parses `\` on Windows only, so a backslash row would pass one leg and fail the other.

**`orient.py`'s `[context] modules` manifest is still wrong, fifth session running.** It names
`registry.rs`, `json.rs`, `arr.rs` and `regex.rs`; this session worked entirely in
`crates/mwl-stdlib/src/path.rs`, which the pack printed no map line for, and the next group is in
the same file. The pack also still truncates the rest-of-group bullets mid-sentence.

## Next group

Three slices, all in the same file set: `crates/mwl-stdlib/src/path.rs`, `tests/differential/core/`
and `tests/conformance/core/`. The first is the last differential gap `Core\Path` has.

- [ ] **`Core\Path::normalize` against PHP's `realpath`** (`path.rs:664`, its `resolved` helper at
      `path.rs:@resolved`) — a `tests/differential/` oracle case, but `realpath` touches the disk and
      answers `false` for a path that does not exist, so the oracle side is a **hand-written** fold
      of `.`/`..` over `explode("/", …)`, exactly as
      `tests/differential/core/uri-compare-to-matches-a-hand-written-rfc-3986-normalization.mwlt`
      already does for RFC 3986. Sweep: a `..` past the root, a `..` in a relative path (which
      survives), a repeated and a trailing separator, `.` alone.
- [ ] **The separator invariant, counted rather than read off a line** — a `tests/conformance/core/`
      case asserting that every `Core\Path` member accepts `/` and `\` alike and emits
      `Core\Path::SEPARATOR` (`path.rs:175` for the emission rule, `path.rs:221` `is_separator` for
      the acceptance one, module doc lines 11-24 for why only the emission is platform-dependent).
      Count agreements over an `array<string>` of subjects into an `int`; a case that reads one row
      off a line passes on one leg only.
- [ ] **The trailing-separator rule asked of every member that shares it** (conventions.md's
      *Agreement* shape) — `basename`, `dirname`, `split` (`path.rs:621`) and `normalize` all route
      through `parse` (`path.rs:245`), which drops an empty component; assert that they **agree**
      about `/var/log/` and `/var/log//` rather than what each answered.

## Backlog

- `Core\Math::gcd` and `lcm` in one oracle case — `crates/mwl-stdlib/src/math.rs:945` and `:960`.
- `Core\Json::decode` and `isValid` oracle cases — `crates/mwl-stdlib/src/json.rs:685` and `:989`.
- `Core\Arr::flattenDeep` against `iterator_to_array` — `crates/mwl-stdlib/src/arr.rs:2150`.
- `[context] modules` in `docs/agent/loop-goal.toml` misses every file the last five sessions
  touched, `path.rs` included.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
