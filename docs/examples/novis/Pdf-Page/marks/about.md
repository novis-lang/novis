Returns the values of the marks with one name on this page.

A mark is an attribute on any HTML element: `data-mark="amount:12.50"`. The part before the colon is
the name, and the part after it is the value. One element can have several marks, separated by `;`,
such as `data-mark="amount:12.50;tax:2.00"`. `marks("amount")` returns the values of every `amount`
mark on this page, in the order of the document.

**Good to know:** the values are strings. To add up amounts, convert each one with `as ?decimal`.
To get the values of this page and every page before it, use `marksUpTo`.

**The examples below** count the rows on each page, read two marks from one element, and add up
the amounts of each page.
