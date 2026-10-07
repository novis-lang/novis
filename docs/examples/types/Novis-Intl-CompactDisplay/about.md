How `NumberFormat::compact` writes a large number in a short form.

You pass a `CompactDisplay` as `display`. `Short`, the default, writes "1.2K" and "3.4M" in English.
`Long` writes "1.2 thousand" and "3.4 million". Both round the number to a few digits, so use them
for counts a person reads at a glance, such as followers or views.

Each locale has its own short forms. German writes "1,2 Mio." for 1.2 million, and does not shorten
numbers below ten thousand in the `Short` form.
