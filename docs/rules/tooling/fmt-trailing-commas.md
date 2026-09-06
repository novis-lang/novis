Every comma-separated list that spans more than one line gets a trailing comma after its last element:
call arguments, parameter lists, array literals, shape-type fields, object literals, `match` arm lists
including the `default` arm, and multi-line enum-case lists. A list kept on one line never gets one.

Whether a list spans several lines is the author's decision, which `rule:tooling/fmt-never-reflows`
preserves; the comma follows from that decision mechanically. This removes the one place PHP's grammar
leaves a genuinely free stylistic choice with no way to derive the right answer from context, and it is
what makes adding an element to a multi-line list a one-line diff.
