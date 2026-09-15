---
summary: what a `decimal` reaches for where the operators cannot say it — two divisions that name their own rounding, the split whose parts add back exactly, and the exact power
keywords: decimal, division, divExact, divRound, allocate, penny split, remainder, pow, power, rounding, scale, money, currency, banker's rounding, half-even, bcmath, bcdiv, exact
---

Novis's `decimal` scalar carries its own arithmetic as operators — `+`, `-`, `*`, `%` and `/` all work
directly on it, exactly, with no class in the way. Division is the one operation that may be inexact, and
the `/` operator's answer to that is fixed in the language: round half to even, at the widest scale the
result admits. There is no `bcscale()` and never will be, because an ambient precision that unrelated later
code reads is a global by another name.

`Core\Decimal` is where a program says something the operator cannot. These questions come up in money
code, and nowhere else does the type system help with them:

- **"this division must come out even"** — `divExact` answers the quotient when it is exact and throws
  `ArithmeticError` when it is not. A third of a bill is not a number you should get back by accident.
- **"round it here, this way"** — `divRound` takes the number of places and the rounding mode as ordinary
  arguments, both required. Neither has a default, because a default for either would be that same ambient
  precision moved into a signature. The mode is `Core\RoundMode`, the same enum `Core\Math::round` takes.
- **"split this sum and lose none of it"** — `allocate` splits an amount by a list of ratios into parts
  that add back to it exactly. Each part is its share truncated to the amount's own scale, and the units
  left over go one each to the earliest parts. The keys are the ratios', so a split written under the
  names of its parties answers under those names.
- **"raise it to a power, exactly"** — `pow` multiplies a base by itself as many times as you ask. `**`
  has no row for a `decimal` base at all, because a power that does not come out exact would have to
  round without being asked; a negative power is written as the division it is.

`divRound` rounds **once**, from the exact quotient. It is not the `/` operator's answer rounded a second
time — rounding twice is how a quotient one digit past the place you asked for carries a tie the exact value
never had.

The scale you name is the scale you get: `divRound($x, $y, 5, …)` answers at five places even where fewer
would do, because a rendered price needs its trailing zeros. A scale past 28 is refused rather than quietly
narrowed, as is a quotient whose digits will not fit at the scale you asked for.

```nvs
<?nvs
decimal $bill = 100.00;
decimal $three = 3;
decimal $four = 4;

// An even split answers exactly, at the scale the quotient needs.
echo Core\Decimal::divExact($bill, $four), "\n";

// An uneven one refuses, rather than handing back 33.33... and letting the
// missing penny turn up in a reconciliation months later.
try {
    echo Core\Decimal::divExact($bill, $three);
} catch (ArithmeticError $uneven) {
    echo "not an even split\n";
}

// Where rounding is the business rule, it is written at the call site.
echo Core\Decimal::divRound($bill, $three, 2, Core\RoundMode::HalfUp), "\n";
echo Core\Decimal::divRound($bill, $three, 2, Core\RoundMode::Up), "\n";

// One eighth is 0.125 exactly, so two places is the tie the `Half*` modes are
// named for -- and the scale is the answer's, so it pads.
decimal $one = 1;
decimal $eight = 8;
echo Core\Decimal::divRound($one, $eight, 2, Core\RoundMode::HalfEven), "|",
     Core\Decimal::divRound($one, $eight, 2, Core\RoundMode::HalfUp), "|",
     Core\Decimal::divRound($one, $four, 5, Core\RoundMode::HalfEven), "\n";
```
```output
25
not an even split
33.33
33.34
0.12|0.13|0.25000
```

A split is the one place none of that is enough: three even shares of five cents do not exist, whatever
rounding you name, and the penny has to land somewhere. `allocate` lands it on the earliest part and
answers parts that add back to the amount exactly.

```nvs
<?nvs
// A five-cent bill split three ways. The parts cannot be equal, and they
// still add back to the amount.
decimal $nickel = 0.05;
foreach (Core\Decimal::allocate($nickel, [1, 1, 1]) as decimal $part) {
    echo $part, "\n";
}

// The keys are the ratios', so the parties keep their names -- and the
// leftover unit goes to the one written first.
array<decimal> $shares = ["alice" => 1, "bob" => 2];
foreach (Core\Decimal::allocate(10.00, $shares) as string $name => decimal $share) {
    echo $name, " ", $share, "\n";
}

// A power is repeated multiplication to the digit, scale included. The
// reciprocal is a division, so it names the rounding it needs.
decimal $rate = 1.05;
echo Core\Decimal::pow($rate, 2), "|",
     Core\Decimal::divRound(1, Core\Decimal::pow($rate, 2), 4, Core\RoundMode::HalfEven), "\n";
```
```output
0.02
0.02
0.01
alice 3.34
bob 6.66
1.1025|0.9070
```
