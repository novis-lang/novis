# Handoff

## State

**Conformance is the only frontier left, at 512 of 600; the differential gate is met at 155 of the
150 it requires.** Verify is green (**1597** cargo tests, 74 suites, clippy and fmt clean) and runs
both `.mwlt` trees itself, so after a green `verify.py` there is nothing else to run (playbook,
*Running things*) — in particular no release rebuild.

**`Core\Path` is closed on both counts and `Core\Math` is clear of the differential list.** The
trailing- and repeated-separator rule is now one case rather than six rows: `parse` (`path.rs:245`)
filters every empty component, so `/var/log`, `/var/log/`, `/var/log//` and `//var//log` are one
path, and `basename`, `dirname`, `extension`, `normalize`, `isAbsolute` and `split` are counted
*agreeing* about it, with four anchoring rows so the agreement cannot be vacuous.
`python tools/gaps.py --differential` is down to **three**, all named in *Next group*.

**The `gmp` extension is not installed in the `php` on `PATH`**, so `Core\Math::gcd`/`lcm` could not
be asked `gmp_gcd`/`gmp_lcm` and took a hand-written PHP Euclid as its oracle — the same shape the
`Core\Path::normalize` case uses, and it closes the gap because `gaps.py` looks for the MWL member's
call rather than the twin's. New playbook bullet under *Writing a test case*.

**`orient.py`'s `[context] modules` manifest is still wrong, seventh session running.** It names
`registry.rs`, `json.rs`, `arr.rs` and `regex.rs`; this session worked in
`crates/mwl-stdlib/src/path.rs` and `math.rs`, for neither of which the pack printed a map line. The
rest-of-group bullets are still truncated mid-sentence, so the full text had to be read back out of
`handoff.md`.

## Next group

Three slices, and they are the whole remaining differential list. All three add a file under
`tests/differential/core/`; the first two share `crates/mwl-stdlib/src/json.rs` and can share one
oracle prologue, the third is `crates/mwl-stdlib/src/arr.rs`.

- [ ] **`Core\Json::isValid` against `json_validate`** (`crates/mwl-stdlib/src/json.rs:989`) —
      `json_validate` is PHP 8.3+; the `php` on this host is **8.5.9**, so it is present, but the WSL
      leg's version is unchecked, so guard the oracle with `function_exists('json_validate')` and
      fall back to `json_decode` + `json_last_error() == JSON_ERROR_NONE`. Depth is the *edges*
      shape: depth limit, a bare scalar, a trailing comma, a lone `NaN`, an unpaired surrogate.
- [ ] **`Core\Json::decode` against `json_decode`/`json_last_error`** (`json.rs:685`) — same file,
      same prologue. Mind the playbook's *cannot index into an `array<mixed>`* trap: assert the whole
      structure with `Core\Json::encode($decoded)` rather than reaching past the first level.
- [ ] **`Core\Arr::flattenDeep` against `iterator_to_array`** (`crates/mwl-stdlib/src/arr.rs:2150`) —
      the last one. PHP's twin renumbers integer keys and keeps string ones (ADR 0069 § 3 refuses
      that), so decide per the plan's rule whether this is a `matches` case over a list or an
      `--ORACLE-DIVERGES--` over a map — likely both, as the rest of `Core\Arr` already is.

## Backlog

- ADR 0088's qualifier classification is missing from every `mwl-stdlib` member row — plan, *Open now*.
- `Core\Json::decodeAs<T>` decodes scalar fields only — `mwl_stdlib::json` gap 2.
- ADR 0086 § 1's substitution table for the terminal sink — `crates/mwl-stdlib/src/cli.rs` gap 1.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
- `[context] modules` in `docs/agent/loop-goal.toml` needs `path.rs`, `math.rs` and `str.rs` selectors.
- `do`/`while` does not lower, and a closure called through a variable panics `mwl-ir` — `mwl-ir` gap 1.
