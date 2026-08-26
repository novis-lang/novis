# Handoff

## State

**Conformance is the only frontier left, at 511 of 600; the differential gate is met at 154 of the
150 it requires.** Verify is green (**1597** cargo tests, 74 suites, clippy and fmt clean) and
executes both `.mwlt` trees itself, so after a green `verify.py` there is nothing else to run
(playbook, *Running things*) — in particular no release rebuild.

**`Core\Path` is finished as far as the two counts can see it**: every § 8 member now has a
conformance case and the class has no differential gap left. `python tools/gaps.py --differential`
is down to five, all outside it — `Core\Math::gcd`/`lcm`, `Core\Json::decode`/`isValid` and
`Core\Arr::flattenDeep`. Two shapes landed this session and both are reusable: an oracle written as
a **hand-written second implementation in PHP** where the twin cannot be asked at all
(`realpath` stats the disk, so the oracle folds `.`/`..` itself, exactly as the RFC 3986 case does),
and § 8's separator rule asserted as **two counted invariants** — acceptance is one grammar on every
platform, emission follows the host — so a member that grew its own parse fails the count while
still looking right on its own row. `==` compares two `array<string>` and two `?string` results
directly, which is what let the acceptance sweep ask `split` and `relativeTo` alongside the rest.

**`orient.py`'s `[context] modules` manifest is still wrong, sixth session running.** It names
`registry.rs`, `json.rs`, `arr.rs` and `regex.rs`; this session worked entirely in
`crates/mwl-stdlib/src/path.rs` and `str.rs`, for neither of which the pack printed a map line. The
pack also still truncates the rest-of-group bullets mid-sentence, so the full text had to be read
back out of `handoff.md`.

## Next group

Three slices. The first shares `crates/mwl-stdlib/src/path.rs` and `tests/conformance/core/` with
the two that just landed; the last two are the next differential frontier and share
`tests/differential/core/`, one stdlib module each.

- [ ] **The trailing-separator rule asked of every member that shares it** (conventions.md's
      *Agreement* shape) — `basename`, `dirname`, `split` (`path.rs:621`) and `normalize`
      (`path.rs:664`) all route through `parse` (`path.rs:245`), which drops an empty component;
      assert that they **agree** about `/var/log/` and `/var/log//` rather than what each answered.
      Count into an `int` declared above the loop, as
      `tests/conformance/core/path-accepts-both-separators-and-emits-only-one.mwlt` does.
- [ ] **`Core\Math::gcd` and `lcm` against `gmp_gcd`/`gmp_lcm`** (`math.rs:945`, `math.rs:960`) — one
      `tests/differential/` case for both, since `lcm` is defined through `gcd`. **Run
      `php -m | grep gmp` first**: the Windows `php` on `PATH` is missing `mbstring` already, so if
      `ext-gmp` is absent too the oracle is a hand-written Euclid in PHP, which is the same shape as
      `path-normalize-matches-a-hand-written-lexical-fold.mwlt`. Sweep the boundaries: zero on either
      side, one, two coprimes, a pair where `lcm` would overflow a narrower type.
- [ ] **`Core\Json::isValid` against `json_validate`** (`json.rs:989`) — `json_validate` is PHP 8.3+,
      so check `php -v` first and fall back to `json_decode` + `json_last_error() === JSON_ERROR_NONE`,
      which is the twin the spec's **Replaces** column names beside it. Sweep the shapes that decide
      it: a bare scalar, a trailing comma, a lone `NaN`, depth past the limit, invalid UTF-8.

## Backlog

- `Core\Json::decode` against `json_decode` (`json.rs:685`) — pairs with `isValid` above, plan *Open now*.
- `Core\Arr::flattenDeep` against `iterator_to_array` (`arr.rs:2150`) — the last differential gap.
- `gaps.py --errors` holds nothing a case can take: 57 `Fault::fatal` and one unreachable `thrown` (plan *Open now*).
- ADR 0088's qualifier classification is missing from every `mwl-stdlib` member row (`mwl_stdlib::hash` module doc).
- `Core\Json::decodeAs<T>` reads a scalar-fielded class only (ADR 0071, `mwl_stdlib::json` gap 2).
- `docs/spec/02-php-migration.md` is 31% classified (`python tools/check-migration.py`).
