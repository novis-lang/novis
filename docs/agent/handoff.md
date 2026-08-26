# Handoff

## State

**Conformance is the only frontier left, at 512 of 600; the differential gate is met at 157 of the
150 it requires, and `python tools/gaps.py --differential` is down to one member.** Verify is green
(**1597** cargo tests, 74 suites, clippy and fmt clean) and runs both `.mwlt` trees itself, so after
a green `verify.py` there is nothing else to run (playbook, *Running things*).

**`Core\Json`'s two readers are pinned against their PHP twins.** `isValid` agrees with
`json_validate` over 57 grammar rows — bare scalars, commas, the number grammar, JavaScript's
spellings, surrogates, containers — counted as well as printed, and on both sides of the 512-level
depth bound reached four ways. `decode` round-trips through `Core\Json::encode` to the text
`json_decode`/`json_encode` produce, refuses exactly what `json_last_error` reports, and its
`maxDepth` counts `$depth`'s way on both sides of four bounds.

**Three number rows do not agree and are the next group's second slice**, with what each answers in
the new *Divergences* playbook bullet and in
`json-decode-matches-json_decode-and-json_last_error.mwlt`'s own header comment.

**`gaps.py --differential` dropped two members for one case** — the `isValid` oracle's
`json_decode` fallback silenced `Core\Json::decode` before it had a case. It has one now; the new
*Tooling* bullet owns the rule.

**`orient.py`'s rest-of-group bullets are still truncated mid-sentence**, so the two follow-on
slices had to be read back out of `handoff.md`. Its `[context] modules` manifest was right this
session — `json.rs` and `arr.rs` are both named.

## Next group

Three slices. The first closes the differential list and is `crates/mwl-stdlib/src/arr.rs` plus a
new file under `tests/differential/core/`; the second and third share
`crates/mwl-stdlib/src/json.rs` and the two cases just landed beside them.

- [ ] **`Core\Arr::flattenDeep` against `iterator_to_array`** (`crates/mwl-stdlib/src/arr.rs:2150`)
      — the one member `gaps.py --differential` still lists. The twin is
      `iterator_to_array(new RecursiveIteratorIterator(new RecursiveArrayIterator($a)), false)`;
      ADR 0069 § 3 refuses PHP's key renumbering, so a list subject should agree and a string-keyed
      one should diverge — run both before choosing `--ORACLE--` or `--ORACLE-DIVERGES--`, since
      that decides which of the two files it is.
- [ ] **The JSON number divergence, one `--ORACLE-DIVERGES--` case**
      (`crates/mwl-stdlib/src/json.rs:685`, `:989`, the writer at `:294`) — the three reader rows
      (`1e999`, `9223372036854775808`, `-0`) and the writer's whole-float rendering, all four
      spelled out in the *Divergences* playbook bullet this session added.
- [ ] **`Core\Json::isValid` and `Core\Json::decode` agree, as a conformance case**
      (`crates/mwl-stdlib/src/json.rs:987`) — `isValid`'s doc says it answers exactly what `decode`
      accepts, by doing it; that is the *agreement* shape over one table, counted, and the two
      tables to sweep are already written in the differential cases. No oracle: this is
      `tests/conformance/`.

## Backlog

- ADR 0088's qualifier classification is missing from every `mwl-stdlib` member row — plan, *Open now*.
- `Core\Json::decodeAs<T>` decodes scalar fields only — `mwl_stdlib::json` gap 2.
- ADR 0086 § 1's substitution table for the terminal sink — `crates/mwl-stdlib/src/cli.rs` gap 1.
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
- `do`/`while` is the one M4 control-flow statement that does not lower — `mwl-ir` gap 1.
- `orient.py` truncates its rest-of-group bullets mid-sentence — `tools/orient.py`.
