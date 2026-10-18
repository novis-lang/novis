A default value is not a write, so it does not call `onPropertySet`. A `static` property calls
neither method.

`onPropertySet` runs after the value is stored. If it throws an error, the write throws that error,
and the property already has the new value. `onPropertyGet` runs after the value is read, and an
error it throws comes out of the read.

A read inside a string, such as `"v=$o->x"`, calls `onPropertyGet`. So does `isset($o->x)`. A write
of one element, such as `$a->rows[] = 1`, calls `onPropertySet` once with the whole array.
`unset($a->rows[0])` does the same. Neither one calls `onPropertyGet`.

For a property with a `get` hook, `onPropertyGet` gets the value the hook returned. A property the
hook reads calls `onPropertyGet` too, before it. A read inside `onPropertyGet` of another property
calls `onPropertyGet` again, for that property.

`$value` has the type `mixed`. Text you build from it, such as `$value as string`, is a
`tainted string`, so `string $s = $value as string;` is the error `E0401`.
