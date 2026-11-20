A written key type must accept the map's key type. A wider type, such as `object`, is allowed. Any
other type is the error `E0401`. The same is true for the value.

A key can be `tainted` (it may come from outside the program). Then `var $key` is `tainted` too. A
plain `string $key` over a `tainted string` key is the error `E0851`. A `tainted` value comes from
outside, so it cannot go into a plain `string`.

A key variable over an `Iterable` or an `Iterator` is the error `E0444`. These interfaces give values
and no keys.

`inout` on the value is the error `E0490`. To change a value, call `$map->set($key, $newValue)`.

`[]` does not work on a `Core\ObjectMap`. Read a value with `get` and write one with `set`.
