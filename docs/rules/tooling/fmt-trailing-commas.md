Every comma-separated list that spans more than one line gets a trailing comma after its last element:
call arguments, parameter lists, array literals, shape-type fields, anonymous objects, `match` arm lists
including the `default` arm, and multi-line enum-case lists. A list kept on one line never gets one.

The test that decides it is where the **closing delimiter** sits: the comma is there when a line break
separates the last element from that delimiter, and is not when the delimiter follows the element on the
element's own line. Those are the same question wherever a closer opens a line of its own, which is what
spreading a list over several lines ordinarily means. Where they come apart — an author who wrote
`foo(\n    $a, $b)` spread the list over two lines and still ended it on one — the comma would sit in
front of the `)` and buy none of the one-line diff this rule exists for, so it is not written. A comma
written there by hand is deleted for the same reason one is inserted: a canonical style has one spelling
of a list, not two.

Whether a list spans several lines is the author's decision, which `rule:tooling/fmt-never-reflows`
preserves; the comma follows from that decision mechanically. This removes the one place PHP's grammar
leaves a genuinely free stylistic choice with no way to derive the right answer from context, and it is
what makes adding an element to a multi-line list a one-line diff.
