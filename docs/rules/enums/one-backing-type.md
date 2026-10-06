`enum E: int` and `enum E: uint` name the backing type; with no `: T` written it is `int`. Nothing
else backs an enum — `enum E: string` is refused with **`E0219`**, and so is a case whose value is
not an integer literal.

That is the only shape. There is no *pure* enum: the "no backing value"
declaration is the auto-incrementing default nobody overrode (`rule:enums/declaration`), and a
program that only ever compares cases never has to look at the integer. There is no `string`
backing: a string-valued case would be a heap-allocated, interned buffer where a scalar fits the
value's existing payload for nothing (`rule:enums/representation`).

The two backings are told apart everywhere the type is. A `uint`-backed case converts with `as uint`
only, an `int`-backed one with `as int` only, and each refuses the other.
