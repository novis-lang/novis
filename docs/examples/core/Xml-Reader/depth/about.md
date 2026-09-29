Returns how deeply the node that `read` returned last is nested, as a whole number.

The root element has depth `0`. A node inside the root element has depth `1`, and each level adds
one. A comment or a processing instruction outside the root element also has depth `0`. Before the
first `read`, and after `read` returns `null`, the depth is `0`.

A reader does not return closing tags, so the depth is how you see where an element ends. When the
next node has the same depth as an element, or a smaller one, that element is closed.

**The examples below** print a document as an outline, show the depth before and after the walk,
and count the lines inside each order.
