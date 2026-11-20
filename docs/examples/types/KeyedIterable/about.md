The interface a class implements so that a `foreach` loop can give a key and a value for each entry.

A `KeyedIterable<K, V>` has one method, `iterateKeyed`. It returns a `KeyedIterator<K, V>`, a cursor
that moves through the entries one at a time. You write the types when you declare the class, for
example `implements KeyedIterable<string, int>`. Every loop over the object then gets a `string` key
and an `int` value. A class without type arguments does not compile.

Write `foreach ($sheet as var $day => var $hours)`. The loop calls `iterateKeyed` once at the start,
so each loop gets a new cursor. You can walk the same object twice, and you can nest two loops over it.

A loop with only a value variable also works, and it gives the values. A class can also implement
`Iterable`. The method is called `iterateKeyed`, not `iterate`, so one class can have both methods.
Then a loop with one variable uses `iterate`. `Core\ObjectMap` is such a class: a loop with one
variable gives its keys.

**Good to know:** if the class does not have the `iterateKeyed` method, the error is `E0449`.
