`|>` binds **tighter than every binary operator and looser than unary**. The four cases that fixes,
each of which a looser placement gets wrong:

| written | groups as |
|---|---|
| `-$a \|> Math::abs($_)` | `Math::abs(-$a)` |
| `"x=" . $a \|> Str::upper($_)` | `"x=" . Str::upper($a)` |
| `$a \|> Str::length($_) > 5` | `Str::length($a) > 5` |
| `$x = $a \|> Str::trim($_)` | `$x = Str::trim($a)` |

Substituting a single hole commutes with the surrounding operator, so `$a |> Str::upper($_) . "!"`
groups as `Str::upper($a) . "!"` and the parenthesis trap a callable-applying pipeline has cannot
arise.
