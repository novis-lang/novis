A class can check every write of its own properties before the value is stored. It implements the
interface `PropertyGuard`.

You write two methods. Novis calls `beforePropertySet` before every write of a property the class
declares, in the constructor too. It gets the name of the property, the new value and a list of keys,
`$keys`. A write of the whole property gives `[]`. A write to one element gives the keys of that
element, for example `["0", "1"]` for `$sheet->cells[0][1] = $v`. Novis calls `beforePropertyUnset`
before `unset` removes an element.

If one of these methods throws an error, nothing changes. The property keeps its old value, and no
`PropertyObserver` method runs.

**Good to know:** on a property with a `set` hook, the guard runs before the hook. It gets the value the
caller wrote.
