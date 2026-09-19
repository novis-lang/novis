A `type` alias gives a long type a short name. Write `type UserId = uint;` and the two are the same
type everywhere after that — not a new kind of value, just another way of writing the one you already
have, so nothing needs converting in either direction.

The name goes next to the imports at the top of a file, or inside a class, interface or enum, where it
belongs to that owner and is reached through it. It may not be a second name for a single class: a
class already has one. What it is for is a shape that would otherwise be spelled out every time — a
union, an array of arrays, or the set of fields a method hands back.

**In plain words:** it is a nickname for a type, and only the compiler ever sees it. An alias costs
nothing while the program runs.
