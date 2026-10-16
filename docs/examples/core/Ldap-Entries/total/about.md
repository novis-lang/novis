Returns how many entries a sorted search found in total, when the search used the `window` option.

A search with `sort` and `window` returns one page of a sorted list. `offset` says how many
entries to skip, and `window` says how many to return. The server also counts the whole list, and
`total` returns that count. Use it to print "page 2 of 5" or "showing 11 to 20 of 48".

The count is the same for every page, and it does not change while the loop reads the entries.

**Good to know:** a search without `window` returns `null`. The server did not count the entries,
so you count them yourself in the loop.
