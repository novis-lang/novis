Joins the items of a list into one string, with the words a language uses, such as "Shop, Blog, and
Wiki".

`ListFormat::join` takes a list of lists and returns one string for each list, in the same order. The
locale decides the words and the commas. In English, the list `["Shop", "Blog", "Wiki"]` gives "Shop,
Blog, and Wiki". In German, it gives "Shop, Blog und Wiki". A list with one item gives that item, and
an empty list gives `""`.

The options change the joining word. `type: ListType::Or` joins with "or", as in "Shop, Blog, or
Wiki". `type: ListType::Unit` joins measurements, such as "3 kg, 200 g", with no "and". The default is
`ListType::And`. `width: Width::Short` and `Width::Narrow` use shorter forms where the language has
them. In English, `Width::Short` gives "Shop, Blog, & Wiki".

`ListFormat::join` throws a `LogicError` when the locale tag is not valid.

**Good to know:** the method joins the strings as they are. It does not translate or change the
items.
