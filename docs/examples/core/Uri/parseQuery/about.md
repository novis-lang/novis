Reads a query string into an array. The query string is the part of a link after the `?`, such as
`q=red+shoes&page=2`.

Each pair gives one name and one value. Both are decoded, so `+` becomes a space and `%3A` becomes
`:`. Each value is `bytes`, because an escape can give any byte. Use `as string` to get text. When
a name appears twice, the last value is kept.

Brackets in a name build nested arrays. `size[]=M&size[]=L` gives a list with two values, and
`filter[color]=red` gives an array with the key `color`. `Core\Uri::buildQuery` writes the array
back as text.

**The examples below** read a search query, read lists and nested values, and read the page number
of a list.
