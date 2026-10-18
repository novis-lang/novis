These forms do not compile:

- `->name`, `->value` or any other property on a case is the error `E0405`. A write such as
  `$level->value = 1` is `E0405` too.
- A method call on a case, such as `Level::Info->label()`, is the error `E0405`.
- `Level::from($n)`, `Level::tryFrom($n)` and `Level::cases()` are the error `E0309`. So is a
  constant or a static property read on the enum, such as `Level::COUNT` or `Level::$x`.
- `extends` after an enum name is the error `E0220`.
- `abstract` or `final` before `enum` is a syntax error (`E0101`).
- `clone` on a case is the error `E0781`.

A case is not an object. Putting a case in an `object` variable is the error `E0401`, and
`$case is object` is `false`.

`Level::class` gives the full name of the enum. `Level::Info::class` is the error `E0702`.
