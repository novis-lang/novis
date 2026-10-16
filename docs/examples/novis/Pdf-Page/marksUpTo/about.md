Returns the values of the marks with one name on this page and on every page before it.

A mark is an attribute on any HTML element, such as `data-mark="amount:12.50"`. `marksUpTo("amount")`
returns the values of every `amount` mark from page 1 to this page, in the order of the document.
Add them up to get a running total, which a long invoice shows at the bottom of each page.

The values are strings. Convert each one with `as ?decimal` before you add it, so the total is exact.

**The examples below** carry a running total in the footer of an invoice, show the total of the
earlier pages in the header, and count the items so far.
