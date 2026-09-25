---
milestone: post-parity
---
# Loop goal 69 — `Core\BigInt` — the arbitrary-magnitude integer class ADR 0054 promised beside the `decimal` scalar

`Core\BigInt` is registered: an immutable, heap-allocated, method-based integer of any magnitude — the
class ADR 0054 § 5 named as what replaces `gmp` and the integer half of `bcmath`: `bcpowmod`, `bcpow`
with a large exponent, a factorial past 2^63. It is `Stringable` and `Comparable`, converts to and from
`int`, `uint`, `decimal` and a `string` in a radix, and refuses with `ArithmeticError` where the
operators do. The `§13 Core\BigInt` key is struck from
`crates/nvs-stdlib/tests/spec-members-compiler-facing-outstanding.txt`, and
`rule:core-api/tier-roster`'s *designed, not shipped* paragraph no longer names big integers.

## Why here

The user's call of 2026-09-18, made while goal `gap-zero` held the run: the spec § 13 row
(`docs/spec/01-core-library.md:1012`) names the class beside `Core\Decimal`, `docs/plan/m8.md:52`
scoped the pair together, and M8 closed with the decimal half alone — so the key was `unowned`, and a
milestone tag would have named work already gone green. Nothing later is needed to build it: the
`decimal` scalar it converts to, `instance.rs`'s `Core`-owned object, and the `Comparable` and
`Stringable` descriptor fields are all landed. It runs after goal `test-doubles` — the larger of the
two, with a disjoint file set — and in front of `gap-zero`, whose ratchet half needs every key to name
a goal that has walked. `Core\BigDecimal`, which ADR 0054 § 6 also names, stays unscheduled: no spec
row and no key carries it.

## Stage 0 — the catch-up

`docs/rules/core-api/tier-roster.md`'s *Designed, not shipped* paragraph lists big integers among the
Tier 0 classes `registry.rs` does not hold: rewritten as a whole when the class registers, then
`python tools/rules.py --render`. `docs/spec/02-php-migration.md:363`'s `bcpowmod → Core\BigInt` row is
already right, and `:366`'s `bcsqrt → Core\Decimal` stays — `Decimal` is the human-magnitude answer.

## Stage 1 — the floor

Goal `test-doubles`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. It holds
every closure goal's checks. Never traded.

## Stage 2 — the keystone: the class and its representation

`crates/nvs-stdlib/src/bigint.rs`, registered in `registry::CLASSES` beside
`crates/nvs-stdlib/src/registry.rs:1530`'s `decimal::CLASS`, over the `num-bigint` crate — named by
ADR 0054 § 5, pure Rust, so `rule:packaging/a-c-dependency-answers-two-questions` is answered at its first
question. A value is a `Core`-owned instance under `crates/nvs-stdlib/src/instance.rs`'s first
decision: two slots, the sign as an `int` in {-1, 0, 1} and the magnitude as `bytes`, little-endian,
`BigUint::to_bytes_le`'s own form. Every member reads its operands back into a `BigUint`, computes, and
produces a new instance — values are immutable, as `decimal` is. The descriptor's `renderer` is the
decimal rendering and its `comparer` is `compareTo` (`crates/nvs-stdlib/src/instance.rs:185`), so
`echo`, `<=>`, `==` and `Core\Arr::sort` follow `rule:classes/stringable` and `rule:classes/comparable`
with no rule of their own. `crates/nvs-stdlib/src/decimal.rs` is the model for every row's shape, and
`crates/nvs-stdlib/src/math.rs`'s `gcd`/`lcm` for the two members that mirror `Core\Math`.

## Stage 3 — the roster

Every row under `docs/agent/conventions.md` § *A `Core` member — the five edits*. Construction:
`of(int $value)`, `ofUint(uint $value)`, `parse(string $text, {radix?: uint} $options)` — radix 2 to 36,
default 10, a text that is not an integer in that radix a `ParseError`. Arithmetic, each answering a new
`BigInt`: `add`, `sub`, `mul`, `div` (truncating, as `intdiv`), `mod` (the sign of the dividend, as `%`
on `int`), `pow(uint $exponent)`, `powMod(BigInt $exponent, BigInt $modulus)`, `sqrt` (the floor),
`gcd`, `lcm`, `neg`, `abs`, `shl(uint $bits)`, `shr(uint $bits)`. Reading: `sign(): int`,
`compareTo(BigInt $other): int`, `toInt(): int`, `toUint(): uint`, `toDecimal(): decimal`,
`toString(): string`, `format({radix: uint} $options): string`. Nothing else: bitwise `and`/`or`/`xor`,
primality and random generation are not on the row and are not this goal.

## Stage 4 — the proofs

One `examples/bigint.nvs` fixture with exact output — a factorial, a power, `powMod`, the `div`/`mod`
identity on a negative dividend, and `toInt` on both sides of the `int` bound. Conformance cases under
`tests/conformance/core/` in the depth shapes: `div` and `mod` **agreeing** with `intdiv` and `%` over a
sign sweep, by counting; `toInt` and `toDecimal` asserted on both sides of their bounds; `parse`
refusing at the edge of each radix; `compareTo`, `<=>` and `Core\Arr::sort` agreeing over a table.
The named guard tests the acceptance list carries. The key struck in the same edit that registers the
class.

## Standing decisions

- **`num-bigint` is the dependency.** ADR 0054 § 5's own naming, not a new choice; `num-traits` and
  `num-integer` come with it. Pinned the way `Cargo.lock` already pins every crate in the tree.
- **Representation**: sign plus little-endian magnitude bytes, two slots, re-materialised per member.
  What it spends: one object per result value plus one `bytes` copy per operand read, charged to the
  request and released with it; a `BigInt` is never on a served request's hot path by design, and if a
  program puts one there the cost is the program's. Fallback if a helper cannot write a `bytes` slot: a
  decimal-text `string` slot under the same contract, recorded in the module doc.
- **Immutable, and no operators.** `+` on two `BigInt`s is the compile error it is on any object; ADR
  0054's ground — Novis has no operator overloading — holds. `nvs convert` (M11) maps `bcpowmod` to
  `powMod`, which the migration row already says.
- **Refusals**: `ArithmeticError` for a zero divisor, a zero modulus, a negative `sqrt`, a `shl` past a
  bound the session sets and records, and every conversion out of range; `ParseError` for `parse`;
  `LogicError` for a radix outside 2 to 36. The same classes `decimal.rs` draws, for the same reasons.
- **The spec row stays as it is.** § 13's row names no member, and `Core\Decimal`'s half is registered
  against that same prose; the roster's home is `bigint.rs`'s module doc and each row's reference card.
- **No `Core\BigDecimal`.** ADR 0054 § 6 named it; no spec row and no key asks for it. Not this goal,
  and not a gap — a feature request the user has not made.
- **ADR slots**: none. ADR 0054 decided the class; the roster above is this goal's, recorded in the
  module doc.
- **Not this goal**: `test-doubles`; `gmp_*` rows in the migration table (the inventory holds none);
  the feature proofs beyond the cases above (goal `dossier`).
