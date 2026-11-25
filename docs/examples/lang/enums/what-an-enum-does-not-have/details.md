These forms do not compile:

- `->name`, `->value` or any other property on a case is the error `E0405`. A write such as
  `$level->value = 1` is `E0405` too.
- A call of a method that the enum does not declare, such as `Level::Info->label()`, is the error
  `E0405`.
- `Level::from($n)`, `Level::tryFrom($n)` and `Level::cases()` are the error `E0309`. So is a
  constant that the enum does not declare, such as `Level::COUNT`, and a static property such as
  `Level::$x`.
- A property in an enum body is the error `E0220`.
- `implements` on an enum is the error `E0218`.
- `extends` after an enum name is a syntax error (`E0101`).
- `abstract` or `final` before `enum` is a syntax error (`E0101`).
- `clone` on a case is the error `E0781`.

A case is not an object. Putting a case in an `object` variable is the error `E0401`, and
`$case is object` is `false`.

`Level::class` gives the full name of the enum. `Level::Info::class` is the error `E0702`.
