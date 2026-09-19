`.` joins two pieces of text into one, and `.=` adds a piece to the end of text you already have.
Both read each side as text first, so `"Order " . 1042` gives `Order 1042` with no cast written
anywhere.

Numbers all have a text form, and so do `true`, `false` and `null`: `true` becomes `1`, while
`false` and `null` become nothing at all. An object has a text form when its class implements
`Stringable`, and then it is that method's answer that gets joined in.

Four things have no text form, and using one here is refused while the program is compiled: a
`bytes` value, an array, an enum case, and a call that returns nothing. Every one of those refusals
names what to write instead, because the choice is yours to make — which encoding the octets are in,
which fields of the array matter, what an enum case should be called.

Text in double quotes fills in values too: `"Hello $name"` and `"{$order->total}"` read the same way
a join does.

**The examples below** take these in turn: joining text and numbers into one message, growing a line
a piece at a time with `.=`, and rendering the confirmation an order sends out.
