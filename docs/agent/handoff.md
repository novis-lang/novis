# Handoff

## State

**`Core\Env`'s two depth cases are closed** — spec § 15's `get` answers a value whole, and an empty
value is a value rather than an absence. Two new `.nvst` cases and one doc paragraph on
`nvs_core_env_get`; no behaviour changed anywhere, so no new refcount edge and no valgrind run.
Conformance 1346, and `Core\Env` has left `gaps.py`'s thinnest twenty-five.

**A value has no syntax, and the case asserts that by counting twice.** A second `=`, a `;`, a pair of
quotes and leading padding all arrive as the bytes an operator set. The second count is over the four
readings a member *with* an opinion would answer instead — split at the second `=`, cut at the `;`,
unwrapped, trimmed — each of which is a prefix or an interior of the value beside it and so prints
plausibly on its own line. `crates/nvs-stdlib/src/env.rs:188`'s doc comment is the home of both laws.

**An empty-valued `--ENV--` entry survives the runner on Windows**, which is what makes the
empty/absent bound assertable on every leg: `NVS_CASE_EMPTY=` reads back as a length-0 `string` from
`get` and as a present key in `all`, not as an unset name. `case.rs`'s `--ENV--` parse trims each line
at its *end* only, so a leading-space value is expressible and a trailing-space one is not.

**The acceptance check still names `every_part_two_spec_member_is_registered`**, and no test by that
name is on disk in any crate. It is stage 10's gate over a complete Part II and needs spec §§ 15-19
from goal 6. Not a regression and not closable here. Nothing was missing from this session's pack.

## Next group

**`Core\Uuid` is `gaps.py`'s thinnest class that needs neither a fixture nor a capability — one file,
`crates/nvs-stdlib/src/uuid.rs`, plus `tests/conformance/core/`.** Its three thinnest members are
`tryParse` (3 cases), `parse` (5) and `v7` (5), and the two below are one file set with the third.
Spec § 11's second table is the section; `Core\Uuid` replaces `uniqid`, `com_create_guid` and every
userland library with one type, so the questions are about the *type*, not about a formatter.

- [ ] **`parse` and `tryParse` are one bound named on both sides** — every spelling `tryParse` answers
      a value for is one `parse` accepts, and every spelling it answers `null` for is one `parse`
      throws on, asserted by counting a sweep of near-miss spellings (a lost hyphen, an extra nibble, a
      non-hex digit, surrounding braces, upper case) rather than read off a line. The two are one
      reading of one grammar reached two ways, so a member that grew its own leniency fails here while
      still looking right alone. `crates/nvs-stdlib/src/uuid.rs:415`, `crates/nvs-stdlib/src/uuid.rs:443`.
- [ ] **A `v7` is time-ordered across a sweep, and that is the whole reason it is not `v4`** — a run of
      them compares in the order they were made, asserted by counting adjacent pairs rather than by
      printing any one of them, since none of the bytes is predictable. `crates/nvs-stdlib/src/uuid.rs:382`.
- [ ] **A `v7`'s version and variant nibbles are fixed whatever the clock says** — the sweep above,
      re-read as an invariant: every value carries version 7 and the RFC variant, so a generator that
      spent those bits on the timestamp fails on a count. `crates/nvs-stdlib/src/uuid.rs:382`.

## Backlog

- `Core\Math`'s `atan2`, `hypot` and `lcm` are the next thinnest pure class — spec § 3.
- `Core\Validate`'s `isAscii`, `isDomain` and `isEmail`, five cases each — spec § 12.
- `Core\Task::afterResponse` is the last differential gap, twin `fastcgi_finish_request` — `crates/nvs-stdlib/src/task.rs:561`.
- `Core\Env::get`'s non-UTF-8 throw is unasserted and unreachable from source — `crates/nvs-stdlib/src/env.rs:208`.
- `every_part_two_spec_member_is_registered` needs spec §§ 15-19, which are goal 6's — `docs/agent/loop-goal.toml` stage 10.
