`Core\Attributes::get` and `Core\Attributes::all` read the attributes attached to a declaration. You
write the name of an attribute declaration between `<` and `>`, as in `get<Tag>`. `get` returns the
one `Tag` on the declaration, or `null` when it has none. `all` returns every `Tag`, in the order they
are written, and an empty list when there is none.

You name the declaration with a method reference: `Users::show` for a method, and
`Users::constructor` for the class itself. A second argument names one property or one parameter of
that declaration, and you then get the attributes on that one. This name must be written out. If you
write a variable there, the result is `null` or an empty list.

The result has the fields of the declaration. Write `var` for the variable that receives it, as in
`var $tag = Core\Attributes::get<Tag>(Users::show);`.

Novis does all of this while it compiles your program. The call is replaced by the value it found, so
there is no lookup while the program runs.
