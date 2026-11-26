`keptCss()` returns the list of CSS rules that stay in the `<style>` element of the mail. The
other rules are copied into the elements. You call `keptCss()` on the result of
`Core\Mail::preview`.

A rule stays when a `style` attribute cannot do the same thing:

- an at-rule, such as `@media` for a small screen or `@font-face`,
- a rule with a pseudo-class or a pseudo-element, such as `a:hover` or `p::first-line`,
- a rule that matches no element of the HTML. `unusedCss()` lists these rules too.

The result has one string for each rule, in the order of the HTML. An empty list means that every
rule was copied into the elements.

Some mail programs read a `<style>` element and some do not. A rule in this list works only in a
mail program that reads it. Use `keptCss()` to see which part of your design depends on it. A test
can also check that the list is empty.

**The examples below** show a `@media` rule that stays, an `a:hover` rule that stays, and a test
that checks that every rule is copied.
