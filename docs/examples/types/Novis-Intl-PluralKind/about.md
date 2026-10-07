Which plural rules `Icu::pluralCategories` uses: the rules for counting, or the rules for places in
an order.

`Cardinal` is for counting, as in "1 item" and "3 items". `Ordinal` is for places, as in "1st",
"2nd", "3rd" and "4th". A language has different rules for each. English has two forms for
counting, but four for places.

Most programs do not use this enum. `PluralRules::cardinal` and `PluralRules::ordinal` each use one
kind. `PluralKind` is for a program that calls `Icu::pluralCategories` directly.
