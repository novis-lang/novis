Whether `RelativeTime::format` always writes a number, or uses a word such as "yesterday" where the
language has one.

You pass a `Numeric` as `numeric`. `Always`, the default, writes "1 day ago", "in 0 days" and
"in 1 day". `Auto` writes "yesterday", "today" and "tomorrow" for the same counts. For counts that
have no word of their own, both write the number.

`Auto` reads more naturally on a page. `Always` gives the same pattern for every count, which is
easier to compare in a list.
