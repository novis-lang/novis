`clone` keeps the class of the object. If a variable of type `Animal` has a `Dog`, its clone is a
`Dog` too. A `private` constructor does not stop `clone`, because no constructor runs.

An array property is copied, but an object inside the array is not. After a clone, both arrays
contain the same objects. A write to `$copy->items[0]->price` is seen through the original too.

`!=` on two objects is `true` when they are different objects. `==` on objects of two classes that
are not related is the error `E0466`. `==` with `null` tests whether a `?Note` variable has an
object.
