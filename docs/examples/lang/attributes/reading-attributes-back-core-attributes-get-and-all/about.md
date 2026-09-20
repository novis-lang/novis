`Core\Attributes::get` and `Core\Attributes::all` read the attributes attached to a declaration. `get`
returns the one attribute that matches the shape you write, or `null` when nothing matches. `all`
returns every match, in the order the attributes are written, and an empty list when nothing matches.

You name the declaration with a first-class-callable reference: `Users::show(...)` for a method, and
`Users::constructor(...)` for the class itself. A second argument names one property or one parameter of
that declaration, and you then get the attributes on that one. This name must be written out. If you
write a variable there, the result is `null` or an empty list.

Matching is by shape and not by name. An attribute with more fields than the shape you write still
matches, so `{name: string}` matches `#[Tag(name: "alpha", weight: 3)]`.

Novis does all of this while it compiles your program. The call is replaced by the value it found, so
there is no lookup while the program runs.
