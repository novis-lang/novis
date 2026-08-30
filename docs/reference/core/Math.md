---
summary: numeric functions over `int`, `float` and `decimal` — magnitude, rounding, integer division, roots, logarithms, trigonometry, base conversion and number formatting, plus the constants
keywords: abs, min, max, ceil, floor, round, intdiv, fmod, sqrt, pow, hypot, exp, log, log10, log2, sin, cos, tan, asin, acos, atan, atan2, sinh, cosh, tanh, deg2rad, rad2deg, is_nan, is_finite, is_infinite, decbin, dechex, decoct, bindec, hexdec, octdec, base_convert, number_format, gmp_gcd, gmp_lcm, M_PI, M_E, PHP_INT_MAX, PHP_INT_MIN, PHP_FLOAT_EPSILON, PHP_FLOAT_MAX, NAN, INF, PHP_ROUND_HALF_UP, PHP_ROUND_HALF_EVEN
---

`Core\Math` is the numeric function set; the arithmetic operators `+ - * / % **` and `<=>` are the
language's and have no member here. `abs`, `sign`, `min`, `max` and `clamp` take an `int`, a
`float` or a `decimal` and answer in the argument's own type; the rounding, root, exponential,
logarithmic and trigonometric members take and answer `float`; `intDiv`, `gcd`, `lcm`, `toBase` and
`fromBase` work over `int`. `round` names its tie rule as a `Core\RoundMode` case and defaults to
`HalfUp` — and answers a `float`, so `round(6.0, {decimals: 2})` prints `6`; a number shown with a
fixed count of decimals is `format`'s job, not `round`'s. `format` groups digits only when asked,
since there is no locale. A refusal throws —
`fromBase` on a digit outside the base, `format` on an infinity — never answers `false`. The
constants are class constants: `Core\Math::PI`, `INT_MAX`, `EPSILON`, `NAN`, `INFINITY`.

```nvs
<?nvs
echo Core\Math::abs(-7), " ", Core\Math::abs(-2.5), " ", Core\Math::sign(-3), "\n";
echo Core\Math::max(3, 9), " ", Core\Math::clamp(15, 0, 10), "\n";
echo Core\Math::round(2.5), " ", Core\Math::round(1.2345, {precision: 2}), " ",
     Core\Math::round(2.5, {mode: Core\RoundMode::HalfEven}), "\n";
echo Core\Math::floor(1.8), " ", Core\Math::ceil(1.2), " ", Core\Math::truncate(-1.7), "\n";
echo Core\Math::intDiv(7, 2), " ", 7 % 2, " ", Core\Math::mod(7.5, 2.0), "\n";
echo Core\Math::sqrt(16.0), " ", Core\Math::hypot(3.0, 4.0), " ", Core\Math::log(100.0, {base: 10.0}), "\n";
echo Core\Math::round(Core\Math::sin(Core\Math::PI / 2), {precision: 3}), " ", Core\Math::toDegrees(Core\Math::PI), "\n";
echo Core\Math::toBase(255, 16), " ", Core\Math::fromBase("ff", 16), " ", Core\Math::toBase(5, 2), "\n";
echo Core\Math::format(1234567.891, {decimals: 2, groupSeparator: ","}), "\n";
echo Core\Math::gcd(12, 18), " ", Core\Math::lcm(4, 6), "\n";
echo Core\Math::isNan(Core\Math::NAN) ? "nan" : "number", " ", Core\Math::INT_MAX, "\n";
```
```output
7 2.5 -1
9 10
3 1.23 2
1 2 -1
3 1 1.5
4 5 2
1 180
ff 255 101
1,234,567.89
6 12
nan 9223372036854775807
```
