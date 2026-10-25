A field name is a plain name with no quotes. `{"x": 1}` is the error `E0101`, and so is
`{...$p, y: 2}`, because there is no spread for an object. A comma after the last field is allowed,
and `{}` is an object with no fields. A field name written twice is the error `E0494`.

At the start of a statement and directly after the `=>` of an anonymous function, `{` starts a
block. An anonymous object there is the error `E0117`. Write it in parentheses: `({ x: 1 });` and
`fn(int $x): Point => ({ x: $x, y: 0 })`.

The type of a field does not change. `$p->x = "a";` on `{ x: 1 }` is the error `E0401`. A
field with another anonymous object in it, or a function, is allowed. Call that function as
`($p->f)(1)`. `$p->f(1)` is the error `E0477`, because an anonymous object has no methods.

Reading a field the object does not have compiles, and throws a `RuntimeError` when it runs:
`` `$shape{x}` has no field `y` ``. A field written `b?: int` is optional. Reading it when it is not
there throws the same error, so read it with `$r->b ?? 0`. `isset($r->b)` is `false` when it is not
there.

An object of a class fits a shape when it has public properties with those names and types.
`$v is Point` checks every field and its type when the program runs.
`$v as Point` runs the same check. If the check fails, it throws a `RuntimeError`.

`clone $p` gives a new object with the same fields, so `$p == clone $p` is `false`.
`Core\Json::encode($p)` writes the fields sorted by name. `echo $p` compiles, and throws a
`RuntimeError` when it runs, because an anonymous object has no string form.
