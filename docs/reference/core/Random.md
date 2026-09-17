---
summary: cryptographically secure random integers, floats, bytes, tokens and array draws
keywords: rand, mt_rand, random_int, lcg_value, mt_getrandmax, random_bytes, openssl_random_pseudo_bytes, bin2hex, array_rand, shuffle, str_shuffle, srand, mt_srand, Random\Randomizer, CSPRNG, nonce, session token
---

`Core\Random` is always a CSPRNG: there is no seeded or insecure generator under this name, and no
`srand`. `int` draws from a closed range and throws on an empty one rather than swapping the bounds;
`pick`, `sample` and `shuffle` draw from an array and answer values — a fresh array for `shuffle`,
`null` from `pick` on an empty one. `token` is the hex text of `bytes`, two characters per byte, for a
session identifier or a reset link.

Reproducibility is a separate **type**, `Core\Random\Seeded`, built from an explicit seed with `new`
and carrying the same seven members: one seed is one sequence, every run, which is what a simulation
or a fixture wants and what a session token must never be. Because it is a type rather than a mode,
a parameter declaring which generator it takes cannot be handed the other one by mistake.

```nvs
<?nvs
echo Core\Random::int(7, 7), "\n";
int $face = Core\Random::int(1, 6);
echo $face >= 1 && $face <= 6 ? "a die face" : "out of range", "\n";

array<string> $only = ["ace"];
echo Core\Random::pick($only), "\n";
array<int> $deck = [1, 2, 3, 4, 5];
echo Core\Arr::count(Core\Random::shuffle($deck)), "\n";
echo Core\Arr::count(Core\Random::sample($deck, 2)), "\n";

echo Core\Str::length(Core\Random::token(8)), "\n";
echo Core\Bytes::length(Core\Random::bytes(16)), "\n";
float $f = Core\Random::float();
echo $f >= 0.0 && $f < 1.0 ? "in [0, 1)" : "outside", "\n";

array<int> $none = [];
echo Core\Random::pick($none) == null ? "nothing to pick" : "picked", "\n";
try {
    Core\Random::int(5, 1);
} catch (RuntimeError $empty) {
    echo "empty range\n";
}

var $left = new Core\Random\Seeded(42);
var $right = new Core\Random\Seeded(42);
echo $left->token(4) == $right->token(4) ? "one seed, one sequence" : "diverged", "\n";
echo $left->int(1, 6) == $right->int(1, 6) ? "and it stays that way" : "diverged", "\n";
```
```output
7
a die face
ace
5
2
16
16
in [0, 1)
nothing to pick
empty range
one seed, one sequence
and it stays that way
```
