A `foreach` loop over a `Core\ObjectMap` can give you each key with its value. Write
`foreach ($map as var $key => var $value)`. `$key` gets the map's key type, and `$value` gets its
value type.

The loop gives the entries in the order you added them. It walks the map as it was when the loop
started. If the loop body adds an entry, the loop does not visit it. If the body removes an entry, the
loop still visits it, with its old value. When the loop ends, the map has every change the body made.

A class of your own can give keys and values too. It implements `KeyedIterable<K, V>`, and its
`iterateKeyed` method returns a `KeyedIterator<K, V>`. A loop with one variable over a map gives only
the keys.

**The examples below** show a loop with written types, then a loop with `var`, then a class of your
own.
