The direction a sort puts things in: `Asc` for smallest first, `Desc` for largest first.

`Core\Arr::sort` and `Core\Arr::sortByKey` both take it as an option, and both put the smallest
first when you say nothing.

Smallest first means whatever smaller means for what you are sorting: lower numbers first, and
earlier letters first for text.

**Good to know:** the direction is a separate question from what is compared. A key function decides
what a sort looks at, and `Core\Order` decides which end of the result it comes out at — so you can
change one without touching the other.
