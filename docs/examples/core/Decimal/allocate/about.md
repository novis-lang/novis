Splits an amount into parts that add back to it exactly.

You give the sum and one ratio for each part. `Core\Decimal::allocate` returns one part per ratio, at
the same scale as the amount. A price written with two digits after the point is split into whole
cents. The parts always add back up to the amount, so no money is lost and none is invented.

Doing this by hand goes wrong. Three equal parts of `0.05` are `0.0166…` each. Rounded, that is
`0.02` three times, which adds up to `0.06`. This member gives each leftover unit to one of the
earliest parts instead. The keys of the ratios become the keys of the result, so a split written
under the names of the people it belongs to returns those names.

**Good to know:** position decides who gets a leftover unit, not weight. A ratio may not be negative,
and at least one ratio must be above zero.

**The examples below** show a bill split evenly, a payout split by named shares, and a discount
spread over the lines of an order.
