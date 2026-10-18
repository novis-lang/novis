`mixed` is the one type Novis does not check. A `mixed` variable can contain a value of any type.
What an operation on it does is decided while the program runs, from the value it contains.

Input from outside the program has the type `mixed`: a query string, a form post, the fields of
decoded JSON. Nothing has checked these values yet. Convert each one to the type you need with `as`.
If the value cannot be converted, `as` throws an error. `is` tests what the value is and does not
convert it.

A `mixed` value is never converted to a narrower type for you. Passing one to a `string` parameter
does not compile until you convert it with `as string`.

**Good to know:** an operation on a `mixed` value is slower than the same operation on a declared
type. Convert the value once, close to where it arrives, and use the typed value after that.

**The examples below** read a page number from a query string, describe each field of a decoded
payload, and convert an order line from outside. The last two use a `mixed` value directly, and show
a `mixed` argument to a `string` parameter as a compile error.
