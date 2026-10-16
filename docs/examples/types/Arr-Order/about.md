The direction a sort puts things in: `Asc` for smallest first, `Desc` for largest first.

`Core\Arr::Order` is declared in the class `Core\Arr`. `Core\Arr::sort` and `Core\Arr::sortByKey`
both take it as an option, and both put the smallest first when you leave it out.

Smallest first means whatever smaller means for what you are sorting: lower numbers first, and
earlier letters first for text.

**Good to know:** the direction and what is compared are separate. A key function sets what a sort
compares. `Core\Arr::Order` sets the direction. You can change one without changing the other.
