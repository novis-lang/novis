`unusedCss()` returns the CSS rules of the mail that match no element of its HTML.
`Core\Mail::preview` returns the `Core\Mail\Preview` it is a method of.

A rule matches nothing for one of two reasons. Its selector has a typo, such as `.buton` for the
class `button`. Or it is for an attribute that a mail program adds later. For example, Outlook adds
`data-ogsc` to the elements in dark mode. Such a rule stays in the `<style>` element, and it still
works in that mail program.

The list has only rules that could match an element. An at-rule such as `@media`, and a rule with
a `:` outside square brackets, such as `a:hover`, are never in it. `keptCss()` returns them.

The result has one string for each rule, in the order of the HTML. Each rule in it is also in
`keptCss()`.

Use it in a test of a mail template. An empty list means that each rule matched an element.

**The examples below** show a rule with a typo, a rule for an attribute that Outlook adds, and a
test of a template.
